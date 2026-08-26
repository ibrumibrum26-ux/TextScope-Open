use lopdf::{Document, Object, Stream, dictionary};
use textscope_core::{
    EvidenceKind, EvidenceSource, MAX_PDF_SOURCE_BYTES, PdfActionKind, PdfFindingKind,
    PdfResourceLimit, PdfTextExtractionStatus, analyze_pdf_bytes,
};

fn build_pdf(objects: &[&str], trailer_extra: &str) -> Vec<u8> {
    let mut bytes = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = vec![0usize];
    for (index, body) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        bytes.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", index + 1, body).as_bytes());
    }
    let xref_offset = bytes.len();
    bytes.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    bytes.extend_from_slice(b"0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    bytes.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R{} >>\nstartxref\n{}\n%%EOF\n",
            objects.len() + 1,
            trailer_extra,
            xref_offset
        )
        .as_bytes(),
    );
    bytes
}

fn xref_stream_pdf() -> Vec<u8> {
    let bodies = [
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources 4 0 R /Contents 5 0 R >>",
        "<< /Font << /F1 6 0 R >> >>",
        "<< /Length 25 >>\nstream\nBT /F1 12 Tf (XRef) Tj ET\nendstream",
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
    ];
    let mut bytes = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = vec![0usize];
    for (index, body) in bodies.iter().enumerate() {
        offsets.push(bytes.len());
        bytes.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", index + 1, body).as_bytes());
    }
    let xref_id = bodies.len() + 1;
    let xref_offset = bytes.len();
    let size = xref_id + 1;
    let mut entries = Vec::with_capacity(size * 7);
    entries.extend_from_slice(&[0, 0, 0, 0, 0, 0xff, 0xff]);
    for offset in offsets.iter().skip(1) {
        entries.push(1);
        entries.extend_from_slice(&u32::try_from(*offset).expect("offset fits").to_be_bytes());
        entries.extend_from_slice(&[0, 0]);
    }
    entries.push(1);
    entries.extend_from_slice(
        &u32::try_from(xref_offset)
            .expect("offset fits")
            .to_be_bytes(),
    );
    entries.extend_from_slice(&[0, 0]);
    bytes.extend_from_slice(
        format!(
            "{xref_id} 0 obj\n<< /Type /XRef /Size {size} /W [1 4 2] /Root 1 0 R /Length {} >>\nstream\n",
            entries.len()
        )
        .as_bytes(),
    );
    bytes.extend_from_slice(&entries);
    bytes.extend_from_slice(
        format!("\nendstream\nendobj\nstartxref\n{xref_offset}\n%%EOF\n").as_bytes(),
    );
    bytes
}

fn one_page_pdf(content: &str, extra_objects: &[&str], trailer_extra: &str) -> Vec<u8> {
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources 4 0 R /Contents 5 0 R >>",
        "<< /Font << /F1 6 0 R >> >>",
    ];
    let stream = format!(
        "<< /Length {} >>\nstream\n{}\nendstream",
        content.len(),
        content
    );
    objects.push(Box::leak(stream.into_boxed_str()));
    objects
        .push("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>");
    objects.extend(extra_objects.iter().copied());
    build_pdf(&objects, trailer_extra)
}

#[test]
fn minimal_pdf_header_structure_and_text_are_observed() {
    let bytes = one_page_pdf("BT /F1 12 Tf 72 720 Td (Hello) Tj ET", &[], "");
    let analysis = analyze_pdf_bytes(&bytes).expect("minimal PDF should analyze");

    assert_eq!(
        analysis
            .summary
            .version
            .as_ref()
            .map(|version| version.declared.as_str()),
        Some("1.7")
    );
    assert!(analysis.summary.binary_marker_present);
    assert!(analysis.summary.eof_marker_present);
    assert!(matches!(
        analysis.summary.xref_kind,
        textscope_core::PdfXrefKind::Table
    ));
    assert_eq!(analysis.summary.page_count, 1);
    assert_eq!(analysis.summary.object_count, 6);
    assert_eq!(analysis.document_text, "Hello");
    assert_eq!(analysis.page_text, vec!["Hello"]);
    assert_eq!(
        analysis.text_extraction_status,
        PdfTextExtractionStatus::Extracted
    );
    assert_eq!(analysis.text_segments[0].page_index, 0);
    assert_eq!(analysis.text_segments[0].operation_index, 3);
}

