use serde_json::{Value, json};
use std::io::{Cursor, Write};
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

const WORD_NS: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

fn minimal_docx(extra_entries: &[(&str, &str)]) -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    let entries = [
        (
            "[Content_Types].xml",
            r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#,
        ),
        (
            "_rels/.rels",
            r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#,
        ),
        (
            "word/document.xml",
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>Boundary body</w:t></w:r></w:p></w:body></w:document>"#,
        ),
    ];

    for (name, content) in entries.into_iter().chain(extra_entries.iter().copied()) {
        writer
            .start_file(name, options)
            .expect("start test ZIP entry");
        writer
            .write_all(content.as_bytes())
            .expect("write test ZIP entry");
    }

    writer.finish().expect("finish test ZIP").into_inner()
}

fn scan_contract(text: &str) -> Result<Value, serde_json::Error> {
    let serialized = super::contract::scan_text_json(text)?;
    serde_json::from_str(&serialized)
}

fn clean_contract(
    text: &str,
    profile: textscope_core::CleanProfile,
) -> Result<Value, super::contract::CleanJsonError> {
    let serialized = super::contract::clean_text_json(text, profile)?;
    serde_json::from_str(&serialized).map_err(super::contract::CleanJsonError::Serialize)
}

fn source_contract(bytes: &[u8], source_kind: &str) -> Result<Value, serde_json::Error> {
    let serialized = super::contract::analyze_text_bytes_json(bytes, source_kind)?;
    serde_json::from_str(&serialized)
}

fn html_contract(bytes: &[u8]) -> Result<Value, serde_json::Error> {
    let serialized = super::contract::analyze_html_bytes_json(bytes)?;
    serde_json::from_str(&serialized)
}

fn clipboard_compare_contract(
    plain_text: Option<&str>,
    html_text: Option<&str>,
) -> Result<Value, serde_json::Error> {
    let serialized =
        super::contract::compare_clipboard_representations_json(plain_text, html_text)?;
    serde_json::from_str(&serialized)
}

#[test]
fn clipboard_comparison_contract_is_rust_owned_and_deterministic() -> Result<(), serde_json::Error>
{
    assert_eq!(
        clipboard_compare_contract(Some("A😀B"), Some("A😀B"))?["state"],
        "equivalent"
    );
    assert_eq!(
        clipboard_compare_contract(Some("A\r\nB"), Some("A\nB"))?["state"],
        "technically_different"
    );
    assert_eq!(
        clipboard_compare_contract(Some("A"), Some("B"))?["state"],
        "content_different"
    );
    assert_eq!(
        clipboard_compare_contract(Some("A"), None)?["state"],
        "needs_review"
    );
    Ok(())
}

fn docx_contract(bytes: &[u8]) -> Result<Value, serde_json::Error> {
    let serialized = super::contract::analyze_docx_bytes_json(bytes)?;
    serde_json::from_str(&serialized)
}

#[test]
fn reveal_uses_core_placeholder_for_zero_width_space() {
    assert_eq!(super::reveal_text("a\u{200b}b"), "a⟦ZWSP⟧b");
}

#[test]
fn contract_preserves_all_character_fields_and_offsets() -> Result<(), serde_json::Error> {
    let result = scan_contract("A😀B")?;

    assert_eq!(result["characters"][0]["code_point_index"], 0);
    assert_eq!(result["characters"][0]["utf16_offset"], 0);
    assert_eq!(result["characters"][0]["utf8_byte_offset"], 0);
    assert_eq!(result["characters"][1]["code_point_index"], 1);
    assert_eq!(result["characters"][1]["utf16_offset"], 1);
    assert_eq!(result["characters"][1]["utf8_byte_offset"], 1);
    assert_eq!(result["characters"][2]["code_point_index"], 2);
    assert_eq!(result["characters"][2]["utf16_offset"], 3);
    assert_eq!(result["characters"][2]["utf8_byte_offset"], 5);

    assert_eq!(
        result["characters"][1],
        json!({
            "code_point_index": 1,
            "utf16_offset": 1,
            "utf8_byte_offset": 1,
            "character": "😀",
            "code_point": 0x1F600,
            "code_point_label": "U+1F600 GRINNING FACE",
            "utf8_bytes": [240, 159, 152, 128],
            "unicode_category": "So",
            "script": "Common",
            "is_whitespace": false,
            "is_control": false,
            "is_format": false,
            "is_invisible": false,
            "is_unusual_whitespace": false,
            "is_bidi_control": false,
            "is_variation_selector": false
        })
    );

    Ok(())
}

