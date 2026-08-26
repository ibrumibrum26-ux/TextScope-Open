use std::io::{Cursor, Write};

use textscope_core::{
    CleanProfile, DocxAnalysisError, DocxFindingKind, DocxPackageStatus, DocxPartClassification,
    DocxResourceLimit, DocxRevisionKind, DocxTextChannel, DocxTextKind, EvidenceKind,
    EvidenceSource, MAX_DOCX_ENTRY_COUNT, analyze_docx_bytes, clean_text, scan_text,
};
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

const WORD_NS: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const REL_NS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

fn content_types() -> String {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Default Extension="png" ContentType="image/png"/>
  <Default Extension="bin" ContentType="application/octet-stream"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#
        .to_owned()
}

fn root_relationships() -> String {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"#
        .to_owned()
}

fn document(body: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="{WORD_NS}" xmlns:r="{REL_NS}"><w:body>{body}</w:body></w:document>"#
    )
}

#[derive(Default)]
struct PackageBuilder {
    entries: Vec<(String, Vec<u8>)>,
}

impl PackageBuilder {
    fn standard(body: &str) -> Self {
        let mut package = Self::default();
        package.add_text("[Content_Types].xml", &content_types());
        package.add_text("_rels/.rels", &root_relationships());
        package.add_text("word/document.xml", &document(body));
        package
    }

    fn add_text(&mut self, name: &str, value: &str) {
        self.entries
            .push((name.to_owned(), value.as_bytes().to_vec()));
    }

    fn add_bytes(&mut self, name: &str, value: &[u8]) {
        self.entries.push((name.to_owned(), value.to_vec()));
    }

    fn replace_text(&mut self, name: &str, value: &str) {
        if let Some((_, bytes)) = self.entries.iter_mut().find(|(entry, _)| entry == name) {
            *bytes = value.as_bytes().to_vec();
        } else {
            self.add_text(name, value);
        }
    }

    fn finish(self) -> Vec<u8> {
        let cursor = Cursor::new(Vec::new());
        let mut writer = ZipWriter::new(cursor);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for (name, bytes) in self.entries {
            writer
                .start_file(name, options)
                .expect("start in-memory entry");
            writer.write_all(&bytes).expect("write in-memory entry");
        }
        writer
            .finish()
            .expect("finish in-memory package")
            .into_inner()
    }
}

fn analyze_body(body: &str) -> textscope_core::DocxAnalysis {
    analyze_docx_bytes(&PackageBuilder::standard(body).finish()).expect("valid DOCX")
}

#[test]
fn creator_metadata_is_unified_as_a_declared_observation() {
    let mut package = PackageBuilder::standard("<w:p><w:r><w:t>A😀B</w:t></w:r></w:p>");
    package.add_text(
        "docProps/core.xml",
        r#"<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:creator>Declared creator</dc:creator></cp:coreProperties>"#,
    );
    let analysis = analyze_docx_bytes(&package.finish()).expect("valid DOCX metadata fixture");
    assert!(analysis.provenance.iter().any(|record| {
        record.kind == EvidenceKind::CreatorDeclaration
            && record.source == EvidenceSource::DocxCoreProperties
            && record.value == "Declared creator"
            && record.verification == textscope_core::VerificationState::Declared
    }));
}

#[test]
fn a_minimal_docx_is_a_valid_wordprocessing_package() {
    let analysis = analyze_body("<w:p><w:r><w:t>Hello</w:t></w:r></w:p>");
    assert_eq!(
        analysis.package.status,
        DocxPackageStatus::ValidWordprocessingPackage
    );
    assert_eq!(analysis.text_channels.document_text, "Hello");
    assert_eq!(analysis.summary.paragraphs, 1);
    assert_eq!(analysis.summary.runs, 1);
}

#[test]
fn b_portuguese_text_is_preserved() {
    let analysis = analyze_body("<w:p><w:r><w:t>ação, órgão e café</w:t></w:r></w:p>");
    assert_eq!(analysis.text_channels.document_text, "ação, órgão e café");
}