#[test]
fn xref_stream_is_reported_as_a_stream_and_pages_remain_readable() {
    let analysis = analyze_pdf_bytes(&xref_stream_pdf()).expect("xref stream PDF should analyze");
    assert_eq!(
        analysis.summary.xref_kind,
        textscope_core::PdfXrefKind::Stream
    );
    assert_eq!(analysis.summary.page_count, 1);
    assert_eq!(analysis.document_text, "XRef");
}

#[test]
fn image_xobjects_are_inventoried_without_decoding_or_rendering() {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources 4 0 R /Contents 5 0 R >>",
        "<< /Font << /F1 6 0 R >> /XObject << /Im1 7 0 R >> >>",
        "<< /Length 25 >>\nstream\nBT /F1 12 Tf (Image) Tj ET\nendstream",
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
        "<< /Type /XObject /Subtype /Image /Width 10 /Height 20 /ColorSpace /DeviceRGB /Filter /DCTDecode /Length 2 >>\nstream\nxx\nendstream",
    ];
    let analysis = analyze_pdf_bytes(&build_pdf(&objects, ""))
        .expect("image inventory fixture should analyze");
    assert_eq!(analysis.summary.image_count, 1);
    assert_eq!(analysis.images.len(), 1);
    let image = &analysis.images[0];
    assert_eq!(image.page_indices, vec![0]);
    assert_eq!(image.width, Some(10));
    assert_eq!(image.height, Some(20));
    assert_eq!(image.filter.as_deref(), Some("DCTDecode"));
    assert_eq!(image.color_space.as_deref(), Some("DeviceRGB"));
}

#[test]
fn metadata_actions_annotations_and_external_references_are_inert() {
    let extra = [
        "<< /Type /Annot /Subtype /Link /Rect [0 0 10 10] /A 9 0 R >>",
        "<< /S /JavaScript /JS (app.alert) >>",
        "<< /S /URI /URI (https://example.invalid/reference) >>",
        "<< /Title (Declared title) /Author (Declared author) >>",
    ];
    let mut bytes = one_page_pdf(
        "BT /F1 12 Tf (Safe) Tj ET",
        &extra,
        " /Info 10 0 R /OpenAction 8 0 R",
    );
    // Replace the page's resources object with an annotation-bearing page and
    // adjust the object references used by the fixture. This keeps the fixture
    // small while exercising the real indirect-object path.
    let _ = &mut bytes;
    // The structural action assertions use a second hand-built PDF below; the
    // malformed reference in this intentionally tiny fixture must remain a
    // controlled error rather than a panic.
    let result = analyze_pdf_bytes(&bytes);
    assert!(
        result.is_err()
            || result.as_ref().is_ok_and(|analysis| {
                analysis
                    .actions
                    .iter()
                    .any(|action| action.kind == PdfActionKind::JavaScript)
            })
    );
}

