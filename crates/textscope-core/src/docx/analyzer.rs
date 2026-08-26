use std::{
    collections::{BTreeMap, BTreeSet},
    io::{Cursor, Read},
};

use roxmltree::{Document, Node, ParsingOptions};
use unicode_normalization::UnicodeNormalization;
use zip::{CompressionMethod, ZipArchive};

use crate::{
    BomKind, EvidenceKind, EvidenceSource, ProvenanceEvidence, TextDecodeResult,
    ingestion::decode_text_bytes, scan_text,
};

use super::model::{
    DocxAnalysis, DocxAnalysisError, DocxByteRange, DocxComment, DocxContentTypeDeclaration,
    DocxContentTypeDeclarationKind, DocxCrcStatus, DocxDocumentSummary, DocxFinding,
    DocxFindingKind, DocxFindingSeverity, DocxHyperlink, DocxMetadata, DocxMetadataProperty,
    DocxPackageStatus, DocxPackageSummary, DocxPartClassification, DocxPartRecord,
    DocxPartTextChannel, DocxRelationship, DocxResourceLimit, DocxRevision, DocxRevisionKind,
    DocxSourceLocation, DocxTextChannel, DocxTextChannels, DocxTextKind, DocxTextSegment,
    MAX_DOCX_COMPRESSION_RATIO, MAX_DOCX_ENTRY_COUNT, MAX_DOCX_ENTRY_NAME_BYTES,
    MAX_DOCX_PART_BYTES, MAX_DOCX_RELATIONSHIPS, MAX_DOCX_SOURCE_BYTES, MAX_DOCX_TEXT_SEGMENTS,
    MAX_DOCX_TOTAL_UNCOMPRESSED_BYTES, MAX_DOCX_XML_DEPTH, MAX_DOCX_XML_NODES,
    MAX_DOCX_XML_PART_BYTES,
};

const CONTENT_TYPES_PART: &str = "[Content_Types].xml";
const ROOT_RELATIONSHIPS_PART: &str = "_rels/.rels";
const MAIN_DOCUMENT_PART: &str = "word/document.xml";
const MAIN_DOCUMENT_CONTENT_TYPE: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml";
const XML_NAMESPACE: &str = "http://www.w3.org/XML/1998/namespace";
const WORDPROCESSING_NAMESPACE: &str =
    "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const OFFICE_RELATIONSHIPS_NAMESPACE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const CONTENT_TYPES_NAMESPACE: &str =
    "http://schemas.openxmlformats.org/package/2006/content-types";
const PACKAGE_RELATIONSHIPS_NAMESPACE: &str =
    "http://schemas.openxmlformats.org/package/2006/relationships";

#[derive(Debug)]
struct EntryNameAssessment {
    unsafe_path: bool,
    ambiguous: bool,
}

#[derive(Debug)]
struct CentralDirectoryEvidence {
    entry_count: usize,
    zip64_present: bool,
}

#[derive(Debug)]
struct RelationshipInspection {
    relationships: Vec<DocxRelationship>,
    malformed: Vec<(String, DocxSourceLocation)>,
}

#[derive(Debug)]
struct PartExtraction {
    text: String,
}

#[derive(Debug)]
struct ExtractionState {
    summary: DocxDocumentSummary,
    segments: Vec<DocxTextSegment>,
    comments: Vec<DocxComment>,
    revisions: Vec<DocxRevision>,
    hyperlinks: Vec<DocxHyperlink>,
    deleted_text: String,
    paragraph_index: usize,
    table_index: usize,
}

impl ExtractionState {
    fn new() -> Self {
        Self {
            summary: DocxDocumentSummary::default(),
            segments: Vec::new(),
            comments: Vec::new(),
            revisions: Vec::new(),
            hyperlinks: Vec::new(),
            deleted_text: String::new(),
            paragraph_index: 0,
            table_index: 0,
        }
    }
}