#[test]
fn contract_preserves_bidirectional_finding() -> Result<(), serde_json::Error> {
    let result = scan_contract("a\u{202e}b")?;

    assert_eq!(result["characters"][1]["is_bidi_control"], true);
    assert_eq!(result["findings"][0]["kind"], "bidirectional_control");
    assert_eq!(result["findings"][0]["severity"], "caution");
    assert_eq!(result["findings"][0]["code_point"], 0x202E);

    Ok(())
}

#[test]
fn contract_preserves_conservative_emoji_joiner_observation() -> Result<(), serde_json::Error> {
    let result = scan_contract("👩\u{200d}💻")?;

    assert_eq!(result["characters"][1]["code_point"], 0x200D);
    assert_eq!(result["characters"][1]["is_invisible"], true);
    assert_eq!(result["findings"][0]["kind"], "invisible_character");
    assert_eq!(result["findings"][0]["severity"], "informational");

    Ok(())
}

#[test]
fn contract_preserves_normalization_inspection() -> Result<(), serde_json::Error> {
    let result = scan_contract("e\u{301}")?;

    assert_eq!(
        result["normalization"],
        json!({
            "nfc_would_change": true,
            "nfd_would_change": false,
            "nfkc_would_change": true,
            "nfkd_would_change": false
        })
    );

    Ok(())
}

#[test]
fn empty_input_has_a_valid_contract() -> Result<(), serde_json::Error> {
    let result = scan_contract("")?;

    assert_eq!(result["characters"], json!([]));
    assert_eq!(result["findings"], json!([]));
    assert_eq!(
        result["normalization"],
        json!({
            "nfc_would_change": false,
            "nfd_would_change": false,
            "nfkc_would_change": false,
            "nfkd_would_change": false
        })
    );

    Ok(())
}

#[test]
fn normal_portuguese_text_gains_no_boundary_findings() -> Result<(), serde_json::Error> {
    let result = scan_contract("Olá, ação e coração.")?;

    assert_eq!(result["findings"], json!([]));

    Ok(())
}

#[test]
fn tcs_version_and_profile_metadata_cross_the_boundary() -> Result<(), serde_json::Error> {
    assert_eq!(super::tcs_version(), "0.1.0");

    let serialized = super::contract::clean_profiles_json()?;
    let profiles: Value = serde_json::from_str(&serialized)?;
    assert_eq!(profiles.as_array().map(Vec::len), Some(5));
    assert_eq!(profiles[0]["id"], "safe_clean");
    assert_eq!(profiles[0]["supported"], true);
    assert_eq!(
        profiles[0]["active_rule_ids"],
        json!(["TCS-LINE-ENDINGS-001"])
    );
    assert_eq!(profiles[3]["id"], "document_clean");
    assert_eq!(profiles[3]["supported"], false);
    assert_eq!(profiles[4]["id"], "developer_clean");
    assert_eq!(profiles[4]["supported"], false);

    Ok(())
}

#[test]
fn canonical_clean_contract_contains_manifest_diff_and_verification()
-> Result<(), super::contract::CleanJsonError> {
    let result = clean_contract(
        "cafe\u{301}\r\n",
        textscope_core::CleanProfile::CanonicalClean,
    )?;

    assert_eq!(result["original_text"], "cafe\u{301}\r\n");
    assert_eq!(result["clean_text"], "café\n");
    assert_eq!(result["profile"], "canonical_clean");
    assert_eq!(result["tcs_version"], "0.1.0");
    assert_eq!(result["manifest"]["tcs_version"], "0.1.0");
    assert_eq!(
        result["manifest"]["transformations"]
            .as_array()
            .map(Vec::len),
        Some(2)
    );
    assert_eq!(
        result["manifest"]["transformations"][0]["rule_id"],
        "TCS-NORMALIZATION-001"
    );
    assert_eq!(
        result["manifest"]["transformations"][1]["rule_id"],
        "TCS-LINE-ENDINGS-001"
    );
    assert_eq!(result["technical_diff"]["changed"], true);
    assert_eq!(result["content_preservation_diff"]["status"], "preserved");
    assert_eq!(result["verification"]["verification_status"], "verified");
    assert_eq!(result["verification"]["checks"]["deterministic"], true);
    assert_eq!(result["verification"]["checks"]["idempotent"], true);

    Ok(())
}