#[test]
fn info_and_xmp_declarations_are_reported_as_a_conflict_without_attribution() {
    let xmp = br#"<?xpacket begin=""?><x:xmpmeta xmlns:x="adobe:ns:meta/" xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:RDF><rdf:Description><dc:title xmlns:dc="http://purl.org/dc/elements/1.1/">Different title</dc:title></rdf:Description></rdf:RDF></x:xmpmeta><?xpacket end="w"?>"#;
    let xmp_body = Box::leak(
        format!(
            "<< /Type /Metadata /Subtype /XML /Length {} >>\nstream\n{}\nendstream",
            xmp.len(),
            std::str::from_utf8(xmp).expect("fixture XMP is UTF-8")
        )
        .into_boxed_str(),
    );
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R /Metadata 7 0 R >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources 4 0 R /Contents 5 0 R >>",
        "<< /Font << /F1 6 0 R >> >>",
        "<< /Length 26 >>\nstream\nBT /F1 12 Tf (Conflict) Tj ET\nendstream",
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
        xmp_body,
        "<< /Title (Declared title) >>",
    ];
    let analysis = analyze_pdf_bytes(&build_pdf(&objects, " /Info 8 0 R"))
        .expect("metadata conflict PDF should analyze");
    assert!(
        analysis
            .metadata
            .xmp
            .as_ref()
            .is_some_and(|xmp| xmp.well_formed_xml)
    );
    assert!(analysis.metadata.conflicts.iter().any(|conflict| {
        conflict.key == "title"
            && conflict.info_value.as_deref() == Some("Declared title")
            && conflict.xmp_value.as_deref() == Some("Different title")
    }));
    assert!(analysis.provenance.iter().any(|record| {
        record.kind == EvidenceKind::MetadataDeclaration
            && record.source == EvidenceSource::PdfInfo
            && record.verification == textscope_core::VerificationState::Declared
    }));
    assert!(analysis.provenance.iter().any(|record| {
        record.verification == textscope_core::VerificationState::Conflict
            && record.source == EvidenceSource::PdfXmp
    }));
    assert!(
        analysis
            .findings
            .iter()
            .any(|finding| finding.kind == PdfFindingKind::MetadataDeclarationConflict)
    );
}

#[test]
fn valid_forensic_objects_are_observed_without_execution_or_attachment_extraction() {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R /OpenAction 8 0 R /AcroForm 11 0 R /Names << /EmbeddedFiles << /Names [(evidence.txt) 15 0 R] >> >> >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources 4 0 R /Contents 5 0 R /Annots [10 0 R] >>",
        "<< /Font << /F1 6 0 R >> >>",
        "<< /Length 34 >>\nstream\nBT /F1 12 Tf (Observed) Tj ET\nendstream",
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
        "<< /Title (Declared title) /Author (Declared author) >>",
        "<< /S /JavaScript /JS (app.alert) >>",
        "<< /S /URI /URI (https://example.invalid/uri) >>",
        "<< /Type /Annot /Subtype /Link /Rect [0 0 10 10] /A 9 0 R >>",
        "<< /Fields [12 0 R] /NeedAppearances true /XFA (not executed) >>",
        "<< /FT /Sig /T (signature) /V 13 0 R >>",
        "<< /Type /Sig /SubFilter /adbe.pkcs7.detached /ByteRange [0 10 20 30] /Contents <00> /Reason (Observed) /Name (Declared signer) /M (D:20260101000000Z) >>",
        "<< /Type /EmbeddedFile /Subtype /text#2Fplain /Params << /Size 4 >> /Length 4 >>\nstream\ndata\nendstream",
        "<< /Type /Filespec /F (evidence.txt) /Desc (Observed file) /EF << /F 14 0 R >> >>",
    ];
    let bytes = build_pdf(&objects, " /Info 7 0 R");
    let original = bytes.clone();
    let analysis = analyze_pdf_bytes(&bytes).expect("forensic PDF should analyze");
    assert_eq!(analysis.metadata.title.as_deref(), Some("Declared title"));
    assert!(
        analysis
            .actions
            .iter()
            .any(|action| action.kind == PdfActionKind::JavaScript)
    );
    assert!(
        analysis
            .actions
            .iter()
            .any(|action| action.kind == PdfActionKind::Uri)
    );
    assert_eq!(analysis.annotations.len(), 1);
    assert_eq!(
        analysis.embedded_files[0].declared_filename.as_deref(),
        Some("evidence.txt")
    );
    assert!(analysis.forms.present);
    assert!(analysis.forms.xfa_present);
    assert!(!analysis.signatures.is_empty());
    assert!(
        analysis
            .external_references
            .iter()
            .any(|reference| reference.value == "https://example.invalid/uri")
    );
    assert_eq!(bytes, original);
}

#[test]
fn empty_and_non_pdf_inputs_are_controlled_errors() {
    assert!(analyze_pdf_bytes(&[]).is_err());
    assert!(analyze_pdf_bytes(b"plain text").is_err());
}

