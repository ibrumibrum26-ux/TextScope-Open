use std::collections::BTreeSet;

use textscope_core::{
    CleanError, CleanProfile, ContentPreservationStatus, RuleId, TransformationOperation,
    VerificationChecks, VerificationStatus, available_clean_profiles, clean_text, scan_text,
    tcs_rules, tcs_version,
};

const SUPPORTED_PROFILES: [CleanProfile; 3] = [
    CleanProfile::SafeClean,
    CleanProfile::CanonicalClean,
    CleanProfile::PurePlainText,
];

fn assert_checks_pass(checks: VerificationChecks) {
    assert!(checks.manifest_reproduces_clean_text);
    assert!(checks.rules_allowed_by_profile);
    assert!(checks.no_undeclared_transformations);
    assert!(checks.clean_rescan_completed);
    assert!(checks.expected_findings_reassessed);
    assert!(checks.preserved_findings_visible);
    assert!(checks.deterministic);
    assert!(checks.idempotent);
}

fn replay(original: &str, result: &textscope_core::CleanResult) -> String {
    let mut output = String::new();
    let mut cursor = 0;

    for (index, transformation) in result.manifest.transformations.iter().enumerate() {
        assert_eq!(transformation.sequence, index + 1);
        let range = transformation.original_utf8_byte_range;
        assert!(range.start >= cursor);
        assert!(original.is_char_boundary(range.start));
        assert!(original.is_char_boundary(range.end));
        assert_eq!(&original[range.start..range.end], transformation.before);
        assert_eq!(transformation.after, transformation.replacement);
        assert_eq!(
            original[..range.start].chars().count(),
            transformation.original_code_point_range.start
        );
        assert_eq!(
            original[..range.end].chars().count(),
            transformation.original_code_point_range.end
        );
        output.push_str(&original[cursor..range.start]);
        output.push_str(&transformation.replacement);
        cursor = range.end;
    }

    output.push_str(&original[cursor..]);
    output
}

#[test]
fn tcs_version_and_registry_are_formal_and_stable() {
    assert_eq!(tcs_version(), "0.1.0");

    let rules = tcs_rules();
    assert_eq!(rules.len(), 4);
    let ids: Vec<_> = rules.iter().map(|rule| rule.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "TCS-NORMALIZATION-001",
            "TCS-LINE-ENDINGS-001",
            "TCS-CONTROL-001",
            "TCS-WHITESPACE-001",
        ]
    );
    assert_eq!(ids.iter().collect::<BTreeSet<_>>().len(), ids.len());
    for rule in rules {
        assert_eq!(rule.version.as_str(), "0.1.0");
        assert!(!rule.title.is_empty());
        assert!(!rule.description.is_empty());
    }
}

#[test]
fn profiles_declare_rules_and_reserve_document_specific_behavior() {
    let profiles = available_clean_profiles();
    assert_eq!(profiles.len(), 5);

    let safe = profiles
        .iter()
        .find(|definition| definition.profile == CleanProfile::SafeClean)
        .expect("safe metadata");
    assert!(safe.supported);
    assert_eq!(safe.active_rules, [RuleId::LineEndings]);

    let canonical = profiles
        .iter()
        .find(|definition| definition.profile == CleanProfile::CanonicalClean)
        .expect("canonical metadata");
    assert_eq!(canonical.active_rules.len(), 3);

    let plain = profiles
        .iter()
        .find(|definition| definition.profile == CleanProfile::PurePlainText)
        .expect("plain metadata");
    assert_eq!(plain.active_rules.len(), 4);

    for reserved in [CleanProfile::DocumentClean, CleanProfile::DeveloperClean] {
        let definition = profiles
            .iter()
            .find(|definition| definition.profile == reserved)
            .expect("reserved metadata");
        assert!(!definition.supported);
        assert!(definition.active_rules.is_empty());
        assert_eq!(
            clean_text("text", reserved),
            Err(CleanError::UnsupportedProfile)
        );
    }
}

#[test]
fn clean_is_byte_for_byte_deterministic() -> Result<(), CleanError> {
    let input = "\u{FEFF}cafe\u{301}\r\nA😀B\u{00A0}👩\u{200D}💻";

    for profile in SUPPORTED_PROFILES {
        assert_eq!(clean_text(input, profile)?, clean_text(input, profile)?);
    }

    Ok(())
}

