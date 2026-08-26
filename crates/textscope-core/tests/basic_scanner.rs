use textscope_core::{
    FindingKind, FindingSeverity, NormalizationInspection, ScanResult, reveal_text, scan_text,
};

#[test]
fn tracks_code_point_utf8_and_utf16_offsets_independently() {
    let result = scan_text("A😀B");
    let observed: Vec<_> = result
        .characters
        .iter()
        .map(|record| {
            (
                record.character,
                record.code_point_index,
                record.utf8_byte_offset,
                record.utf16_offset,
                record.utf8_bytes.clone(),
            )
        })
        .collect();

    assert_eq!(
        observed,
        vec![
            ('A', 0, 0, 0, vec![0x41]),
            ('😀', 1, 1, 1, vec![0xF0, 0x9F, 0x98, 0x80]),
            ('B', 2, 5, 3, vec![0x42]),
        ]
    );
}

#[test]
fn reports_general_category_script_and_unicode_name() {
    let result = scan_text("Aع😀");

    assert_eq!(result.characters[0].unicode_category, "Lu");
    assert_eq!(result.characters[0].script, "Latin");
    assert_eq!(
        result.characters[0].code_point_label,
        "U+0041 LATIN CAPITAL LETTER A"
    );

    assert_eq!(result.characters[1].unicode_category, "Lo");
    assert_eq!(result.characters[1].script, "Arabic");

    assert_eq!(result.characters[2].unicode_category, "So");
    assert_eq!(result.characters[2].script, "Common");
}

#[test]
fn detects_and_reveals_zero_width_space() {
    let result = scan_text("a\u{200B}b");
    let record = &result.characters[1];

    assert_eq!(record.character, '\u{200B}');
    assert_eq!(record.code_point_label, "U+200B ZERO WIDTH SPACE");
    assert!(record.is_format);
    assert!(record.is_invisible);
    assert!(!record.is_bidi_control);
    assert_eq!(result.findings.len(), 1);
    assert_eq!(result.findings[0].kind, FindingKind::InvisibleCharacter);
    assert_eq!(result.findings[0].severity, FindingSeverity::Informational);
    assert_eq!(reveal_text("a\u{200B}b"), "a⟦ZWSP⟧b");
}

#[test]
fn detects_no_break_space_as_unusual_whitespace() {
    let result = scan_text("a\u{00A0}b");
    let record = &result.characters[1];

    assert!(record.is_whitespace);
    assert!(record.is_unusual_whitespace);
    assert!(!record.is_invisible);
    assert_eq!(record.unicode_category, "Zs");
    assert_eq!(result.findings.len(), 1);
    assert_eq!(result.findings[0].kind, FindingKind::UnusualWhitespace);
    assert_eq!(reveal_text("a\u{00A0}b"), "a⟦NBSP⟧b");
}

#[test]
fn detects_right_to_left_override_conservatively() {
    let result = scan_text("a\u{202E}b");
    let record = &result.characters[1];

    assert!(record.is_bidi_control);
    assert!(record.is_format);
    assert!(record.is_invisible);
    assert_eq!(result.findings.len(), 1);
    assert_eq!(result.findings[0].kind, FindingKind::BidirectionalControl);
    assert_eq!(result.findings[0].severity, FindingSeverity::Caution);
    assert_eq!(reveal_text("a\u{202E}b"), "a⟦RLO⟧b");
}

#[test]
fn emoji_joiner_is_observed_without_removal_claims() {
    let input = "👩\u{200D}💻";
    let result = scan_text(input);
    let offsets: Vec<_> = result
        .characters
        .iter()
        .map(|record| {
            (
                record.character,
                record.code_point_index,
                record.utf8_byte_offset,
                record.utf16_offset,
            )
        })
        .collect();

    assert_eq!(
        offsets,
        vec![('👩', 0, 0, 0), ('\u{200D}', 1, 4, 2), ('💻', 2, 7, 3)]
    );
    assert!(result.characters[1].is_invisible);
    assert_eq!(result.findings.len(), 1);
    assert_eq!(result.findings[0].kind, FindingKind::InvisibleCharacter);
    assert_eq!(result.findings[0].severity, FindingSeverity::Informational);
    assert!(
        !result.findings[0]
            .description
            .to_ascii_lowercase()
            .contains("remov")
    );
    assert_eq!(reveal_text(input), "👩⟦ZWJ⟧💻");
}

#[test]
fn empty_input_is_valid_and_predictable() {
    let result = scan_text("");

    assert!(result.characters.is_empty());
    assert!(result.findings.is_empty());
    assert_eq!(result.normalization, NormalizationInspection::default());
    assert_eq!(reveal_text(""), "");
}