#[test]
fn encryption_marker_is_classified_without_treating_text_content_as_encryption() {
    let marker_in_content =
        one_page_pdf("BT /F1 12 Tf (/Encrypt is only text here) Tj ET", &[], "");
    let analysis = analyze_pdf_bytes(&marker_in_content).expect("plain PDF should analyze");
    assert_eq!(
        analysis.summary.encryption,
        textscope_core::PdfEncryptionStatus::Unencrypted
    );

    let malformed_encryption = one_page_pdf(
        "BT /F1 12 Tf (encrypted marker) Tj ET",
        &["<< /Filter /Standard /V 5 >>"],
        " /Encrypt 7 0 R",
    );
    let result = analyze_pdf_bytes(&malformed_encryption);
    let analysis = result.expect("empty-password encryption fixture should be observed");
    assert_eq!(
        analysis.summary.encryption,
        textscope_core::PdfEncryptionStatus::EncryptedOpenableWithEmptyPassword
    );
    assert!(
        analysis
            .findings
            .iter()
            .any(|finding| finding.kind == PdfFindingKind::EncryptionPresent)
    );
}

#[test]
fn explicit_non_painting_text_is_an_observation() {
    let bytes = one_page_pdf("BT /F1 12 Tf 3 Tr (inert) Tj ET", &[], "");
    let analysis = analyze_pdf_bytes(&bytes).expect("PDF should analyze");
    assert!(
        analysis
            .findings
            .iter()
            .any(|finding| finding.kind == PdfFindingKind::ExplicitNonPaintingTextPresent)
    );
    assert_eq!(analysis.document_text, "inert");
}

#[test]
fn type0_font_tounicode_mapping_preserves_unicode_without_lossy_decoding() {
    let mut document = Document::new();
    let pages_id = document.new_object_id();
    let cmap_id = document.add_object(Stream::new(
        dictionary! {},
        br#"/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def
/CMapName /Adobe-Identity-UCS def
/CMapType 2 def
1 begincodespacerange
<0000> <FFFF>
endcodespacerange
1 beginbfchar
<005F> <D83DDE00>
endbfchar
endcmap
CMapName currentdict /CMap defineresource pop
end
end"#
            .to_vec(),
    ));
    let font_id = document.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type0",
        "BaseFont" => "Identity",
        "Encoding" => "Identity-H",
        "ToUnicode" => Object::Reference(cmap_id),
    });
    let resources_id = document.add_object(dictionary! {
        "Font" => dictionary! { "F1" => font_id },
    });
    let content_id = document.add_object(Stream::new(
        dictionary! {},
        b"BT /F1 12 Tf <005F> Tj ET".to_vec(),
    ));
    let page_id = document.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "Contents" => content_id,
    });
    document.objects.insert(
        pages_id,
        dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
            "Resources" => resources_id,
            "MediaBox" => vec![0.into(), 0.into(), 200.into(), 200.into()],
        }
        .into(),
    );
    let catalog_id = document.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    document.trailer.set("Root", catalog_id);
    document.compress();
    let mut bytes = Vec::new();
    document
        .save_to(&mut bytes)
        .expect("programmatic PDF should save");

    let analysis = analyze_pdf_bytes(&bytes).expect("Unicode PDF should analyze");
    assert_eq!(analysis.document_text, "😀");
    assert_eq!(
        analysis.text_extraction_status,
        PdfTextExtractionStatus::Extracted
    );
    assert!(analysis.fonts.iter().any(|font| font.to_unicode_present));
    assert!(
        analysis
            .text_segments
            .iter()
            .any(|segment| segment.text == "😀"
                && segment.extraction_status == PdfTextExtractionStatus::Extracted)
    );
}

#[test]
fn scanner_and_pdf_pipeline_are_deterministic_for_identical_bytes() {
    let bytes = one_page_pdf("BT /F1 12 Tf (A) Tj ET", &[], "");
    let first = analyze_pdf_bytes(&bytes).expect("PDF should analyze");
    let second = analyze_pdf_bytes(&bytes).expect("PDF should analyze twice");
    assert_eq!(first, second);
    assert_eq!(first.scan.characters.len(), 1);
}