#[test]
fn every_supported_profile_is_idempotent() -> Result<(), CleanError> {
    let input = "\u{FEFF}cafe\u{301}\rline\u{00A0}👩\u{200D}💻";

    for profile in SUPPORTED_PROFILES {
        let first = clean_text(input, profile)?;
        let second = clean_text(&first.clean_text, profile)?;
        assert_eq!(second.clean_text, first.clean_text);
        assert!(second.manifest.transformations.is_empty());
        assert!(first.verification.checks.idempotent);
        assert_checks_pass(first.verification.checks);
    }

    Ok(())
}

#[test]
fn normal_portuguese_text_is_not_changed() -> Result<(), CleanError> {
    let input = "Olá, ação e coração. Tudo bem?";

    for profile in SUPPORTED_PROFILES {
        let result = clean_text(input, profile)?;
        assert_eq!(result.original_text, input);
        assert_eq!(result.clean_text, input);
        assert!(result.manifest.transformations.is_empty());
        assert_eq!(
            result.verification.verification_status,
            VerificationStatus::Verified
        );
    }

    Ok(())
}

#[test]
fn ascii_emoji_boundary_remains_unchanged_and_coherent() -> Result<(), CleanError> {
    for profile in SUPPORTED_PROFILES {
        let result = clean_text("A😀B", profile)?;
        assert_eq!(result.clean_text, "A😀B");
        assert_eq!(result.technical_diff.original_code_points, 3);
        assert_eq!(result.technical_diff.clean_code_points, 3);
        assert_eq!(result.technical_diff.original_utf8_bytes, 6);
        assert_eq!(result.technical_diff.clean_utf8_bytes, 6);
        assert!(!result.technical_diff.changed);
    }

    Ok(())
}

#[test]
fn emoji_zwj_is_preserved_by_every_profile() -> Result<(), CleanError> {
    let input = "👩\u{200D}💻";

    for profile in SUPPORTED_PROFILES {
        let result = clean_text(input, profile)?;
        assert_eq!(result.clean_text, input);
        assert!(result.manifest.transformations.is_empty());
        assert_eq!(result.verification.remaining_findings.len(), 1);
        assert_eq!(
            result.verification.verification_status,
            VerificationStatus::VerifiedWithRemainingFindings
        );
        assert!(result.verification.checks.preserved_findings_visible);
    }

    Ok(())
}

#[test]
fn zwnj_is_never_removed_blindly() -> Result<(), CleanError> {
    let input = "می\u{200C}خواهم";

    for profile in SUPPORTED_PROFILES {
        let result = clean_text(input, profile)?;
        assert_eq!(result.clean_text, input);
        assert!(
            result
                .verification
                .remaining_findings
                .iter()
                .any(|finding| finding.code_point == 0x200C)
        );
    }

    Ok(())
}

#[test]
fn variation_selectors_are_preserved() -> Result<(), CleanError> {
    let inputs = ["✈\u{FE0F}".to_owned(), format!("木{}", '\u{E0100}')];

    for input in inputs {
        for profile in SUPPORTED_PROFILES {
            let result = clean_text(&input, profile)?;
            assert_eq!(result.clean_text, input);
            assert_eq!(
                result.verification.clean_scan_summary.variation_selectors,
                1
            );
            assert!(result.verification.checks.preserved_findings_visible);
        }
    }

    Ok(())
}

#[test]
fn combining_marks_are_preserved_or_canonically_composed() -> Result<(), CleanError> {
    let decomposed = "cafe\u{301}";
    let safe = clean_text(decomposed, CleanProfile::SafeClean)?;
    assert_eq!(safe.clean_text, decomposed);
    assert!(safe.manifest.transformations.is_empty());

    for profile in [CleanProfile::CanonicalClean, CleanProfile::PurePlainText] {
        let result = clean_text(decomposed, profile)?;
        assert_eq!(result.clean_text, "café");
        assert_eq!(result.manifest.transformations.len(), 1);
        assert_eq!(
            result.manifest.transformations[0].rule_id,
            RuleId::UnicodeNfc
        );
        assert_eq!(
            result.content_preservation_diff.status,
            ContentPreservationStatus::Preserved
        );
    }

    Ok(())
}

