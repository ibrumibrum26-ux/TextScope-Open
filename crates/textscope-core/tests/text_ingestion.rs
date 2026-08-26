use textscope_core::{
    BomKind, CleanProfile, DecodeStatus, EncodingEvidence, SourceEncoding, TextIngestionError,
    TextSourceKind, analyze_text_bytes, clean_text,
};

fn utf16_bytes(text: &str, little_endian: bool) -> Vec<u8> {
    let mut bytes = if little_endian {
        vec![0xFF, 0xFE]
    } else {
        vec![0xFE, 0xFF]
    };

    for unit in text.encode_utf16() {
        let encoded = if little_endian {
            unit.to_le_bytes()
        } else {
            unit.to_be_bytes()
        };
        bytes.extend_from_slice(&encoded);
    }

    bytes
}

#[test]
fn strict_utf8_plain_text_and_markdown_remain_literal() -> Result<(), TextIngestionError> {
    for (kind, input) in [
        (TextSourceKind::PlainText, "Olá, mundo!"),
        (TextSourceKind::Markdown, "# Olá\n\n**literal**"),
    ] {
        let result = analyze_text_bytes(input.as_bytes(), kind)?;
        assert_eq!(result.raw_byte_length, input.len());
        assert_eq!(result.source_kind, kind);
        assert_eq!(result.decode.encoding, SourceEncoding::Utf8);
        assert_eq!(
            result.decode.encoding_evidence,
            EncodingEvidence::StrictUtf8
        );
        assert_eq!(result.decode.bom, BomKind::None);
        assert_eq!(result.decode.status, DecodeStatus::Decoded);
        assert_eq!(result.decode.decoded_text, input);
        assert_eq!(result.scan.characters.len(), input.chars().count());
    }

    Ok(())
}

#[test]
fn utf8_multibyte_source_mapping_uses_original_bytes() -> Result<(), TextIngestionError> {
    let result = analyze_text_bytes("A😀B".as_bytes(), TextSourceKind::PlainText)?;
    let mappings = &result.decode.source_mappings;

    assert_eq!(mappings.len(), 3);
    assert_eq!(
        (
            mappings[0].source_byte_range.start,
            mappings[0].source_byte_range.end
        ),
        (0, 1)
    );
    assert_eq!(
        (
            mappings[1].source_byte_range.start,
            mappings[1].source_byte_range.end
        ),
        (1, 5)
    );
    assert_eq!(
        (
            mappings[2].source_byte_range.start,
            mappings[2].source_byte_range.end
        ),
        (5, 6)
    );
    assert_eq!(mappings[1].decoded_utf8_byte_offset, 1);
    assert_eq!(mappings[2].decoded_utf8_byte_offset, 5);
    assert_eq!(mappings[1].utf16_offset, 1);
    assert_eq!(mappings[2].utf16_offset, 3);
    assert_eq!(result.scan.characters[2].utf8_byte_offset, 5);

    Ok(())
}

#[test]
fn utf8_bom_is_metadata_and_not_a_text_character() -> Result<(), TextIngestionError> {
    let bytes = [0xEF, 0xBB, 0xBF, b'A'];
    let result = analyze_text_bytes(&bytes, TextSourceKind::PlainText)?;

    assert_eq!(result.decode.encoding, SourceEncoding::Utf8);
    assert_eq!(
        result.decode.encoding_evidence,
        EncodingEvidence::ByteOrderMark
    );
    assert_eq!(result.decode.bom, BomKind::Utf8);
    assert_eq!(result.decode.decoded_text, "A");
    assert_eq!(result.scan.characters.len(), 1);
    assert_eq!(result.scan.characters[0].code_point, u32::from(b'A'));
    assert_eq!(result.decode.source_mappings[0].source_byte_range.start, 3);
    assert!(result.scan.findings.is_empty());

    Ok(())
}

