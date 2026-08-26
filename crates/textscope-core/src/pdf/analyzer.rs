use std::collections::{BTreeMap, HashMap, HashSet};

use lopdf::{
    DecompressError, Dictionary, Document, Error as LopdfError, LoadOptions, Object, ObjectId,
    content::Content,
    xref::{XrefEntry, XrefType},
};

use super::model::*;
use crate::{EvidenceKind, EvidenceSource, ProvenanceEvidence, scan_text};

#[derive(Clone, Debug)]
struct RawPdfEvidence {
    version: Option<PdfVersion>,
    binary_marker_present: bool,
    eof_marker_present: bool,
    startxref_offsets: Vec<usize>,
    startxref_values: Vec<Option<usize>>,
    prev_offsets: Vec<usize>,
    trailer_count: usize,
    encrypted_marker: bool,
    linearized_marker: bool,
}

/// Analyze a caller-provided PDF snapshot entirely in memory.
///
/// The adapter never opens a path, renders a page, follows an external target,
/// executes an action, extracts an attachment, or mutates the source bytes.
pub fn analyze_pdf_bytes(bytes: &[u8]) -> Result<PdfAnalysis, PdfAnalysisError> {
    if bytes.is_empty() {
        return Err(PdfAnalysisError::EmptyInput);
    }
    if bytes.len() > MAX_PDF_SOURCE_BYTES {
        return Err(PdfAnalysisError::ResourceLimitExceeded(
            PdfResourceLimit::SourceBytes,
        ));
    }
    if !bytes.starts_with(b"%PDF-") {
        return Err(PdfAnalysisError::NonPdfBytes);
    }

    let evidence = inspect_raw_evidence(bytes);
    if evidence.version.is_none() {
        return Err(PdfAnalysisError::InvalidHeader);
    }

    let options = LoadOptions {
        strict: false,
        max_decompressed_size: Some(MAX_PDF_DECODED_STREAM_BYTES),
        ..LoadOptions::default()
    };
    let document = match Document::load_mem_with_options(bytes, options) {
        Ok(document) => document,
        Err(error) => return Err(map_load_error(error, evidence.encrypted_marker)),
    };

    let encryption = if document.was_encrypted() || document.trailer.has(b"Encrypt") {
        PdfEncryptionStatus::EncryptedOpenableWithEmptyPassword
    } else {
        PdfEncryptionStatus::Unencrypted
    };

    if document.objects.len() > MAX_PDF_OBJECTS {
        return Err(PdfAnalysisError::ResourceLimitExceeded(
            PdfResourceLimit::Objects,
        ));
    }

    let xref_kind = match document.reference_table.cross_reference_type {
        XrefType::CrossReferenceTable => PdfXrefKind::Table,
        XrefType::CrossReferenceStream => PdfXrefKind::Stream,
    };
    let mut findings = Vec::new();
    if evidence.startxref_offsets.len() > 1 || !evidence.prev_offsets.is_empty() {
        push_finding(
            &mut findings,
            PdfFindingKind::IncrementalUpdatesPresent,
            PdfFindingSeverity::Informational,
            "Incremental revisions observed; their presence alone does not establish tampering.",
            None,
        )?;
    }
    if !matches!(encryption, PdfEncryptionStatus::Unencrypted) {
        push_finding(
            &mut findings,
            PdfFindingKind::EncryptionPresent,
            PdfFindingSeverity::Informational,
            "PDF encryption structure observed; no password was requested or brute-forced.",
            None,
        )?;
    }
    if evidence.linearized_marker {
        push_finding(
            &mut findings,
            PdfFindingKind::LinearizationPresent,
            PdfFindingSeverity::Informational,
            "A PDF linearization dictionary was observed as a structural characteristic.",
            None,
        )?;
    }
    if !evidence.eof_marker_present || evidence.startxref_offsets.is_empty() {
        push_finding(
            &mut findings,
            PdfFindingKind::MalformedStructure,
            PdfFindingSeverity::Caution,
            "The PDF is parseable but its EOF/startxref evidence is incomplete.",
            None,
        )?;
    }

    let objects = inventory_objects(&document);
    let revisions = build_revisions(&evidence, xref_kind);
    let metadata = inspect_metadata(&document, &mut findings)?;

    let mut actions = Vec::new();
    let mut external_references = Vec::new();
    inspect_actions(
        &document,
        &mut actions,
        &mut external_references,
        &mut findings,
    )?;

    let (forms, signatures) = inspect_forms_and_signatures(&document, &mut findings)?;
    let embedded_files = inspect_embedded_files(&document, &mut findings)?;

    let mut pages = Vec::new();
    let mut fonts = Vec::new();
    let mut annotations = Vec::new();
    let mut text_segments = Vec::new();
    let mut page_text = Vec::new();
    let mut images = BTreeMap::new();
    let page_map = document.get_pages();
    if page_map.len() > MAX_PDF_PAGES {
        return Err(PdfAnalysisError::ResourceLimitExceeded(
            PdfResourceLimit::Pages,
        ));
    }

    for (page_number, page_id) in page_map {
        let page_index = usize::try_from(page_number.saturating_sub(1)).unwrap_or(usize::MAX);
        let page = document
            .get_dictionary(page_id)
            .map_err(|_| PdfAnalysisError::MalformedStructure)?;
        let content_streams = document.get_page_contents(page_id);
        let annotation_refs = page_annotation_refs(&document, page);
        if annotations.len().saturating_add(annotation_refs.len()) > MAX_PDF_ANNOTATIONS {
            return Err(PdfAnalysisError::ResourceLimitExceeded(
                PdfResourceLimit::Annotations,
            ));
        }
        for (annotation_number, annotation_ref, annotation) in &annotation_refs {
            let mut record = annotation_record(
                &document,
                page_index,
                *annotation_number,
                *annotation_ref,
                annotation,
                &mut external_references,
            );
            record.sequence = annotations.len();
            annotations.push(record);
        }

        let page_fonts = inspect_page_fonts(&document, page_id, page_index, &mut fonts);
        inspect_page_images(&document, page_id, page_index, &mut images);
        let (text, status) = extract_page_text(
            &document,
            page_id,
            page_index,
            &page_fonts,
            &mut text_segments,
            &mut findings,
        )?;
        if matches!(
            status,
            PdfTextExtractionStatus::TextPresentButMappingUnavailable
        ) {
            push_finding(
                &mut findings,
                PdfFindingKind::TextMappingUnavailable,
                PdfFindingSeverity::Caution,
                "Text operators were observed but at least one glyph-to-Unicode mapping was unavailable; no replacement character was fabricated.",
                Some(source_reference(&document, page_id, "page")),
            )?;
        }
        if matches!(status, PdfTextExtractionStatus::NoTextOperators) {
            // The final document-level finding is emitted after all pages so an
            // empty page does not create a duplicate finding for every channel.
        }

        let summary = PdfPageSummary {
            page_index,
            object: source_reference(&document, page_id, "page"),
            media_box: inherited_box(&document, page_id, b"MediaBox"),
            crop_box: inherited_box(&document, page_id, b"CropBox"),
            rotate: inherited_integer(&document, page_id, b"Rotate"),
            resources_present: inherited_value(&document, page_id, b"Resources").is_some(),
            content_stream_count: content_streams.len(),
            annotation_count: annotation_refs.len(),
            text,
        };
        pages.push(summary);
        page_text.push(
            pages
                .last()
                .map(|page| page.text.clone())
                .unwrap_or_default(),
        );
    }

    let text_extraction_status = document_text_status(&page_text, &text_segments, &fonts);
    if matches!(
        text_extraction_status,
        PdfTextExtractionStatus::NoTextOperators
    ) {
        push_finding(
            &mut findings,
            PdfFindingKind::NoTextLayer,
            PdfFindingSeverity::Informational,
            "No extractable text layer detected; OCR is not implemented in this phase.",
            None,
        )?;
    }
    let document_text = page_text.join("\n\n");
    let provenance = pdf_provenance(&metadata);

    let images = images
        .into_values()
        .enumerate()
        .map(|(sequence, mut image)| {
            image.sequence = sequence;
            image
        })
        .collect::<Vec<_>>();
    let summary = PdfDocumentSummary {
        raw_byte_length: bytes.len(),
        version: evidence.version,
        binary_marker_present: evidence.binary_marker_present,
        eof_marker_present: evidence.eof_marker_present,
        startxref_count: evidence.startxref_offsets.len(),
        xref_kind,
        trailer_count: evidence.trailer_count,
        object_count: objects.len(),
        stream_count: objects.iter().filter(|object| object.has_stream).count(),
        object_stream_count: objects
            .iter()
            .filter(|object| object.kind == "object_stream")
            .count(),
        compressed_object_count: objects
            .iter()
            .filter(|object| object.compressed_in_object_stream)
            .count(),
        page_count: pages.len(),
        font_count: fonts.len(),
        image_count: images.len(),
        annotation_count: annotations.len(),
        action_count: actions.len(),
        embedded_file_count: embedded_files.len(),
        form_present: forms.present,
        metadata_present: !metadata.info_entries.is_empty()
            || metadata.xmp.as_ref().is_some_and(|xmp| xmp.present),
        javascript_present: actions
            .iter()
            .any(|action| matches!(action.kind, PdfActionKind::JavaScript)),
        linearized: evidence.linearized_marker,
        encryption,
    };

    Ok(PdfAnalysis {
        summary,
        objects,
        revisions,
        metadata,
        provenance,
        pages,
        images,
        fonts,
        actions,
        annotations,
        external_references,
        embedded_files,
        forms,
        signatures,
        text_segments,
        page_text,
        document_text: document_text.clone(),
        text_extraction_status,
        findings,
        scan: scan_text(&document_text),
    })
}