#[test]
fn nbsp_policy_is_profile_specific_and_audited() -> Result<(), CleanError> {
    let input = "Olá\u{00A0}mundo";

    for profile in [CleanProfile::SafeClean, CleanProfile::CanonicalClean] {
        let result = clean_text(input, profile)?;
        assert_eq!(result.clean_text, input);
        assert!(result.manifest.transformations.is_empty());
        assert_eq!(result.verification.clean_scan_summary.unusual_whitespace, 1);
        assert_eq!(
            result.verification.verification_status,
            VerificationStatus::VerifiedWithRemainingFindings
        );
    }

    let plain = clean_text(input, CleanProfile::PurePlainText)?;
    assert_eq!(plain.clean_text, "Olá mundo");
    assert_eq!(plain.manifest.transformations.len(), 1);
    assert_eq!(
        plain.manifest.transformations[0].rule_id,
        RuleId::NonBreakingSpace
    );
    assert_eq!(
        plain.manifest.transformations[0].operation,
        TransformationOperation::ReplaceNonBreakingSpace
    );
    assert_eq!(
        plain.content_preservation_diff.status,
        ContentPreservationStatus::NeedsReview
    );
    assert_eq!(
        plain.verification.verification_status,
        VerificationStatus::NeedsReview
    );
    assert_eq!(plain.verification.clean_scan_summary.unusual_whitespace, 0);

    Ok(())
}

#[test]
fn soft_hyphen_and_word_joiner_are_preserved() -> Result<(), CleanError> {
    let input = "co\u{00AD}operate word\u{2060}join";

    for profile in SUPPORTED_PROFILES {
        let result = clean_text(input, profile)?;
        assert_eq!(result.clean_text, input);
        assert_eq!(result.verification.remaining_findings.len(), 2);
        assert!(result.verification.checks.preserved_findings_visible);
    }

    Ok(())
}

#[test]
fn bidi_controls_remain_visible_for_review() -> Result<(), CleanError> {
    let input = "abc\u{202E}def";

    for profile in SUPPORTED_PROFILES {
        let result = clean_text(input, profile)?;
        assert_eq!(result.clean_text, input);
        assert_eq!(result.verification.clean_scan_summary.bidi_controls, 1);
        assert!(
            result
                .verification
                .remaining_findings
                .iter()
                .any(|finding| finding.code_point == 0x202E)
        );
    }

    Ok(())
}

#[test]
fn nfc_manifest_uses_original_half_open_offsets() -> Result<(), CleanError> {
    let input = "cafe\u{301}";
    let result = clean_text(input, CleanProfile::CanonicalClean)?;
    let transformation = &result.manifest.transformations[0];

    assert_eq!(transformation.rule_id, RuleId::UnicodeNfc);
    assert_eq!(transformation.original_code_point_range.start, 3);
    assert_eq!(transformation.original_code_point_range.end, 5);
    assert_eq!(transformation.original_utf8_byte_range.start, 3);
    assert_eq!(transformation.original_utf8_byte_range.end, 6);
    assert_eq!(transformation.before, "e\u{301}");
    assert_eq!(transformation.after, "é");
    assert_eq!(replay(input, &result), result.clean_text);

    Ok(())
}

#[test]
fn line_endings_are_canonicalized_without_changing_line_count() -> Result<(), CleanError> {
    let input = "a\r\nb\rc\n";

    for profile in SUPPORTED_PROFILES {
        let result = clean_text(input, profile)?;
        assert_eq!(result.clean_text, "a\nb\nc\n");
        assert_eq!(result.manifest.transformations.len(), 2);
        assert!(
            result
                .manifest
                .transformations
                .iter()
                .all(|transformation| {
                    transformation.rule_id == RuleId::LineEndings
                        && transformation.operation
                            == TransformationOperation::CanonicalizeLineEnding
                })
        );
        assert_eq!(replay(input, &result), result.clean_text);
        assert_eq!(result.clean_text.matches('\n').count(), 3);
    }

    Ok(())
}

#[test]
fn one_leading_feff_is_removed_only_by_applicable_profiles() -> Result<(), CleanError> {
    let input = "\u{FEFF}Olá";
    let safe = clean_text(input, CleanProfile::SafeClean)?;
    assert_eq!(safe.clean_text, input);

    for profile in [CleanProfile::CanonicalClean, CleanProfile::PurePlainText] {
        let result = clean_text(input, profile)?;
        assert_eq!(result.clean_text, "Olá");
        assert_eq!(result.manifest.transformations.len(), 1);
        assert_eq!(
            result.manifest.transformations[0].rule_id,
            RuleId::LeadingByteOrderMark
        );
    }

    let ambiguous = "\u{FEFF}\u{FEFF}Olá";
    for profile in SUPPORTED_PROFILES {
        let result = clean_text(ambiguous, profile)?;
        assert_eq!(result.clean_text, ambiguous);
        assert!(result.manifest.transformations.is_empty());
        assert!(result.verification.checks.idempotent);
    }

    let internal = "Olá\u{FEFF}mundo";
    for profile in SUPPORTED_PROFILES {
        let result = clean_text(internal, profile)?;
        assert_eq!(result.clean_text, internal);
        assert!(result.manifest.transformations.is_empty());
        assert!(result.verification.checks.preserved_findings_visible);
    }

    Ok(())
}

