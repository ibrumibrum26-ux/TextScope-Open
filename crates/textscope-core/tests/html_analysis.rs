use textscope_core::{
    BomKind, CleanProfile, EvidenceKind, EvidenceSource, HtmlAnalysisError,
    HtmlEncodingDeclarationKind, HtmlFindingKind, HtmlReferenceKind, HtmlResourceLimit,
    HtmlTextClassification, MAX_HTML_ATTRIBUTE_VALUE_BYTES, MAX_HTML_COMMENT_BYTES,
    MAX_HTML_NESTING_DEPTH, MAX_HTML_SOURCE_BYTES, analyze_html_bytes, clean_text,
};

fn analyze(source: &str) -> textscope_core::HtmlAnalysis {
    analyze_html_bytes(source.as_bytes()).expect("HTML analysis should succeed")
}

#[test]
fn generator_and_canonical_metadata_are_unified_observations() {
    let result = analyze(
        r#"<html><head><meta name="generator" content="Example Tool"><link rel="canonical" href="https://example.invalid/doc"></head><body>A😀B</body></html>"#,
    );
    assert!(result.provenance.iter().any(|record| {
        record.kind == EvidenceKind::GeneratorDeclaration
            && record.source == EvidenceSource::HtmlGenerator
            && record.verification == textscope_core::VerificationState::Declared
            && record.value == "Example Tool"
    }));
    assert!(result.provenance.iter().any(|record| {
        record.kind == EvidenceKind::CanonicalDeclaration
            && record.source == EvidenceSource::HtmlCanonicalLink
    }));
}

fn utf16_le_html(source: &str) -> Vec<u8> {
    let mut bytes = vec![0xFF, 0xFE];
    for unit in source.encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    bytes
}

#[test]
fn a_minimal_html_reports_doctype_structure_and_document_text() {
    let result = analyze("<!doctype html><html><body>Hello</body></html>");

    assert_eq!(result.text_channels.document_text, "Hello");
    assert_eq!(
        result
            .summary
            .doctype
            .as_ref()
            .map(|doctype| doctype.name.as_str()),
        Some("html")
    );
    assert!(result.summary.html_present);
    assert!(result.summary.head_present);
    assert!(result.summary.body_present);
    assert_eq!(result.summary.source_start_tags, 2);
    assert_eq!(result.summary.parsed_elements, 3);
}

#[test]
fn b_portuguese_text_is_preserved() {
    let result = analyze("<p>Olá, ação e coração.</p>");

    assert_eq!(result.text_channels.document_text, "Olá, ação e coração.");
    assert!(result.scan.findings.is_empty());
}

#[test]
fn c_emoji_and_multibyte_unicode_are_preserved() {
    let result = analyze("<p>Olá 👩‍💻 😀</p>");

    assert_eq!(result.text_channels.document_text, "Olá 👩‍💻 😀");
    assert!(
        result
            .scan
            .characters
            .iter()
            .any(|record| record.character == '😀')
    );
}

#[test]
fn d_entities_decode_to_unicode_with_source_representation() {
    let result = analyze("<p>A&amp;B&nbsp;C&#x200B;D&#8203;E&lt;F</p>");
    let segment = result
        .text_segments
        .iter()
        .find(|segment| {
            segment.classification == HtmlTextClassification::VisibleContentCandidate
                && segment.text.contains("A&B")
        })
        .expect("entity text segment");

    assert_eq!(
        result.text_channels.document_text,
        "A&B\u{00A0}C\u{200B}D\u{200B}E<F"
    );
    assert_eq!(
        segment.source_text.as_deref(),
        Some("A&amp;B&nbsp;C&#x200B;D&#8203;E&lt;F")
    );
    assert!(segment.source_range.is_some());
}