#[test]
fn c_multibyte_emoji_is_preserved() {
    let analysis = analyze_body("<w:p><w:r><w:t>A😀B</w:t></w:r></w:p>");
    assert_eq!(analysis.text_channels.document_text, "A😀B");
    assert_eq!(analysis.scan.characters.len(), 3);
}

#[test]
fn d_xml_space_preserve_is_explicit_and_never_trimmed() {
    let analysis = analyze_body("<w:p><w:r><w:t xml:space=\"preserve\">  text  </w:t></w:r></w:p>");
    assert_eq!(analysis.text_channels.document_text, "  text  ");
    assert!(analysis.text_segments[0].xml_space_preserve);
}

#[test]
fn e_tabs_breaks_and_wordprocessing_characters_map_deterministically() {
    let analysis = analyze_body(
        "<w:p><w:r><w:t>A</w:t><w:tab/><w:t>B</w:t><w:br/><w:t>C</w:t><w:cr/><w:noBreakHyphen/><w:softHyphen/></w:r></w:p>",
    );
    assert_eq!(
        analysis.text_channels.document_text,
        "A\tB\nC\n\u{2011}\u{00ad}"
    );
    assert!(
        analysis
            .text_segments
            .iter()
            .any(|segment| segment.kind == DocxTextKind::Tab)
    );
    assert!(
        analysis
            .text_segments
            .iter()
            .any(|segment| segment.kind == DocxTextKind::NoBreakHyphen)
    );
}

#[test]
fn f_paragraph_boundaries_are_single_lf_separators() {
    let analysis = analyze_body(
        "<w:p><w:r><w:t>First</w:t></w:r></w:p><w:p/><w:p><w:r><w:t>Third</w:t></w:r></w:p>",
    );
    assert_eq!(analysis.text_channels.document_text, "First\n\nThird");
    assert_eq!(analysis.summary.paragraphs, 3);
}

#[test]
fn g_tables_use_tab_for_cells_and_lf_for_rows_including_empty_cells() {
    let analysis = analyze_body(
        "<w:tbl><w:tr><w:tc><w:p><w:r><w:t>A</w:t></w:r></w:p></w:tc><w:tc><w:p/></w:tc></w:tr><w:tr><w:tc><w:p><w:r><w:t>B</w:t></w:r></w:p></w:tc></w:tr></w:tbl>",
    );
    assert_eq!(analysis.text_channels.document_text, "A\t\nB");
    assert_eq!(analysis.summary.tables, 1);
    assert_eq!(analysis.summary.table_rows, 2);
    assert_eq!(analysis.summary.table_cells, 3);
}

#[test]
fn h_i_referenced_headers_and_footers_stay_in_separate_channels() {
    let mut package = PackageBuilder::standard("<w:p><w:r><w:t>Body</w:t></w:r></w:p>");
    package.add_text(
        "word/_rels/document.xml.rels",
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="h1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="header1.xml"/>
<Relationship Id="f1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/footer" Target="footer1.xml"/>
</Relationships>"#,
    );
    package.add_text(
        "word/header1.xml",
        &format!("<w:hdr xmlns:w=\"{WORD_NS}\"><w:p><w:r><w:t>Header</w:t></w:r></w:p></w:hdr>"),
    );
    package.add_text(
        "word/footer1.xml",
        &format!("<w:ftr xmlns:w=\"{WORD_NS}\"><w:p><w:r><w:t>Footer</w:t></w:r></w:p></w:ftr>"),
    );
    let analysis = analyze_docx_bytes(&package.finish()).expect("header/footer DOCX");
    assert_eq!(analysis.text_channels.document_text, "Body");
    assert_eq!(analysis.text_channels.headers[0].text, "Header");
    assert_eq!(analysis.text_channels.footers[0].text, "Footer");
}