#[test]
fn empty_text_is_valid_for_every_supported_profile() -> Result<(), CleanError> {
    for profile in SUPPORTED_PROFILES {
        let result = clean_text("", profile)?;
        assert!(result.original_text.is_empty());
        assert!(result.clean_text.is_empty());
        assert!(result.manifest.transformations.is_empty());
        assert_eq!(
            result.verification.verification_status,
            VerificationStatus::Verified
        );
        assert_checks_pass(result.verification.checks);
    }

    Ok(())
}

#[test]
fn complex_manifest_is_ordered_and_fully_replayable() -> Result<(), CleanError> {
    let input = "\u{FEFF}e\u{301}\r\nx\u{00A0}y";
    let result = clean_text(input, CleanProfile::PurePlainText)?;

    assert_eq!(result.clean_text, "é\nx y");
    assert_eq!(result.manifest.transformations.len(), 4);
    assert_eq!(
        result
            .manifest
            .transformations
            .iter()
            .map(|transformation| transformation.rule_id)
            .collect::<Vec<_>>(),
        [
            RuleId::LeadingByteOrderMark,
            RuleId::UnicodeNfc,
            RuleId::LineEndings,
            RuleId::NonBreakingSpace,
        ]
    );
    assert_eq!(replay(input, &result), result.clean_text);
    assert!(result.verification.checks.no_undeclared_transformations);
    assert_eq!(result.technical_diff.transformation_count, 4);

    Ok(())
}

#[test]
fn independent_rescan_matches_a_fresh_public_scan() -> Result<(), CleanError> {
    let result = clean_text("e\u{301}\r\n👩\u{200D}💻", CleanProfile::CanonicalClean)?;
    let independent = scan_text(&result.clean_text);

    assert_eq!(
        result.verification.clean_scan_summary.characters,
        independent.characters.len()
    );
    assert_eq!(
        result.verification.clean_scan_summary.findings,
        independent.findings.len()
    );
    assert_eq!(result.verification.remaining_findings, independent.findings);
    assert!(result.verification.checks.clean_rescan_completed);

    Ok(())
}

#[test]
fn verification_states_distinguish_clean_remaining_and_review() -> Result<(), CleanError> {
    assert_eq!(
        clean_text("texto", CleanProfile::SafeClean)?
            .verification
            .verification_status,
        VerificationStatus::Verified
    );
    assert_eq!(
        clean_text("a\u{200B}b", CleanProfile::SafeClean)?
            .verification
            .verification_status,
        VerificationStatus::VerifiedWithRemainingFindings
    );
    assert_eq!(
        clean_text("a\u{00A0}b", CleanProfile::PurePlainText)?
            .verification
            .verification_status,
        VerificationStatus::NeedsReview
    );

    Ok(())
}

#[test]
fn broad_valid_unicode_sample_never_panics_and_remains_consistent() -> Result<(), CleanError> {
    let scalars = [
        '\0',
        '\u{007F}',
        '\u{00AD}',
        '\u{00A0}',
        '\u{0301}',
        '\u{061C}',
        '\u{200B}',
        '\u{200C}',
        '\u{200D}',
        '\u{202E}',
        '\u{2060}',
        '\u{FE0F}',
        '\u{FEFF}',
        '\u{E0100}',
        '\u{10FFFF}',
        'A',
        'é',
        'ع',
        '中',
        '😀',
        '\r',
        '\n',
    ];

    let mut samples = vec![String::new()];
    samples.extend(scalars.iter().map(char::to_string));
    samples.push(scalars.into_iter().collect());
    samples.push("e\u{301} x\u{00A0}y\r\n👩\u{200D}💻".to_owned());

    for sample in samples {
        for profile in SUPPORTED_PROFILES {
            let first = clean_text(&sample, profile)?;
            let second = clean_text(&sample, profile)?;
            let reapplied = clean_text(&first.clean_text, profile)?;

            assert_eq!(first, second);
            assert_eq!(replay(&sample, &first), first.clean_text);
            assert_eq!(reapplied.clean_text, first.clean_text);
            assert!(reapplied.manifest.transformations.is_empty());
            assert_ne!(
                first.verification.verification_status,
                VerificationStatus::Failed
            );
            assert_checks_pass(first.verification.checks);
        }
    }

    Ok(())
}