fn pdf_provenance(metadata: &PdfMetadata) -> Vec<ProvenanceEvidence> {
    let mut evidence = Vec::new();
    for entry in &metadata.info_entries {
        evidence.push(ProvenanceEvidence::declared(
            pdf_metadata_kind(&entry.key),
            EvidenceSource::PdfInfo,
            format!("Info.{}", entry.key),
            &entry.value,
        ));
    }
    if let Some(xmp) = &metadata.xmp {
        for entry in &xmp.properties {
            evidence.push(ProvenanceEvidence::declared(
                pdf_metadata_kind(&entry.key),
                EvidenceSource::PdfXmp,
                format!("XMP.{}", entry.key),
                &entry.value,
            ));
        }
    }
    for conflict in &metadata.conflicts {
        evidence.push(ProvenanceEvidence::conflict(
            pdf_metadata_kind(&conflict.key),
            EvidenceSource::PdfXmp,
            format!("conflict.{}", conflict.key),
            format!(
                "Info={}; XMP={}",
                conflict.info_value.as_deref().unwrap_or("(absent)"),
                conflict.xmp_value.as_deref().unwrap_or("(absent)")
            ),
        ));
    }
    evidence
}

fn pdf_metadata_kind(key: &str) -> EvidenceKind {
    match key
        .rsplit_once(':')
        .map(|(_, local)| local)
        .unwrap_or(key)
        .to_ascii_lowercase()
        .as_str()
    {
        "author" => EvidenceKind::AuthorDeclaration,
        "creator" => EvidenceKind::CreatorDeclaration,
        "producer" => EvidenceKind::ProducerDeclaration,
        "generator" => EvidenceKind::GeneratorDeclaration,
        _ => EvidenceKind::MetadataDeclaration,
    }
}

fn map_load_error(error: LopdfError, encrypted_marker: bool) -> PdfAnalysisError {
    match error {
        LopdfError::Decompress(DecompressError::MemoryLimitExceeded { .. }) => {
            PdfAnalysisError::ResourceLimitExceeded(PdfResourceLimit::DecodedStreamBytes)
        }
        LopdfError::InvalidPassword => PdfAnalysisError::PasswordRequired,
        LopdfError::UnsupportedSecurityHandler(_) => PdfAnalysisError::UnsupportedEncryption,
        LopdfError::Unimplemented(_) if encrypted_marker => PdfAnalysisError::PasswordRequired,
        _ if encrypted_marker => PdfAnalysisError::EncryptedPdfUnsupported,
        _ => PdfAnalysisError::MalformedStructure,
    }
}

fn inspect_raw_evidence(bytes: &[u8]) -> RawPdfEvidence {
    let version = parse_version(bytes);
    let binary_marker_present = bytes.iter().skip(8).take(1024).any(|byte| *byte >= 0x80);
    let eof_marker_present = bytes.windows(5).any(|window| window == b"%%EOF");
    let mut startxref_offsets = Vec::new();
    let mut startxref_values = Vec::new();
    for offset in find_all(bytes, b"startxref") {
        startxref_offsets.push(offset);
        startxref_values.push(parse_decimal_after(bytes, offset.saturating_add(9)));
    }
    let prev_offsets = find_all(bytes, b"/Prev");
    let trailer_count = find_all(bytes, b"trailer").len();
    let encrypted_marker = find_all(bytes, b"/Encrypt").into_iter().next().is_some();
    let linearized_marker = find_all(bytes, b"/Linearized").into_iter().next().is_some();
    RawPdfEvidence {
        version,
        binary_marker_present,
        eof_marker_present,
        startxref_offsets,
        startxref_values,
        prev_offsets,
        trailer_count,
        encrypted_marker,
        linearized_marker,
    }
}

fn parse_version(bytes: &[u8]) -> Option<PdfVersion> {
    let header = bytes.get(0..8)?;
    if &header[0..5] != b"%PDF-" || header[6] != b'.' {
        return None;
    }
    let major = (header[5] as char).to_digit(10)? as u8;
    let minor = (header[7] as char).to_digit(10)? as u8;
    Some(PdfVersion {
        major,
        minor,
        declared: String::from_utf8(header[5..8].to_vec()).ok()?,
    })
}

fn find_all(bytes: &[u8], needle: &[u8]) -> Vec<usize> {
    if needle.is_empty() {
        return Vec::new();
    }
    bytes
        .windows(needle.len())
        .enumerate()
        .filter_map(|(index, window)| (window == needle).then_some(index))
        .collect()
}

fn parse_decimal_after(bytes: &[u8], mut offset: usize) -> Option<usize> {
    while bytes
        .get(offset)
        .is_some_and(|byte| byte.is_ascii_whitespace())
    {
        offset = offset.saturating_add(1);
    }
    let start = offset;
    while bytes.get(offset).is_some_and(|byte| byte.is_ascii_digit()) {
        offset = offset.saturating_add(1);
    }
    (offset > start)
        .then(|| std::str::from_utf8(&bytes[start..offset]).ok())
        .flatten()
        .and_then(|value| value.parse().ok())
}

fn inventory_objects(document: &Document) -> Vec<PdfObjectSummary> {
    document
        .objects
        .iter()
        .map(|(&(object_number, generation), object)| {
            let (kind, has_stream) = object_kind(object);
            let raw_byte_offset = match document.reference_table.get(object_number) {
                Some(XrefEntry::Normal {
                    offset,
                    generation: entry_generation,
                }) if *entry_generation == generation => usize::try_from(*offset).ok(),
                _ => None,
            };
            PdfObjectSummary {
                object_number,
                generation,
                kind,
                has_stream,
                compressed_in_object_stream: matches!(
                    document.reference_table.get(object_number),
                    Some(XrefEntry::Compressed { .. })
                ),
                raw_byte_offset,
            }
        })
        .collect()
}

