use std::{collections::BTreeSet, error::Error, fmt};

use unicode_normalization::UnicodeNormalization;

use crate::{ScanResult, scan_text, special};

use super::{
    model::{
        CleanProfile, CleanRequest, CleanResult, CleanScanSummary, ContentPreservationDiff,
        ContentPreservationStatus, RuleId, TechnicalDiff, TextRange, Transformation,
        TransformationManifest, VerificationChecks, VerificationReport, VerificationStatus,
    },
    registry::{profile_definition, profile_uses_rule, rule_definition, tcs_version},
};

/// Controlled error returned when a clean request cannot be executed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CleanError {
    /// The profile is reserved but has no behavior in the pasted-text phase.
    UnsupportedProfile,
    /// A deterministic internal manifest invariant was not satisfied.
    InvariantViolation,
}

impl fmt::Display for CleanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedProfile => formatter.write_str(
                "The selected TextScope clean profile is not supported for pasted text.",
            ),
            Self::InvariantViolation => {
                formatter.write_str("The TextScope clean manifest failed an internal invariant.")
            }
        }
    }
}

impl Error for CleanError {}

/// Cleans pasted text with one explicit, versioned TCS profile.
///
/// The input is copied into the result but never mutated. The clean output is
/// independently rescanned before the verification report is constructed.
pub fn clean_text(text: &str, profile: CleanProfile) -> Result<CleanResult, CleanError> {
    clean(CleanRequest { text, profile })
}