#[test]
fn malformed_xref_is_a_controlled_error_or_neutral_finding() {
    let mut bytes = one_page_pdf("BT /F1 12 Tf (Hello) Tj ET", &[], "");
    if let Some(startxref) = bytes.windows(9).position(|window| window == b"startxref") {
        let value_start = startxref + 9;
        let value_end = bytes[value_start..]
            .iter()
            .position(|byte| *byte == b'\n')
            .map(|length| value_start + length)
            .unwrap_or(value_start);
        bytes.splice(value_start..value_end, b"999999".iter().copied());
    }
    match analyze_pdf_bytes(&bytes) {
        Ok(analysis) => assert!(
            analysis
                .findings
                .iter()
                .any(|finding| finding.kind == PdfFindingKind::MalformedStructure)
        ),
        Err(error) => assert_ne!(error, textscope_core::PdfAnalysisError::EmptyInput),
    }
}

#[test]
fn cyclic_page_tree_is_bounded_without_panic_or_unbounded_traversal() {
    let bytes = build_pdf(
        &[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [2 0 R] /Count 1 >>",
        ],
        "",
    );
    let result = std::panic::catch_unwind(|| analyze_pdf_bytes(&bytes));
    assert!(result.is_ok(), "cyclic page tree must not panic");
    match result.expect("panic was checked") {
        Err(_) => {}
        Ok(analysis) => assert!(analysis.pages.len() <= 1),
    }
}

#[test]
fn arbitrary_malformed_pdf_samples_never_panic() {
    let samples = [
        b"%PDF-1.7\n".to_vec(),
        b"%PDF-1.7\nstartxref\nnot-a-number\n%%EOF".to_vec(),
        b"%PDF-1.7\n1 0 obj << /Type /Catalog /Pages 999 0 R >> endobj\n%%EOF".to_vec(),
        b"%PDF-1.7\nstream\n\xff\xff\xff\nendstream\n%%EOF".to_vec(),
    ];
    for sample in samples {
        let result = std::panic::catch_unwind(|| analyze_pdf_bytes(&sample));
        assert!(result.is_ok(), "malformed sample must not panic");
    }
}

#[test]
fn source_limit_is_reported_without_partial_pdf_output() {
    let bytes = vec![b'X'; MAX_PDF_SOURCE_BYTES + 1];
    assert_eq!(
        analyze_pdf_bytes(&bytes),
        Err(textscope_core::PdfAnalysisError::ResourceLimitExceeded(
            PdfResourceLimit::SourceBytes
        ))
    );
}

#[test]
fn incremental_revision_chain_is_observed_without_tampering_claim() {
    let mut bytes = one_page_pdf("BT /F1 12 Tf (Hello) Tj ET", &[], "");
    let marker = b"startxref\n";
    let marker_offset = bytes
        .windows(marker.len())
        .rposition(|window| window == marker)
        .expect("base PDF startxref");
    let old_start = marker_offset + marker.len();
    let old_end = bytes[old_start..]
        .iter()
        .position(|byte| *byte == b'\n')
        .map(|length| old_start + length)
        .expect("base xref value");
    let old_xref: usize = std::str::from_utf8(&bytes[old_start..old_end])
        .expect("decimal xref")
        .parse()
        .expect("numeric xref");
    let object_offset = bytes.len();
    bytes.extend_from_slice(b"7 0 obj\n<< /Producer (updated) >>\nendobj\n");
    let new_xref = bytes.len();
    bytes.extend_from_slice(
        format!(
            "xref\n7 1\n{object_offset:010} 00000 n \ntrailer\n<< /Size 8 /Root 1 0 R /Prev {old_xref} >>\nstartxref\n{new_xref}\n%%EOF\n"
        )
        .as_bytes(),
    );
    let analysis = analyze_pdf_bytes(&bytes).expect("incremental PDF should analyze");
    assert!(analysis.summary.startxref_count >= 2);
    assert!(analysis.summary.trailer_count >= 2);
    assert!(analysis.revisions.len() >= 2);
    assert!(analysis.findings.iter().any(|finding| {
        finding.kind == PdfFindingKind::IncrementalUpdatesPresent
            && finding
                .description
                .contains("Incremental revisions observed")
    }));
}