fn object_kind(object: &Object) -> (String, bool) {
    match object {
        Object::Stream(stream) => {
            let type_name = stream
                .dict
                .get(b"Type")
                .ok()
                .and_then(|object| object.as_name().ok())
                .and_then(|name| String::from_utf8(name.to_vec()).ok());
            let subtype = stream
                .dict
                .get(b"Subtype")
                .ok()
                .and_then(|object| object.as_name().ok())
                .and_then(|name| String::from_utf8(name.to_vec()).ok());
            let kind = match (type_name.as_deref(), subtype.as_deref()) {
                (Some("ObjStm"), _) => "object_stream",
                (_, Some("Image")) => "image_stream",
                (_, Some(value)) => value,
                (Some(value), _) => value,
                _ => "stream",
            };
            (kind.to_owned(), true)
        }
        Object::Dictionary(dictionary) => (
            dictionary
                .get(b"Type")
                .ok()
                .and_then(|object| object.as_name().ok())
                .and_then(|name| String::from_utf8(name.to_vec()).ok())
                .unwrap_or_else(|| "dictionary".to_owned()),
            false,
        ),
        Object::Array(_) => ("array".to_owned(), false),
        Object::String(_, _) => ("string".to_owned(), false),
        Object::Name(_) => ("name".to_owned(), false),
        Object::Integer(_) => ("integer".to_owned(), false),
        Object::Real(_) => ("real".to_owned(), false),
        Object::Boolean(_) => ("boolean".to_owned(), false),
        Object::Null => ("null".to_owned(), false),
        Object::Reference(_) => ("reference".to_owned(), false),
    }
}

fn build_revisions(evidence: &RawPdfEvidence, xref_kind: PdfXrefKind) -> Vec<PdfRevisionSummary> {
    let count = evidence
        .startxref_offsets
        .len()
        .max(evidence.prev_offsets.len().saturating_add(1))
        .max(1);
    (0..count)
        .map(|sequence| PdfRevisionSummary {
            sequence,
            startxref_offset: evidence.startxref_offsets.get(sequence).copied(),
            xref_kind,
            prev_offset: evidence
                .startxref_values
                .get(sequence)
                .copied()
                .flatten()
                .or_else(|| evidence.prev_offsets.get(sequence).copied()),
            trailer_observed: sequence < evidence.trailer_count,
            // lopdf exposes the merged current object graph. It does not expose
            // a proof-preserving per-revision object delta, so this list stays
            // empty rather than inventing historical provenance.
            updated_object_ids: Vec::new(),
        })
        .collect()
}

fn source_reference(document: &Document, object_id: ObjectId, context: &str) -> PdfSourceReference {
    let raw_byte_offset = match document.reference_table.get(object_id.0) {
        Some(XrefEntry::Normal { offset, generation }) if *generation == object_id.1 => {
            usize::try_from(*offset).ok()
        }
        _ => None,
    };
    PdfSourceReference {
        object_number: Some(object_id.0),
        generation: Some(object_id.1),
        raw_byte_offset,
        context: context.to_owned(),
    }
}

fn direct_object<'a>(document: &'a Document, object: &'a Object) -> Option<&'a Object> {
    match object {
        Object::Reference(id) => document.get_object(*id).ok(),
        _ => Some(object),
    }
}

fn direct_dict<'a>(document: &'a Document, object: &'a Object) -> Option<&'a Dictionary> {
    direct_object(document, object).and_then(|object| object.as_dict().ok())
}

fn object_text(document: &Document, object: &Object) -> Option<String> {
    let object = direct_object(document, object)?;
    match object {
        Object::String(_, _) => lopdf::decode_text_string(object).ok(),
        Object::Name(name) => String::from_utf8(name.clone()).ok(),
        Object::Integer(value) => Some(value.to_string()),
        _ => None,
    }
}

fn object_bool(document: &Document, object: &Object) -> Option<bool> {
    direct_object(document, object).and_then(|object| object.as_bool().ok())
}

fn object_integer(document: &Document, object: &Object) -> Option<i64> {
    direct_object(document, object).and_then(|object| object.as_i64().ok())
}

fn info_dictionary(document: &Document) -> Option<(ObjectId, &Dictionary)> {
    let info = document.trailer.get(b"Info").ok()?;
    match info {
        Object::Reference(id) => document.get_dictionary(*id).ok().map(|dict| (*id, dict)),
        Object::Dictionary(dict) => Some(((0, 0), dict)),
        _ => None,
    }
}

fn inspect_metadata(
    document: &Document,
    findings: &mut Vec<PdfFinding>,
) -> Result<PdfMetadata, PdfAnalysisError> {
    let mut metadata = PdfMetadata::default();
    if let Some((info_id, dictionary)) = info_dictionary(document) {
        for (key, value) in dictionary.iter() {
            let Some(key) = String::from_utf8(key.clone()).ok() else {
                continue;
            };
            if let Some(value) = object_text(document, value) {
                if value.len() > MAX_PDF_METADATA_BYTES {
                    return Err(PdfAnalysisError::ResourceLimitExceeded(
                        PdfResourceLimit::MetadataBytes,
                    ));
                }
                let source = if info_id == (0, 0) {
                    PdfSourceReference {
                        object_number: None,
                        generation: None,
                        raw_byte_offset: None,
                        context: "Info dictionary".to_owned(),
                    }
                } else {
                    source_reference(document, info_id, "Info dictionary")
                };
                metadata.info_entries.push(PdfMetadataEntry {
                    key: key.clone(),
                    value: value.clone(),
                    source,
                });
                set_metadata_field(&mut metadata, &key, value);
            }
        }
    }

    if let Some(metadata_object) = document
        .catalog()
        .ok()
        .and_then(|catalog| catalog.get(b"Metadata").ok())
        .and_then(|object| match object {
            Object::Reference(id) => document.get_object(*id).ok().map(|object| (*id, object)),
            object => Some(((0, 0), object)),
        })
        && let Ok(stream) = metadata_object.1.as_stream()
    {
        let content = stream
            .get_plain_content_with_limit(MAX_PDF_METADATA_BYTES)
            .map_err(|error| map_stream_error(error, PdfResourceLimit::MetadataBytes))?;
        let source = if metadata_object.0 == (0, 0) {
            PdfSourceReference {
                object_number: None,
                generation: None,
                raw_byte_offset: None,
                context: "XMP metadata stream".to_owned(),
            }
        } else {
            source_reference(document, metadata_object.0, "XMP metadata stream")
        };
        let (well_formed_xml, properties) = parse_xmp_properties(&content, &source);
        metadata.xmp = Some(PdfXmpMetadata {
            present: true,
            well_formed_xml,
            properties,
            source: Some(source),
        });
    }

    let xmp_values = metadata
        .xmp
        .as_ref()
        .map(|xmp| {
            xmp.properties
                .iter()
                .filter_map(|property| {
                    let key = normalize_xmp_property_key(&property.key)?;
                    Some((key, property.value.clone()))
                })
                .collect::<HashMap<_, _>>()
        })
        .unwrap_or_default();
    for key in [
        "title",
        "author",
        "subject",
        "keywords",
        "creator",
        "producer",
        "creationdate",
        "moddate",
        "trapped",
    ] {
        let info_value = metadata.info_entries.iter().find_map(|entry| {
            (entry.key.to_ascii_lowercase() == key).then_some(entry.value.clone())
        });
        let xmp_value = xmp_values.get(key).cloned();
        if info_value.is_some() && xmp_value.is_some() && info_value != xmp_value {
            metadata.conflicts.push(PdfMetadataConflict {
                key: key.to_owned(),
                info_value,
                xmp_value,
                description: "Info dictionary and XMP declarations differ; neither was selected as authoritative.",
            });
        }
    }
    if !metadata.conflicts.is_empty() {
        push_finding(
            findings,
            PdfFindingKind::MetadataDeclarationConflict,
            PdfFindingSeverity::Caution,
            "PDF metadata declarations differ between the Info dictionary and XMP; this is an observation, not a fraud claim.",
            None,
        )?;
    }
    Ok(metadata)
}

fn normalize_xmp_property_key(key: &str) -> Option<String> {
    let local = key
        .rsplit_once(':')
        .map(|(_, local)| local)
        .unwrap_or(key)
        .to_ascii_lowercase();
    match local.as_str() {
        "title" | "author" | "creator" | "subject" | "description" | "keywords" | "producer"
        | "creationdate" | "createdate" | "modifydate" | "moddate" | "trapped" => {
            Some(match local.as_str() {
                "description" => "subject".to_owned(),
                "createdate" => "creationdate".to_owned(),
                "modifydate" => "moddate".to_owned(),
                value => value.to_owned(),
            })
        }
        _ => None,
    }
}