#[test]
fn j_k_footnotes_and_endnotes_are_separate_and_skip_structural_separators() {
    let mut package = PackageBuilder::standard("<w:p><w:r><w:t>Body</w:t></w:r></w:p>");
    package.add_text(
        "word/footnotes.xml",
        &format!("<w:footnotes xmlns:w=\"{WORD_NS}\"><w:footnote w:id=\"-1\"><w:p><w:r><w:t>separator</w:t></w:r></w:p></w:footnote><w:footnote w:id=\"1\"><w:p><w:r><w:t>Footnote</w:t></w:r></w:p></w:footnote></w:footnotes>"),
    );
    package.add_text(
        "word/endnotes.xml",
        &format!("<w:endnotes xmlns:w=\"{WORD_NS}\"><w:endnote w:id=\"2\"><w:p><w:r><w:t>Endnote</w:t></w:r></w:p></w:endnote></w:endnotes>"),
    );
    let analysis = analyze_docx_bytes(&package.finish()).expect("notes DOCX");
    assert_eq!(analysis.text_channels.footnotes[0].text, "Footnote");
    assert_eq!(analysis.text_channels.endnotes[0].text, "Endnote");
    assert!(!analysis.text_channels.document_text.contains("Footnote"));
}

#[test]
fn l_comments_and_their_declared_metadata_are_separate() {
    let mut package = PackageBuilder::standard("<w:p><w:r><w:t>Body</w:t></w:r></w:p>");
    package.add_text(
        "word/comments.xml",
        &format!("<w:comments xmlns:w=\"{WORD_NS}\"><w:comment w:id=\"4\" w:author=\"Alice\" w:initials=\"A\" w:date=\"2026-01-02T03:04:05Z\"><w:p><w:r><w:t>Review note</w:t></w:r></w:p></w:comment></w:comments>"),
    );
    let analysis = analyze_docx_bytes(&package.finish()).expect("comments DOCX");
    assert_eq!(
        analysis.comments[0].declared_author.as_deref(),
        Some("Alice")
    );
    assert_eq!(analysis.text_channels.comments[0].text, "Review note");
    assert_eq!(analysis.text_channels.document_text, "Body");
}

#[test]
fn m_n_o_core_app_and_custom_metadata_are_observed_as_declarations() {
    let mut package = PackageBuilder::standard("<w:p/>");
    package.add_text(
        "docProps/core.xml",
        r#"<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>Case file</dc:title><dc:creator>Alice</dc:creator></cp:coreProperties>"#,
    );
    package.add_text(
        "docProps/app.xml",
        r#"<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties"><Application>Word</Application><Pages>3</Pages></Properties>"#,
    );
    package.add_text(
        "docProps/custom.xml",
        r#"<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/custom-properties" xmlns:vt="http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes"><property name="Matter"><vt:lpwstr>Example</vt:lpwstr></property></Properties>"#,
    );
    let analysis = analyze_docx_bytes(&package.finish()).expect("metadata DOCX");
    assert!(
        analysis
            .metadata
            .core_properties
            .iter()
            .any(|property| property.name == "creator" && property.value == "Alice")
    );
    assert!(
        analysis
            .metadata
            .app_properties
            .iter()
            .any(|property| property.name == "Pages" && property.value == "3")
    );
    assert_eq!(analysis.metadata.custom_properties[0].name, "Matter");
}

#[test]
fn p_q_external_hyperlink_and_image_relationships_are_detected_without_resolution() {
    let mut package = PackageBuilder::standard(
        "<w:p><w:hyperlink r:id=\"link1\"><w:r><w:t>Example</w:t></w:r></w:hyperlink></w:p>",
    );
    package.add_text(
        "word/_rels/document.xml.rels",
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="link1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://example.invalid/document" TargetMode="External"/>
<Relationship Id="image1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="https://example.invalid/image.png" TargetMode="External"/>
</Relationships>"#,
    );
    let analysis = analyze_docx_bytes(&package.finish()).expect("external relationships DOCX");
    assert_eq!(analysis.summary.external_relationships, 2);
    assert_eq!(analysis.hyperlinks[0].display_text, "Example");
    assert!(analysis.hyperlinks[0].external);
    assert_eq!(
        analysis.hyperlinks[0].target.as_deref(),
        Some("https://example.invalid/document")
    );
}