#[test]
fn utf16_little_endian_decodes_portuguese_and_surrogate_pairs() -> Result<(), TextIngestionError> {
    let bytes = utf16_bytes("Olá 😀", true);
    let result = analyze_text_bytes(&bytes, TextSourceKind::PlainText)?;

    assert_eq!(result.decode.encoding, SourceEncoding::Utf16LittleEndian);
    assert_eq!(result.decode.bom, BomKind::Utf16LittleEndian);
    assert_eq!(result.decode.decoded_text, "Olá 😀");
    let emoji = &result.decode.source_mappings[4];
    assert_eq!(
        (emoji.source_byte_range.start, emoji.source_byte_range.end),
        (10, 14)
    );
    assert_eq!(emoji.decoded_utf8_byte_offset, 5);
    assert_eq!(emoji.utf16_offset, 4);

    Ok(())
}

#[test]
fn utf16_big_endian_decodes_portuguese_and_surrogate_pairs() -> Result<(), TextIngestionError> {
    let bytes = utf16_bytes("Olá 😀", false);
    let result = analyze_text_bytes(&bytes, TextSourceKind::Markdown)?;

    assert_eq!(result.decode.encoding, SourceEncoding::Utf16BigEndian);
    assert_eq!(result.decode.bom, BomKind::Utf16BigEndian);
    assert_eq!(result.decode.decoded_text, "Olá 😀");
    let emoji = &result.decode.source_mappings[4];
    assert_eq!(
        (emoji.source_byte_range.start, emoji.source_byte_range.end),
        (10, 14)
    );
    assert_eq!(emoji.decoded_utf8_byte_offset, 5);
    assert_eq!(emoji.utf16_offset, 4);

    Ok(())
}

#[test]
fn invalid_utf8_is_rejected_without_replacement() {
    assert_eq!(
        analyze_text_bytes(&[0xC3, 0x28], TextSourceKind::PlainText),
        Err(TextIngestionError::InvalidUtf8)
    );
    assert_eq!(
        analyze_text_bytes(&[0x00, 0xFF, 0x02], TextSourceKind::PlainText),
        Err(TextIngestionError::InvalidUtf8)
    );
}

#[test]
fn invalid_utf16_lengths_are_rejected() {
    assert_eq!(
        analyze_text_bytes(&[0xFF, 0xFE, 0x41], TextSourceKind::PlainText),
        Err(TextIngestionError::InvalidUtf16)
    );
    assert_eq!(
        analyze_text_bytes(&[0xFE, 0xFF, 0x00], TextSourceKind::PlainText),
        Err(TextIngestionError::InvalidUtf16)
    );
}

#[test]
fn invalid_utf16_surrogates_are_rejected() {
    let cases: [&[u8]; 4] = [
        &[0xFF, 0xFE, 0x00, 0xD8],
        &[0xFF, 0xFE, 0x00, 0xDC],
        &[0xFF, 0xFE, 0x00, 0xD8, 0x41, 0x00],
        &[0xFE, 0xFF, 0xD8, 0x00, 0x00, 0x41],
    ];

    for bytes in cases {
        assert_eq!(
            analyze_text_bytes(bytes, TextSourceKind::PlainText),
            Err(TextIngestionError::InvalidUtf16)
        );
    }
}

#[test]
fn bomless_utf16_patterns_are_ambiguous_but_weak_nul_samples_are_not() {
    for bytes in [b"A\0B\0".as_slice(), b"\0A\0B".as_slice()] {
        assert_eq!(
            analyze_text_bytes(bytes, TextSourceKind::PlainText),
            Err(TextIngestionError::AmbiguousEncoding)
        );
    }

    for bytes in [b"A\0".as_slice(), b"A\0B".as_slice()] {
        let result = analyze_text_bytes(bytes, TextSourceKind::PlainText)
            .expect("an incomplete pattern remains strict valid UTF-8");
        assert_eq!(result.decode.decoded_text.as_bytes(), bytes);
    }
}

#[test]
fn unsupported_utf32_signatures_are_not_misread_as_utf16() {
    for bytes in [[0xFF, 0xFE, 0x00, 0x00], [0x00, 0x00, 0xFE, 0xFF]] {
        assert_eq!(
            analyze_text_bytes(&bytes, TextSourceKind::PlainText),
            Err(TextIngestionError::UnsupportedEncoding)
        );
    }
}