fn set_metadata_field(metadata: &mut PdfMetadata, key: &str, value: String) {
    match key.to_ascii_lowercase().as_str() {
        "title" => metadata.title = Some(value),
        "author" => metadata.author = Some(value),
        "subject" => metadata.subject = Some(value),
        "keywords" => metadata.keywords = Some(value),
        "creator" => metadata.creator = Some(value),
        "producer" => metadata.producer = Some(value),
        "creationdate" => metadata.creation_date = Some(value),
        "moddate" => metadata.modification_date = Some(value),
        "trapped" => metadata.trapped = Some(value),
        _ => {}
    }
}

fn parse_xmp_properties(
    content: &[u8],
    source: &PdfSourceReference,
) -> (bool, Vec<PdfMetadataEntry>) {
    let Ok(xml) = std::str::from_utf8(content) else {
        return (false, Vec::new());
    };
    let Ok(document) = roxmltree::Document::parse(xml) else {
        return (false, Vec::new());
    };
    let properties = document
        .descendants()
        .filter(|node| node.is_element())
        .filter_map(|node| {
            let value = node.text()?.trim();
            (!value.is_empty()).then(|| PdfMetadataEntry {
                key: node.tag_name().name().to_owned(),
                value: value.to_owned(),
                source: source.clone(),
            })
        })
        .take(512)
        .collect();
    (true, properties)
}

fn inherited_value<'a>(
    document: &'a Document,
    page_id: ObjectId,
    key: &[u8],
) -> Option<&'a Object> {
    let mut current = Some(page_id);
    let mut seen = HashSet::new();
    for _ in 0..=MAX_PDF_REFERENCE_DEPTH {
        let id = current?;
        if !seen.insert(id) {
            return None;
        }
        let page = document.get_dictionary(id).ok()?;
        if let Ok(value) = page.get(key) {
            return Some(value);
        }
        current = page
            .get(b"Parent")
            .ok()
            .and_then(|object| object.as_reference().ok());
    }
    None
}

fn inherited_integer(document: &Document, page_id: ObjectId, key: &[u8]) -> Option<i32> {
    inherited_value(document, page_id, key)
        .and_then(|value| object_integer(document, value))
        .and_then(|value| i32::try_from(value).ok())
}

fn inherited_box(document: &Document, page_id: ObjectId, key: &[u8]) -> Option<PdfBox> {
    let value = inherited_value(document, page_id, key)?;
    let object = direct_object(document, value)?;
    let values = object
        .as_array()
        .ok()?
        .iter()
        .map(pdf_number)
        .collect::<Option<Vec<_>>>()?;
    (values.len() == 4).then_some(PdfBox { values })
}

fn pdf_number(object: &Object) -> Option<String> {
    match object {
        Object::Integer(value) => Some(value.to_string()),
        Object::Real(value) => Some(value.to_string()),
        _ => None,
    }
}

fn page_annotation_refs<'a>(
    document: &'a Document,
    page: &'a Dictionary,
) -> Vec<(usize, Option<ObjectId>, &'a Dictionary)> {
    let Some(annots) = page.get(b"Annots").ok() else {
        return Vec::new();
    };
    let annots = direct_object(document, annots).and_then(|object| object.as_array().ok());
    annots
        .into_iter()
        .flat_map(|array| array.iter().enumerate())
        .filter_map(|(index, object)| {
            let object_id = object.as_reference().ok();
            let dictionary = direct_dict(document, object)?;
            Some((index, object_id, dictionary))
        })
        .collect()
}

fn annotation_record(
    document: &Document,
    page_index: usize,
    annotation_number: usize,
    annotation_ref: Option<ObjectId>,
    annotation: &Dictionary,
    external_references: &mut Vec<PdfExternalReference>,
) -> PdfAnnotationRecord {
    let source = annotation_ref
        .map(|id| source_reference(document, id, "annotation"))
        .unwrap_or(PdfSourceReference {
            object_number: None,
            generation: None,
            raw_byte_offset: None,
            context: "direct annotation dictionary".to_owned(),
        });
    let subtype = annotation
        .get(b"Subtype")
        .ok()
        .and_then(|object| object.as_name().ok())
        .and_then(|name| String::from_utf8(name.to_vec()).ok())
        .unwrap_or_else(|| "Unknown".to_owned());
    let contents = annotation
        .get(b"Contents")
        .ok()
        .and_then(|value| object_text(document, value));
    let title = annotation
        .get(b"T")
        .ok()
        .and_then(|value| object_text(document, value));
    let flags = annotation
        .get(b"F")
        .ok()
        .and_then(|value| object_integer(document, value));
    let (uri, action) = annotation
        .get(b"A")
        .ok()
        .and_then(|object| direct_dict(document, object))
        .map(|action| {
            let kind = action_kind(action);
            let uri = action
                .get(b"URI")
                .ok()
                .and_then(|value| object_text(document, value));
            if let Some(uri) = uri.as_ref() {
                external_references.push(PdfExternalReference {
                    sequence: external_references.len(),
                    value: uri.clone(),
                    kind: classify_external_reference(uri),
                    source: source.clone(),
                });
            }
            (uri, kind)
        })
        .unwrap_or((None, None));
    PdfAnnotationRecord {
        sequence: annotation_number,
        page_index,
        subtype,
        contents,
        title,
        uri,
        action,
        flags,
        source,
    }
}

fn inspect_page_fonts(
    document: &Document,
    page_id: ObjectId,
    page_index: usize,
    fonts: &mut Vec<PdfFontRecord>,
) -> BTreeMap<Vec<u8>, PdfSourceReference> {
    let mut result = BTreeMap::new();
    let Ok((resource_dict, resource_ids)) = document.get_page_resources(page_id) else {
        return result;
    };
    let mut resource_dicts = Vec::new();
    if let Some(resources) = resource_dict {
        resource_dicts.push(resources);
    }
    for id in resource_ids {
        if let Ok(resources) = document.get_dictionary(id) {
            resource_dicts.push(resources);
        }
    }
    let mut seen = HashSet::new();
    for resources in resource_dicts {
        let Some(font_object) = resources.get(b"Font").ok() else {
            continue;
        };
        let Some(fonts_dict) = direct_dict(document, font_object) else {
            continue;
        };
        for (name, font_object) in fonts_dict.iter() {
            let Some(font_id) = font_object.as_reference().ok() else {
                continue;
            };
            if !seen.insert((name.clone(), font_id)) {
                continue;
            }
            let Some(font) = document.get_dictionary(font_id).ok() else {
                continue;
            };
            let to_unicode_present = font.has(b"ToUnicode");
            let mapping_status = if font
                .get_font_encoding_with_limit(document, MAX_PDF_FONT_STREAM_BYTES)
                .is_ok()
            {
                PdfTextExtractionStatus::Extracted
            } else {
                PdfTextExtractionStatus::TextPresentButMappingUnavailable
            };
            let record_index = fonts.iter().position(|record| {
                record.source.object_number == Some(font_id.0)
                    && record.source.generation == Some(font_id.1)
            });
            if let Some(index) = record_index {
                if !fonts[index].page_indices.contains(&page_index) {
                    fonts[index].page_indices.push(page_index);
                }
            } else {
                fonts.push(PdfFontRecord {
                    sequence: fonts.len(),
                    page_indices: vec![page_index],
                    resource_name: String::from_utf8(name.clone())
                        .unwrap_or_else(|_| hex_bytes(name)),
                    source: source_reference(document, font_id, "font dictionary"),
                    subtype: font
                        .get(b"Subtype")
                        .ok()
                        .and_then(|object| object.as_name().ok())
                        .and_then(|value| String::from_utf8(value.to_vec()).ok()),
                    base_font: font
                        .get(b"BaseFont")
                        .ok()
                        .and_then(|object| object.as_name().ok())
                        .and_then(|value| String::from_utf8(value.to_vec()).ok()),
                    encoding: font
                        .get(b"Encoding")
                        .ok()
                        .and_then(|value| direct_object(document, value))
                        .and_then(|value| match value {
                            Object::Name(name) => String::from_utf8(name.clone()).ok(),
                            Object::Dictionary(_) => Some("dictionary".to_owned()),
                            _ => None,
                        }),
                    to_unicode_present,
                    mapping_status,
                });
            }
            result.insert(
                name.clone(),
                source_reference(document, font_id, "font dictionary"),
            );
        }
    }
    result
}