#[test]
fn pure_plain_text_nbsp_contract_requires_review() -> Result<(), super::contract::CleanJsonError> {
    let result = clean_contract(
        "Olá\u{00A0}mundo",
        textscope_core::CleanProfile::PurePlainText,
    )?;

    assert_eq!(result["clean_text"], "Olá mundo");
    assert_eq!(
        result["manifest"]["transformations"][0]["rule_id"],
        "TCS-WHITESPACE-001"
    );
    assert_eq!(
        result["manifest"]["transformations"][0]["operation"],
        "replace_non_breaking_space"
    );
    assert_eq!(
        result["content_preservation_diff"]["status"],
        "needs_review"
    );
    assert_eq!(
        result["verification"]["verification_status"],
        "needs_review"
    );
    assert_eq!(result["verification"]["remaining_findings"], json!([]));

    Ok(())
}

#[test]
fn clean_contract_preserves_zwj_and_bidi_findings() -> Result<(), super::contract::CleanJsonError> {
    let input = "👩\u{200D}💻\u{202E}";
    let result = clean_contract(input, textscope_core::CleanProfile::CanonicalClean)?;

    assert_eq!(result["clean_text"], input);
    assert_eq!(result["manifest"]["transformations"], json!([]));
    assert_eq!(
        result["verification"]["verification_status"],
        "verified_with_remaining_findings"
    );
    assert_eq!(
        result["verification"]["remaining_findings"]
            .as_array()
            .map(Vec::len),
        Some(2)
    );

    Ok(())
}

#[test]
fn reserved_profiles_are_rejected_without_a_boundary_rule() {
    let error = super::contract::clean_text_json(
        "sensitive input",
        textscope_core::CleanProfile::DocumentClean,
    )
    .expect_err("reserved profile must fail");

    assert!(matches!(
        error,
        super::contract::CleanJsonError::Clean(textscope_core::CleanError::UnsupportedProfile)
    ));
    assert!(!error.to_string().contains("sensitive input"));
}

#[test]
fn byte_contract_preserves_utf8_mapping_and_bom_metadata() -> Result<(), serde_json::Error> {
    let bytes = [0xEF, 0xBB, 0xBF, b'A', 0xF0, 0x9F, 0x98, 0x80, b'B'];
    let result = source_contract(&bytes, "plain_text")?;

    assert_eq!(result["status"], "success");
    assert_eq!(result["analysis"]["raw_byte_length"], 9);
    assert_eq!(result["analysis"]["decode"]["encoding"], "utf_8");
    assert_eq!(result["analysis"]["decode"]["bom"], "utf_8");
    assert_eq!(result["analysis"]["decode"]["decoded_text"], "A😀B");
    assert_eq!(
        result["analysis"]["decode"]["source_mappings"][1]["source_byte_range"],
        json!({ "start": 4, "end": 8 })
    );
    assert_eq!(
        result["analysis"]["decode"]["source_mappings"][2]["utf16_offset"],
        3
    );

    Ok(())
}

#[test]
fn byte_contract_decodes_utf16_little_endian() -> Result<(), serde_json::Error> {
    let bytes = [0xFF, 0xFE, 0x41, 0x00, 0x3D, 0xD8, 0x00, 0xDE, 0x42, 0x00];
    let result = source_contract(&bytes, "plain_text")?;

    assert_eq!(result["status"], "success");
    assert_eq!(result["analysis"]["decode"]["encoding"], "utf_16_le");
    assert_eq!(result["analysis"]["decode"]["decoded_text"], "A😀B");
    assert_eq!(
        result["analysis"]["decode"]["source_mappings"][1]["source_byte_range"],
        json!({ "start": 4, "end": 8 })
    );

    Ok(())
}