#[test]
fn e_metadata_is_observed_without_attribution() {
    let result = analyze(
        r#"<!doctype html><html lang="pt-BR" dir="ltr"><head>
        <title>Relatório &amp; notas</title>
        <meta charset="utf-8">
        <meta name="author" content="Alice">
        <meta property="og:title" content="Declaração">
        <link rel="canonical alternate" href="https://example.invalid/doc">
        <base href="/base/">
        </head><body>Conteúdo</body></html>"#,
    );

    assert_eq!(result.metadata.title.as_deref(), Some("Relatório & notas"));
    assert_eq!(result.metadata.lang.as_deref(), Some("pt-BR"));
    assert_eq!(result.metadata.dir.as_deref(), Some("ltr"));
    assert_eq!(result.metadata.meta_declarations.len(), 3);
    assert_eq!(
        result.metadata.meta_declarations[1].name.as_deref(),
        Some("author")
    );
    assert_eq!(
        result.metadata.meta_declarations[2].property.as_deref(),
        Some("og:title")
    );
    assert_eq!(
        result.metadata.canonical_links,
        ["https://example.invalid/doc"]
    );
    assert_eq!(result.metadata.base_hrefs, ["/base/"]);
    assert!(result.text_channels.metadata_text.contains("Alice"));
}

#[test]
fn f_comments_are_separate_and_source_mapped() {
    let result = analyze("<body>A<!-- private note -->B</body>");

    assert_eq!(result.comments.len(), 1);
    assert_eq!(result.comments[0].text, " private note ");
    assert_eq!(result.text_channels.document_text, "AB");
    assert!(!result.text_channels.document_text.contains("private note"));
    assert!(
        result.comments[0]
            .source_range
            .raw_source_byte_range
            .is_some()
    );
}

#[test]
fn g_script_is_detected_and_excluded_from_document_text() {
    let source = "<body>Before<script>window.__ran = true;</script>After</body>";
    let result = analyze(source);

    assert_eq!(result.text_channels.document_text, "BeforeAfter");
    assert_eq!(
        result.text_channels.active_content_text,
        "window.__ran = true;"
    );
    assert_eq!(result.summary.script_blocks, 1);
    assert!(
        result
            .findings
            .iter()
            .any(|finding| { finding.kind == HtmlFindingKind::ActiveScriptPresent })
    );
}

#[test]
fn h_style_is_preserved_only_in_its_technical_channel() {
    let result = analyze("<style>p { display: none; }</style><body>Text</body>");

    assert_eq!(result.text_channels.document_text, "Text");
    assert_eq!(result.text_channels.style_text, "p { display: none; }");
    assert_eq!(result.summary.style_blocks, 1);
}