/// Inspects caller-provided DOCX bytes entirely in memory.
///
/// The function performs no filesystem extraction, Office automation,
/// relationship resolution outside the package, network access, macro/OLE
/// execution, or source mutation.
pub fn analyze_docx_bytes(bytes: &[u8]) -> Result<DocxAnalysis, DocxAnalysisError> {
    if bytes.len() > MAX_DOCX_SOURCE_BYTES {
        return Err(DocxAnalysisError::ResourceLimitExceeded(
            DocxResourceLimit::SourceBytes,
        ));
    }

    let mut archive =
        ZipArchive::new(Cursor::new(bytes)).map_err(|_| DocxAnalysisError::InvalidZip)?;
    let archive_offset = archive.offset();
    let central_directory = inspect_central_directory(bytes, archive_offset)?;
    if central_directory.entry_count > MAX_DOCX_ENTRY_COUNT || archive.len() > MAX_DOCX_ENTRY_COUNT
    {
        return Err(DocxAnalysisError::ResourceLimitExceeded(
            DocxResourceLimit::EntryCount,
        ));
    }

    let mut zip64_present =
        central_directory.zip64_present || archive.raw_zip64_extensible_data_sector().is_some();
    let mut compressed_total = 0_u64;
    let mut uncompressed_total = 0_u64;
    let mut entry_indices = BTreeMap::new();
    let mut duplicate_names = BTreeSet::new();
    let mut parts = Vec::with_capacity(archive.len());
    let mut findings = Vec::new();
    let mut invalid_entry_names = 0;

    if archive_offset > 0 {
        push_finding(
            &mut findings,
            DocxFindingKind::PrependedZipDataPresent,
            DocxFindingSeverity::Informational,
            None,
            None,
            "The ZIP archive begins after prepended package bytes.",
            None,
        );
    }

    for index in 0..archive.len() {
        let file = archive
            .by_index(index)
            .map_err(|_| DocxAnalysisError::InvalidZip)?;
        let raw_name = file.name_raw();
        if raw_name.len() > MAX_DOCX_ENTRY_NAME_BYTES {
            return Err(DocxAnalysisError::ResourceLimitExceeded(
                DocxResourceLimit::EntryNameBytes,
            ));
        }

        let name = file.name().to_owned();
        if entry_indices.insert(name.clone(), index).is_some() {
            duplicate_names.insert(name.clone());
        }

        let name_assessment = assess_entry_name(&name, raw_name);
        if name_assessment.unsafe_path {
            invalid_entry_names += 1;
            push_finding(
                &mut findings,
                DocxFindingKind::UnsafeEntryName,
                DocxFindingSeverity::Caution,
                Some(name.clone()),
                None,
                "A ZIP entry name contains an unsafe absolute, parent, drive, UNC, or NUL form.",
                None,
            );
        }
        if name_assessment.ambiguous {
            push_finding(
                &mut findings,
                DocxFindingKind::AmbiguousEntryName,
                DocxFindingSeverity::Caution,
                Some(name.clone()),
                None,
                "A ZIP entry name has an ambiguous separator, encoding, or Unicode normalization form.",
                None,
            );
        }

        if file.encrypted() {
            return Err(DocxAnalysisError::EncryptedEntry);
        }
        if file.compression() != CompressionMethod::Stored
            && file.compression() != CompressionMethod::Deflated
        {
            return Err(DocxAnalysisError::UnsupportedCompression);
        }
        if file.size() > MAX_DOCX_PART_BYTES {
            return Err(DocxAnalysisError::ResourceLimitExceeded(
                DocxResourceLimit::PartBytes,
            ));
        }
        if ratio_exceeds(file.size(), file.compressed_size()) {
            return Err(DocxAnalysisError::ResourceLimitExceeded(
                DocxResourceLimit::CompressionRatio,
            ));
        }

        compressed_total = compressed_total.checked_add(file.compressed_size()).ok_or(
            DocxAnalysisError::ResourceLimitExceeded(DocxResourceLimit::TotalUncompressedBytes),
        )?;
        uncompressed_total = uncompressed_total.checked_add(file.size()).ok_or(
            DocxAnalysisError::ResourceLimitExceeded(DocxResourceLimit::TotalUncompressedBytes),
        )?;
        if uncompressed_total > MAX_DOCX_TOTAL_UNCOMPRESSED_BYTES {
            return Err(DocxAnalysisError::ResourceLimitExceeded(
                DocxResourceLimit::TotalUncompressedBytes,
            ));
        }

        zip64_present |= contains_extra_field(file.extra_data(), 0x0001);
        if file.is_symlink() {
            push_finding(
                &mut findings,
                DocxFindingKind::SymlinkEntryPresent,
                DocxFindingSeverity::Caution,
                Some(name.clone()),
                None,
                "The ZIP package declares a symbolic-link entry; TextScope does not extract it.",
                None,
            );
        }

        parts.push(DocxPartRecord {
            sequence: index + 1,
            part_name: name,
            content_type: None,
            compressed_size: file.compressed_size(),
            uncompressed_size: file.size(),
            crc32: file.crc32(),
            crc_status: DocxCrcStatus::DeclaredOnly,
            compression_method: format!("{:?}", file.compression()),
            classification: DocxPartClassification::Other,
            relationship_count: 0,
            encrypted: file.encrypted(),
            directory: file.is_dir(),
            symlink: file.is_symlink(),
            local_header_offset: file.header_start(),
            data_offset: file.data_start(),
            xml_encoding: None,
        });
    }

    if !duplicate_names.is_empty() {
        return Err(DocxAnalysisError::DuplicateEntryNames);
    }
    if ratio_exceeds(uncompressed_total, compressed_total) {
        return Err(DocxAnalysisError::ResourceLimitExceeded(
            DocxResourceLimit::CompressionRatio,
        ));
    }

    let content_types_index = required_entry(
        &entry_indices,
        CONTENT_TYPES_PART,
        DocxAnalysisError::MissingContentTypes,
    )?;
    required_entry(
        &entry_indices,
        ROOT_RELATIONSHIPS_PART,
        DocxAnalysisError::MissingRootRelationships,
    )?;
    required_entry(
        &entry_indices,
        MAIN_DOCUMENT_PART,
        DocxAnalysisError::MissingMainDocument,
    )?;

    let content_types_decode = read_xml_part(
        &mut archive,
        &mut parts,
        content_types_index,
        CONTENT_TYPES_PART,
    )?;
    let content_types = parse_content_types(CONTENT_TYPES_PART, &content_types_decode)?;

    for part in &mut parts {
        part.content_type = content_type_for(&part.part_name, &content_types);
        part.classification = classify_part(&part.part_name, part.content_type.as_deref());
    }

    let main_content_type = content_type_for(MAIN_DOCUMENT_PART, &content_types)
        .ok_or(DocxAnalysisError::UnsupportedPackage)?;
    if main_content_type != MAIN_DOCUMENT_CONTENT_TYPE {
        return Err(DocxAnalysisError::UnsupportedPackage);
    }

    for part in &parts {
        match part.classification {
            DocxPartClassification::EmbeddedObject => push_finding(
                &mut findings,
                DocxFindingKind::EmbeddedObjectPresent,
                DocxFindingSeverity::Caution,
                Some(part.part_name.clone()),
                None,
                "An embedded-object-related package part is present; TextScope does not execute it.",
                None,
            ),
            DocxPartClassification::ActiveContentRelated => push_finding(
                &mut findings,
                DocxFindingKind::ActiveContentRelatedPartPresent,
                DocxFindingSeverity::Caution,
                Some(part.part_name.clone()),
                None,
                "An active-content-related or macro-incompatible package part is present.",
                None,
            ),
            DocxPartClassification::CustomXml => push_finding(
                &mut findings,
                DocxFindingKind::CustomXmlPresent,
                DocxFindingSeverity::Informational,
                Some(part.part_name.clone()),
                None,
                "A custom XML package part is present and remains outside document text.",
                None,
            ),
            DocxPartClassification::Other if is_unknown_binary_part(part) => push_finding(
                &mut findings,
                DocxFindingKind::UnexpectedPackagePart,
                DocxFindingSeverity::Caution,
                Some(part.part_name.clone()),
                None,
                "An unclassified binary part is present in the declared DOCX package.",
                None,
            ),
            _ => {}
        }
    }

    let relationship_part_names: Vec<String> = parts
        .iter()
        .filter(|part| part.classification == DocxPartClassification::Relationships)
        .map(|part| part.part_name.clone())
        .collect();
    let mut relationship_inspection = RelationshipInspection {
        relationships: Vec::new(),
        malformed: Vec::new(),
    };

    for relationship_part in relationship_part_names {
        let Some(index) = entry_indices.get(&relationship_part).copied() else {
            continue;
        };
        let decode = read_xml_part(&mut archive, &mut parts, index, &relationship_part)?;
        parse_relationships(&relationship_part, &decode, &mut relationship_inspection)?;
    }

    if relationship_inspection.relationships.len() > MAX_DOCX_RELATIONSHIPS {
        return Err(DocxAnalysisError::ResourceLimitExceeded(
            DocxResourceLimit::Relationships,
        ));
    }

    let root_office_document = relationship_inspection
        .relationships
        .iter()
        .any(|relationship| {
            relationship.source_part.is_none()
                && !relationship.external
                && relationship.relationship_type.ends_with("/officeDocument")
                && relationship.resolved_internal_part.as_deref() == Some(MAIN_DOCUMENT_PART)
        });
    if !root_office_document {
        return Err(DocxAnalysisError::MalformedRelationships);
    }

    for relationship in &relationship_inspection.relationships {
        if let Some(source_part) = &relationship.source_part
            && let Some(part) = parts.iter_mut().find(|part| part.part_name == *source_part)
        {
            part.relationship_count += 1;
        }
        if relationship.external {
            push_finding(
                &mut findings,
                DocxFindingKind::ExternalRelationshipPresent,
                DocxFindingSeverity::Caution,
                relationship.source_part.clone(),
                Some(relationship.sequence),
                "An external relationship is declared; TextScope records but never follows it.",
                Some(relationship.source_location.clone()),
            );
        }
        if is_embedded_relationship(&relationship.relationship_type) {
            push_finding(
                &mut findings,
                DocxFindingKind::EmbeddedObjectPresent,
                DocxFindingSeverity::Caution,
                relationship.source_part.clone(),
                Some(relationship.sequence),
                "An embedded-object-related relationship is declared and remains inert.",
                Some(relationship.source_location.clone()),
            );
        }
        if is_active_relationship(&relationship.relationship_type) {
            push_finding(
                &mut findings,
                DocxFindingKind::ActiveContentRelatedPartPresent,
                DocxFindingSeverity::Caution,
                relationship.source_part.clone(),
                Some(relationship.sequence),
                "An active-content-related relationship is declared and remains inert.",
                Some(relationship.source_location.clone()),
            );
        }
    }
    for (part_name, location) in &relationship_inspection.malformed {
        push_finding(
            &mut findings,
            DocxFindingKind::MalformedRelationship,
            DocxFindingSeverity::Caution,
            Some(part_name.clone()),
            None,
            "A relationship element is missing a required declaration or has an unsafe internal target.",
            Some(location.clone()),
        );
    }

    let mut metadata = DocxMetadata::default();
    parse_metadata_if_present(
        &mut archive,
        &entry_indices,
        &mut parts,
        "docProps/core.xml",
        MetadataKind::Core,
        &mut metadata,
    )?;
    parse_metadata_if_present(
        &mut archive,
        &entry_indices,
        &mut parts,
        "docProps/app.xml",
        MetadataKind::App,
        &mut metadata,
    )?;
    parse_metadata_if_present(
        &mut archive,
        &entry_indices,
        &mut parts,
        "docProps/custom.xml",
        MetadataKind::Custom,
        &mut metadata,
    )?;
    if !metadata.core_properties.is_empty()
        || !metadata.app_properties.is_empty()
        || !metadata.custom_properties.is_empty()
    {
        push_finding(
            &mut findings,
            DocxFindingKind::MetadataPresent,
            DocxFindingSeverity::Informational,
            None,
            None,
            "The package stores document metadata declarations; they are not verified attribution.",
            None,
        );
    }

    let mut extraction = ExtractionState::new();
    extraction.summary.main_document_present = true;
    extraction.summary.styles_present = entry_indices.contains_key("word/styles.xml");
    extraction.summary.numbering_present = entry_indices.contains_key("word/numbering.xml");
    extraction.summary.media_parts = parts
        .iter()
        .filter(|part| part.classification == DocxPartClassification::Media)
        .count();
    extraction.summary.embedded_object_parts = parts
        .iter()
        .filter(|part| part.classification == DocxPartClassification::EmbeddedObject)
        .count();
    extraction.summary.custom_xml_parts = parts
        .iter()
        .filter(|part| part.classification == DocxPartClassification::CustomXml)
        .count();
    extraction.summary.external_relationships = relationship_inspection
        .relationships
        .iter()
        .filter(|relationship| relationship.external)
        .count();

    let main_index = required_entry(
        &entry_indices,
        MAIN_DOCUMENT_PART,
        DocxAnalysisError::MissingMainDocument,
    )?;
    let main_decode = read_xml_part(&mut archive, &mut parts, main_index, MAIN_DOCUMENT_PART)?;
    let document_text = extract_standard_part(
        MAIN_DOCUMENT_PART,
        &main_decode,
        DocxTextChannel::Body,
        &relationship_inspection.relationships,
        &mut extraction,
    )?
    .text;

    let mut text_channels = DocxTextChannels {
        document_text,
        ..DocxTextChannels::default()
    };

    let referenced_headers = referenced_parts(
        &relationship_inspection.relationships,
        MAIN_DOCUMENT_PART,
        "/header",
    );
    for part_name in referenced_headers {
        if let Some(channel) = extract_named_part_if_present(
            &mut archive,
            &entry_indices,
            &mut parts,
            &part_name,
            DocxTextChannel::Header,
            &relationship_inspection.relationships,
            &mut extraction,
        )? {
            text_channels.headers.push(channel);
        }
    }

    let referenced_footers = referenced_parts(
        &relationship_inspection.relationships,
        MAIN_DOCUMENT_PART,
        "/footer",
    );
    for part_name in referenced_footers {
        if let Some(channel) = extract_named_part_if_present(
            &mut archive,
            &entry_indices,
            &mut parts,
            &part_name,
            DocxTextChannel::Footer,
            &relationship_inspection.relationships,
            &mut extraction,
        )? {
            text_channels.footers.push(channel);
        }
    }

    if let Some(channel) = extract_notes_if_present(
        &mut archive,
        &entry_indices,
        &mut parts,
        "word/footnotes.xml",
        "footnote",
        DocxTextChannel::Footnote,
        &relationship_inspection.relationships,
        &mut extraction,
    )? {
        text_channels.footnotes.push(channel);
    }
    if let Some(channel) = extract_notes_if_present(
        &mut archive,
        &entry_indices,
        &mut parts,
        "word/endnotes.xml",
        "endnote",
        DocxTextChannel::Endnote,
        &relationship_inspection.relationships,
        &mut extraction,
    )? {
        text_channels.endnotes.push(channel);
    }
    if let Some(channel) = extract_comments_if_present(
        &mut archive,
        &entry_indices,
        &mut parts,
        &relationship_inspection.relationships,
        &mut extraction,
    )? {
        text_channels.comments.push(channel);
    }
    text_channels.deleted_content = extraction.deleted_text.clone();

    extraction.summary.headers = text_channels.headers.len();
    extraction.summary.footers = text_channels.footers.len();
    extraction.summary.tracked_changes = extraction.revisions.len();
    extraction.summary.hyperlinks = extraction.hyperlinks.len();
    if !extraction.revisions.is_empty() {
        push_finding(
            &mut findings,
            DocxFindingKind::TrackedChangesPresent,
            DocxFindingSeverity::Informational,
            Some(MAIN_DOCUMENT_PART.to_owned()),
            None,
            "Tracked changes are present; TextScope does not accept or reject them.",
            extraction
                .revisions
                .first()
                .map(|revision| revision.source_location.clone()),
        );
    }
    if !extraction.comments.is_empty() {
        push_finding(
            &mut findings,
            DocxFindingKind::CommentsPresent,
            DocxFindingSeverity::Informational,
            Some("word/comments.xml".to_owned()),
            None,
            "Document comments are present and remain outside the main text channel.",
            extraction
                .comments
                .first()
                .map(|comment| comment.source_location.clone()),
        );
    }
    if extraction.summary.explicit_hidden_runs > 0 {
        push_finding(
            &mut findings,
            DocxFindingKind::ExplicitHiddenTextPresent,
            DocxFindingSeverity::Informational,
            None,
            None,
            "One or more runs carry an explicit w:vanish marker; effective rendering is not inferred.",
            extraction
                .segments
                .iter()
                .find(|segment| segment.explicit_hidden)
                .map(|segment| segment.source_location.clone()),
        );
    }

    let status = if invalid_entry_names > 0
        || !relationship_inspection.malformed.is_empty()
        || parts.iter().any(|part| part.symlink)
    {
        DocxPackageStatus::MalformedPackage
    } else {
        DocxPackageStatus::ValidWordprocessingPackage
    };
    let package = DocxPackageSummary {
        status,
        raw_byte_length: bytes.len(),
        entry_count: central_directory.entry_count,
        compressed_total,
        uncompressed_total,
        compression_ratio_milli: ratio_milli(uncompressed_total, compressed_total),
        zip64_present,
        archive_offset,
        duplicate_entry_names: 0,
        invalid_entry_names,
    };
    let scan = scan_text(&text_channels.document_text);
    let provenance = docx_provenance(&metadata);

    Ok(DocxAnalysis {
        package,
        parts,
        content_types,
        relationships: relationship_inspection.relationships,
        metadata,
        provenance,
        summary: extraction.summary,
        text_segments: extraction.segments,
        comments: extraction.comments,
        revisions: extraction.revisions,
        hyperlinks: extraction.hyperlinks,
        findings,
        text_channels,
        scan,
    })
}