fn inspect_page_images(
    document: &Document,
    page_id: ObjectId,
    page_index: usize,
    images: &mut BTreeMap<(u32, u16), PdfImageRecord>,
) {
    let Ok((resource_dict, resource_ids)) = document.get_page_resources(page_id) else {
        return;
    };
    let mut resource_dicts = Vec::new();
    if let Some(resources) = resource_dict {
        resource_dicts.push(resources);
    }
    for id in resource_ids {
        if let Ok(resources) = document.get_dictionary(id) {
            resource_dicts.push(resources);
        }
    }
    for resources in resource_dicts {
        let Some(xobjects) = resources
            .get(b"XObject")
            .ok()
            .and_then(|object| direct_dict(document, object))
        else {
            continue;
        };
        for (_, object) in xobjects.iter() {
            let Some(id) = object.as_reference().ok() else {
                continue;
            };
            let Some(stream) = document
                .get_object(id)
                .ok()
                .and_then(|object| object.as_stream().ok())
            else {
                continue;
            };
            if stream
                .dict
                .get(b"Subtype")
                .ok()
                .and_then(|object| object.as_name().ok())
                == Some(b"Image")
            {
                let image = images
                    .entry((id.0, id.1))
                    .or_insert_with(|| PdfImageRecord {
                        sequence: 0,
                        page_indices: Vec::new(),
                        object: source_reference(document, id, "image XObject"),
                        width: stream
                            .dict
                            .get(b"Width")
                            .ok()
                            .and_then(|value| object_integer(document, value))
                            .and_then(|value| u32::try_from(value).ok()),
                        height: stream
                            .dict
                            .get(b"Height")
                            .ok()
                            .and_then(|value| object_integer(document, value))
                            .and_then(|value| u32::try_from(value).ok()),
                        filter: stream
                            .dict
                            .get(b"Filter")
                            .ok()
                            .and_then(|value| image_object_label(document, value)),
                        color_space: stream
                            .dict
                            .get(b"ColorSpace")
                            .ok()
                            .and_then(|value| image_object_label(document, value)),
                    });
                if !image.page_indices.contains(&page_index) {
                    image.page_indices.push(page_index);
                }
            }
        }
    }
}

fn image_object_label(document: &Document, object: &Object) -> Option<String> {
    match direct_object(document, object)? {
        Object::Name(name) => String::from_utf8(name.clone()).ok(),
        Object::Array(values) => {
            let labels = values
                .iter()
                .filter_map(|value| match direct_object(document, value)? {
                    Object::Name(name) => String::from_utf8(name.clone()).ok(),
                    _ => None,
                })
                .collect::<Vec<_>>();
            (!labels.is_empty()).then(|| labels.join(" "))
        }
        _ => None,
    }
}

fn extract_page_text(
    document: &Document,
    page_id: ObjectId,
    page_index: usize,
    page_fonts: &BTreeMap<Vec<u8>, PdfSourceReference>,
    text_segments: &mut Vec<PdfTextSegment>,
    findings: &mut Vec<PdfFinding>,
) -> Result<(String, PdfTextExtractionStatus), PdfAnalysisError> {
    let mut encodings = BTreeMap::new();
    for (name, font_reference) in page_fonts {
        if let (Some(object_number), Some(generation)) =
            (font_reference.object_number, font_reference.generation)
            && let Ok(font) = document.get_dictionary((object_number, generation))
            && let Ok(encoding) =
                font.get_font_encoding_with_limit(document, MAX_PDF_FONT_STREAM_BYTES)
        {
            encodings.insert(name.clone(), encoding);
        }
    }
    let content_streams = document.get_page_contents(page_id);
    let mut page_text = String::new();
    let mut text_operator_seen = false;
    let mut mapping_unavailable = false;
    let mut rendering_mode = 0_i64;
    let mut current_font: Option<Vec<u8>> = None;
    let mut current_font_reference = None;
    let mut page_decoded_bytes = 0usize;
    let mut operation_total = 0usize;
    let append_operand = |operand: &Object,
                          font_name: Option<&Vec<u8>>,
                          font_reference: Option<&PdfSourceReference>,
                          stream_id: ObjectId,
                          operation_index: usize,
                          explicit_non_painting: bool,
                          page_text: &mut String,
                          text_segments: &mut Vec<PdfTextSegment>,
                          mapping_unavailable: &mut bool|
     -> Result<(), PdfAnalysisError> {
        let decoded = match (font_name, operand.as_str().ok()) {
            (Some(font_name), Some(bytes)) => encodings
                .get(font_name)
                .map(|encoding| lopdf::Document::decode_text(encoding, bytes))
                .unwrap_or(Err(LopdfError::CharacterEncoding)),
            _ => Err(LopdfError::CharacterEncoding),
        };
        append_decoded_text(
            document,
            decoded,
            page_index,
            stream_id,
            operation_index,
            font_reference,
            explicit_non_painting,
            page_text,
            text_segments,
            mapping_unavailable,
        )
    };

    for stream_id in content_streams {
        let Some(stream) = document
            .get_object(stream_id)
            .ok()
            .and_then(|object| object.as_stream().ok())
        else {
            push_finding(
                findings,
                PdfFindingKind::MalformedStructure,
                PdfFindingSeverity::Caution,
                "A page content reference did not resolve to a stream.",
                Some(source_reference(
                    document,
                    stream_id,
                    "page content reference",
                )),
            )?;
            continue;
        };
        let remaining = MAX_PDF_DECODED_STREAM_BYTES.saturating_sub(page_decoded_bytes);
        let decoded = match stream.decompressed_content_with_limit(remaining) {
            Ok(decoded) => decoded,
            Err(error) => {
                return Err(map_stream_error(
                    error,
                    PdfResourceLimit::DecodedStreamBytes,
                ));
            }
        };
        if decoded.len() > MAX_PDF_INDIVIDUAL_STREAM_BYTES {
            return Err(PdfAnalysisError::ResourceLimitExceeded(
                PdfResourceLimit::IndividualStreamBytes,
            ));
        }
        if !stream.content.is_empty()
            && decoded.len()
                > stream
                    .content
                    .len()
                    .saturating_mul(MAX_PDF_DECOMPRESSION_RATIO)
        {
            return Err(PdfAnalysisError::ResourceLimitExceeded(
                PdfResourceLimit::DecompressionRatio,
            ));
        }
        page_decoded_bytes = page_decoded_bytes.saturating_add(decoded.len());
        let content = match Content::decode_strict(&decoded) {
            Ok(content) => content,
            Err(_) => {
                push_finding(
                    findings,
                    PdfFindingKind::MalformedStructure,
                    PdfFindingSeverity::Caution,
                    "A page content stream could not be decoded strictly; no text was fabricated from it.",
                    Some(source_reference(document, stream_id, "page content stream")),
                )?;
                continue;
            }
        };
        operation_total = operation_total.saturating_add(content.operations.len());
        if operation_total > MAX_PDF_CONTENT_OPERATIONS {
            return Err(PdfAnalysisError::ResourceLimitExceeded(
                PdfResourceLimit::ContentOperations,
            ));
        }
        for (operation_index, operation) in content.operations.iter().enumerate() {
            match operation.operator.as_str() {
                "Tf" => {
                    current_font = operation
                        .operands
                        .first()
                        .and_then(|object| object.as_name().ok())
                        .map(ToOwned::to_owned);
                    current_font_reference = current_font
                        .as_ref()
                        .and_then(|font| page_fonts.get(font).cloned());
                }
                "Tr" => {
                    rendering_mode = operation
                        .operands
                        .first()
                        .and_then(|object| object.as_i64().ok())
                        .unwrap_or(0);
                }
                "Tj" => {
                    text_operator_seen = true;
                    let Some(operand) = operation.operands.first() else {
                        continue;
                    };
                    append_operand(
                        operand,
                        current_font.as_ref(),
                        current_font_reference.as_ref(),
                        stream_id,
                        operation_index,
                        rendering_mode == 3,
                        &mut page_text,
                        text_segments,
                        &mut mapping_unavailable,
                    )?;
                }
                "TJ" => {
                    text_operator_seen = true;
                    if let Some(array) = operation
                        .operands
                        .first()
                        .and_then(|object| object.as_array().ok())
                    {
                        for operand in array {
                            if matches!(operand, Object::String(_, _)) {
                                append_operand(
                                    operand,
                                    current_font.as_ref(),
                                    current_font_reference.as_ref(),
                                    stream_id,
                                    operation_index,
                                    rendering_mode == 3,
                                    &mut page_text,
                                    text_segments,
                                    &mut mapping_unavailable,
                                )?;
                            }
                        }
                    }
                }
                "'" => {
                    text_operator_seen = true;
                    if !page_text.ends_with('\n') {
                        page_text.push('\n');
                    }
                    if let Some(operand) = operation.operands.first() {
                        append_operand(
                            operand,
                            current_font.as_ref(),
                            current_font_reference.as_ref(),
                            stream_id,
                            operation_index,
                            rendering_mode == 3,
                            &mut page_text,
                            text_segments,
                            &mut mapping_unavailable,
                        )?;
                    }
                }
                "\"" => {
                    text_operator_seen = true;
                    if !page_text.ends_with('\n') {
                        page_text.push('\n');
                    }
                    if let Some(operand) = operation.operands.get(2) {
                        append_operand(
                            operand,
                            current_font.as_ref(),
                            current_font_reference.as_ref(),
                            stream_id,
                            operation_index,
                            rendering_mode == 3,
                            &mut page_text,
                            text_segments,
                            &mut mapping_unavailable,
                        )?;
                    }
                }
                "T*" | "ET" if text_operator_seen && !page_text.ends_with('\n') => {
                    page_text.push('\n');
                }
                _ => {}
            }
        }
    }

    let explicit_non_painting_seen = text_segments
        .iter()
        .any(|segment| segment.page_index == page_index && segment.explicit_non_painting);
    if explicit_non_painting_seen {
        push_finding(
            findings,
            PdfFindingKind::ExplicitNonPaintingTextPresent,
            PdfFindingSeverity::Informational,
            "Text rendering mode 3 (neither fill nor stroke) was explicitly observed; effective visibility was not inferred.",
            Some(source_reference(document, page_id, "page content")),
        )?;
    }
    while page_text.ends_with('\n') {
        page_text.pop();
    }
    let status = if !text_operator_seen {
        PdfTextExtractionStatus::NoTextOperators
    } else if mapping_unavailable
        && text_segments
            .iter()
            .all(|segment| segment.page_index != page_index || segment.text.is_empty())
    {
        PdfTextExtractionStatus::TextPresentButMappingUnavailable
    } else if mapping_unavailable {
        PdfTextExtractionStatus::PartiallyExtracted
    } else {
        PdfTextExtractionStatus::Extracted
    };
    Ok((page_text, status))
}