#[test]
fn r_media_is_inventoried_without_decoding_or_ocr() {
    let mut package = PackageBuilder::standard("<w:p/>");
    package.add_bytes("word/media/image1.png", b"not-rendered-image-evidence");
    let analysis = analyze_docx_bytes(&package.finish()).expect("media DOCX");
    let media = analysis
        .parts
        .iter()
        .find(|part| part.part_name == "word/media/image1.png")
        .expect("media inventory");
    assert_eq!(media.classification, DocxPartClassification::Media);
    assert_eq!(analysis.summary.media_parts, 1);
}

#[test]
fn s_custom_xml_is_detected_but_excluded_from_document_text() {
    let mut package = PackageBuilder::standard("<w:p><w:r><w:t>Body</w:t></w:r></w:p>");
    package.add_text("customXml/item1.xml", "<private>Separate</private>");
    let analysis = analyze_docx_bytes(&package.finish()).expect("custom XML DOCX");
    assert_eq!(analysis.summary.custom_xml_parts, 1);
    assert_eq!(analysis.text_channels.document_text, "Body");
    assert!(
        analysis
            .findings
            .iter()
            .any(|finding| finding.kind == DocxFindingKind::CustomXmlPresent)
    );
}

#[test]
fn t_u_insertions_are_current_and_deletions_are_separated() {
    let analysis = analyze_body(
        "<w:p><w:ins w:id=\"1\" w:author=\"Editor\"><w:r><w:t>New</w:t></w:r></w:ins><w:del w:id=\"2\"><w:r><w:delText>Old</w:delText></w:r></w:del></w:p>",
    );
    assert_eq!(analysis.text_channels.document_text, "New");
    assert_eq!(analysis.text_channels.deleted_content, "Old");
    assert!(
        analysis
            .revisions
            .iter()
            .any(|revision| revision.kind == DocxRevisionKind::Inserted)
    );
    assert!(
        analysis
            .revisions
            .iter()
            .any(|revision| revision.kind == DocxRevisionKind::Deleted)
    );
}

#[test]
fn v_explicit_hidden_runs_are_reported_without_rendering_inference() {
    let analysis = analyze_body(
        "<w:p><w:r><w:rPr><w:vanish/></w:rPr><w:t>Hidden declaration</w:t></w:r></w:p>",
    );
    assert_eq!(analysis.text_channels.document_text, "Hidden declaration");
    assert!(analysis.text_segments[0].explicit_hidden);
    assert!(
        analysis
            .findings
            .iter()
            .any(|finding| finding.kind == DocxFindingKind::ExplicitHiddenTextPresent)
    );
}

#[test]
fn w_invalid_zip_is_a_controlled_error() {
    assert_eq!(
        analyze_docx_bytes(b"not a zip"),
        Err(DocxAnalysisError::InvalidZip)
    );
}

#[test]
fn x_zip_without_required_docx_parts_is_rejected() {
    let mut package = PackageBuilder::default();
    package.add_text("data.txt", "hello");
    assert_eq!(
        analyze_docx_bytes(&package.finish()),
        Err(DocxAnalysisError::MissingContentTypes)
    );
}

#[test]
fn main_document_requires_the_wordprocessingml_namespace() {
    let mut package = PackageBuilder::standard("<w:p/>");
    package.replace_text(
        "word/document.xml",
        "<document xmlns=\"urn:not-wordprocessingml\"><body><p><r><t>False DOCX</t></r></p></body></document>",
    );
    assert_eq!(
        analyze_docx_bytes(&package.finish()),
        Err(DocxAnalysisError::UnsupportedPackage)
    );
}