#[test]
fn empty_source_and_bom_only_sources_are_valid() -> Result<(), TextIngestionError> {
    let cases: [(&[u8], SourceEncoding, BomKind); 4] = [
        (&[], SourceEncoding::Utf8, BomKind::None),
        (&[0xEF, 0xBB, 0xBF], SourceEncoding::Utf8, BomKind::Utf8),
        (
            &[0xFF, 0xFE],
            SourceEncoding::Utf16LittleEndian,
            BomKind::Utf16LittleEndian,
        ),
        (
            &[0xFE, 0xFF],
            SourceEncoding::Utf16BigEndian,
            BomKind::Utf16BigEndian,
        ),
    ];

    for (bytes, encoding, bom) in cases {
        let result = analyze_text_bytes(bytes, TextSourceKind::PlainText)?;
        assert_eq!(result.decode.encoding, encoding);
        assert_eq!(result.decode.bom, bom);
        assert!(result.decode.decoded_text.is_empty());
        assert!(result.decode.source_mappings.is_empty());
        assert!(result.scan.characters.is_empty());
    }

    Ok(())
}

#[test]
fn ingestion_preserves_crlf_until_tcs_is_explicitly_applied()
-> Result<(), Box<dyn std::error::Error>> {
    let result = analyze_text_bytes(b"a\r\nb\r\n", TextSourceKind::PlainText)?;
    assert_eq!(result.decode.decoded_text, "a\r\nb\r\n");
    assert_eq!(result.scan.characters.len(), 6);

    let cleaned = clean_text(&result.decode.decoded_text, CleanProfile::SafeClean)?;
    assert_eq!(cleaned.clean_text, "a\nb\n");
    assert_eq!(cleaned.manifest.transformations.len(), 2);

    Ok(())
}

#[test]
fn scanner_findings_are_created_after_decode() -> Result<(), TextIngestionError> {
    let result = analyze_text_bytes("a\u{200B}b".as_bytes(), TextSourceKind::PlainText)?;

    assert_eq!(result.scan.findings.len(), 1);
    assert_eq!(result.scan.findings[0].code_point, 0x200B);
    assert_eq!(result.decode.source_mappings[1].code_point_index, 1);

    Ok(())
}

#[test]
fn encoding_bom_and_textual_feff_remain_distinct() -> Result<(), TextIngestionError> {
    let only_encoding_bom = [0xEF, 0xBB, 0xBF, b'A'];
    let with_textual_feff = [0xEF, 0xBB, 0xBF, 0xEF, 0xBB, 0xBF, b'A'];

    let without_textual = analyze_text_bytes(&only_encoding_bom, TextSourceKind::PlainText)?;
    let with_textual = analyze_text_bytes(&with_textual_feff, TextSourceKind::PlainText)?;

    assert_eq!(without_textual.decode.decoded_text, "A");
    assert_eq!(with_textual.decode.decoded_text, "\u{FEFF}A");
    assert!(without_textual.scan.findings.is_empty());
    assert_eq!(with_textual.scan.findings.len(), 1);
    assert_eq!(
        with_textual.decode.source_mappings[0]
            .source_byte_range
            .start,
        3
    );

    Ok(())
}

#[test]
fn repeated_analysis_is_deterministic() -> Result<(), TextIngestionError> {
    let bytes = utf16_bytes("A😀B\r\n", true);
    assert_eq!(
        analyze_text_bytes(&bytes, TextSourceKind::Markdown)?,
        analyze_text_bytes(&bytes, TextSourceKind::Markdown)?
    );

    Ok(())
}

#[test]
fn arbitrary_byte_samples_never_panic() {
    let samples: [&[u8]; 13] = [
        &[],
        &[0x00],
        &[0xFF],
        &[0xC0, 0xAF],
        &[0xEF, 0xBB, 0xBF, 0xFF],
        &[0xFF, 0xFE],
        &[0xFE, 0xFF],
        &[0xFF, 0xFE, 0x00],
        &[0xFE, 0xFF, 0xD8, 0x00],
        &[0x41, 0x00, 0x42, 0x00],
        &[0x00, 0x41, 0x00, 0x42],
        &[0xF0, 0x9F, 0x98, 0x80],
        &[0x7F, 0x00, 0x01, 0x02],
    ];

    for bytes in samples {
        let _ = analyze_text_bytes(bytes, TextSourceKind::PlainText);
        let _ = analyze_text_bytes(bytes, TextSourceKind::Markdown);
    }
}