fn clean(request: CleanRequest<'_>) -> Result<CleanResult, CleanError> {
    if !profile_definition(request.profile).supported {
        return Err(CleanError::UnsupportedProfile);
    }

    let original_scan = scan_text(request.text);
    let execution = execute_profile(request.text, request.profile)?;
    let clean_scan = scan_text(&execution.clean_text);
    let manifest = TransformationManifest {
        tcs_version: tcs_version(),
        profile: request.profile,
        transformations: execution.transformations.clone(),
    };
    let technical_diff = technical_diff(request.text, &execution);
    let content_preservation_diff = content_preservation_diff(&execution.transformations);
    let verification = verify(
        request,
        &original_scan,
        &clean_scan,
        &execution,
        &manifest,
        content_preservation_diff.status,
    )?;

    Ok(CleanResult {
        original_text: request.text.to_owned(),
        clean_text: execution.clean_text,
        profile: request.profile,
        tcs_version: tcs_version(),
        manifest,
        technical_diff,
        content_preservation_diff,
        verification,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Execution {
    clean_text: String,
    transformations: Vec<Transformation>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Candidate {
    rule_id: RuleId,
    original_code_point_range: TextRange,
    original_utf8_byte_range: TextRange,
    replacement: String,
    reason: &'static str,
}

fn execute_profile(text: &str, profile: CleanProfile) -> Result<Execution, CleanError> {
    let mut candidates = Vec::new();

    if profile_uses_rule(profile, RuleId::UnicodeNfc) {
        collect_nfc_candidates(text, &mut candidates);
    }
    if profile_uses_rule(profile, RuleId::LineEndings) {
        collect_line_ending_candidates(text, &mut candidates);
    }
    if profile_uses_rule(profile, RuleId::LeadingByteOrderMark) {
        collect_leading_bom_candidate(text, &mut candidates);
    }
    if profile_uses_rule(profile, RuleId::NonBreakingSpace) {
        collect_nbsp_candidates(text, &mut candidates);
    }

    candidates.sort_by_key(|candidate| {
        (
            candidate.original_utf8_byte_range.start,
            candidate.original_utf8_byte_range.end,
            candidate.rule_id,
        )
    });

    let mut transformations = Vec::with_capacity(candidates.len());
    let mut previous_end = 0;
    for (index, candidate) in candidates.into_iter().enumerate() {
        let byte_range = candidate.original_utf8_byte_range;
        if byte_range.start < previous_end
            || byte_range.end < byte_range.start
            || byte_range.end > text.len()
            || !text.is_char_boundary(byte_range.start)
            || !text.is_char_boundary(byte_range.end)
        {
            return Err(CleanError::InvariantViolation);
        }

        let before = text[byte_range.start..byte_range.end].to_owned();
        if before == candidate.replacement {
            return Err(CleanError::InvariantViolation);
        }

        let rule = rule_definition(candidate.rule_id);
        transformations.push(Transformation {
            sequence: index + 1,
            rule_id: candidate.rule_id,
            rule_version: rule.version,
            operation: rule.operation,
            original_code_point_range: candidate.original_code_point_range,
            original_utf8_byte_range: byte_range,
            replacement: candidate.replacement.clone(),
            reason: candidate.reason,
            before,
            after: candidate.replacement,
        });
        previous_end = byte_range.end;
    }

    let clean_text =
        replay_manifest(text, &transformations).ok_or(CleanError::InvariantViolation)?;

    Ok(Execution {
        clean_text,
        transformations,
    })
}

fn collect_nfc_candidates(text: &str, candidates: &mut Vec<Candidate>) {
    let mut chunk_start_byte = 0;
    let mut chunk_start_code_point = 0;
    let mut code_point_index = 0;

    for (byte_offset, character) in text.char_indices() {
        if is_normalization_boundary(character) {
            push_nfc_candidate(
                text,
                chunk_start_byte,
                byte_offset,
                chunk_start_code_point,
                code_point_index,
                candidates,
            );
            chunk_start_byte = byte_offset + character.len_utf8();
            chunk_start_code_point = code_point_index + 1;
        }
        code_point_index += 1;
    }

    push_nfc_candidate(
        text,
        chunk_start_byte,
        text.len(),
        chunk_start_code_point,
        code_point_index,
        candidates,
    );
}

const fn is_normalization_boundary(character: char) -> bool {
    matches!(character, '\r' | '\n' | '\u{00A0}' | '\u{FEFF}')
}

fn push_nfc_candidate(
    text: &str,
    chunk_start_byte: usize,
    chunk_end_byte: usize,
    chunk_start_code_point: usize,
    chunk_end_code_point: usize,
    candidates: &mut Vec<Candidate>,
) {
    if chunk_start_byte == chunk_end_byte {
        return;
    }

    let chunk = &text[chunk_start_byte..chunk_end_byte];
    let normalized: String = chunk.nfc().collect();
    if normalized == chunk {
        return;
    }

    let original_characters: Vec<char> = chunk.chars().collect();
    let normalized_characters: Vec<char> = normalized.chars().collect();
    let shared_length = original_characters.len().min(normalized_characters.len());
    let prefix = original_characters
        .iter()
        .zip(&normalized_characters)
        .take_while(|(original, normalized)| original == normalized)
        .count();
    let suffix = original_characters[prefix..]
        .iter()
        .rev()
        .zip(normalized_characters[prefix..].iter().rev())
        .take(shared_length.saturating_sub(prefix))
        .take_while(|(original, normalized)| original == normalized)
        .count();

    let original_changed_end = original_characters.len() - suffix;
    let normalized_changed_end = normalized_characters.len() - suffix;
    let original_relative_start = byte_offset_for_char_index(chunk, prefix);
    let original_relative_end = byte_offset_for_char_index(chunk, original_changed_end);
    let normalized_relative_start = byte_offset_for_char_index(&normalized, prefix);
    let normalized_relative_end = byte_offset_for_char_index(&normalized, normalized_changed_end);

    candidates.push(Candidate {
        rule_id: RuleId::UnicodeNfc,
        original_code_point_range: TextRange {
            start: chunk_start_code_point + prefix,
            end: chunk_start_code_point + original_changed_end,
        },
        original_utf8_byte_range: TextRange {
            start: chunk_start_byte + original_relative_start,
            end: chunk_start_byte + original_relative_end,
        },
        replacement: normalized[normalized_relative_start..normalized_relative_end].to_owned(),
        reason: "Canonical Unicode equivalence is represented in deterministic NFC form.",
    });

    debug_assert_eq!(
        chunk_end_code_point - chunk_start_code_point,
        original_characters.len()
    );
}

fn byte_offset_for_char_index(text: &str, character_index: usize) -> usize {
    text.char_indices()
        .nth(character_index)
        .map_or(text.len(), |(offset, _)| offset)
}

fn collect_line_ending_candidates(text: &str, candidates: &mut Vec<Candidate>) {
    let mut characters = text.char_indices().enumerate().peekable();

    while let Some((code_point_index, (byte_offset, character))) = characters.next() {
        if character != '\r' {
            continue;
        }

        let mut end_code_point = code_point_index + 1;
        let mut end_byte = byte_offset + character.len_utf8();
        if let Some((_, (next_byte_offset, '\n'))) = characters.peek().copied() {
            characters.next();
            end_code_point += 1;
            end_byte = next_byte_offset + '\n'.len_utf8();
        }

        candidates.push(Candidate {
            rule_id: RuleId::LineEndings,
            original_code_point_range: TextRange {
                start: code_point_index,
                end: end_code_point,
            },
            original_utf8_byte_range: TextRange {
                start: byte_offset,
                end: end_byte,
            },
            replacement: "\n".to_owned(),
            reason: "The original line break is represented as one LF without changing line count.",
        });
    }
}

fn collect_leading_bom_candidate(text: &str, candidates: &mut Vec<Candidate>) {
    let mut characters = text.chars();
    if characters.next() != Some('\u{FEFF}') || characters.next() == Some('\u{FEFF}') {
        return;
    }

    candidates.push(Candidate {
        rule_id: RuleId::LeadingByteOrderMark,
        original_code_point_range: TextRange { start: 0, end: 1 },
        original_utf8_byte_range: TextRange {
            start: 0,
            end: '\u{FEFF}'.len_utf8(),
        },
        replacement: String::new(),
        reason: "A single U+FEFF at absolute input start is treated as a pasted-text encoding marker.",
    });
}

fn collect_nbsp_candidates(text: &str, candidates: &mut Vec<Candidate>) {
    for (code_point_index, (byte_offset, character)) in text.char_indices().enumerate() {
        if character != '\u{00A0}' {
            continue;
        }

        candidates.push(Candidate {
            rule_id: RuleId::NonBreakingSpace,
            original_code_point_range: TextRange {
                start: code_point_index,
                end: code_point_index + 1,
            },
            original_utf8_byte_range: TextRange {
                start: byte_offset,
                end: byte_offset + character.len_utf8(),
            },
            replacement: " ".to_owned(),
            reason: "Pure Plain Text explicitly trades non-breaking layout behavior for U+0020.",
        });
    }
}

fn replay_manifest(original: &str, transformations: &[Transformation]) -> Option<String> {
    let mut output = String::with_capacity(original.len());
    let mut cursor = 0;

    for (index, transformation) in transformations.iter().enumerate() {
        let range = transformation.original_utf8_byte_range;
        if transformation.sequence != index + 1
            || range.start < cursor
            || range.end < range.start
            || range.end > original.len()
            || !original.is_char_boundary(range.start)
            || !original.is_char_boundary(range.end)
            || original[range.start..range.end] != transformation.before
            || transformation.after != transformation.replacement
            || original[..range.start].chars().count()
                != transformation.original_code_point_range.start
            || original[..range.end].chars().count() != transformation.original_code_point_range.end
        {
            return None;
        }

        let rule = rule_definition(transformation.rule_id);
        if transformation.rule_version != rule.version || transformation.operation != rule.operation
        {
            return None;
        }

        output.push_str(&original[cursor..range.start]);
        output.push_str(&transformation.replacement);
        cursor = range.end;
    }

    output.push_str(&original[cursor..]);
    Some(output)
}

fn technical_diff(original: &str, execution: &Execution) -> TechnicalDiff {
    TechnicalDiff {
        changed: original != execution.clean_text,
        original_utf8_bytes: original.len(),
        clean_utf8_bytes: execution.clean_text.len(),
        original_code_points: original.chars().count(),
        clean_code_points: execution.clean_text.chars().count(),
        transformation_count: execution.transformations.len(),
    }
}

fn content_preservation_diff(transformations: &[Transformation]) -> ContentPreservationDiff {
    if transformations
        .iter()
        .any(|transformation| transformation.rule_id == RuleId::NonBreakingSpace)
    {
        ContentPreservationDiff {
            status: ContentPreservationStatus::NeedsReview,
            reason: "NBSP replacement changes non-breaking layout behavior and requires review.",
        }
    } else if transformations.is_empty() {
        ContentPreservationDiff {
            status: ContentPreservationStatus::Preserved,
            reason: "No text transformation was applied.",
        }
    } else {
        ContentPreservationDiff {
            status: ContentPreservationStatus::Preserved,
            reason: "Only canonical-equivalent or technical representation rules were applied.",
        }
    }
}

fn verify(
    request: CleanRequest<'_>,
    original_scan: &ScanResult,
    clean_scan: &ScanResult,
    execution: &Execution,
    manifest: &TransformationManifest,
    content_preservation_status: ContentPreservationStatus,
) -> Result<VerificationReport, CleanError> {
    let replayed = replay_manifest(request.text, &manifest.transformations);
    let repeated = execute_profile(request.text, request.profile)?;
    let reapplied = execute_profile(&execution.clean_text, request.profile)?;
    let rules_allowed_by_profile = manifest.transformations.iter().all(|transformation| {
        profile_uses_rule(request.profile, transformation.rule_id)
            && transformation.rule_version == rule_definition(transformation.rule_id).version
    });
    let manifest_matches = replayed.as_deref() == Some(execution.clean_text.as_str());
    let changes_declared = (request.text == execution.clean_text)
        == manifest.transformations.is_empty()
        && manifest.transformations.len() == execution.transformations.len();
    let checks = VerificationChecks {
        manifest_reproduces_clean_text: manifest_matches,
        rules_allowed_by_profile,
        no_undeclared_transformations: manifest_matches && changes_declared,
        clean_rescan_completed: true,
        expected_findings_reassessed: expected_findings_reassessed(
            &execution.clean_text,
            request.profile,
            clean_scan,
        ),
        preserved_findings_visible: preserved_findings_visible(
            request.text,
            request.profile,
            clean_scan,
        ),
        deterministic: repeated == *execution,
        idempotent: reapplied.clean_text == execution.clean_text
            && reapplied.transformations.is_empty(),
    };

    let verification_status = if !checks.all_passed() {
        VerificationStatus::Failed
    } else if matches!(
        content_preservation_status,
        ContentPreservationStatus::NeedsReview | ContentPreservationStatus::Unknown
    ) {
        VerificationStatus::NeedsReview
    } else if clean_scan.findings.is_empty() {
        VerificationStatus::Verified
    } else {
        VerificationStatus::VerifiedWithRemainingFindings
    };

    Ok(VerificationReport {
        original_scan_summary: summarize_scan(original_scan),
        clean_scan_summary: summarize_scan(clean_scan),
        applied_transformations: manifest.transformations.len(),
        remaining_findings: clean_scan.findings.clone(),
        content_preservation_status,
        verification_status,
        checks,
    })
}

fn summarize_scan(scan: &ScanResult) -> CleanScanSummary {
    CleanScanSummary {
        characters: scan.characters.len(),
        findings: scan.findings.len(),
        invisible: scan
            .characters
            .iter()
            .filter(|record| record.is_invisible)
            .count(),
        unusual_whitespace: scan
            .characters
            .iter()
            .filter(|record| record.is_unusual_whitespace)
            .count(),
        bidi_controls: scan
            .characters
            .iter()
            .filter(|record| record.is_bidi_control)
            .count(),
        variation_selectors: scan
            .characters
            .iter()
            .filter(|record| record.is_variation_selector)
            .count(),
    }
}

fn expected_findings_reassessed(
    clean_text: &str,
    profile: CleanProfile,
    clean_scan: &ScanResult,
) -> bool {
    (!profile_uses_rule(profile, RuleId::UnicodeNfc) || !clean_scan.normalization.nfc_would_change)
        && (!profile_uses_rule(profile, RuleId::LineEndings) || !clean_text.contains('\r'))
        && (!profile_uses_rule(profile, RuleId::LeadingByteOrderMark)
            || !has_unambiguous_leading_bom(clean_text))
        && (!profile_uses_rule(profile, RuleId::NonBreakingSpace)
            || !clean_text.contains('\u{00A0}'))
}

fn has_unambiguous_leading_bom(text: &str) -> bool {
    let mut characters = text.chars();
    characters.next() == Some('\u{FEFF}') && characters.next() != Some('\u{FEFF}')
}

fn preserved_findings_visible(
    original: &str,
    profile: CleanProfile,
    clean_scan: &ScanResult,
) -> bool {
    let protected_characters: BTreeSet<char> = original
        .chars()
        .filter(|character| is_protected_observation(*character))
        .collect();

    protected_characters.into_iter().all(|character| {
        let original_count = original
            .chars()
            .filter(|candidate| *candidate == character)
            .count();
        let expected_count =
            if character == '\u{00A0}' && profile_uses_rule(profile, RuleId::NonBreakingSpace) {
                0
            } else if character == '\u{FEFF}'
                && profile_uses_rule(profile, RuleId::LeadingByteOrderMark)
                && has_unambiguous_leading_bom(original)
            {
                original_count - 1
            } else {
                original_count
            };
        let clean_character_count = clean_scan
            .characters
            .iter()
            .filter(|record| record.character == character)
            .count();
        let clean_finding_count = clean_scan
            .findings
            .iter()
            .filter(|finding| finding.code_point == character as u32)
            .count();

        clean_character_count == expected_count && clean_finding_count == expected_count
    })
}

fn is_protected_observation(character: char) -> bool {
    matches!(
        character,
        '\u{00AD}' | '\u{00A0}' | '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{2060}' | '\u{FEFF}'
    ) || special::is_bidi_control(character)
        || special::is_variation_selector(character)
}