fn docx_provenance(metadata: &DocxMetadata) -> Vec<ProvenanceEvidence> {
    let mut evidence = Vec::new();
    let groups = [
        (
            &metadata.core_properties,
            EvidenceSource::DocxCoreProperties,
        ),
        (&metadata.app_properties, EvidenceSource::DocxAppProperties),
        (
            &metadata.custom_properties,
            EvidenceSource::DocxCustomProperties,
        ),
    ];
    for (properties, source) in groups {
        for property in properties {
            let normalized = property.name.trim().to_ascii_lowercase();
            let kind = match normalized.as_str() {
                "creator" | "author" => EvidenceKind::CreatorDeclaration,
                "lastmodifiedby" | "last_modified_by" => EvidenceKind::AuthorDeclaration,
                "application" | "generator" => EvidenceKind::GeneratorDeclaration,
                "producer" => EvidenceKind::ProducerDeclaration,
                _ => EvidenceKind::MetadataDeclaration,
            };
            evidence.push(ProvenanceEvidence::declared(
                kind,
                source,
                format!(
                    "{}[{}].{}",
                    source_label(source),
                    property.sequence,
                    property.name
                ),
                &property.value,
            ));
        }
    }
    evidence
}

fn source_label(source: EvidenceSource) -> &'static str {
    match source {
        EvidenceSource::DocxCoreProperties => "docx.core_properties",
        EvidenceSource::DocxAppProperties => "docx.app_properties",
        EvidenceSource::DocxCustomProperties => "docx.custom_properties",
        _ => "docx.metadata",
    }
}

fn required_entry(
    entries: &BTreeMap<String, usize>,
    name: &str,
    error: DocxAnalysisError,
) -> Result<usize, DocxAnalysisError> {
    entries.get(name).copied().ok_or(error)
}