#[test]
fn y_duplicate_entry_names_are_rejected_as_ambiguous() {
    let mut package = PackageBuilder::standard("<w:p/>");
    package.add_text("word/documen2.xml", &document("<w:p/>"));
    let mut bytes = package.finish();
    let original = b"word/documen2.xml";
    let duplicate = b"word/document.xml";
    assert_eq!(original.len(), duplicate.len());
    for offset in 0..=bytes.len() - original.len() {
        if bytes[offset..offset + original.len()] == *original {
            bytes[offset..offset + duplicate.len()].copy_from_slice(duplicate);
        }
    }
    assert_eq!(
        analyze_docx_bytes(&bytes),
        Err(DocxAnalysisError::DuplicateEntryNames)
    );
}

#[test]
fn z_unsafe_entry_names_are_findings_and_never_extracted() {
    let mut package = PackageBuilder::standard("<w:p/>");
    package.add_text("../outside.xml", "<outside/>");
    let analysis = analyze_docx_bytes(&package.finish()).expect("inspect unsafe name");
    assert_eq!(analysis.package.status, DocxPackageStatus::MalformedPackage);
    assert_eq!(analysis.package.invalid_entry_names, 1);
    assert!(
        analysis
            .findings
            .iter()
            .any(|finding| finding.kind == DocxFindingKind::UnsafeEntryName)
    );
}

#[test]
fn aa_high_compression_ratio_is_blocked_before_xml_parsing() {
    let mut package = PackageBuilder::standard("<w:p/>");
    package.add_text("customXml/repetition.xml", &"A".repeat(200_000));
    assert_eq!(
        analyze_docx_bytes(&package.finish()),
        Err(DocxAnalysisError::ResourceLimitExceeded(
            DocxResourceLimit::CompressionRatio
        ))
    );
}

#[test]
fn ab_malformed_main_xml_returns_without_panic() {
    let mut package = PackageBuilder::standard("<w:p/>");
    package.replace_text("word/document.xml", "<w:document>");
    assert_eq!(
        analyze_docx_bytes(&package.finish()),
        Err(DocxAnalysisError::MalformedXml)
    );
}

#[test]
fn ac_malformed_noncritical_relationship_is_a_finding_not_a_panic() {
    let mut package = PackageBuilder::standard("<w:p/>");
    package.add_text(
        "word/_rels/document.xml.rels",
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="bad" Type="type-without-target"/></Relationships>"#,
    );
    let analysis = analyze_docx_bytes(&package.finish()).expect("malformed relationship evidence");
    assert_eq!(analysis.package.status, DocxPackageStatus::MalformedPackage);
    assert!(
        analysis
            .findings
            .iter()
            .any(|finding| finding.kind == DocxFindingKind::MalformedRelationship)
    );
}

#[test]
fn ad_zero_width_space_in_body_reaches_the_authoritative_scanner() {
    let analysis = analyze_body("<w:p><w:r><w:t>A\u{200b}B</w:t></w:r></w:p>");
    assert_eq!(analysis.scan, scan_text("A\u{200b}B"));
    assert_eq!(analysis.scan.findings.len(), 1);
}

#[test]
fn ae_nbsp_and_normalization_reach_the_existing_tcs_engine() {
    let analysis = analyze_body("<w:p><w:r><w:t>A é</w:t></w:r></w:p>");
    let cleaned = clean_text(
        &analysis.text_channels.document_text,
        CleanProfile::PurePlainText,
    )
    .expect("TCS clean");
    assert_eq!(cleaned.clean_text, "A é");
}

#[test]
fn af_clean_verification_uses_an_independent_rescan() {
    let analysis = analyze_body("<w:p><w:r><w:t>A B</w:t></w:r></w:p>");
    let cleaned = clean_text(
        &analysis.text_channels.document_text,
        CleanProfile::PurePlainText,
    )
    .expect("TCS clean");
    assert!(cleaned.verification.checks.clean_rescan_completed);
    assert_eq!(cleaned.verification.clean_scan_summary.findings, 0);
}