#[allow(clippy::too_many_arguments)]
fn append_decoded_text(
    document: &Document,
    decoded: Result<String, LopdfError>,
    page_index: usize,
    stream_id: ObjectId,
    operation_index: usize,
    current_font_reference: Option<&PdfSourceReference>,
    explicit_non_painting: bool,
    page_text: &mut String,
    text_segments: &mut Vec<PdfTextSegment>,
    mapping_unavailable: &mut bool,
) -> Result<(), PdfAnalysisError> {
    let Ok(decoded) = decoded else {
        *mapping_unavailable = true;
        append_unmapped_segment(
            page_index,
            stream_id,
            operation_index,
            current_font_reference,
            explicit_non_painting,
            text_segments,
        )?;
        return Ok(());
    };
    let had_replacement = decoded.contains('\u{FFFD}');
    let decoded = decoded.replace('\u{FFFD}', "");
    if had_replacement {
        *mapping_unavailable = true;
    }
    if decoded.is_empty() && had_replacement {
        append_unmapped_segment(
            page_index,
            stream_id,
            operation_index,
            current_font_reference,
            explicit_non_painting,
            text_segments,
        )?;
        return Ok(());
    }
    page_text.push_str(&decoded);
    if text_segments.len() >= MAX_PDF_TEXT_SEGMENTS {
        return Err(PdfAnalysisError::ResourceLimitExceeded(
            PdfResourceLimit::TextSegments,
        ));
    }
    text_segments.push(PdfTextSegment {
        sequence: text_segments.len(),
        page_index,
        text: decoded,
        font_reference: current_font_reference.cloned(),
        content_stream_object: Some(source_reference(document, stream_id, "page content stream")),
        operation_index,
        source_object: Some(source_reference(document, stream_id, "text operation")),
        extraction_status: if had_replacement {
            PdfTextExtractionStatus::PartiallyExtracted
        } else {
            PdfTextExtractionStatus::Extracted
        },
        explicit_non_painting,
    });
    Ok(())
}

fn append_unmapped_segment(
    page_index: usize,
    stream_id: ObjectId,
    operation_index: usize,
    font_reference: Option<&PdfSourceReference>,
    explicit_non_painting: bool,
    text_segments: &mut Vec<PdfTextSegment>,
) -> Result<(), PdfAnalysisError> {
    if text_segments.len() >= MAX_PDF_TEXT_SEGMENTS {
        return Err(PdfAnalysisError::ResourceLimitExceeded(
            PdfResourceLimit::TextSegments,
        ));
    }
    text_segments.push(PdfTextSegment {
        sequence: text_segments.len(),
        page_index,
        text: String::new(),
        font_reference: font_reference.cloned(),
        content_stream_object: Some(PdfSourceReference {
            object_number: Some(stream_id.0),
            generation: Some(stream_id.1),
            raw_byte_offset: None,
            context: "page content stream".to_owned(),
        }),
        operation_index,
        source_object: None,
        extraction_status: PdfTextExtractionStatus::TextPresentButMappingUnavailable,
        explicit_non_painting,
    });
    Ok(())
}

fn document_text_status(
    page_text: &[String],
    text_segments: &[PdfTextSegment],
    fonts: &[PdfFontRecord],
) -> PdfTextExtractionStatus {
    if text_segments.is_empty() {
        if page_text.iter().all(String::is_empty) {
            PdfTextExtractionStatus::NoTextOperators
        } else {
            PdfTextExtractionStatus::TextPresentButMappingUnavailable
        }
    } else if text_segments.iter().any(|segment| {
        matches!(
            segment.extraction_status,
            PdfTextExtractionStatus::TextPresentButMappingUnavailable
                | PdfTextExtractionStatus::PartiallyExtracted
        )
    }) || fonts.iter().any(|font| {
        matches!(
            font.mapping_status,
            PdfTextExtractionStatus::TextPresentButMappingUnavailable
        )
    }) {
        PdfTextExtractionStatus::PartiallyExtracted
    } else {
        PdfTextExtractionStatus::Extracted
    }
}