fn inspect_central_directory(
    bytes: &[u8],
    archive_offset: u64,
) -> Result<CentralDirectoryEvidence, DocxAnalysisError> {
    let eocd = find_end_of_central_directory(bytes).ok_or(DocxAnalysisError::InvalidZip)?;
    let entries_16 = read_u16(bytes, eocd + 10).ok_or(DocxAnalysisError::InvalidZip)?;
    let size_32 = read_u32(bytes, eocd + 12).ok_or(DocxAnalysisError::InvalidZip)?;
    let offset_32 = read_u32(bytes, eocd + 16).ok_or(DocxAnalysisError::InvalidZip)?;
    let zip64 = entries_16 == u16::MAX || size_32 == u32::MAX || offset_32 == u32::MAX;

    let (entry_count_u64, central_size_u64, central_offset_u64) = if zip64 {
        let locator = eocd.checked_sub(20).ok_or(DocxAnalysisError::InvalidZip)?;
        if bytes.get(locator..locator + 4) != Some(b"PK\x06\x07") {
            return Err(DocxAnalysisError::InvalidZip);
        }
        let record_relative = read_u64(bytes, locator + 8).ok_or(DocxAnalysisError::InvalidZip)?;
        let record = archive_offset
            .checked_add(record_relative)
            .and_then(|value| usize::try_from(value).ok())
            .ok_or(DocxAnalysisError::InvalidZip)?;
        if bytes.get(record..record + 4) != Some(b"PK\x06\x06") {
            return Err(DocxAnalysisError::InvalidZip);
        }
        (
            read_u64(bytes, record + 32).ok_or(DocxAnalysisError::InvalidZip)?,
            read_u64(bytes, record + 40).ok_or(DocxAnalysisError::InvalidZip)?,
            read_u64(bytes, record + 48).ok_or(DocxAnalysisError::InvalidZip)?,
        )
    } else {
        (
            u64::from(entries_16),
            u64::from(size_32),
            u64::from(offset_32),
        )
    };
    let entry_count = usize::try_from(entry_count_u64)
        .map_err(|_| DocxAnalysisError::ResourceLimitExceeded(DocxResourceLimit::EntryCount))?;
    if entry_count > MAX_DOCX_ENTRY_COUNT {
        return Err(DocxAnalysisError::ResourceLimitExceeded(
            DocxResourceLimit::EntryCount,
        ));
    }
    let central_start = archive_offset
        .checked_add(central_offset_u64)
        .and_then(|value| usize::try_from(value).ok())
        .ok_or(DocxAnalysisError::InvalidZip)?;
    let central_size =
        usize::try_from(central_size_u64).map_err(|_| DocxAnalysisError::InvalidZip)?;
    let central_end = central_start
        .checked_add(central_size)
        .filter(|end| *end <= bytes.len())
        .ok_or(DocxAnalysisError::InvalidZip)?;
    let mut cursor = central_start;
    let mut names = BTreeSet::new();
    for _ in 0..entry_count {
        if cursor + 46 > central_end || bytes.get(cursor..cursor + 4) != Some(b"PK\x01\x02") {
            return Err(DocxAnalysisError::InvalidZip);
        }
        let name_length =
            usize::from(read_u16(bytes, cursor + 28).ok_or(DocxAnalysisError::InvalidZip)?);
        let extra_length =
            usize::from(read_u16(bytes, cursor + 30).ok_or(DocxAnalysisError::InvalidZip)?);
        let comment_length =
            usize::from(read_u16(bytes, cursor + 32).ok_or(DocxAnalysisError::InvalidZip)?);
        if name_length > MAX_DOCX_ENTRY_NAME_BYTES {
            return Err(DocxAnalysisError::ResourceLimitExceeded(
                DocxResourceLimit::EntryNameBytes,
            ));
        }
        let name_start = cursor + 46;
        let name_end = name_start
            .checked_add(name_length)
            .filter(|end| *end <= central_end)
            .ok_or(DocxAnalysisError::InvalidZip)?;
        let name = bytes
            .get(name_start..name_end)
            .ok_or(DocxAnalysisError::InvalidZip)?;
        if !names.insert(name.to_vec()) {
            return Err(DocxAnalysisError::DuplicateEntryNames);
        }
        cursor = name_end
            .checked_add(extra_length)
            .and_then(|value| value.checked_add(comment_length))
            .filter(|value| *value <= central_end)
            .ok_or(DocxAnalysisError::InvalidZip)?;
    }
    if cursor != central_end {
        return Err(DocxAnalysisError::InvalidZip);
    }
    Ok(CentralDirectoryEvidence {
        entry_count,
        zip64_present: zip64,
    })
}

fn find_end_of_central_directory(bytes: &[u8]) -> Option<usize> {
    if bytes.len() < 22 {
        return None;
    }
    let start = bytes.len().saturating_sub(22 + usize::from(u16::MAX));
    for offset in (start..=bytes.len() - 22).rev() {
        if bytes.get(offset..offset + 4) != Some(b"PK\x05\x06") {
            continue;
        }
        let comment_length = usize::from(read_u16(bytes, offset + 20)?);
        if offset.checked_add(22 + comment_length) == Some(bytes.len()) {
            return Some(offset);
        }
    }
    None
}

fn read_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        bytes.get(offset..offset + 2)?.try_into().ok()?,
    ))
}

fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        bytes.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

fn read_u64(bytes: &[u8], offset: usize) -> Option<u64> {
    Some(u64::from_le_bytes(
        bytes.get(offset..offset + 8)?.try_into().ok()?,
    ))
}

fn ratio_exceeds(uncompressed: u64, compressed: u64) -> bool {
    uncompressed > 0
        && (compressed == 0
            || uncompressed / compressed > MAX_DOCX_COMPRESSION_RATIO
            || (uncompressed / compressed == MAX_DOCX_COMPRESSION_RATIO
                && !uncompressed.is_multiple_of(compressed)))
}

fn ratio_milli(uncompressed: u64, compressed: u64) -> u64 {
    if uncompressed == 0 || compressed == 0 {
        return 0;
    }
    uncompressed
        .saturating_mul(1_000)
        .checked_div(compressed)
        .unwrap_or(u64::MAX)
}

fn contains_extra_field(extra_data: Option<&[u8]>, wanted: u16) -> bool {
    let Some(data) = extra_data else {
        return false;
    };
    let mut offset = 0;
    while offset + 4 <= data.len() {
        let id = u16::from_le_bytes([data[offset], data[offset + 1]]);
        let length = usize::from(u16::from_le_bytes([data[offset + 2], data[offset + 3]]));
        offset += 4;
        if id == wanted {
            return true;
        }
        let Some(next) = offset.checked_add(length) else {
            return false;
        };
        if next > data.len() {
            return false;
        }
        offset = next;
    }
    false
}

fn assess_entry_name(name: &str, raw_name: &[u8]) -> EntryNameAssessment {
    let ascii = name.as_bytes();
    let drive_prefix = ascii.len() >= 2 && ascii[0].is_ascii_alphabetic() && ascii[1] == b':';
    let unsafe_path = raw_name.contains(&0)
        || name.starts_with('/')
        || name.starts_with('\\')
        || name.starts_with("//")
        || name.starts_with("\\\\")
        || drive_prefix
        || name.split(['/', '\\']).any(|component| component == "..");
    let normalized: String = name.nfc().collect();
    let ambiguous = std::str::from_utf8(raw_name).is_err()
        || name.contains('\\')
        || name.contains("//")
        || name.split('/').any(|component| component == ".")
        || normalized != name;
    EntryNameAssessment {
        unsafe_path,
        ambiguous,
    }
}