#[test]
fn byte_contract_decodes_utf16_big_endian() -> Result<(), serde_json::Error> {
    let bytes = [0xFE, 0xFF, 0x00, 0x41, 0xD8, 0x3D, 0xDE, 0x00, 0x00, 0x42];
    let result = source_contract(&bytes, "markdown")?;

    assert_eq!(result["status"], "success");
    assert_eq!(result["analysis"]["source_kind"], "markdown");
    assert_eq!(result["analysis"]["decode"]["encoding"], "utf_16_be");
    assert_eq!(result["analysis"]["decode"]["decoded_text"], "A😀B");

    Ok(())
}

#[test]
fn byte_contract_returns_controlled_encoding_errors() -> Result<(), serde_json::Error> {
    let invalid_utf8 = source_contract(&[0xC3, 0x28], "plain_text")?;
    let ambiguous = source_contract(b"A\0B\0", "plain_text")?;

    assert_eq!(invalid_utf8["status"], "error");
    assert_eq!(invalid_utf8["error"]["code"], "invalid_utf8");
    assert_eq!(ambiguous["status"], "error");
    assert_eq!(ambiguous["error"]["code"], "ambiguous_encoding");

    Ok(())
}

#[test]
fn byte_contract_accepts_empty_uint8array_equivalent() -> Result<(), serde_json::Error> {
    let result = source_contract(&[], "plain_text")?;

    assert_eq!(result["status"], "success");
    assert_eq!(result["analysis"]["raw_byte_length"], 0);
    assert_eq!(
        result["analysis"]["decode"]["encoding_evidence"],
        "empty_input"
    );
    assert_eq!(result["analysis"]["decode"]["decoded_text"], "");
    assert_eq!(result["analysis"]["scan"]["characters"], json!([]));

    Ok(())
}

#[test]
fn byte_contract_rejects_unknown_source_kind_without_reading_content()
-> Result<(), serde_json::Error> {
    let sensitive = b"private file content";
    let result = source_contract(sensitive, "html")?;

    assert_eq!(result["status"], "error");
    assert_eq!(result["error"]["code"], "unsupported_file_type");
    assert!(!result.to_string().contains("private file content"));

    Ok(())
}

#[test]
fn html_byte_contract_returns_structure_metadata_and_utf8_evidence() -> Result<(), serde_json::Error>
{
    let result = html_contract(
        br#"<!doctype html><html lang="pt-BR"><head><meta charset="utf-8"><title>Relatorio</title></head><body>Texto</body></html>"#,
    )?;

    assert_eq!(result["status"], "success");
    assert_eq!(result["analysis"]["decode"]["encoding"], "utf_8");
    assert_eq!(result["analysis"]["metadata"]["title"], "Relatorio");
    assert_eq!(result["analysis"]["metadata"]["lang"], "pt-BR");
    assert_eq!(result["analysis"]["summary"]["doctype"]["name"], "html");
    assert_eq!(
        result["analysis"]["encoding"]["declarations"][0]["kind"],
        "meta_charset"
    );
    assert_eq!(
        result["analysis"]["text_channels"]["document_text"],
        "Texto"
    );

    Ok(())
}

#[test]
fn html_contract_decodes_entities_into_scan_input() -> Result<(), serde_json::Error> {
    let result = html_contract(b"<p>A&amp;B&nbsp;C&#x200B;D</p>")?;

    assert_eq!(
        result["analysis"]["text_channels"]["document_text"],
        "A&B\u{00A0}C\u{200B}D"
    );
    assert_eq!(
        result["analysis"]["text_segments"][0]["source_text"],
        "A&amp;B&nbsp;C&#x200B;D"
    );
    assert_eq!(
        result["analysis"]["scan"]["findings"]
            .as_array()
            .map(Vec::len),
        Some(2)
    );

    Ok(())
}

#[test]
fn html_contract_keeps_script_outside_document_text() -> Result<(), serde_json::Error> {
    let result = html_contract(b"<body>A<script>window.executed=true</script>B</body>")?;

    assert_eq!(result["analysis"]["text_channels"]["document_text"], "AB");
    assert_eq!(
        result["analysis"]["text_channels"]["active_content_text"],
        "window.executed=true"
    );
    assert_eq!(
        result["analysis"]["findings"][0]["kind"],
        "active_script_present"
    );

    Ok(())
}