#[test]
fn i_recognized_inline_event_handler_is_structurally_detected() {
    let result = analyze(r#"<button onclick="window.__ran=true">Safe label</button>"#);

    assert!(result.findings.iter().any(|finding| {
        finding.kind == HtmlFindingKind::InlineEventHandlerPresent
            && finding.attribute.as_deref() == Some("onclick")
    }));
    assert_eq!(result.text_channels.document_text, "Safe label");
}

#[test]
fn j_external_references_are_classified_without_resolution() {
    let result = analyze(
        r##"<a href="#part">fragment</a>
        <img src="https://example.invalid/a.png">
        <form action="/submit"></form>
        <object data="data:text/plain,hello"></object>"##,
    );

    assert_eq!(result.external_references.len(), 4);
    assert_eq!(
        result.external_references[0].kind,
        HtmlReferenceKind::Fragment
    );
    assert_eq!(
        result.external_references[1].kind,
        HtmlReferenceKind::Absolute
    );
    assert_eq!(
        result.external_references[2].kind,
        HtmlReferenceKind::Relative
    );
    assert_eq!(
        result.external_references[3].kind,
        HtmlReferenceKind::DataUrl
    );
}

#[test]
fn k_only_explicit_hidden_markers_are_reported() {
    let result = analyze(
        r#"<p hidden>A</p><p aria-hidden="true">B</p>
        <p style="display:none">C</p><p style="visibility: hidden">D</p>
        <p class="maybe-hidden">E</p>"#,
    );

    assert_eq!(
        result
            .findings
            .iter()
            .filter(|finding| finding.kind == HtmlFindingKind::ExplicitHiddenMarker)
            .count(),
        4
    );
}

#[test]
fn l_malformed_html_recovers_without_analysis_failure() {
    let result = analyze("<p>one<div>two</p></span>");

    assert!(!result.text_channels.document_text.is_empty());
    assert!(!result.parser_recoveries.is_empty());
    assert!(
        result
            .findings
            .iter()
            .any(|finding| { finding.kind == HtmlFindingKind::ParserRecoveryObserved })
    );
}

#[test]
fn m_empty_html_is_a_valid_empty_document_analysis() {
    let result = analyze_html_bytes(&[]).expect("empty HTML should be valid");

    assert_eq!(result.raw_byte_length, 0);
    assert_eq!(result.text_channels.document_text, "");
    assert!(result.scan.characters.is_empty());
    assert_eq!(result.summary.source_start_tags, 0);
}

#[test]
fn n_utf8_bom_is_evidence_not_document_text() {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(b"<p>A</p>");
    let result = analyze_html_bytes(&bytes).expect("BOM HTML should decode");

    assert_eq!(result.decode.bom, BomKind::Utf8);
    assert_eq!(result.text_channels.document_text, "A");
    assert_eq!(
        result.elements[0]
            .source_range
            .raw_source_byte_range
            .map(|range| range.start),
        Some(3)
    );
}

#[test]
fn o_meta_charset_and_historical_http_equiv_are_observed() {
    let direct = analyze(r#"<meta charset="UTF-8"><p>A</p>"#);
    let historical =
        analyze(r#"<meta http-equiv="Content-Type" content="text/html; charset=utf-8"><p>B</p>"#);

    assert_eq!(direct.encoding.declarations.len(), 1);
    assert_eq!(
        direct.encoding.declarations[0].kind,
        HtmlEncodingDeclarationKind::MetaCharset
    );
    assert_eq!(direct.encoding.declarations[0].normalized_label, "utf-8");
    assert_eq!(
        historical.encoding.declarations[0].kind,
        HtmlEncodingDeclarationKind::HttpEquivContentType
    );
}

#[test]
fn p_bom_takes_precedence_and_conflict_remains_evidence() {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(b"<meta charset=windows-1252><p>A</p>");
    let result = analyze_html_bytes(&bytes).expect("BOM must take precedence");

    assert!(result.encoding.bom_precedence_applied);
    assert!(result.encoding.declaration_conflict);
    assert!(result.findings.iter().any(|finding| {
        finding.kind == HtmlFindingKind::EncodingDeclarationConflict
            && finding.source_range.is_some()
    }));

    let error = analyze_html_bytes(b"<meta charset=windows-1252><p>A</p>")
        .expect_err("unsupported BOM-less declaration must be rejected");
    assert_eq!(error, HtmlAnalysisError::UnsupportedDeclaredEncoding);
}

#[test]
fn charset_declarations_after_the_explicit_prescan_window_remain_metadata_only() {
    let source = format!("{}<meta charset=windows-1252><p>A</p>", " ".repeat(1024));
    let result = analyze_html_bytes(source.as_bytes())
        .expect("late declarations must not override strict UTF-8 evidence");

    assert!(result.encoding.declarations.is_empty());
    assert_eq!(result.metadata.meta_declarations.len(), 1);
    assert_eq!(
        result.metadata.meta_declarations[0].charset.as_deref(),
        Some("windows-1252")
    );
}

#[test]
fn q_decoded_and_raw_ranges_are_distinct_for_utf16_source() {
    let result =
        analyze_html_bytes(&utf16_le_html("<p>😀</p>")).expect("UTF-16 BOM HTML should decode");
    let tag_range = result.elements[0].source_range;

    assert_eq!(tag_range.decoded_utf8_byte_range.start, 0);
    assert_eq!(tag_range.decoded_utf8_byte_range.end, 3);
    assert_eq!(
        tag_range
            .raw_source_byte_range
            .map(|range| (range.start, range.end)),
        Some((2, 8))
    );
    let text = result
        .text_segments
        .iter()
        .find(|segment| segment.text == "😀")
        .expect("emoji segment");
    assert!(text.source_range.is_some());
}

#[test]
fn r_entity_derived_zero_width_space_reaches_unicode_scanner() {
    let result = analyze("<p>A&#x200B;B</p>");

    assert!(
        result
            .scan
            .characters
            .iter()
            .any(|record| record.code_point == 0x200B)
    );
    assert_eq!(result.scan.findings.len(), 1);
}

#[test]
fn s_entity_derived_nbsp_reaches_existing_tcs_without_html_rewrite() {
    let result = analyze("<p>A&nbsp;B</p>");
    let clean = clean_text(
        &result.text_channels.document_text,
        CleanProfile::PurePlainText,
    )
    .expect("existing TCS profile should clean extracted text");

    assert_eq!(result.text_channels.document_text, "A\u{00A0}B");
    assert_eq!(clean.clean_text, "A B");
    assert_eq!(clean.manifest.transformations.len(), 1);
}

#[test]
fn t_same_bytes_produce_identical_analysis() {
    let bytes = b"<!doctype html><title>X</title><p>A&amp;B</p><!--C-->";

    assert_eq!(analyze_html_bytes(bytes), analyze_html_bytes(bytes));
}

#[test]
fn u_pathological_inputs_fail_with_controlled_limits_and_no_truncation() {
    let oversized = vec![b' '; MAX_HTML_SOURCE_BYTES + 1];
    assert_eq!(
        analyze_html_bytes(&oversized),
        Err(HtmlAnalysisError::ResourceLimitExceeded(
            HtmlResourceLimit::SourceBytes
        ))
    );

    let deep = format!(
        "{}x{}",
        "<div>".repeat(MAX_HTML_NESTING_DEPTH + 1),
        "</div>".repeat(MAX_HTML_NESTING_DEPTH + 1)
    );
    assert_eq!(
        analyze_html_bytes(deep.as_bytes()),
        Err(HtmlAnalysisError::ResourceLimitExceeded(
            HtmlResourceLimit::NestingDepth
        ))
    );

    let huge_attribute = format!(
        "<p title=\"{}\">x</p>",
        "a".repeat(MAX_HTML_ATTRIBUTE_VALUE_BYTES + 1)
    );
    assert_eq!(
        analyze_html_bytes(huge_attribute.as_bytes()),
        Err(HtmlAnalysisError::ResourceLimitExceeded(
            HtmlResourceLimit::AttributeValueBytes
        ))
    );

    let huge_comment = format!("<!--{}-->", "a".repeat(MAX_HTML_COMMENT_BYTES + 1));
    assert_eq!(
        analyze_html_bytes(huge_comment.as_bytes()),
        Err(HtmlAnalysisError::ResourceLimitExceeded(
            HtmlResourceLimit::CommentBytes
        ))
    );
}

#[test]
fn block_boundaries_use_documented_deterministic_newlines_without_collapsing_text() {
    let result = analyze("<p>A <span>B</span></p><p>C\tD</p><div>E<br>F</div>");

    assert_eq!(result.text_channels.document_text, "A B\nC\tD\nE\nF");
}

#[test]
fn template_and_noscript_text_remain_explicit_non_document_channels() {
    let result = analyze("<body>A<template>T</template><noscript>N</noscript>B</body>");

    assert_eq!(result.text_channels.document_text, "AB");
    assert_eq!(result.text_channels.template_text, "T");
    assert_eq!(result.text_channels.noscript_text, "N");
}