#[test]
fn normal_portuguese_text_has_no_spurious_findings() {
    let input = "Olá, coração!\tTudo bem?\r\n";
    let result = scan_text(input);

    assert!(result.findings.is_empty());
    assert_eq!(
        result
            .characters
            .iter()
            .map(|record| record.character)
            .collect::<String>(),
        input
    );
}

#[test]
fn normalization_is_inspected_without_mutating_input() {
    let composed = "café";
    let composed_result = scan_text(composed);
    assert!(!composed_result.normalization.nfc_would_change);
    assert!(composed_result.normalization.nfd_would_change);

    let decomposed = "cafe\u{0301}";
    let decomposed_result = scan_text(decomposed);
    assert!(decomposed_result.normalization.nfc_would_change);
    assert!(!decomposed_result.normalization.nfd_would_change);
    assert_eq!(
        decomposed_result
            .characters
            .iter()
            .map(|record| record.character)
            .collect::<String>(),
        decomposed
    );

    let compatibility_character = scan_text("ﬁ");
    assert!(compatibility_character.normalization.nfkc_would_change);
    assert!(compatibility_character.normalization.nfkd_would_change);
}

#[test]
fn detects_variation_selectors_in_both_unicode_ranges() {
    for input in ["✈\u{FE0F}".to_owned(), format!("木{}", '\u{E0100}')] {
        let result = scan_text(&input);
        let selector = result.characters.last().expect("selector record");
        let finding = result.findings.last().expect("selector finding");

        assert!(selector.is_variation_selector);
        assert!(selector.is_invisible);
        assert_eq!(selector.unicode_category, "Mn");
        assert_eq!(finding.kind, FindingKind::VariationSelector);
        assert_eq!(finding.severity, FindingSeverity::Informational);
    }

    assert_eq!(reveal_text("✈\u{FE0F}"), "✈⟦U+FE0F⟧");
}

#[test]
fn all_required_bidi_controls_are_classified_and_revealed() {
    let cases = [
        ('\u{061C}', "⟦ALM⟧"),
        ('\u{200E}', "⟦LRM⟧"),
        ('\u{200F}', "⟦RLM⟧"),
        ('\u{202A}', "⟦LRE⟧"),
        ('\u{202B}', "⟦RLE⟧"),
        ('\u{202C}', "⟦PDF⟧"),
        ('\u{202D}', "⟦LRO⟧"),
        ('\u{202E}', "⟦RLO⟧"),
        ('\u{2066}', "⟦LRI⟧"),
        ('\u{2067}', "⟦RLI⟧"),
        ('\u{2068}', "⟦FSI⟧"),
        ('\u{2069}', "⟦PDI⟧"),
    ];

    for (character, expected_reveal) in cases {
        let input = character.to_string();
        let result = scan_text(&input);

        assert!(
            result.characters[0].is_bidi_control,
            "U+{:04X}",
            character as u32
        );
        assert_eq!(result.findings.len(), 1);
        assert_eq!(result.findings[0].kind, FindingKind::BidirectionalControl);
        assert_eq!(result.findings[0].severity, FindingSeverity::Caution);
        assert_eq!(reveal_text(&input), expected_reveal);
    }
}

#[test]
fn special_invisibles_are_classified_and_revealed() {
    let cases = [
        ('\u{00AD}', "⟦SHY⟧"),
        ('\u{200B}', "⟦ZWSP⟧"),
        ('\u{200C}', "⟦ZWNJ⟧"),
        ('\u{200D}', "⟦ZWJ⟧"),
        ('\u{2060}', "⟦WJ⟧"),
        ('\u{FEFF}', "⟦ZWNBSP/BOM⟧"),
    ];

    for (character, expected_reveal) in cases {
        let input = character.to_string();
        let result = scan_text(&input);

        assert!(
            result.characters[0].is_invisible,
            "U+{:04X}",
            character as u32
        );
        assert_eq!(result.findings.len(), 1);
        assert_eq!(result.findings[0].kind, FindingKind::InvisibleCharacter);
        assert_eq!(reveal_text(&input), expected_reveal);
    }
}

#[test]
fn valid_unicode_samples_do_not_panic_or_change_input() {
    let input: String = [
        '\0',
        '\u{007F}',
        '\u{0301}',
        '\u{10FFFF}',
        '\u{E0100}',
        '中',
        '😀',
    ]
    .into_iter()
    .collect();
    let result: ScanResult = scan_text(&input);

    assert_eq!(result.characters.len(), input.chars().count());
    assert_eq!(
        result
            .characters
            .iter()
            .map(|record| record.character)
            .collect::<String>(),
        input
    );
}