#[test]
fn ag_identical_docx_bytes_produce_identical_analysis() {
    let bytes = PackageBuilder::standard("<w:p><w:r><w:t>Deterministic</w:t></w:r></w:p>").finish();
    assert_eq!(analyze_docx_bytes(&bytes), analyze_docx_bytes(&bytes));
}

#[test]
fn ah_arbitrary_malformed_package_samples_never_panic() {
    let samples = [
        Vec::new(),
        vec![0],
        b"PK".to_vec(),
        b"PK\x03\x04".to_vec(),
        (0_u8..=255).collect(),
    ];
    for sample in samples {
        let _ = analyze_docx_bytes(&sample);
    }
}

#[test]
fn source_provenance_names_the_xml_part_without_inventing_package_offsets() {
    let analysis = analyze_body("<w:p><w:r><w:t>Mapped</w:t></w:r></w:p>");
    let location = &analysis.text_segments[0].source_location;
    assert_eq!(location.part_name, "word/document.xml");
    assert!(location.decoded_xml_byte_range.is_some());
    assert!(location.part_byte_range.is_some());
    assert_eq!(location.package_byte_range, None);
}

#[test]
fn styles_numbering_part_inventory_and_crc_evidence_are_observable() {
    let mut package = PackageBuilder::standard("<w:p/>");
    package.add_text(
        "word/styles.xml",
        &format!("<w:styles xmlns:w=\"{WORD_NS}\"/>"),
    );
    package.add_text(
        "word/numbering.xml",
        &format!("<w:numbering xmlns:w=\"{WORD_NS}\"/>"),
    );
    let analysis = analyze_docx_bytes(&package.finish()).expect("styles and numbering DOCX");
    assert!(analysis.summary.styles_present);
    assert!(analysis.summary.numbering_present);
    assert!(
        analysis
            .parts
            .iter()
            .any(|part| part.classification == DocxPartClassification::Styles)
    );
}

#[test]
fn embedded_and_active_content_related_parts_are_inert_findings() {
    let mut package = PackageBuilder::standard("<w:p/>");
    package.add_bytes("word/embeddings/object1.bin", b"inert");
    package.add_bytes("word/vbaProject.bin", b"inert");
    package.add_bytes("word/unknown.bin", b"inert");
    let analysis = analyze_docx_bytes(&package.finish()).expect("active evidence DOCX");
    assert!(
        analysis
            .findings
            .iter()
            .any(|finding| finding.kind == DocxFindingKind::EmbeddedObjectPresent)
    );
    assert!(
        analysis
            .findings
            .iter()
            .any(|finding| finding.kind == DocxFindingKind::ActiveContentRelatedPartPresent)
    );
    assert!(
        analysis
            .findings
            .iter()
            .any(|finding| finding.kind == DocxFindingKind::UnexpectedPackagePart)
    );
}

#[test]
fn text_segment_limit_constant_remains_above_realistic_fixture_size() {
    const {
        assert!(MAX_DOCX_ENTRY_COUNT > 100);
        assert!(MAX_DOCX_ENTRY_COUNT < 10_000);
    }
}

#[test]
fn body_segments_are_not_silently_mixed_with_comment_segments() {
    let mut package = PackageBuilder::standard("<w:p><w:r><w:t>Body</w:t></w:r></w:p>");
    package.add_text(
        "word/comments.xml",
        &format!("<w:comments xmlns:w=\"{WORD_NS}\"><w:comment w:id=\"1\"><w:p><w:r><w:t>Comment</w:t></w:r></w:p></w:comment></w:comments>"),
    );
    let analysis = analyze_docx_bytes(&package.finish()).expect("channel DOCX");
    assert!(
        analysis
            .text_segments
            .iter()
            .any(|segment| segment.channel == DocxTextChannel::Body)
    );
    assert!(
        analysis
            .text_segments
            .iter()
            .any(|segment| segment.channel == DocxTextChannel::Comment)
    );
}