fn read_xml_part(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    parts: &mut [DocxPartRecord],
    index: usize,
    expected_name: &str,
) -> Result<TextDecodeResult, DocxAnalysisError> {
    let mut file = archive
        .by_index(index)
        .map_err(|_| DocxAnalysisError::PartReadFailure)?;
    if file.name() != expected_name {
        return Err(DocxAnalysisError::PartReadFailure);
    }
    if file.size() > MAX_DOCX_XML_PART_BYTES {
        return Err(DocxAnalysisError::ResourceLimitExceeded(
            DocxResourceLimit::XmlPartBytes,
        ));
    }
    let capacity = usize::try_from(file.size())
        .map_err(|_| DocxAnalysisError::ResourceLimitExceeded(DocxResourceLimit::XmlPartBytes))?;
    let mut bytes = Vec::with_capacity(capacity);
    file.by_ref()
        .take(MAX_DOCX_XML_PART_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| DocxAnalysisError::PartReadFailure)?;
    if bytes.len() as u64 > MAX_DOCX_XML_PART_BYTES {
        return Err(DocxAnalysisError::ResourceLimitExceeded(
            DocxResourceLimit::XmlPartBytes,
        ));
    }
    let decode = decode_text_bytes(&bytes).map_err(|_| DocxAnalysisError::MalformedXml)?;
    if let Some(part) = parts.get_mut(index) {
        part.crc_status = DocxCrcStatus::Verified;
        part.xml_encoding = Some(decode.encoding);
    }
    Ok(decode)
}

fn parse_document<'a>(decode: &'a TextDecodeResult) -> Result<Document<'a>, DocxAnalysisError> {
    let options = ParsingOptions {
        allow_dtd: false,
        nodes_limit: MAX_DOCX_XML_NODES,
        entity_resolver: None,
    };
    let document =
        Document::parse_with_options(&decode.decoded_text, options).map_err(|error| {
            if matches!(error, roxmltree::Error::NodesLimitReached) {
                DocxAnalysisError::ResourceLimitExceeded(DocxResourceLimit::XmlNodes)
            } else {
                DocxAnalysisError::MalformedXml
            }
        })?;
    for node in document.descendants() {
        if node.ancestors().take(MAX_DOCX_XML_DEPTH + 2).count() > MAX_DOCX_XML_DEPTH + 1 {
            return Err(DocxAnalysisError::ResourceLimitExceeded(
                DocxResourceLimit::XmlDepth,
            ));
        }
    }
    Ok(document)
}

fn parse_content_types(
    part_name: &str,
    decode: &TextDecodeResult,
) -> Result<Vec<DocxContentTypeDeclaration>, DocxAnalysisError> {
    let document = parse_document(decode)?;
    if document.root_element().tag_name().name() != "Types"
        || document.root_element().tag_name().namespace() != Some(CONTENT_TYPES_NAMESPACE)
    {
        return Err(DocxAnalysisError::UnsupportedPackage);
    }
    let mut declarations = Vec::new();
    for node in document.root_element().children().filter(|node| {
        node.is_element() && node.tag_name().namespace() == Some(CONTENT_TYPES_NAMESPACE)
    }) {
        let (kind, extension, declared_part) = match node.tag_name().name() {
            "Default" => (
                DocxContentTypeDeclarationKind::Default,
                node.attribute("Extension").map(str::to_owned),
                None,
            ),
            "Override" => (
                DocxContentTypeDeclarationKind::Override,
                None,
                node.attribute("PartName").map(str::to_owned),
            ),
            _ => continue,
        };
        let Some(content_type) = node.attribute("ContentType") else {
            return Err(DocxAnalysisError::UnsupportedPackage);
        };
        if extension.is_none() && declared_part.is_none() {
            return Err(DocxAnalysisError::UnsupportedPackage);
        }
        declarations.push(DocxContentTypeDeclaration {
            sequence: declarations.len() + 1,
            kind,
            extension,
            part_name: declared_part,
            content_type: content_type.to_owned(),
            source_location: source_location(part_name, decode, Some(node.range())),
        });
    }
    Ok(declarations)
}

fn content_type_for(name: &str, declarations: &[DocxContentTypeDeclaration]) -> Option<String> {
    let override_name = format!("/{name}");
    if let Some(declaration) = declarations.iter().find(|declaration| {
        declaration.kind == DocxContentTypeDeclarationKind::Override
            && declaration.part_name.as_deref() == Some(override_name.as_str())
    }) {
        return Some(declaration.content_type.clone());
    }
    let extension = name.rsplit_once('.').map(|(_, extension)| extension)?;
    declarations
        .iter()
        .find(|declaration| {
            declaration.kind == DocxContentTypeDeclarationKind::Default
                && declaration
                    .extension
                    .as_deref()
                    .is_some_and(|candidate| candidate.eq_ignore_ascii_case(extension))
        })
        .map(|declaration| declaration.content_type.clone())
}

fn classify_part(name: &str, content_type: Option<&str>) -> DocxPartClassification {
    let lower_name = name.to_ascii_lowercase();
    let lower_type = content_type.unwrap_or_default().to_ascii_lowercase();
    if name == CONTENT_TYPES_PART {
        DocxPartClassification::ContentTypes
    } else if name == MAIN_DOCUMENT_PART {
        DocxPartClassification::MainDocument
    } else if name.ends_with(".rels")
        && (name == ROOT_RELATIONSHIPS_PART || name.contains("/_rels/"))
    {
        DocxPartClassification::Relationships
    } else if name == "word/styles.xml" {
        DocxPartClassification::Styles
    } else if name == "word/numbering.xml" {
        DocxPartClassification::Numbering
    } else if name == "word/settings.xml" {
        DocxPartClassification::Settings
    } else if name.starts_with("word/theme/") {
        DocxPartClassification::Theme
    } else if name == "word/fontTable.xml" {
        DocxPartClassification::FontTable
    } else if name.starts_with("word/header") && name.ends_with(".xml") {
        DocxPartClassification::Header
    } else if name.starts_with("word/footer") && name.ends_with(".xml") {
        DocxPartClassification::Footer
    } else if name == "word/footnotes.xml" {
        DocxPartClassification::Footnotes
    } else if name == "word/endnotes.xml" {
        DocxPartClassification::Endnotes
    } else if name == "word/comments.xml" {
        DocxPartClassification::Comments
    } else if name.starts_with("docProps/") {
        DocxPartClassification::Metadata
    } else if name.starts_with("word/media/") {
        DocxPartClassification::Media
    } else if name.starts_with("word/embeddings/")
        || lower_type.contains("oleobject")
        || lower_type.contains("embedded-package")
    {
        DocxPartClassification::EmbeddedObject
    } else if name.starts_with("customXml/") {
        DocxPartClassification::CustomXml
    } else if lower_name.contains("vbaproject")
        || lower_name.starts_with("word/activex/")
        || lower_type.contains("macroenabled")
        || lower_type.contains("vba")
        || lower_type.contains("activex")
    {
        DocxPartClassification::ActiveContentRelated
    } else {
        DocxPartClassification::Other
    }
}

fn is_unknown_binary_part(part: &DocxPartRecord) -> bool {
    let lower_name = part.part_name.to_ascii_lowercase();
    let lower_type = part
        .content_type
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();
    lower_name.ends_with(".bin")
        || lower_name.ends_with(".exe")
        || lower_name.ends_with(".dll")
        || lower_type == "application/octet-stream"
}

fn parse_relationships(
    relationship_part: &str,
    decode: &TextDecodeResult,
    inspection: &mut RelationshipInspection,
) -> Result<(), DocxAnalysisError> {
    let document = parse_document(decode)?;
    if document.root_element().tag_name().name() != "Relationships"
        || document.root_element().tag_name().namespace() != Some(PACKAGE_RELATIONSHIPS_NAMESPACE)
    {
        return Err(DocxAnalysisError::MalformedRelationships);
    }
    let source_part = relationship_source_part(relationship_part);
    let mut ids = BTreeSet::new();
    for node in document.root_element().children().filter(|node| {
        node.is_element()
            && node.tag_name().name() == "Relationship"
            && node.tag_name().namespace() == Some(PACKAGE_RELATIONSHIPS_NAMESPACE)
    }) {
        let location = source_location(relationship_part, decode, Some(node.range()));
        let (Some(id), Some(relationship_type), Some(target)) = (
            node.attribute("Id"),
            node.attribute("Type"),
            node.attribute("Target"),
        ) else {
            inspection
                .malformed
                .push((relationship_part.to_owned(), location));
            continue;
        };
        if !ids.insert(id.to_owned()) {
            inspection
                .malformed
                .push((relationship_part.to_owned(), location));
            continue;
        }
        let external = node
            .attribute("TargetMode")
            .is_some_and(|mode| mode.eq_ignore_ascii_case("External"));
        let resolved_internal_part = if external {
            None
        } else {
            resolve_internal_target(source_part.as_deref(), target)
        };
        if !external && resolved_internal_part.is_none() {
            inspection
                .malformed
                .push((relationship_part.to_owned(), location.clone()));
        }
        if inspection.relationships.len() >= MAX_DOCX_RELATIONSHIPS {
            return Err(DocxAnalysisError::ResourceLimitExceeded(
                DocxResourceLimit::Relationships,
            ));
        }
        inspection.relationships.push(DocxRelationship {
            sequence: inspection.relationships.len() + 1,
            relationship_part: relationship_part.to_owned(),
            source_part: source_part.clone(),
            id: id.to_owned(),
            relationship_type: relationship_type.to_owned(),
            target: target.to_owned(),
            external,
            resolved_internal_part,
            source_location: location,
        });
    }
    Ok(())
}