#[test]
fn html_contract_keeps_comments_separate_and_mapped() -> Result<(), serde_json::Error> {
    let result = html_contract(b"<p>A<!-- note -->B</p>")?;

    assert_eq!(result["analysis"]["comments"][0]["text"], " note ");
    assert_eq!(result["analysis"]["text_channels"]["document_text"], "AB");
    assert_eq!(
        result["analysis"]["comments"][0]["source_range"]["raw_source_byte_range"],
        json!({ "start": 4, "end": 17 })
    );

    Ok(())
}

#[test]
fn html_contract_returns_controlled_invalid_and_declared_encoding_errors()
-> Result<(), serde_json::Error> {
    let invalid = html_contract(&[0xC3, 0x28])?;
    let unsupported = html_contract(b"<meta charset=windows-1252><p>A</p>")?;

    assert_eq!(invalid["status"], "error");
    assert_eq!(invalid["error"]["code"], "invalid_utf8");
    assert_eq!(unsupported["status"], "error");
    assert_eq!(
        unsupported["error"]["code"],
        "unsupported_declared_encoding"
    );
    assert_eq!(unsupported["error"]["resource_limit"], Value::Null);

    Ok(())
}

#[test]
fn html_contract_accepts_empty_uint8array_equivalent() -> Result<(), serde_json::Error> {
    let result = html_contract(&[])?;

    assert_eq!(result["status"], "success");
    assert_eq!(result["analysis"]["raw_byte_length"], 0);
    assert_eq!(result["analysis"]["text_channels"]["document_text"], "");
    assert_eq!(result["analysis"]["scan"]["characters"], json!([]));

    Ok(())
}

#[test]
fn docx_boundary_preserves_package_body_metadata_relationships_and_comments()
-> Result<(), serde_json::Error> {
    let metadata = r#"<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:creator>Declared Person</dc:creator></cp:coreProperties>"#;
    let relationships = r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rLink" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://example.invalid/never" TargetMode="External"/></Relationships>"#;
    let comments = format!(
        "<w:comments xmlns:w=\"{WORD_NS}\"><w:comment w:id=\"7\" w:author=\"Declared Reviewer\"><w:p><w:r><w:t>Separate comment</w:t></w:r></w:p></w:comment></w:comments>"
    );
    let bytes = minimal_docx(&[
        ("docProps/core.xml", metadata),
        ("word/_rels/document.xml.rels", relationships),
        ("word/comments.xml", &comments),
    ]);
    let result = docx_contract(&bytes)?;

    assert_eq!(result["status"], "success");
    assert_eq!(
        result["analysis"]["package"]["status"],
        "valid_wordprocessing_package"
    );
    assert_eq!(
        result["analysis"]["text_channels"]["document_text"],
        "Boundary body"
    );
    assert_eq!(
        result["analysis"]["metadata"]["core_properties"][0]["value"],
        "Declared Person"
    );
    assert_eq!(result["analysis"]["relationships"][1]["external"], true);
    assert_eq!(
        result["analysis"]["comments"][0]["text"],
        "Separate comment"
    );
    assert_eq!(
        result["analysis"]["text_channels"]["comments"][0]["text"],
        "Separate comment"
    );

    Ok(())
}

#[test]
fn docx_boundary_returns_controlled_invalid_zip_error() -> Result<(), serde_json::Error> {
    let result = docx_contract(b"not a ZIP")?;

    assert_eq!(result["status"], "error");
    assert_eq!(result["error"]["code"], "invalid_zip");
    assert_eq!(result["error"]["resource_limit"], Value::Null);

    Ok(())
}

#[test]
fn docx_boundary_reports_source_size_resource_limit() -> Result<(), serde_json::Error> {
    let bytes = vec![0_u8; textscope_core::MAX_DOCX_SOURCE_BYTES + 1];
    let result = docx_contract(&bytes)?;

    assert_eq!(result["status"], "error");
    assert_eq!(result["error"]["code"], "resource_limit_exceeded");
    assert_eq!(result["error"]["resource_limit"], "source_bytes");

    Ok(())
}

#[test]
fn docx_boundary_rejects_empty_uint8array_without_panicking() -> Result<(), serde_json::Error> {
    let result = docx_contract(&[])?;

    assert_eq!(result["status"], "error");
    assert_eq!(result["error"]["code"], "invalid_zip");

    Ok(())
}