fn inspect_actions(
    document: &Document,
    actions: &mut Vec<PdfActionRecord>,
    external_references: &mut Vec<PdfExternalReference>,
    findings: &mut Vec<PdfFinding>,
) -> Result<(), PdfAnalysisError> {
    let mut visited = HashSet::new();
    let mut action_objects = HashSet::new();
    for (&id, object) in &document.objects {
        walk_action_object(
            document,
            object,
            Some(id),
            0,
            &mut visited,
            &mut action_objects,
            actions,
            external_references,
            findings,
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn walk_action_object(
    document: &Document,
    object: &Object,
    source_id: Option<ObjectId>,
    depth: usize,
    visited: &mut HashSet<ObjectId>,
    action_objects: &mut HashSet<ObjectId>,
    actions: &mut Vec<PdfActionRecord>,
    external_references: &mut Vec<PdfExternalReference>,
    findings: &mut Vec<PdfFinding>,
) -> Result<(), PdfAnalysisError> {
    if depth > MAX_PDF_REFERENCE_DEPTH {
        return Err(PdfAnalysisError::ResourceLimitExceeded(
            PdfResourceLimit::ReferenceDepth,
        ));
    }
    match object {
        Object::Reference(id) => {
            if visited.insert(*id)
                && let Some(target) = document.objects.get(id)
            {
                walk_action_object(
                    document,
                    target,
                    Some(*id),
                    depth.saturating_add(1),
                    visited,
                    action_objects,
                    actions,
                    external_references,
                    findings,
                )?;
            }
        }
        Object::Dictionary(dictionary) => {
            if dictionary.has(b"S") {
                if let Some(id) = source_id {
                    if action_objects.insert(id) {
                        add_action(
                            document,
                            dictionary,
                            Some(id),
                            actions,
                            external_references,
                            findings,
                        )?;
                    }
                } else {
                    add_action(
                        document,
                        dictionary,
                        None,
                        actions,
                        external_references,
                        findings,
                    )?;
                }
            }
            for (_, value) in dictionary.iter() {
                walk_action_object(
                    document,
                    value,
                    source_id,
                    depth.saturating_add(1),
                    visited,
                    action_objects,
                    actions,
                    external_references,
                    findings,
                )?;
            }
        }
        Object::Array(array) => {
            for value in array {
                walk_action_object(
                    document,
                    value,
                    source_id,
                    depth.saturating_add(1),
                    visited,
                    action_objects,
                    actions,
                    external_references,
                    findings,
                )?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn action_kind(dictionary: &Dictionary) -> Option<PdfActionKind> {
    let name = dictionary.get(b"S").ok()?.as_name().ok()?;
    Some(match name {
        b"URI" => PdfActionKind::Uri,
        b"GoTo" => PdfActionKind::GoTo,
        b"GoToR" => PdfActionKind::GoToRemote,
        b"Launch" => PdfActionKind::Launch,
        b"JavaScript" => PdfActionKind::JavaScript,
        b"SubmitForm" => PdfActionKind::SubmitForm,
        b"ImportData" => PdfActionKind::ImportData,
        b"Rendition" => PdfActionKind::Rendition,
        b"Named" => PdfActionKind::Named,
        _ => PdfActionKind::Other,
    })
}

fn add_action(
    document: &Document,
    dictionary: &Dictionary,
    source_id: Option<ObjectId>,
    actions: &mut Vec<PdfActionRecord>,
    external_references: &mut Vec<PdfExternalReference>,
    findings: &mut Vec<PdfFinding>,
) -> Result<(), PdfAnalysisError> {
    if actions.len() >= MAX_PDF_ACTIONS {
        return Err(PdfAnalysisError::ResourceLimitExceeded(
            PdfResourceLimit::Actions,
        ));
    }
    let Some(kind) = action_kind(dictionary) else {
        return Ok(());
    };
    let source = source_id
        .map(|id| source_reference(document, id, "action dictionary"))
        .unwrap_or(PdfSourceReference {
            object_number: None,
            generation: None,
            raw_byte_offset: None,
            context: "direct action dictionary".to_owned(),
        });
    let target = match kind {
        PdfActionKind::Uri => dictionary
            .get(b"URI")
            .ok()
            .and_then(|value| object_text(document, value)),
        PdfActionKind::JavaScript => None,
        PdfActionKind::Launch | PdfActionKind::SubmitForm | PdfActionKind::GoToRemote => dictionary
            .get(b"F")
            .ok()
            .and_then(|value| object_text(document, value)),
        PdfActionKind::Named => dictionary
            .get(b"N")
            .ok()
            .and_then(|value| object_text(document, value)),
        _ => None,
    };
    if let Some(target) = target.as_ref()
        && matches!(
            kind,
            PdfActionKind::Uri
                | PdfActionKind::Launch
                | PdfActionKind::SubmitForm
                | PdfActionKind::GoToRemote
        )
    {
        external_references.push(PdfExternalReference {
            sequence: external_references.len(),
            value: target.clone(),
            kind: classify_external_reference(target),
            source: source.clone(),
        });
    }
    let context = match kind {
        PdfActionKind::JavaScript => "JavaScript action (not executed)".to_owned(),
        PdfActionKind::Launch => "Launch action (not executed)".to_owned(),
        PdfActionKind::SubmitForm => "SubmitForm action (not submitted)".to_owned(),
        PdfActionKind::Uri => "URI action (not followed)".to_owned(),
        _ => "PDF action".to_owned(),
    };
    actions.push(PdfActionRecord {
        sequence: actions.len(),
        kind,
        target,
        source: source.clone(),
        context,
    });
    match kind {
        PdfActionKind::JavaScript => push_finding(
            findings,
            PdfFindingKind::JavaScriptActionPresent,
            PdfFindingSeverity::Caution,
            "JavaScript action observed structurally; TextScope never executes or interprets it.",
            Some(source),
        )?,
        PdfActionKind::Launch => push_finding(
            findings,
            PdfFindingKind::LaunchActionPresent,
            PdfFindingSeverity::Caution,
            "Launch action observed structurally; TextScope never opens or executes its target.",
            Some(source),
        )?,
        PdfActionKind::Uri => push_finding(
            findings,
            PdfFindingKind::ExternalUriPresent,
            PdfFindingSeverity::Informational,
            "External URI action observed; TextScope never follows or fetches it.",
            Some(source),
        )?,
        PdfActionKind::SubmitForm => push_finding(
            findings,
            PdfFindingKind::SubmitFormActionPresent,
            PdfFindingSeverity::Caution,
            "SubmitForm action observed; TextScope never submits a form.",
            Some(source),
        )?,
        _ => {}
    }
    Ok(())
}

fn classify_external_reference(value: &str) -> PdfExternalReferenceKind {
    if value.contains("://") || value.starts_with("mailto:") || value.starts_with("data:") {
        PdfExternalReferenceKind::AbsoluteUri
    } else if value.contains('/') || value.contains('\\') {
        PdfExternalReferenceKind::FileLikeTarget
    } else if value.starts_with('#') || value.contains(':') {
        PdfExternalReferenceKind::RelativeReference
    } else {
        PdfExternalReferenceKind::Other
    }
}

fn inspect_forms_and_signatures(
    document: &Document,
    findings: &mut Vec<PdfFinding>,
) -> Result<(PdfFormSummary, Vec<PdfSignatureRecord>), PdfAnalysisError> {
    let mut form = PdfFormSummary {
        present: false,
        field_count: 0,
        field_types: Vec::new(),
        need_appearances: None,
        xfa_present: false,
    };
    let mut signatures = Vec::new();
    let Some(catalog) = document.catalog().ok() else {
        return Ok((form, signatures));
    };
    let Some(acro_form_object) = catalog.get(b"AcroForm").ok() else {
        return Ok((form, signatures));
    };
    let Some(acro_form) = direct_dict(document, acro_form_object) else {
        return Ok((form, signatures));
    };
    form.present = true;
    form.need_appearances = acro_form
        .get(b"NeedAppearances")
        .ok()
        .and_then(|value| object_bool(document, value));
    form.xfa_present = acro_form.has(b"XFA");
    let fields = acro_form
        .get(b"Fields")
        .ok()
        .and_then(|value| direct_object(document, value))
        .and_then(|object| object.as_array().ok());
    if let Some(fields) = fields {
        let mut stack = fields.iter().collect::<Vec<_>>();
        let mut visited = HashSet::new();
        while let Some(field_object) = stack.pop() {
            if let Ok(id) = field_object.as_reference()
                && !visited.insert(id)
            {
                continue;
            }
            let Some(field) = direct_dict(document, field_object) else {
                continue;
            };
            form.field_count = form.field_count.saturating_add(1);
            if let Some(field_type) = field
                .get(b"FT")
                .ok()
                .and_then(|value| direct_object(document, value))
                .and_then(|object| object.as_name().ok())
                .and_then(|value| String::from_utf8(value.to_vec()).ok())
            {
                if !form.field_types.contains(&field_type) {
                    form.field_types.push(field_type.clone());
                }
                if field_type == "Sig"
                    && let Ok(id) = field_object.as_reference()
                {
                    signatures.push(signature_record(document, id, field));
                }
            }
            if let Some(kids) = field
                .get(b"Kids")
                .ok()
                .and_then(|value| direct_object(document, value))
                .and_then(|object| object.as_array().ok())
            {
                stack.extend(kids.iter());
            }
        }
    }
    for (&id, object) in &document.objects {
        if let Ok(dictionary) = object.as_dict()
            && (dictionary.has(b"ByteRange")
                || dictionary.has(b"Contents")
                    && dictionary
                        .get(b"Type")
                        .ok()
                        .and_then(|object| object.as_name().ok())
                        == Some(b"Sig"))
            && !signatures.iter().any(|signature| {
                signature.source.object_number == Some(id.0)
                    && signature.source.generation == Some(id.1)
            })
        {
            signatures.push(signature_record(document, id, dictionary));
        }
    }
    for (sequence, signature) in signatures.iter_mut().enumerate() {
        signature.sequence = sequence;
    }
    if form.present {
        push_finding(
            findings,
            PdfFindingKind::FormPresent,
            PdfFindingSeverity::Informational,
            "AcroForm structure observed; calculations and form actions were not executed.",
            None,
        )?;
    }
    if form.xfa_present {
        push_finding(
            findings,
            PdfFindingKind::XfaPresent,
            PdfFindingSeverity::Caution,
            "XFA presence observed structurally; XFA was not executed or rendered.",
            None,
        )?;
    }
    if !signatures.is_empty() {
        push_finding(
            findings,
            PdfFindingKind::SignatureStructurePresent,
            PdfFindingSeverity::Informational,
            "Signature structure observed; TextScope did not cryptographically validate the signature.",
            signatures.first().map(|signature| signature.source.clone()),
        )?;
    }
    Ok((form, signatures))
}

fn signature_record(
    document: &Document,
    id: ObjectId,
    dictionary: &Dictionary,
) -> PdfSignatureRecord {
    PdfSignatureRecord {
        sequence: 0,
        source: source_reference(document, id, "signature structure"),
        byte_range_present: dictionary.has(b"ByteRange"),
        contents_present: dictionary.has(b"Contents"),
        sub_filter: dictionary
            .get(b"SubFilter")
            .ok()
            .and_then(|object| object.as_name().ok())
            .and_then(|value| String::from_utf8(value.to_vec()).ok()),
        reason: dictionary
            .get(b"Reason")
            .ok()
            .and_then(|value| object_text(document, value)),
        name: dictionary
            .get(b"Name")
            .ok()
            .and_then(|value| object_text(document, value)),
        modification_date: dictionary
            .get(b"M")
            .ok()
            .and_then(|value| object_text(document, value)),
    }
}

fn inspect_embedded_files(
    document: &Document,
    findings: &mut Vec<PdfFinding>,
) -> Result<Vec<PdfEmbeddedFileRecord>, PdfAnalysisError> {
    let mut result = Vec::new();
    let mut seen_streams = HashSet::new();
    for (&id, object) in &document.objects {
        let Ok(dictionary) = object.as_dict() else {
            if let Ok(stream) = object.as_stream()
                && stream
                    .dict
                    .get(b"Type")
                    .ok()
                    .and_then(|object| object.as_name().ok())
                    == Some(b"EmbeddedFile")
                && seen_streams.insert(id)
            {
                result.push(PdfEmbeddedFileRecord {
                    sequence: result.len(),
                    declared_filename: None,
                    description: None,
                    mime_type: stream
                        .dict
                        .get(b"Subtype")
                        .ok()
                        .and_then(|object| object.as_name().ok())
                        .and_then(|value| String::from_utf8(value.to_vec()).ok()),
                    size: stream
                        .dict
                        .get(b"Params")
                        .ok()
                        .and_then(|value| direct_dict(document, value))
                        .and_then(|params| params.get(b"Size").ok())
                        .and_then(|value| object_integer(document, value))
                        .and_then(|value| u64::try_from(value).ok())
                        .or_else(|| u64::try_from(stream.content.len()).ok()),
                    object: source_reference(document, id, "embedded file stream"),
                    stream_object: Some(source_reference(document, id, "embedded file stream")),
                });
            }
            continue;
        };
        let is_file_spec = dictionary.has(b"EF")
            || dictionary
                .get(b"Type")
                .ok()
                .and_then(|object| object.as_name().ok())
                == Some(b"Filespec");
        if !is_file_spec {
            continue;
        }
        let Some(ef) = dictionary
            .get(b"EF")
            .ok()
            .and_then(|value| direct_dict(document, value))
        else {
            continue;
        };
        let stream_object = ef
            .get(b"F")
            .or_else(|_| ef.get(b"UF"))
            .ok()
            .and_then(|object| object.as_reference().ok())
            .and_then(|stream_id| {
                let stream = document.get_object(stream_id).ok()?.as_stream().ok()?;
                if stream
                    .dict
                    .get(b"Type")
                    .ok()
                    .and_then(|object| object.as_name().ok())
                    == Some(b"EmbeddedFile")
                {
                    Some((stream_id, stream))
                } else {
                    None
                }
            });
        let Some((stream_id, stream)) = stream_object else {
            continue;
        };
        if !seen_streams.insert(stream_id) {
            if let Some(existing) = result
                .iter_mut()
                .find(|record: &&mut PdfEmbeddedFileRecord| {
                    record
                        .stream_object
                        .as_ref()
                        .and_then(|source| source.object_number)
                        == Some(stream_id.0)
                })
            {
                existing.declared_filename = dictionary
                    .get(b"UF")
                    .or_else(|_| dictionary.get(b"F"))
                    .ok()
                    .and_then(|value| object_text(document, value));
                existing.description = dictionary
                    .get(b"Desc")
                    .ok()
                    .and_then(|value| object_text(document, value));
                existing.object = source_reference(document, id, "FileSpec");
            }
            continue;
        }
        if result.len() >= MAX_PDF_EMBEDDED_FILES {
            return Err(PdfAnalysisError::ResourceLimitExceeded(
                PdfResourceLimit::EmbeddedFiles,
            ));
        }
        result.push(PdfEmbeddedFileRecord {
            sequence: result.len(),
            declared_filename: dictionary
                .get(b"UF")
                .or_else(|_| dictionary.get(b"F"))
                .ok()
                .and_then(|value| object_text(document, value)),
            description: dictionary
                .get(b"Desc")
                .ok()
                .and_then(|value| object_text(document, value)),
            mime_type: stream
                .dict
                .get(b"Subtype")
                .ok()
                .and_then(|object| object.as_name().ok())
                .and_then(|value| String::from_utf8(value.to_vec()).ok()),
            size: stream
                .dict
                .get(b"Params")
                .ok()
                .and_then(|value| direct_dict(document, value))
                .and_then(|params| params.get(b"Size").ok())
                .and_then(|value| object_integer(document, value))
                .and_then(|value| u64::try_from(value).ok())
                .or_else(|| u64::try_from(stream.content.len()).ok()),
            object: source_reference(document, id, "FileSpec"),
            stream_object: Some(source_reference(
                document,
                stream_id,
                "embedded file stream",
            )),
        });
    }
    if !result.is_empty() {
        push_finding(
            findings,
            PdfFindingKind::EmbeddedFilePresent,
            PdfFindingSeverity::Caution,
            "Embedded file structure observed; TextScope did not extract, open, execute, or recursively analyze it.",
            result.first().map(|record| record.object.clone()),
        )?;
    }
    Ok(result)
}

fn map_stream_error(error: LopdfError, limit: PdfResourceLimit) -> PdfAnalysisError {
    match error {
        LopdfError::Decompress(DecompressError::MemoryLimitExceeded { .. }) => {
            PdfAnalysisError::ResourceLimitExceeded(limit)
        }
        _ => PdfAnalysisError::MalformedStructure,
    }
}

fn push_finding(
    findings: &mut Vec<PdfFinding>,
    kind: PdfFindingKind,
    severity: PdfFindingSeverity,
    description: &'static str,
    source: Option<PdfSourceReference>,
) -> Result<(), PdfAnalysisError> {
    if findings.len() >= MAX_PDF_FINDINGS {
        return Err(PdfAnalysisError::ResourceLimitExceeded(
            PdfResourceLimit::Findings,
        ));
    }
    findings.push(PdfFinding {
        sequence: findings.len(),
        kind,
        severity,
        description,
        source,
    });
    Ok(())
}

fn hex_bytes(bytes: &[u8]) -> String {
    let mut result = String::from("0x");
    for byte in bytes.iter().take(128) {
        result.push_str(&format!("{byte:02x}"));
    }
    result
}