fn relationship_source_part(relationship_part: &str) -> Option<String> {
    if relationship_part == ROOT_RELATIONSHIPS_PART {
        return None;
    }
    let marker = "/_rels/";
    let (directory, file) = relationship_part.split_once(marker)?;
    let source_file = file.strip_suffix(".rels")?;
    Some(format!("{directory}/{source_file}"))
}

fn resolve_internal_target(source_part: Option<&str>, target: &str) -> Option<String> {
    if target.is_empty()
        || target.contains('\0')
        || target.contains('\\')
        || target.contains('#')
        || has_uri_scheme(target)
    {
        return None;
    }
    let mut components: Vec<&str> = Vec::new();
    if !target.starts_with('/')
        && let Some(source) = source_part
        && let Some((directory, _)) = source.rsplit_once('/')
    {
        components.extend(
            directory
                .split('/')
                .filter(|component| !component.is_empty()),
        );
    }
    for component in target.trim_start_matches('/').split('/') {
        match component {
            "" | "." => {}
            ".." => {
                components.pop()?;
            }
            value => components.push(value),
        }
    }
    if components.is_empty() {
        None
    } else {
        Some(components.join("/"))
    }
}

fn has_uri_scheme(value: &str) -> bool {
    let Some((scheme, _)) = value.split_once(':') else {
        return false;
    };
    !scheme.is_empty()
        && scheme.bytes().enumerate().all(|(index, byte)| {
            if index == 0 {
                byte.is_ascii_alphabetic()
            } else {
                byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-' | b'.')
            }
        })
}

fn is_embedded_relationship(relationship_type: &str) -> bool {
    relationship_type.ends_with("/oleObject")
        || relationship_type.ends_with("/package")
        || relationship_type.ends_with("/control")
}

fn is_active_relationship(relationship_type: &str) -> bool {
    relationship_type.ends_with("/attachedTemplate")
        || relationship_type.ends_with("/vbaProject")
        || relationship_type.ends_with("/activeXControl")
}

#[derive(Clone, Copy)]
enum MetadataKind {
    Core,
    App,
    Custom,
}

fn parse_metadata_if_present(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    entries: &BTreeMap<String, usize>,
    parts: &mut [DocxPartRecord],
    part_name: &str,
    kind: MetadataKind,
    metadata: &mut DocxMetadata,
) -> Result<(), DocxAnalysisError> {
    let Some(index) = entries.get(part_name).copied() else {
        return Ok(());
    };
    let decode = read_xml_part(archive, parts, index, part_name)?;
    let document = parse_document(&decode)?;
    let destination = match kind {
        MetadataKind::Core => &mut metadata.core_properties,
        MetadataKind::App => &mut metadata.app_properties,
        MetadataKind::Custom => &mut metadata.custom_properties,
    };
    match kind {
        MetadataKind::Core | MetadataKind::App => {
            for node in document.root_element().children().filter(Node::is_element) {
                destination.push(DocxMetadataProperty {
                    sequence: destination.len() + 1,
                    name: node.tag_name().name().to_owned(),
                    value: descendant_text(node),
                    value_type: None,
                    source_location: source_location(part_name, &decode, Some(node.range())),
                });
            }
        }
        MetadataKind::Custom => {
            for node in document
                .root_element()
                .children()
                .filter(|node| node.is_element() && node.tag_name().name() == "property")
            {
                let value_node = node.children().find(Node::is_element);
                destination.push(DocxMetadataProperty {
                    sequence: destination.len() + 1,
                    name: node.attribute("name").unwrap_or("(unnamed)").to_owned(),
                    value: value_node.map(descendant_text).unwrap_or_default(),
                    value_type: value_node.map(|value| value.tag_name().name().to_owned()),
                    source_location: source_location(part_name, &decode, Some(node.range())),
                });
            }
        }
    }
    Ok(())
}

fn descendant_text(node: Node<'_, '_>) -> String {
    node.descendants()
        .filter(Node::is_text)
        .filter_map(|child| child.text())
        .collect()
}

fn referenced_parts(
    relationships: &[DocxRelationship],
    source_part: &str,
    type_suffix: &str,
) -> Vec<String> {
    let mut values = Vec::new();
    for relationship in relationships {
        if relationship.source_part.as_deref() == Some(source_part)
            && !relationship.external
            && relationship.relationship_type.ends_with(type_suffix)
            && let Some(target) = &relationship.resolved_internal_part
            && !values.contains(target)
        {
            values.push(target.clone());
        }
    }
    values
}

fn extract_named_part_if_present(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    entries: &BTreeMap<String, usize>,
    parts: &mut [DocxPartRecord],
    part_name: &str,
    channel: DocxTextChannel,
    relationships: &[DocxRelationship],
    state: &mut ExtractionState,
) -> Result<Option<DocxPartTextChannel>, DocxAnalysisError> {
    let Some(index) = entries.get(part_name).copied() else {
        return Ok(None);
    };
    let decode = read_xml_part(archive, parts, index, part_name)?;
    let extraction = extract_standard_part(part_name, &decode, channel, relationships, state)?;
    Ok(Some(DocxPartTextChannel {
        part_name: part_name.to_owned(),
        text: extraction.text,
    }))
}

fn extract_standard_part(
    part_name: &str,
    decode: &TextDecodeResult,
    channel: DocxTextChannel,
    relationships: &[DocxRelationship],
    state: &mut ExtractionState,
) -> Result<PartExtraction, DocxAnalysisError> {
    let document = parse_document(decode)?;
    let root = document.root_element();
    if part_name == MAIN_DOCUMENT_PART
        && (!is_word_element(root, "document")
            || !root.descendants().any(|node| is_word_element(node, "body")))
    {
        return Err(DocxAnalysisError::UnsupportedPackage);
    }
    let container = if part_name == MAIN_DOCUMENT_PART {
        root.descendants()
            .find(|node| is_word_element(*node, "body"))
            .ok_or(DocxAnalysisError::UnsupportedPackage)?
    } else {
        root
    };
    let text = extract_block_children(
        container,
        part_name,
        decode,
        channel,
        relationships,
        state,
        None,
        None,
        None,
    )?;
    Ok(PartExtraction { text })
}

fn is_word_element(node: Node<'_, '_>, name: &str) -> bool {
    node.is_element()
        && node.tag_name().name() == name
        && node.tag_name().namespace() == Some(WORDPROCESSING_NAMESPACE)
}

#[allow(clippy::too_many_arguments)]
fn extract_block_children(
    container: Node<'_, '_>,
    part_name: &str,
    decode: &TextDecodeResult,
    channel: DocxTextChannel,
    relationships: &[DocxRelationship],
    state: &mut ExtractionState,
    table_index: Option<usize>,
    row_index: Option<usize>,
    cell_index: Option<usize>,
) -> Result<String, DocxAnalysisError> {
    let mut blocks = Vec::new();
    for child in container.children().filter(Node::is_element) {
        match (
            child.tag_name().namespace() == Some(WORDPROCESSING_NAMESPACE),
            child.tag_name().name(),
        ) {
            (true, "p") => blocks.push(extract_paragraph(
                child,
                part_name,
                decode,
                channel,
                relationships,
                state,
                table_index,
                row_index,
                cell_index,
            )?),
            (true, "tbl") => blocks.push(extract_table(
                child,
                part_name,
                decode,
                channel,
                relationships,
                state,
            )?),
            _ => {
                let nested = extract_block_children(
                    child,
                    part_name,
                    decode,
                    channel,
                    relationships,
                    state,
                    table_index,
                    row_index,
                    cell_index,
                )?;
                if !nested.is_empty() {
                    blocks.push(nested);
                }
            }
        }
    }
    Ok(blocks.join("\n"))
}

