use crate::{
    classification::{Classification, classify_character, code_point_label},
    model::{CharacterRecord, Finding, FindingKind, FindingSeverity, ScanResult},
    normalization::inspect_normalization,
    special::is_join_control,
};

/// Scans valid UTF-8 text without modifying it or performing I/O.
///
/// Offsets are tracked independently in Unicode scalar values, UTF-8 bytes,
/// and UTF-16 code units. Findings are neutral observations and do not imply
/// malicious intent or recommend removal.
#[must_use]
pub fn scan_text(text: &str) -> ScanResult {
    let mut characters = Vec::new();
    let mut findings = Vec::new();
    let mut utf16_offset = 0;

    for (code_point_index, (utf8_byte_offset, character)) in text.char_indices().enumerate() {
        let classification = classify_character(character);

        if let Some(finding) = finding_for(
            character,
            code_point_index,
            utf16_offset,
            utf8_byte_offset,
            &classification,
        ) {
            findings.push(finding);
        }

        let mut utf8_buffer = [0_u8; 4];
        let utf8_bytes = character.encode_utf8(&mut utf8_buffer).as_bytes().to_vec();

        characters.push(CharacterRecord {
            code_point_index,
            utf16_offset,
            utf8_byte_offset,
            character,
            code_point: character as u32,
            code_point_label: code_point_label(character),
            utf8_bytes,
            unicode_category: classification.unicode_category,
            script: classification.script,
            is_whitespace: classification.is_whitespace,
            is_control: classification.is_control,
            is_format: classification.is_format,
            is_invisible: classification.is_invisible,
            is_unusual_whitespace: classification.is_unusual_whitespace,
            is_bidi_control: classification.is_bidi_control,
            is_variation_selector: classification.is_variation_selector,
        });

        utf16_offset += character.len_utf16();
    }

    ScanResult {
        characters,
        findings,
        normalization: inspect_normalization(text),
    }
}

fn finding_for(
    character: char,
    code_point_index: usize,
    utf16_offset: usize,
    utf8_byte_offset: usize,
    classification: &Classification,
) -> Option<Finding> {
    let (kind, severity, description) = if classification.is_bidi_control {
        (
            FindingKind::BidirectionalControl,
            FindingSeverity::Caution,
            "Bidirectional control detected; display order may differ from storage order.",
        )
    } else if classification.is_unusual_whitespace {
        (
            FindingKind::UnusualWhitespace,
            FindingSeverity::Informational,
            "Unusual Unicode whitespace detected; its visual width or behavior may differ.",
        )
    } else if classification.is_variation_selector {
        (
            FindingKind::VariationSelector,
            FindingSeverity::Informational,
            "Variation selector detected; presentation may depend on adjacent content.",
        )
    } else if classification.is_invisible {
        let description = if is_join_control(character) {
            "Join control detected; it may carry legitimate linguistic or emoji semantics."
        } else {
            "Normally non-rendering character detected; context determines its meaning."
        };
        (
            FindingKind::InvisibleCharacter,
            FindingSeverity::Informational,
            description,
        )
    } else {
        return None;
    };

    Some(Finding {
        kind,
        severity,
        code_point_index,
        utf16_offset,
        utf8_byte_offset,
        code_point: character as u32,
        description,
    })
}