#[allow(clippy::too_many_arguments)]
fn extract_paragraph(
    paragraph: Node<'_, '_>,
    part_name: &str,
    decode: &TextDecodeResult,
    channel: DocxTextChannel,
    relationships: &[DocxRelationship],
    state: &mut ExtractionState,
    table_index: Option<usize>,
    row_index: Option<usize>,
    cell_index: Option<usize>,
) -> Result<String, DocxAnalysisError> {
    state.paragraph_index += 1;
    state.summary.paragraphs += 1;
    let paragraph_index = state.paragraph_index;

    for revision_node in paragraph.descendants().filter(|node| {
        node.is_element()
            && node.tag_name().namespace() == Some(WORDPROCESSING_NAMESPACE)
            && matches!(
                node.tag_name().name(),
                "ins" | "del" | "moveFrom" | "moveTo"
            )
    }) {
        let kind = revision_kind_from_name(revision_node.tag_name().name());
        state.revisions.push(DocxRevision {
            sequence: state.revisions.len() + 1,
            kind,
            id: attribute_word(revision_node, "id"),
            declared_author: attribute_word(revision_node, "author"),
            declared_date: attribute_word(revision_node, "date"),
            text: collect_word_text(revision_node),
            source_location: source_location(part_name, decode, Some(revision_node.range())),
        });
    }

    for hyperlink_node in paragraph
        .descendants()
        .filter(|node| is_word_element(*node, "hyperlink"))
    {
        let relationship_id = hyperlink_node
            .attribute((OFFICE_RELATIONSHIPS_NAMESPACE, "id"))
            .map(str::to_owned);
        let anchor = attribute_word(hyperlink_node, "anchor");
        let relationship = relationship_id.as_deref().and_then(|id| {
            relationships.iter().find(|relationship| {
                relationship.source_part.as_deref() == Some(part_name) && relationship.id == id
            })
        });
        state.hyperlinks.push(DocxHyperlink {
            sequence: state.hyperlinks.len() + 1,
            part_name: part_name.to_owned(),
            display_text: collect_current_word_text(hyperlink_node),
            relationship_id,
            target: relationship
                .map(|value| value.target.clone())
                .or_else(|| anchor.map(|value| format!("#{value}"))),
            external: relationship.is_some_and(|value| value.external),
            source_location: source_location(part_name, decode, Some(hyperlink_node.range())),
        });
    }

    let mut current_text = String::new();
    let mut run_index = 0;
    for run in paragraph
        .descendants()
        .filter(|node| is_word_element(*node, "r"))
    {
        run_index += 1;
        state.summary.runs += 1;
        let explicit_hidden = run.children().any(|child| {
            is_word_element(child, "rPr")
                && child
                    .descendants()
                    .any(|property| is_word_element(property, "vanish"))
        });
        if explicit_hidden {
            state.summary.explicit_hidden_runs += 1;
        }
        let revision_kind = revision_kind_for_node(run, paragraph);

        for element in run.descendants().filter(Node::is_element) {
            if element.tag_name().namespace() != Some(WORDPROCESSING_NAMESPACE) {
                continue;
            }
            match element.tag_name().name() {
                "t" | "delText" => {
                    let deleted = element.tag_name().name() == "delText"
                        || matches!(
                            revision_kind,
                            Some(DocxRevisionKind::Deleted | DocxRevisionKind::MoveFrom)
                        );
                    let target_channel = if deleted {
                        DocxTextChannel::DeletedContent
                    } else {
                        channel
                    };
                    let preserve = element.attribute((XML_NAMESPACE, "space")) == Some("preserve");
                    for text_node in element.children().filter(Node::is_text) {
                        let value = text_node.text().unwrap_or_default();
                        push_segment(
                            state,
                            value,
                            part_name,
                            target_channel,
                            DocxTextKind::Text,
                            Some(paragraph_index),
                            Some(run_index),
                            table_index,
                            row_index,
                            cell_index,
                            preserve,
                            explicit_hidden,
                            revision_kind,
                            source_location(part_name, decode, Some(text_node.range())),
                        )?;
                        if deleted {
                            state.deleted_text.push_str(value);
                        } else {
                            current_text.push_str(value);
                        }
                    }
                }
                "tab" | "br" | "cr" | "noBreakHyphen" | "softHyphen" => {
                    let (value, kind) = structural_text(element.tag_name().name());
                    let deleted = matches!(
                        revision_kind,
                        Some(DocxRevisionKind::Deleted | DocxRevisionKind::MoveFrom)
                    );
                    let target_channel = if deleted {
                        DocxTextChannel::DeletedContent
                    } else {
                        channel
                    };
                    push_segment(
                        state,
                        value,
                        part_name,
                        target_channel,
                        kind,
                        Some(paragraph_index),
                        Some(run_index),
                        table_index,
                        row_index,
                        cell_index,
                        false,
                        explicit_hidden,
                        revision_kind,
                        source_location(part_name, decode, Some(element.range())),
                    )?;
                    if deleted {
                        state.deleted_text.push_str(value);
                    } else {
                        current_text.push_str(value);
                    }
                }
                _ => {}
            }
        }
    }
    Ok(current_text)
}

fn extract_table(
    table: Node<'_, '_>,
    part_name: &str,
    decode: &TextDecodeResult,
    channel: DocxTextChannel,
    relationships: &[DocxRelationship],
    state: &mut ExtractionState,
) -> Result<String, DocxAnalysisError> {
    state.table_index += 1;
    state.summary.tables += 1;
    let table_index = state.table_index;
    let mut rows = Vec::new();
    let mut row_index = 0;
    for row in table.children().filter(|node| is_word_element(*node, "tr")) {
        row_index += 1;
        state.summary.table_rows += 1;
        let mut cells = Vec::new();
        let mut cell_index = 0;
        for cell in row.children().filter(|node| is_word_element(*node, "tc")) {
            cell_index += 1;
            state.summary.table_cells += 1;
            cells.push(extract_block_children(
                cell,
                part_name,
                decode,
                channel,
                relationships,
                state,
                Some(table_index),
                Some(row_index),
                Some(cell_index),
            )?);
        }
        rows.push(cells.join("\t"));
    }
    Ok(rows.join("\n"))
}

#[allow(clippy::too_many_arguments)]
fn push_segment(
    state: &mut ExtractionState,
    text: &str,
    part_name: &str,
    channel: DocxTextChannel,
    kind: DocxTextKind,
    paragraph_index: Option<usize>,
    run_index: Option<usize>,
    table_index: Option<usize>,
    row_index: Option<usize>,
    cell_index: Option<usize>,
    xml_space_preserve: bool,
    explicit_hidden: bool,
    revision_kind: Option<DocxRevisionKind>,
    source_location: DocxSourceLocation,
) -> Result<(), DocxAnalysisError> {
    if state.segments.len() >= MAX_DOCX_TEXT_SEGMENTS {
        return Err(DocxAnalysisError::ResourceLimitExceeded(
            DocxResourceLimit::TextSegments,
        ));
    }
    state.segments.push(DocxTextSegment {
        sequence: state.segments.len() + 1,
        text: text.to_owned(),
        part_name: part_name.to_owned(),
        channel,
        kind,
        paragraph_index,
        run_index,
        table_index,
        row_index,
        cell_index,
        xml_space_preserve,
        explicit_hidden,
        revision_kind,
        source_location,
    });
    Ok(())
}

fn revision_kind_for_node(node: Node<'_, '_>, boundary: Node<'_, '_>) -> Option<DocxRevisionKind> {
    for ancestor in node.ancestors() {
        if ancestor == boundary {
            break;
        }
        if ancestor.is_element()
            && ancestor.tag_name().namespace() == Some(WORDPROCESSING_NAMESPACE)
        {
            match ancestor.tag_name().name() {
                "ins" => return Some(DocxRevisionKind::Inserted),
                "del" => return Some(DocxRevisionKind::Deleted),
                "moveFrom" => return Some(DocxRevisionKind::MoveFrom),
                "moveTo" => return Some(DocxRevisionKind::MoveTo),
                _ => {}
            }
        }
    }
    None
}

fn revision_kind_from_name(name: &str) -> DocxRevisionKind {
    match name {
        "ins" => DocxRevisionKind::Inserted,
        "del" => DocxRevisionKind::Deleted,
        "moveFrom" => DocxRevisionKind::MoveFrom,
        "moveTo" => DocxRevisionKind::MoveTo,
        _ => DocxRevisionKind::Inserted,
    }
}

fn structural_text(name: &str) -> (&'static str, DocxTextKind) {
    match name {
        "tab" => ("\t", DocxTextKind::Tab),
        "br" | "cr" => ("\n", DocxTextKind::LineBreak),
        "noBreakHyphen" => ("\u{2011}", DocxTextKind::NoBreakHyphen),
        "softHyphen" => ("\u{00ad}", DocxTextKind::SoftHyphen),
        _ => ("", DocxTextKind::Text),
    }
}

fn collect_word_text(node: Node<'_, '_>) -> String {
    let mut output = String::new();
    for element in node.descendants().filter(|element| {
        element.is_element() && element.tag_name().namespace() == Some(WORDPROCESSING_NAMESPACE)
    }) {
        match element.tag_name().name() {
            "t" | "delText" => {
                for text in element.children().filter_map(|child| child.text()) {
                    output.push_str(text);
                }
            }
            "tab" | "br" | "cr" | "noBreakHyphen" | "softHyphen" => {
                output.push_str(structural_text(element.tag_name().name()).0);
            }
            _ => {}
        }
    }
    output
}

fn collect_current_word_text(node: Node<'_, '_>) -> String {
    let mut output = String::new();
    for element in node.descendants().filter(Node::is_element) {
        let deleted = element
            .ancestors()
            .take_while(|ancestor| *ancestor != node)
            .any(|ancestor| {
                ancestor.is_element()
                    && ancestor.tag_name().namespace() == Some(WORDPROCESSING_NAMESPACE)
                    && matches!(ancestor.tag_name().name(), "del" | "moveFrom")
            });
        if deleted {
            continue;
        }
        if element.tag_name().namespace() != Some(WORDPROCESSING_NAMESPACE) {
            continue;
        }
        match element.tag_name().name() {
            "t" => {
                for text in element.children().filter_map(|child| child.text()) {
                    output.push_str(text);
                }
            }
            "tab" | "br" | "cr" | "noBreakHyphen" | "softHyphen" => {
                output.push_str(structural_text(element.tag_name().name()).0);
            }
            _ => {}
        }
    }
    output
}

fn attribute_word(node: Node<'_, '_>, name: &str) -> Option<String> {
    node.attribute((WORDPROCESSING_NAMESPACE, name))
        .map(str::to_owned)
}

#[allow(clippy::too_many_arguments)]
fn extract_notes_if_present(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    entries: &BTreeMap<String, usize>,
    parts: &mut [DocxPartRecord],
    part_name: &str,
    note_element: &str,
    channel: DocxTextChannel,
    relationships: &[DocxRelationship],
    state: &mut ExtractionState,
) -> Result<Option<DocxPartTextChannel>, DocxAnalysisError> {
    let Some(index) = entries.get(part_name).copied() else {
        return Ok(None);
    };
    let decode = read_xml_part(archive, parts, index, part_name)?;
    let document = parse_document(&decode)?;
    let mut notes = Vec::new();
    for note in document
        .root_element()
        .children()
        .filter(|node| is_word_element(*node, note_element))
    {
        let id = attribute_word(note, "id");
        if id
            .as_deref()
            .and_then(|value| value.parse::<i64>().ok())
            .is_some_and(|value| value <= 0)
        {
            continue;
        }
        let text = extract_block_children(
            note,
            part_name,
            &decode,
            channel,
            relationships,
            state,
            None,
            None,
            None,
        )?;
        notes.push(text);
        match channel {
            DocxTextChannel::Footnote => state.summary.footnotes += 1,
            DocxTextChannel::Endnote => state.summary.endnotes += 1,
            _ => {}
        }
    }
    Ok(Some(DocxPartTextChannel {
        part_name: part_name.to_owned(),
        text: notes.join("\n"),
    }))
}

fn extract_comments_if_present(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    entries: &BTreeMap<String, usize>,
    parts: &mut [DocxPartRecord],
    relationships: &[DocxRelationship],
    state: &mut ExtractionState,
) -> Result<Option<DocxPartTextChannel>, DocxAnalysisError> {
    let part_name = "word/comments.xml";
    let Some(index) = entries.get(part_name).copied() else {
        return Ok(None);
    };
    let decode = read_xml_part(archive, parts, index, part_name)?;
    let document = parse_document(&decode)?;
    let mut comment_texts = Vec::new();
    for comment in document
        .root_element()
        .children()
        .filter(|node| is_word_element(*node, "comment"))
    {
        let text = extract_block_children(
            comment,
            part_name,
            &decode,
            DocxTextChannel::Comment,
            relationships,
            state,
            None,
            None,
            None,
        )?;
        state.comments.push(DocxComment {
            sequence: state.comments.len() + 1,
            id: attribute_word(comment, "id"),
            declared_author: attribute_word(comment, "author"),
            initials: attribute_word(comment, "initials"),
            declared_date: attribute_word(comment, "date"),
            text: text.clone(),
            source_location: source_location(part_name, &decode, Some(comment.range())),
        });
        state.summary.comments += 1;
        comment_texts.push(text);
    }
    Ok(Some(DocxPartTextChannel {
        part_name: part_name.to_owned(),
        text: comment_texts.join("\n"),
    }))
}

fn source_location(
    part_name: &str,
    decode: &TextDecodeResult,
    decoded_range: Option<std::ops::Range<usize>>,
) -> DocxSourceLocation {
    let decoded_xml_byte_range = decoded_range.as_ref().map(|range| DocxByteRange {
        start: range.start,
        end: range.end,
    });
    let part_byte_range = decoded_range.and_then(|range| {
        Some(DocxByteRange {
            start: raw_boundary(decode, range.start)?,
            end: raw_boundary(decode, range.end)?,
        })
    });
    DocxSourceLocation {
        part_name: part_name.to_owned(),
        decoded_xml_byte_range,
        part_byte_range,
        package_byte_range: None,
    }
}

fn raw_boundary(decode: &TextDecodeResult, decoded_offset: usize) -> Option<usize> {
    if decoded_offset == decode.decoded_text.len() {
        return decode.source_mappings.last().map_or_else(
            || Some(bom_length(decode.bom)),
            |mapping| Some(mapping.source_byte_range.end),
        );
    }
    decode
        .source_mappings
        .iter()
        .find(|mapping| mapping.decoded_utf8_byte_offset == decoded_offset)
        .map(|mapping| mapping.source_byte_range.start)
}

const fn bom_length(bom: BomKind) -> usize {
    match bom {
        BomKind::None => 0,
        BomKind::Utf8 => 3,
        BomKind::Utf16LittleEndian | BomKind::Utf16BigEndian => 2,
    }
}

fn push_finding(
    findings: &mut Vec<DocxFinding>,
    kind: DocxFindingKind,
    severity: DocxFindingSeverity,
    part_name: Option<String>,
    relationship_sequence: Option<usize>,
    description: &'static str,
    source_location: Option<DocxSourceLocation>,
) {
    findings.push(DocxFinding {
        sequence: findings.len() + 1,
        kind,
        severity,
        part_name,
        relationship_sequence,
        description,
        source_location,
    });
}
