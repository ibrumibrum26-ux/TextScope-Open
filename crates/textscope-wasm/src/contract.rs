use std::{error::Error, fmt};

use serde::Serialize;
use textscope_core::{
    BomKind, CharacterRecord, CleanError, CleanProfile, CleanProfileDefinition, CleanResult,
    CleanScanSummary, ContentPreservationDiff, ContentPreservationStatus, DecodeStatus,
    EncodingEvidence, Finding, FindingKind, FindingSeverity, HtmlAnalysis, HtmlAnalysisError,
    HtmlAttributeRecord, HtmlCommentRecord, HtmlDoctypeRecord, HtmlDocumentSummary,
    HtmlElementRecord, HtmlEncodingDeclaration, HtmlEncodingDeclarationKind, HtmlEncodingEvidence,
    HtmlExternalReference, HtmlFinding, HtmlFindingKind, HtmlFindingSeverity, HtmlMetaDeclaration,
    HtmlMetadata, HtmlParserLayer, HtmlParserRecovery, HtmlReferenceKind, HtmlResourceLimit,
    HtmlSourceRange, HtmlTextChannels, HtmlTextClassification, HtmlTextSegment,
    NormalizationInspection, RuleId, ScanResult, SourceCharacterMapping, SourceEncoding,
    TechnicalDiff, TextDecodeResult, TextIngestionError, TextRange, TextSourceAnalysis,
    TextSourceKind, Transformation, TransformationManifest, TransformationOperation,
    VerificationChecks, VerificationReport, VerificationStatus,
};
use textscope_core::{ClipboardComparisonState, ClipboardRepresentationComparison};
use textscope_core::{
    DocxAnalysis, DocxAnalysisError, DocxByteRange, DocxComment, DocxContentTypeDeclaration,
    DocxContentTypeDeclarationKind, DocxCrcStatus, DocxDocumentSummary, DocxFinding,
    DocxFindingKind, DocxFindingSeverity, DocxHyperlink, DocxMetadata, DocxMetadataProperty,
    DocxPackageStatus, DocxPackageSummary, DocxPartClassification, DocxPartRecord,
    DocxPartTextChannel, DocxRelationship, DocxResourceLimit, DocxRevision, DocxRevisionKind,
    DocxSourceLocation, DocxTextChannel, DocxTextChannels, DocxTextKind, DocxTextSegment,
};
use textscope_core::{
    EvidenceKind, EvidenceLevel, EvidenceSource, ProvenanceEvidence, VerificationState,
};
use textscope_core::{
    PdfActionKind, PdfActionRecord, PdfAnalysis, PdfAnalysisError, PdfAnnotationRecord, PdfBox,
    PdfDocumentSummary, PdfEmbeddedFileRecord, PdfEncryptionStatus, PdfExternalReference,
    PdfExternalReferenceKind, PdfFinding, PdfFindingKind, PdfFindingSeverity, PdfFontRecord,
    PdfFormSummary, PdfImageRecord, PdfMetadata, PdfMetadataConflict, PdfMetadataEntry,
    PdfObjectSummary, PdfPageSummary, PdfResourceLimit, PdfRevisionSummary, PdfSignatureRecord,
    PdfSourceReference, PdfTextExtractionStatus, PdfTextSegment, PdfVersion, PdfXrefKind,
};

#[derive(Serialize)]
struct ScanResultContract {
    characters: Vec<CharacterRecordContract>,
    findings: Vec<FindingContract>,
    normalization: NormalizationContract,
}

#[derive(Serialize)]
struct ProvenanceEvidenceContract {
    kind: &'static str,
    source: &'static str,
    subject: String,
    value: String,
    level: &'static str,
    verification: &'static str,
}

#[derive(Serialize)]
struct ClipboardComparisonContract {
    state: &'static str,
    plain_byte_length: Option<usize>,
    html_text_byte_length: Option<usize>,
}

#[derive(Serialize)]
struct CharacterRecordContract {
    code_point_index: usize,
    utf16_offset: usize,
    utf8_byte_offset: usize,
    character: char,
    code_point: u32,
    code_point_label: String,
    utf8_bytes: Vec<u8>,
    unicode_category: &'static str,
    script: &'static str,
    is_whitespace: bool,
    is_control: bool,
    is_format: bool,
    is_invisible: bool,
    is_unusual_whitespace: bool,
    is_bidi_control: bool,
    is_variation_selector: bool,
}

#[derive(Serialize)]
struct FindingContract {
    kind: &'static str,
    severity: &'static str,
    code_point_index: usize,
    utf16_offset: usize,
    utf8_byte_offset: usize,
    code_point: u32,
    description: &'static str,
}

#[derive(Serialize)]
struct NormalizationContract {
    nfc_would_change: bool,
    nfd_would_change: bool,
    nfkc_would_change: bool,
    nfkd_would_change: bool,
}

#[derive(Serialize)]
struct CleanResultContract {
    original_text: String,
    clean_text: String,
    profile: &'static str,
    tcs_version: &'static str,
    manifest: TransformationManifestContract,
    technical_diff: TechnicalDiffContract,
    content_preservation_diff: ContentPreservationDiffContract,
    verification: VerificationReportContract,
}

#[derive(Serialize)]
struct TransformationManifestContract {
    tcs_version: &'static str,
    profile: &'static str,
    transformations: Vec<TransformationContract>,
}

#[derive(Serialize)]
struct TransformationContract {
    sequence: usize,
    rule_id: &'static str,
    rule_version: &'static str,
    operation: &'static str,
    original_code_point_range: TextRangeContract,
    original_utf8_byte_range: TextRangeContract,
    replacement: String,
    reason: &'static str,
    before: String,
    after: String,
}

#[derive(Serialize)]
struct TextRangeContract {
    start: usize,
    end: usize,
}

#[derive(Serialize)]
struct TechnicalDiffContract {
    changed: bool,
    original_utf8_bytes: usize,
    clean_utf8_bytes: usize,
    original_code_points: usize,
    clean_code_points: usize,
    transformation_count: usize,
}

#[derive(Serialize)]
struct ContentPreservationDiffContract {
    status: &'static str,
    reason: &'static str,
}

#[derive(Serialize)]
struct VerificationReportContract {
    original_scan_summary: CleanScanSummaryContract,
    clean_scan_summary: CleanScanSummaryContract,
    applied_transformations: usize,
    remaining_findings: Vec<FindingContract>,
    content_preservation_status: &'static str,
    verification_status: &'static str,
    checks: VerificationChecksContract,
}

#[derive(Serialize)]
struct CleanScanSummaryContract {
    characters: usize,
    findings: usize,
    invisible: usize,
    unusual_whitespace: usize,
    bidi_controls: usize,
    variation_selectors: usize,
}

#[derive(Serialize)]
struct VerificationChecksContract {
    manifest_reproduces_clean_text: bool,
    rules_allowed_by_profile: bool,
    no_undeclared_transformations: bool,
    clean_rescan_completed: bool,
    expected_findings_reassessed: bool,
    preserved_findings_visible: bool,
    deterministic: bool,
    idempotent: bool,
}

#[derive(Serialize)]
struct CleanProfileContract {
    id: &'static str,
    label: &'static str,
    description: &'static str,
    supported: bool,
    active_rule_ids: Vec<&'static str>,
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum TextSourceAnalysisResponseContract {
    Success {
        analysis: TextSourceAnalysisContract,
    },
    Error {
        error: TextIngestionErrorContract,
    },
}

#[derive(Serialize)]
struct TextSourceAnalysisContract {
    raw_byte_length: usize,
    source_kind: &'static str,
    decode: TextDecodeResultContract,
    scan: ScanResultContract,
}

#[derive(Serialize)]
struct TextDecodeResultContract {
    encoding: &'static str,
    encoding_evidence: &'static str,
    bom: &'static str,
    status: &'static str,
    decoded_text: String,
    source_mappings: Vec<SourceCharacterMappingContract>,
}

#[derive(Serialize)]
struct SourceCharacterMappingContract {
    code_point_index: usize,
    source_byte_range: TextRangeContract,
    decoded_utf8_byte_offset: usize,
    utf16_offset: usize,
}

#[derive(Serialize)]
struct TextIngestionErrorContract {
    code: &'static str,
    message: &'static str,
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum HtmlAnalysisResponseContract {
    Success { analysis: Box<HtmlAnalysisContract> },
    Error { error: HtmlAnalysisErrorContract },
}

#[derive(Serialize)]
struct HtmlAnalysisErrorContract {
    code: &'static str,
    message: &'static str,
    resource_limit: Option<&'static str>,
}

#[derive(Serialize)]
struct HtmlAnalysisContract {
    raw_byte_length: usize,
    decode: TextDecodeResultContract,
    encoding: HtmlEncodingEvidenceContract,
    summary: HtmlDocumentSummaryContract,
    metadata: HtmlMetadataContract,
    provenance: Vec<ProvenanceEvidenceContract>,
    elements: Vec<HtmlElementRecordContract>,
    text_segments: Vec<HtmlTextSegmentContract>,
    comments: Vec<HtmlCommentRecordContract>,
    external_references: Vec<HtmlExternalReferenceContract>,
    findings: Vec<HtmlFindingContract>,
    parser_recoveries: Vec<HtmlParserRecoveryContract>,
    text_channels: HtmlTextChannelsContract,
    scan: ScanResultContract,
}

#[derive(Serialize)]
struct HtmlEncodingEvidenceContract {
    declarations: Vec<HtmlEncodingDeclarationContract>,
    bom_precedence_applied: bool,
    declaration_conflict: bool,
}

#[derive(Serialize)]
struct HtmlEncodingDeclarationContract {
    sequence: usize,
    kind: &'static str,
    declared_label: String,
    normalized_label: String,
    source_range: HtmlSourceRangeContract,
}

#[derive(Serialize)]
struct HtmlSourceRangeContract {
    decoded_utf8_byte_range: TextRangeContract,
    raw_source_byte_range: Option<TextRangeContract>,
}

#[derive(Serialize)]
struct HtmlDocumentSummaryContract {
    doctype: Option<HtmlDoctypeRecordContract>,
    quirks_mode: String,
    parsed_elements: usize,
    source_start_tags: usize,
    attributes: usize,
    text_segments: usize,
    comments: usize,
    script_blocks: usize,
    style_blocks: usize,
    noscript_blocks: usize,
    template_blocks: usize,
    external_references: usize,
    tokenizer_recoveries: usize,
    tree_builder_recoveries: usize,
    html_present: bool,
    head_present: bool,
    body_present: bool,
    title_present: bool,
    meta_elements: usize,
    link_elements: usize,
}

#[derive(Serialize)]
struct HtmlDoctypeRecordContract {
    name: String,
    public_identifier: Option<String>,
    system_identifier: Option<String>,
    force_quirks: bool,
    source_range: HtmlSourceRangeContract,
}

#[derive(Serialize)]
struct HtmlMetadataContract {
    title: Option<String>,
    lang: Option<String>,
    dir: Option<String>,
    meta_declarations: Vec<HtmlMetaDeclarationContract>,
    canonical_links: Vec<String>,
    base_hrefs: Vec<String>,
}

#[derive(Serialize)]
struct HtmlMetaDeclarationContract {
    sequence: usize,
    charset: Option<String>,
    name: Option<String>,
    property: Option<String>,
    content: Option<String>,
    source_range: HtmlSourceRangeContract,
}

#[derive(Serialize)]
struct HtmlElementRecordContract {
    sequence: usize,
    name: String,
    attributes: Vec<HtmlAttributeRecordContract>,
    self_closing: bool,
    source_range: HtmlSourceRangeContract,
}

#[derive(Serialize)]
struct HtmlAttributeRecordContract {
    name: String,
    value: String,
    source_range: HtmlSourceRangeContract,
}

#[derive(Serialize)]
struct HtmlTextSegmentContract {
    sequence: usize,
    text: String,
    context_element: String,
    classification: &'static str,
    source_text: Option<String>,
    source_range: Option<HtmlSourceRangeContract>,
}

#[derive(Serialize)]
struct HtmlCommentRecordContract {
    sequence: usize,
    text: String,
    source_range: HtmlSourceRangeContract,
}

#[derive(Serialize)]
struct HtmlExternalReferenceContract {
    sequence: usize,
    element: String,
    attribute: String,
    value: String,
    kind: &'static str,
    source_range: HtmlSourceRangeContract,
}

#[derive(Serialize)]
struct HtmlFindingContract {
    sequence: usize,
    kind: &'static str,
    severity: &'static str,
    element_sequence: Option<usize>,
    attribute: Option<String>,
    description: &'static str,
    source_range: Option<HtmlSourceRangeContract>,
}

#[derive(Serialize)]
struct HtmlParserRecoveryContract {
    sequence: usize,
    layer: &'static str,
    message: String,
    source_range: Option<HtmlSourceRangeContract>,
}

#[derive(Serialize)]
struct HtmlTextChannelsContract {
    document_text: String,
    metadata_text: String,
    active_content_text: String,
    style_text: String,
    template_text: String,
    noscript_text: String,
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum DocxAnalysisResponseContract {
    Success { analysis: Box<DocxAnalysisContract> },
    Error { error: DocxAnalysisErrorContract },
}

#[derive(Serialize)]
struct DocxAnalysisErrorContract {
    code: &'static str,
    message: &'static str,
    resource_limit: Option<&'static str>,
}

#[derive(Serialize)]
struct DocxAnalysisContract {
    package: DocxPackageSummaryContract,
    parts: Vec<DocxPartRecordContract>,
    content_types: Vec<DocxContentTypeDeclarationContract>,
    relationships: Vec<DocxRelationshipContract>,
    metadata: DocxMetadataContract,
    provenance: Vec<ProvenanceEvidenceContract>,
    summary: DocxDocumentSummaryContract,
    text_segments: Vec<DocxTextSegmentContract>,
    comments: Vec<DocxCommentContract>,
    revisions: Vec<DocxRevisionContract>,
    hyperlinks: Vec<DocxHyperlinkContract>,
    findings: Vec<DocxFindingContract>,
    text_channels: DocxTextChannelsContract,
    scan: ScanResultContract,
}

#[derive(Serialize)]
struct DocxPackageSummaryContract {
    status: &'static str,
    raw_byte_length: usize,
    entry_count: usize,
    compressed_total: u64,
    uncompressed_total: u64,
    compression_ratio_milli: u64,
    zip64_present: bool,
    archive_offset: u64,
    duplicate_entry_names: usize,
    invalid_entry_names: usize,
}

#[derive(Serialize)]
struct DocxPartRecordContract {
    sequence: usize,
    part_name: String,
    content_type: Option<String>,
    compressed_size: u64,
    uncompressed_size: u64,
    crc32: u32,
    crc_status: &'static str,
    compression_method: String,
    classification: &'static str,
    relationship_count: usize,
    encrypted: bool,
    directory: bool,
    symlink: bool,
    local_header_offset: u64,
    data_offset: Option<u64>,
    xml_encoding: Option<&'static str>,
}

#[derive(Serialize)]
struct DocxContentTypeDeclarationContract {
    sequence: usize,
    kind: &'static str,
    extension: Option<String>,
    part_name: Option<String>,
    content_type: String,
    source_location: DocxSourceLocationContract,
}

#[derive(Serialize)]
struct DocxRelationshipContract {
    sequence: usize,
    relationship_part: String,
    source_part: Option<String>,
    id: String,
    relationship_type: String,
    target: String,
    external: bool,
    resolved_internal_part: Option<String>,
    source_location: DocxSourceLocationContract,
}

#[derive(Serialize)]
struct DocxMetadataContract {
    core_properties: Vec<DocxMetadataPropertyContract>,
    app_properties: Vec<DocxMetadataPropertyContract>,
    custom_properties: Vec<DocxMetadataPropertyContract>,
}

#[derive(Serialize)]
struct DocxMetadataPropertyContract {
    sequence: usize,
    name: String,
    value: String,
    value_type: Option<String>,
    source_location: DocxSourceLocationContract,
}

#[derive(Serialize)]
struct DocxDocumentSummaryContract {
    main_document_present: bool,
    styles_present: bool,
    numbering_present: bool,
    paragraphs: usize,
    runs: usize,
    tables: usize,
    table_rows: usize,
    table_cells: usize,
    headers: usize,
    footers: usize,
    footnotes: usize,
    endnotes: usize,
    comments: usize,
    tracked_changes: usize,
    explicit_hidden_runs: usize,
    hyperlinks: usize,
    external_relationships: usize,
    media_parts: usize,
    embedded_object_parts: usize,
    custom_xml_parts: usize,
}

#[derive(Serialize)]
struct DocxTextSegmentContract {
    sequence: usize,
    text: String,
    part_name: String,
    channel: &'static str,
    kind: &'static str,
    paragraph_index: Option<usize>,
    run_index: Option<usize>,
    table_index: Option<usize>,
    row_index: Option<usize>,
    cell_index: Option<usize>,
    xml_space_preserve: bool,
    explicit_hidden: bool,
    revision_kind: Option<&'static str>,
    source_location: DocxSourceLocationContract,
}

#[derive(Serialize)]
struct DocxCommentContract {
    sequence: usize,
    id: Option<String>,
    declared_author: Option<String>,
    initials: Option<String>,
    declared_date: Option<String>,
    text: String,
    source_location: DocxSourceLocationContract,
}

#[derive(Serialize)]
struct DocxRevisionContract {
    sequence: usize,
    kind: &'static str,
    id: Option<String>,
    declared_author: Option<String>,
    declared_date: Option<String>,
    text: String,
    source_location: DocxSourceLocationContract,
}

#[derive(Serialize)]
struct DocxHyperlinkContract {
    sequence: usize,
    part_name: String,
    display_text: String,
    relationship_id: Option<String>,
    target: Option<String>,
    external: bool,
    source_location: DocxSourceLocationContract,
}

#[derive(Serialize)]
struct DocxFindingContract {
    sequence: usize,
    kind: &'static str,
    severity: &'static str,
    part_name: Option<String>,
    relationship_sequence: Option<usize>,
    description: &'static str,
    source_location: Option<DocxSourceLocationContract>,
}

#[derive(Serialize)]
struct DocxTextChannelsContract {
    document_text: String,
    headers: Vec<DocxPartTextChannelContract>,
    footers: Vec<DocxPartTextChannelContract>,
    footnotes: Vec<DocxPartTextChannelContract>,
    endnotes: Vec<DocxPartTextChannelContract>,
    comments: Vec<DocxPartTextChannelContract>,
    deleted_content: String,
}

#[derive(Serialize)]
struct DocxPartTextChannelContract {
    part_name: String,
    text: String,
}

#[derive(Serialize)]
struct DocxSourceLocationContract {
    part_name: String,
    decoded_xml_byte_range: Option<DocxByteRangeContract>,
    part_byte_range: Option<DocxByteRangeContract>,
    package_byte_range: Option<DocxByteRangeContract>,
}

#[derive(Serialize)]
struct DocxByteRangeContract {
    start: usize,
    end: usize,
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum PdfAnalysisResponseContract {
    Success { analysis: Box<PdfAnalysisContract> },
    Error { error: PdfAnalysisErrorContract },
}

#[derive(Serialize)]
struct PdfAnalysisErrorContract {
    code: &'static str,
    message: &'static str,
    resource_limit: Option<&'static str>,
}

#[derive(Serialize)]
struct PdfAnalysisContract {
    summary: PdfDocumentSummaryContract,
    objects: Vec<PdfObjectSummaryContract>,
    revisions: Vec<PdfRevisionSummaryContract>,
    metadata: PdfMetadataContract,
    provenance: Vec<ProvenanceEvidenceContract>,
    pages: Vec<PdfPageSummaryContract>,
    images: Vec<PdfImageRecordContract>,
    fonts: Vec<PdfFontRecordContract>,
    actions: Vec<PdfActionRecordContract>,
    annotations: Vec<PdfAnnotationRecordContract>,
    external_references: Vec<PdfExternalReferenceContract>,
    embedded_files: Vec<PdfEmbeddedFileRecordContract>,
    forms: PdfFormSummaryContract,
    signatures: Vec<PdfSignatureRecordContract>,
    text_segments: Vec<PdfTextSegmentContract>,
    page_text: Vec<String>,
    document_text: String,
    text_extraction_status: &'static str,
    findings: Vec<PdfFindingContract>,
    scan: ScanResultContract,
}

#[derive(Serialize)]
struct PdfDocumentSummaryContract {
    raw_byte_length: usize,
    version: Option<PdfVersionContract>,
    binary_marker_present: bool,
    eof_marker_present: bool,
    startxref_count: usize,
    xref_kind: &'static str,
    trailer_count: usize,
    object_count: usize,
    stream_count: usize,
    object_stream_count: usize,
    compressed_object_count: usize,
    page_count: usize,
    font_count: usize,
    image_count: usize,
    annotation_count: usize,
    action_count: usize,
    embedded_file_count: usize,
    form_present: bool,
    metadata_present: bool,
    javascript_present: bool,
    linearized: bool,
    encryption: &'static str,
}

#[derive(Serialize)]
struct PdfVersionContract {
    major: u8,
    minor: u8,
    declared: String,
}

#[derive(Serialize)]
struct PdfObjectSummaryContract {
    object_number: u32,
    generation: u16,
    kind: String,
    has_stream: bool,
    compressed_in_object_stream: bool,
    raw_byte_offset: Option<usize>,
}

#[derive(Serialize)]
struct PdfRevisionSummaryContract {
    sequence: usize,
    startxref_offset: Option<usize>,
    xref_kind: &'static str,
    prev_offset: Option<usize>,
    trailer_observed: bool,
    updated_object_ids: Vec<PdfSourceReferenceContract>,
}

#[derive(Serialize)]
struct PdfPageSummaryContract {
    page_index: usize,
    object: PdfSourceReferenceContract,
    media_box: Option<PdfBoxContract>,
    crop_box: Option<PdfBoxContract>,
    rotate: Option<i32>,
    resources_present: bool,
    content_stream_count: usize,
    annotation_count: usize,
    text: String,
}

#[derive(Serialize)]
struct PdfImageRecordContract {
    sequence: usize,
    page_indices: Vec<usize>,
    object: PdfSourceReferenceContract,
    width: Option<u32>,
    height: Option<u32>,
    filter: Option<String>,
    color_space: Option<String>,
}

#[derive(Serialize)]
struct PdfBoxContract {
    values: Vec<String>,
}

#[derive(Serialize)]
struct PdfMetadataContract {
    title: Option<String>,
    author: Option<String>,
    subject: Option<String>,
    keywords: Option<String>,
    creator: Option<String>,
    producer: Option<String>,
    creation_date: Option<String>,
    modification_date: Option<String>,
    trapped: Option<String>,
    info_entries: Vec<PdfMetadataEntryContract>,
    xmp: Option<PdfXmpMetadataContract>,
    conflicts: Vec<PdfMetadataConflictContract>,
}

#[derive(Serialize)]
struct PdfMetadataEntryContract {
    key: String,
    value: String,
    source: PdfSourceReferenceContract,
}

#[derive(Serialize)]
struct PdfXmpMetadataContract {
    present: bool,
    well_formed_xml: bool,
    properties: Vec<PdfMetadataEntryContract>,
    source: Option<PdfSourceReferenceContract>,
}

#[derive(Serialize)]
struct PdfMetadataConflictContract {
    key: String,
    info_value: Option<String>,
    xmp_value: Option<String>,
    description: &'static str,
}

#[derive(Serialize)]
struct PdfActionRecordContract {
    sequence: usize,
    kind: &'static str,
    target: Option<String>,
    source: PdfSourceReferenceContract,
    context: String,
}

#[derive(Serialize)]
struct PdfAnnotationRecordContract {
    sequence: usize,
    page_index: usize,
    subtype: String,
    contents: Option<String>,
    title: Option<String>,
    uri: Option<String>,
    action: Option<&'static str>,
    flags: Option<i64>,
    source: PdfSourceReferenceContract,
}

#[derive(Serialize)]
struct PdfExternalReferenceContract {
    sequence: usize,
    value: String,
    kind: &'static str,
    source: PdfSourceReferenceContract,
}

#[derive(Serialize)]
struct PdfEmbeddedFileRecordContract {
    sequence: usize,
    declared_filename: Option<String>,
    description: Option<String>,
    mime_type: Option<String>,
    size: Option<u64>,
    object: PdfSourceReferenceContract,
    stream_object: Option<PdfSourceReferenceContract>,
}

#[derive(Serialize)]
struct PdfFormSummaryContract {
    present: bool,
    field_count: usize,
    field_types: Vec<String>,
    need_appearances: Option<bool>,
    xfa_present: bool,
}

#[derive(Serialize)]
struct PdfSignatureRecordContract {
    sequence: usize,
    source: PdfSourceReferenceContract,
    byte_range_present: bool,
    contents_present: bool,
    sub_filter: Option<String>,
    reason: Option<String>,
    name: Option<String>,
    modification_date: Option<String>,
}

#[derive(Serialize)]
struct PdfTextSegmentContract {
    sequence: usize,
    page_index: usize,
    text: String,
    font_reference: Option<PdfSourceReferenceContract>,
    content_stream_object: Option<PdfSourceReferenceContract>,
    operation_index: usize,
    source_object: Option<PdfSourceReferenceContract>,
    extraction_status: &'static str,
    explicit_non_painting: bool,
}

#[derive(Serialize)]
struct PdfFindingContract {
    sequence: usize,
    kind: &'static str,
    severity: &'static str,
    description: &'static str,
    source: Option<PdfSourceReferenceContract>,
}

#[derive(Serialize)]
struct PdfFontRecordContract {
    sequence: usize,
    page_indices: Vec<usize>,
    resource_name: String,
    source: PdfSourceReferenceContract,
    subtype: Option<String>,
    base_font: Option<String>,
    encoding: Option<String>,
    to_unicode_present: bool,
    mapping_status: &'static str,
}

#[derive(Serialize)]
struct PdfSourceReferenceContract {
    object_number: Option<u32>,
    generation: Option<u16>,
    raw_byte_offset: Option<usize>,
    context: String,
}

#[derive(Debug)]
pub(crate) enum CleanJsonError {
    Clean(CleanError),
    Serialize(serde_json::Error),
}

impl fmt::Display for CleanJsonError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Clean(_) => formatter.write_str("core clean operation failed"),
            Self::Serialize(_) => formatter.write_str("clean result serialization failed"),
        }
    }
}

impl Error for CleanJsonError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Clean(error) => Some(error),
            Self::Serialize(error) => Some(error),
        }
    }
}

pub(crate) fn scan_text_json(text: &str) -> Result<String, serde_json::Error> {
    serde_json::to_string(&ScanResultContract::from(textscope_core::scan_text(text)))
}

pub(crate) fn compare_clipboard_representations_json(
    plain_text: Option<&str>,
    html_text: Option<&str>,
) -> Result<String, serde_json::Error> {
    let comparison = textscope_core::compare_clipboard_representations(plain_text, html_text);
    serde_json::to_string(&ClipboardComparisonContract::from(comparison))
}

impl From<ClipboardRepresentationComparison> for ClipboardComparisonContract {
    fn from(comparison: ClipboardRepresentationComparison) -> Self {
        Self {
            state: clipboard_comparison_state(comparison.state),
            plain_byte_length: comparison.plain_byte_length,
            html_text_byte_length: comparison.html_text_byte_length,
        }
    }
}

pub(crate) fn clean_text_json(text: &str, profile: CleanProfile) -> Result<String, CleanJsonError> {
    let result = textscope_core::clean_text(text, profile).map_err(CleanJsonError::Clean)?;
    serde_json::to_string(&CleanResultContract::from(result)).map_err(CleanJsonError::Serialize)
}

pub(crate) fn clean_profiles_json() -> Result<String, serde_json::Error> {
    let profiles: Vec<_> = textscope_core::available_clean_profiles()
        .iter()
        .copied()
        .map(CleanProfileContract::from)
        .collect();
    serde_json::to_string(&profiles)
}

pub(crate) fn analyze_text_bytes_json(
    bytes: &[u8],
    source_kind: &str,
) -> Result<String, serde_json::Error> {
    let response = match TextSourceKind::from_id(source_kind) {
        Some(source_kind) => match textscope_core::analyze_text_bytes(bytes, source_kind) {
            Ok(analysis) => TextSourceAnalysisResponseContract::Success {
                analysis: analysis.into(),
            },
            Err(error) => TextSourceAnalysisResponseContract::Error {
                error: error.into(),
            },
        },
        None => TextSourceAnalysisResponseContract::Error {
            error: TextIngestionErrorContract {
                code: "unsupported_file_type",
                message: "The selected file type is not supported in this TextScope phase.",
            },
        },
    };

    serde_json::to_string(&response)
}

pub(crate) fn analyze_html_bytes_json(bytes: &[u8]) -> Result<String, serde_json::Error> {
    let response = match textscope_core::analyze_html_bytes(bytes) {
        Ok(analysis) => HtmlAnalysisResponseContract::Success {
            analysis: Box::new(analysis.into()),
        },
        Err(error) => HtmlAnalysisResponseContract::Error {
            error: error.into(),
        },
    };

    serde_json::to_string(&response)
}

pub(crate) fn analyze_docx_bytes_json(bytes: &[u8]) -> Result<String, serde_json::Error> {
    let response = match textscope_core::analyze_docx_bytes(bytes) {
        Ok(analysis) => DocxAnalysisResponseContract::Success {
            analysis: Box::new(analysis.into()),
        },
        Err(error) => DocxAnalysisResponseContract::Error {
            error: error.into(),
        },
    };

    serde_json::to_string(&response)
}

pub(crate) fn analyze_pdf_bytes_json(bytes: &[u8]) -> Result<String, serde_json::Error> {
    let response = match textscope_core::analyze_pdf_bytes(bytes) {
        Ok(analysis) => PdfAnalysisResponseContract::Success {
            analysis: Box::new(analysis.into()),
        },
        Err(error) => PdfAnalysisResponseContract::Error {
            error: error.into(),
        },
    };

    serde_json::to_string(&response)
}

impl From<ScanResult> for ScanResultContract {
    fn from(result: ScanResult) -> Self {
        Self {
            characters: result
                .characters
                .into_iter()
                .map(CharacterRecordContract::from)
                .collect(),
            findings: result
                .findings
                .into_iter()
                .map(FindingContract::from)
                .collect(),
            normalization: result.normalization.into(),
        }
    }
}

impl From<ProvenanceEvidence> for ProvenanceEvidenceContract {
    fn from(evidence: ProvenanceEvidence) -> Self {
        Self {
            kind: evidence_kind(evidence.kind),
            source: evidence_source(evidence.source),
            subject: evidence.subject,
            value: evidence.value,
            level: evidence_level(evidence.level),
            verification: verification_state(evidence.verification),
        }
    }
}

impl From<CharacterRecord> for CharacterRecordContract {
    fn from(record: CharacterRecord) -> Self {
        Self {
            code_point_index: record.code_point_index,
            utf16_offset: record.utf16_offset,
            utf8_byte_offset: record.utf8_byte_offset,
            character: record.character,
            code_point: record.code_point,
            code_point_label: record.code_point_label,
            utf8_bytes: record.utf8_bytes,
            unicode_category: record.unicode_category,
            script: record.script,
            is_whitespace: record.is_whitespace,
            is_control: record.is_control,
            is_format: record.is_format,
            is_invisible: record.is_invisible,
            is_unusual_whitespace: record.is_unusual_whitespace,
            is_bidi_control: record.is_bidi_control,
            is_variation_selector: record.is_variation_selector,
        }
    }
}

impl From<Finding> for FindingContract {
    fn from(finding: Finding) -> Self {
        Self {
            kind: finding_kind(finding.kind),
            severity: finding_severity(finding.severity),
            code_point_index: finding.code_point_index,
            utf16_offset: finding.utf16_offset,
            utf8_byte_offset: finding.utf8_byte_offset,
            code_point: finding.code_point,
            description: finding.description,
        }
    }
}

impl From<NormalizationInspection> for NormalizationContract {
    fn from(inspection: NormalizationInspection) -> Self {
        Self {
            nfc_would_change: inspection.nfc_would_change,
            nfd_would_change: inspection.nfd_would_change,
            nfkc_would_change: inspection.nfkc_would_change,
            nfkd_would_change: inspection.nfkd_would_change,
        }
    }
}

impl From<CleanResult> for CleanResultContract {
    fn from(result: CleanResult) -> Self {
        Self {
            original_text: result.original_text,
            clean_text: result.clean_text,
            profile: clean_profile(result.profile),
            tcs_version: result.tcs_version,
            manifest: result.manifest.into(),
            technical_diff: result.technical_diff.into(),
            content_preservation_diff: result.content_preservation_diff.into(),
            verification: result.verification.into(),
        }
    }
}

impl From<TransformationManifest> for TransformationManifestContract {
    fn from(manifest: TransformationManifest) -> Self {
        Self {
            tcs_version: manifest.tcs_version,
            profile: clean_profile(manifest.profile),
            transformations: manifest
                .transformations
                .into_iter()
                .map(TransformationContract::from)
                .collect(),
        }
    }
}

impl From<Transformation> for TransformationContract {
    fn from(transformation: Transformation) -> Self {
        Self {
            sequence: transformation.sequence,
            rule_id: transformation.rule_id.as_str(),
            rule_version: transformation.rule_version.as_str(),
            operation: transformation_operation(transformation.operation),
            original_code_point_range: transformation.original_code_point_range.into(),
            original_utf8_byte_range: transformation.original_utf8_byte_range.into(),
            replacement: transformation.replacement,
            reason: transformation.reason,
            before: transformation.before,
            after: transformation.after,
        }
    }
}

impl From<TextRange> for TextRangeContract {
    fn from(range: TextRange) -> Self {
        Self {
            start: range.start,
            end: range.end,
        }
    }
}

impl From<TechnicalDiff> for TechnicalDiffContract {
    fn from(diff: TechnicalDiff) -> Self {
        Self {
            changed: diff.changed,
            original_utf8_bytes: diff.original_utf8_bytes,
            clean_utf8_bytes: diff.clean_utf8_bytes,
            original_code_points: diff.original_code_points,
            clean_code_points: diff.clean_code_points,
            transformation_count: diff.transformation_count,
        }
    }
}

impl From<ContentPreservationDiff> for ContentPreservationDiffContract {
    fn from(diff: ContentPreservationDiff) -> Self {
        Self {
            status: content_preservation_status(diff.status),
            reason: diff.reason,
        }
    }
}

impl From<VerificationReport> for VerificationReportContract {
    fn from(report: VerificationReport) -> Self {
        Self {
            original_scan_summary: report.original_scan_summary.into(),
            clean_scan_summary: report.clean_scan_summary.into(),
            applied_transformations: report.applied_transformations,
            remaining_findings: report
                .remaining_findings
                .into_iter()
                .map(FindingContract::from)
                .collect(),
            content_preservation_status: content_preservation_status(
                report.content_preservation_status,
            ),
            verification_status: verification_status(report.verification_status),
            checks: report.checks.into(),
        }
    }
}

impl From<CleanScanSummary> for CleanScanSummaryContract {
    fn from(summary: CleanScanSummary) -> Self {
        Self {
            characters: summary.characters,
            findings: summary.findings,
            invisible: summary.invisible,
            unusual_whitespace: summary.unusual_whitespace,
            bidi_controls: summary.bidi_controls,
            variation_selectors: summary.variation_selectors,
        }
    }
}

impl From<VerificationChecks> for VerificationChecksContract {
    fn from(checks: VerificationChecks) -> Self {
        Self {
            manifest_reproduces_clean_text: checks.manifest_reproduces_clean_text,
            rules_allowed_by_profile: checks.rules_allowed_by_profile,
            no_undeclared_transformations: checks.no_undeclared_transformations,
            clean_rescan_completed: checks.clean_rescan_completed,
            expected_findings_reassessed: checks.expected_findings_reassessed,
            preserved_findings_visible: checks.preserved_findings_visible,
            deterministic: checks.deterministic,
            idempotent: checks.idempotent,
        }
    }
}

impl From<CleanProfileDefinition> for CleanProfileContract {
    fn from(definition: CleanProfileDefinition) -> Self {
        Self {
            id: clean_profile(definition.profile),
            label: definition.label,
            description: definition.description,
            supported: definition.supported,
            active_rule_ids: definition
                .active_rules
                .iter()
                .copied()
                .map(RuleId::as_str)
                .collect(),
        }
    }
}

impl From<TextSourceAnalysis> for TextSourceAnalysisContract {
    fn from(analysis: TextSourceAnalysis) -> Self {
        Self {
            raw_byte_length: analysis.raw_byte_length,
            source_kind: text_source_kind(analysis.source_kind),
            decode: analysis.decode.into(),
            scan: analysis.scan.into(),
        }
    }
}

impl From<TextDecodeResult> for TextDecodeResultContract {
    fn from(result: TextDecodeResult) -> Self {
        Self {
            encoding: source_encoding(result.encoding),
            encoding_evidence: encoding_evidence(result.encoding_evidence),
            bom: bom_kind(result.bom),
            status: decode_status(result.status),
            decoded_text: result.decoded_text,
            source_mappings: result
                .source_mappings
                .into_iter()
                .map(SourceCharacterMappingContract::from)
                .collect(),
        }
    }
}

impl From<SourceCharacterMapping> for SourceCharacterMappingContract {
    fn from(mapping: SourceCharacterMapping) -> Self {
        Self {
            code_point_index: mapping.code_point_index,
            source_byte_range: TextRangeContract {
                start: mapping.source_byte_range.start,
                end: mapping.source_byte_range.end,
            },
            decoded_utf8_byte_offset: mapping.decoded_utf8_byte_offset,
            utf16_offset: mapping.utf16_offset,
        }
    }
}

impl From<TextIngestionError> for TextIngestionErrorContract {
    fn from(error: TextIngestionError) -> Self {
        let (code, message) = match error {
            TextIngestionError::UnsupportedEncoding => (
                "unsupported_encoding",
                "The file encoding is not supported in this TextScope phase.",
            ),
            TextIngestionError::AmbiguousEncoding => (
                "ambiguous_encoding",
                "The file resembles BOM-less UTF-16 and cannot be decoded without guessing.",
            ),
            TextIngestionError::InvalidUtf8 => {
                ("invalid_utf8", "The file is not strictly valid UTF-8.")
            }
            TextIngestionError::InvalidUtf16 => {
                ("invalid_utf16", "The file contains invalid UTF-16.")
            }
        };

        Self { code, message }
    }
}

impl From<HtmlAnalysis> for HtmlAnalysisContract {
    fn from(analysis: HtmlAnalysis) -> Self {
        Self {
            raw_byte_length: analysis.raw_byte_length,
            decode: analysis.decode.into(),
            encoding: analysis.encoding.into(),
            summary: analysis.summary.into(),
            metadata: analysis.metadata.into(),
            provenance: analysis
                .provenance
                .into_iter()
                .map(ProvenanceEvidenceContract::from)
                .collect(),
            elements: analysis
                .elements
                .into_iter()
                .map(HtmlElementRecordContract::from)
                .collect(),
            text_segments: analysis
                .text_segments
                .into_iter()
                .map(HtmlTextSegmentContract::from)
                .collect(),
            comments: analysis
                .comments
                .into_iter()
                .map(HtmlCommentRecordContract::from)
                .collect(),
            external_references: analysis
                .external_references
                .into_iter()
                .map(HtmlExternalReferenceContract::from)
                .collect(),
            findings: analysis
                .findings
                .into_iter()
                .map(HtmlFindingContract::from)
                .collect(),
            parser_recoveries: analysis
                .parser_recoveries
                .into_iter()
                .map(HtmlParserRecoveryContract::from)
                .collect(),
            text_channels: analysis.text_channels.into(),
            scan: analysis.scan.into(),
        }
    }
}

impl From<HtmlAnalysisError> for HtmlAnalysisErrorContract {
    fn from(error: HtmlAnalysisError) -> Self {
        match error {
            HtmlAnalysisError::ByteIngestion(error) => {
                let error = TextIngestionErrorContract::from(error);
                Self {
                    code: error.code,
                    message: error.message,
                    resource_limit: None,
                }
            }
            HtmlAnalysisError::UnsupportedDeclaredEncoding => Self {
                code: "unsupported_declared_encoding",
                message: "The HTML declares an encoding that is not supported in this TextScope phase.",
                resource_limit: None,
            },
            HtmlAnalysisError::ResourceLimitExceeded(limit) => Self {
                code: "resource_limit_exceeded",
                message: "The HTML exceeds an explicit TextScope resource-safety limit.",
                resource_limit: Some(html_resource_limit(limit)),
            },
            HtmlAnalysisError::ParserFailure => Self {
                code: "html_parser_failure",
                message: "The local Rust HTML parser could not complete the analysis.",
                resource_limit: None,
            },
        }
    }
}

impl From<HtmlEncodingEvidence> for HtmlEncodingEvidenceContract {
    fn from(evidence: HtmlEncodingEvidence) -> Self {
        Self {
            declarations: evidence
                .declarations
                .into_iter()
                .map(HtmlEncodingDeclarationContract::from)
                .collect(),
            bom_precedence_applied: evidence.bom_precedence_applied,
            declaration_conflict: evidence.declaration_conflict,
        }
    }
}

impl From<HtmlEncodingDeclaration> for HtmlEncodingDeclarationContract {
    fn from(declaration: HtmlEncodingDeclaration) -> Self {
        Self {
            sequence: declaration.sequence,
            kind: html_encoding_declaration_kind(declaration.kind),
            declared_label: declaration.declared_label,
            normalized_label: declaration.normalized_label,
            source_range: declaration.source_range.into(),
        }
    }
}

impl From<HtmlSourceRange> for HtmlSourceRangeContract {
    fn from(range: HtmlSourceRange) -> Self {
        Self {
            decoded_utf8_byte_range: TextRangeContract {
                start: range.decoded_utf8_byte_range.start,
                end: range.decoded_utf8_byte_range.end,
            },
            raw_source_byte_range: range.raw_source_byte_range.map(|raw| TextRangeContract {
                start: raw.start,
                end: raw.end,
            }),
        }
    }
}

impl From<HtmlDocumentSummary> for HtmlDocumentSummaryContract {
    fn from(summary: HtmlDocumentSummary) -> Self {
        Self {
            doctype: summary.doctype.map(HtmlDoctypeRecordContract::from),
            quirks_mode: summary.quirks_mode,
            parsed_elements: summary.parsed_elements,
            source_start_tags: summary.source_start_tags,
            attributes: summary.attributes,
            text_segments: summary.text_segments,
            comments: summary.comments,
            script_blocks: summary.script_blocks,
            style_blocks: summary.style_blocks,
            noscript_blocks: summary.noscript_blocks,
            template_blocks: summary.template_blocks,
            external_references: summary.external_references,
            tokenizer_recoveries: summary.tokenizer_recoveries,
            tree_builder_recoveries: summary.tree_builder_recoveries,
            html_present: summary.html_present,
            head_present: summary.head_present,
            body_present: summary.body_present,
            title_present: summary.title_present,
            meta_elements: summary.meta_elements,
            link_elements: summary.link_elements,
        }
    }
}

impl From<HtmlDoctypeRecord> for HtmlDoctypeRecordContract {
    fn from(doctype: HtmlDoctypeRecord) -> Self {
        Self {
            name: doctype.name,
            public_identifier: doctype.public_identifier,
            system_identifier: doctype.system_identifier,
            force_quirks: doctype.force_quirks,
            source_range: doctype.source_range.into(),
        }
    }
}

impl From<HtmlMetadata> for HtmlMetadataContract {
    fn from(metadata: HtmlMetadata) -> Self {
        Self {
            title: metadata.title,
            lang: metadata.lang,
            dir: metadata.dir,
            meta_declarations: metadata
                .meta_declarations
                .into_iter()
                .map(HtmlMetaDeclarationContract::from)
                .collect(),
            canonical_links: metadata.canonical_links,
            base_hrefs: metadata.base_hrefs,
        }
    }
}

impl From<HtmlMetaDeclaration> for HtmlMetaDeclarationContract {
    fn from(declaration: HtmlMetaDeclaration) -> Self {
        Self {
            sequence: declaration.sequence,
            charset: declaration.charset,
            name: declaration.name,
            property: declaration.property,
            content: declaration.content,
            source_range: declaration.source_range.into(),
        }
    }
}

impl From<HtmlElementRecord> for HtmlElementRecordContract {
    fn from(element: HtmlElementRecord) -> Self {
        Self {
            sequence: element.sequence,
            name: element.name,
            attributes: element
                .attributes
                .into_iter()
                .map(HtmlAttributeRecordContract::from)
                .collect(),
            self_closing: element.self_closing,
            source_range: element.source_range.into(),
        }
    }
}

impl From<HtmlAttributeRecord> for HtmlAttributeRecordContract {
    fn from(attribute: HtmlAttributeRecord) -> Self {
        Self {
            name: attribute.name,
            value: attribute.value,
            source_range: attribute.source_range.into(),
        }
    }
}

impl From<HtmlTextSegment> for HtmlTextSegmentContract {
    fn from(segment: HtmlTextSegment) -> Self {
        Self {
            sequence: segment.sequence,
            text: segment.text,
            context_element: segment.context_element,
            classification: html_text_classification(segment.classification),
            source_text: segment.source_text,
            source_range: segment.source_range.map(HtmlSourceRangeContract::from),
        }
    }
}

impl From<HtmlCommentRecord> for HtmlCommentRecordContract {
    fn from(comment: HtmlCommentRecord) -> Self {
        Self {
            sequence: comment.sequence,
            text: comment.text,
            source_range: comment.source_range.into(),
        }
    }
}

impl From<HtmlExternalReference> for HtmlExternalReferenceContract {
    fn from(reference: HtmlExternalReference) -> Self {
        Self {
            sequence: reference.sequence,
            element: reference.element,
            attribute: reference.attribute,
            value: reference.value,
            kind: html_reference_kind(reference.kind),
            source_range: reference.source_range.into(),
        }
    }
}

impl From<HtmlFinding> for HtmlFindingContract {
    fn from(finding: HtmlFinding) -> Self {
        Self {
            sequence: finding.sequence,
            kind: html_finding_kind(finding.kind),
            severity: html_finding_severity(finding.severity),
            element_sequence: finding.element_sequence,
            attribute: finding.attribute,
            description: finding.description,
            source_range: finding.source_range.map(HtmlSourceRangeContract::from),
        }
    }
}

impl From<HtmlParserRecovery> for HtmlParserRecoveryContract {
    fn from(recovery: HtmlParserRecovery) -> Self {
        Self {
            sequence: recovery.sequence,
            layer: html_parser_layer(recovery.layer),
            message: recovery.message,
            source_range: recovery.source_range.map(HtmlSourceRangeContract::from),
        }
    }
}

impl From<HtmlTextChannels> for HtmlTextChannelsContract {
    fn from(channels: HtmlTextChannels) -> Self {
        Self {
            document_text: channels.document_text,
            metadata_text: channels.metadata_text,
            active_content_text: channels.active_content_text,
            style_text: channels.style_text,
            template_text: channels.template_text,
            noscript_text: channels.noscript_text,
        }
    }
}

impl From<DocxAnalysis> for DocxAnalysisContract {
    fn from(analysis: DocxAnalysis) -> Self {
        Self {
            package: analysis.package.into(),
            parts: analysis
                .parts
                .into_iter()
                .map(DocxPartRecordContract::from)
                .collect(),
            content_types: analysis
                .content_types
                .into_iter()
                .map(DocxContentTypeDeclarationContract::from)
                .collect(),
            relationships: analysis
                .relationships
                .into_iter()
                .map(DocxRelationshipContract::from)
                .collect(),
            metadata: analysis.metadata.into(),
            provenance: analysis
                .provenance
                .into_iter()
                .map(ProvenanceEvidenceContract::from)
                .collect(),
            summary: analysis.summary.into(),
            text_segments: analysis
                .text_segments
                .into_iter()
                .map(DocxTextSegmentContract::from)
                .collect(),
            comments: analysis
                .comments
                .into_iter()
                .map(DocxCommentContract::from)
                .collect(),
            revisions: analysis
                .revisions
                .into_iter()
                .map(DocxRevisionContract::from)
                .collect(),
            hyperlinks: analysis
                .hyperlinks
                .into_iter()
                .map(DocxHyperlinkContract::from)
                .collect(),
            findings: analysis
                .findings
                .into_iter()
                .map(DocxFindingContract::from)
                .collect(),
            text_channels: analysis.text_channels.into(),
            scan: analysis.scan.into(),
        }
    }
}

impl From<DocxAnalysisError> for DocxAnalysisErrorContract {
    fn from(error: DocxAnalysisError) -> Self {
        let (code, message, resource_limit) = match error {
            DocxAnalysisError::InvalidZip => (
                "invalid_zip",
                "The selected file is not a valid ZIP package.",
                None,
            ),
            DocxAnalysisError::DuplicateEntryNames => (
                "duplicate_entry_names",
                "The DOCX package contains duplicate entry names and is structurally ambiguous.",
                None,
            ),
            DocxAnalysisError::EncryptedEntry => (
                "encrypted_entry",
                "The DOCX package contains an encrypted entry.",
                None,
            ),
            DocxAnalysisError::UnsupportedCompression => (
                "unsupported_compression",
                "The DOCX package uses an unsupported compression method.",
                None,
            ),
            DocxAnalysisError::ResourceLimitExceeded(limit) => (
                "resource_limit_exceeded",
                "The DOCX package exceeds an explicit TextScope resource-safety limit.",
                Some(docx_resource_limit(limit)),
            ),
            DocxAnalysisError::MissingContentTypes => (
                "missing_content_types",
                "The package is missing [Content_Types].xml.",
                None,
            ),
            DocxAnalysisError::MissingRootRelationships => (
                "missing_root_relationships",
                "The package is missing _rels/.rels.",
                None,
            ),
            DocxAnalysisError::MissingMainDocument => (
                "missing_main_document",
                "The package is missing word/document.xml.",
                None,
            ),
            DocxAnalysisError::MalformedXml => (
                "malformed_xml",
                "An OOXML part is not well-formed supported XML.",
                None,
            ),
            DocxAnalysisError::MalformedRelationships => (
                "malformed_relationships",
                "A required OOXML relationship declaration is malformed.",
                None,
            ),
            DocxAnalysisError::UnsupportedPackage => (
                "unsupported_package",
                "The ZIP package is not a supported WordprocessingML DOCX package.",
                None,
            ),
            DocxAnalysisError::PartReadFailure => (
                "part_read_failure",
                "TextScope could not read a required DOCX package part.",
                None,
            ),
        };

        Self {
            code,
            message,
            resource_limit,
        }
    }
}

impl From<DocxPackageSummary> for DocxPackageSummaryContract {
    fn from(summary: DocxPackageSummary) -> Self {
        Self {
            status: docx_package_status(summary.status),
            raw_byte_length: summary.raw_byte_length,
            entry_count: summary.entry_count,
            compressed_total: summary.compressed_total,
            uncompressed_total: summary.uncompressed_total,
            compression_ratio_milli: summary.compression_ratio_milli,
            zip64_present: summary.zip64_present,
            archive_offset: summary.archive_offset,
            duplicate_entry_names: summary.duplicate_entry_names,
            invalid_entry_names: summary.invalid_entry_names,
        }
    }
}

impl From<DocxPartRecord> for DocxPartRecordContract {
    fn from(part: DocxPartRecord) -> Self {
        Self {
            sequence: part.sequence,
            part_name: part.part_name,
            content_type: part.content_type,
            compressed_size: part.compressed_size,
            uncompressed_size: part.uncompressed_size,
            crc32: part.crc32,
            crc_status: docx_crc_status(part.crc_status),
            compression_method: part.compression_method,
            classification: docx_part_classification(part.classification),
            relationship_count: part.relationship_count,
            encrypted: part.encrypted,
            directory: part.directory,
            symlink: part.symlink,
            local_header_offset: part.local_header_offset,
            data_offset: part.data_offset,
            xml_encoding: part.xml_encoding.map(source_encoding),
        }
    }
}

impl From<DocxContentTypeDeclaration> for DocxContentTypeDeclarationContract {
    fn from(declaration: DocxContentTypeDeclaration) -> Self {
        Self {
            sequence: declaration.sequence,
            kind: docx_content_type_kind(declaration.kind),
            extension: declaration.extension,
            part_name: declaration.part_name,
            content_type: declaration.content_type,
            source_location: declaration.source_location.into(),
        }
    }
}

impl From<DocxRelationship> for DocxRelationshipContract {
    fn from(relationship: DocxRelationship) -> Self {
        Self {
            sequence: relationship.sequence,
            relationship_part: relationship.relationship_part,
            source_part: relationship.source_part,
            id: relationship.id,
            relationship_type: relationship.relationship_type,
            target: relationship.target,
            external: relationship.external,
            resolved_internal_part: relationship.resolved_internal_part,
            source_location: relationship.source_location.into(),
        }
    }
}

impl From<DocxMetadata> for DocxMetadataContract {
    fn from(metadata: DocxMetadata) -> Self {
        Self {
            core_properties: metadata
                .core_properties
                .into_iter()
                .map(DocxMetadataPropertyContract::from)
                .collect(),
            app_properties: metadata
                .app_properties
                .into_iter()
                .map(DocxMetadataPropertyContract::from)
                .collect(),
            custom_properties: metadata
                .custom_properties
                .into_iter()
                .map(DocxMetadataPropertyContract::from)
                .collect(),
        }
    }
}

impl From<DocxMetadataProperty> for DocxMetadataPropertyContract {
    fn from(property: DocxMetadataProperty) -> Self {
        Self {
            sequence: property.sequence,
            name: property.name,
            value: property.value,
            value_type: property.value_type,
            source_location: property.source_location.into(),
        }
    }
}

impl From<DocxDocumentSummary> for DocxDocumentSummaryContract {
    fn from(summary: DocxDocumentSummary) -> Self {
        Self {
            main_document_present: summary.main_document_present,
            styles_present: summary.styles_present,
            numbering_present: summary.numbering_present,
            paragraphs: summary.paragraphs,
            runs: summary.runs,
            tables: summary.tables,
            table_rows: summary.table_rows,
            table_cells: summary.table_cells,
            headers: summary.headers,
            footers: summary.footers,
            footnotes: summary.footnotes,
            endnotes: summary.endnotes,
            comments: summary.comments,
            tracked_changes: summary.tracked_changes,
            explicit_hidden_runs: summary.explicit_hidden_runs,
            hyperlinks: summary.hyperlinks,
            external_relationships: summary.external_relationships,
            media_parts: summary.media_parts,
            embedded_object_parts: summary.embedded_object_parts,
            custom_xml_parts: summary.custom_xml_parts,
        }
    }
}

impl From<DocxTextSegment> for DocxTextSegmentContract {
    fn from(segment: DocxTextSegment) -> Self {
        Self {
            sequence: segment.sequence,
            text: segment.text,
            part_name: segment.part_name,
            channel: docx_text_channel(segment.channel),
            kind: docx_text_kind(segment.kind),
            paragraph_index: segment.paragraph_index,
            run_index: segment.run_index,
            table_index: segment.table_index,
            row_index: segment.row_index,
            cell_index: segment.cell_index,
            xml_space_preserve: segment.xml_space_preserve,
            explicit_hidden: segment.explicit_hidden,
            revision_kind: segment.revision_kind.map(docx_revision_kind),
            source_location: segment.source_location.into(),
        }
    }
}

impl From<DocxComment> for DocxCommentContract {
    fn from(comment: DocxComment) -> Self {
        Self {
            sequence: comment.sequence,
            id: comment.id,
            declared_author: comment.declared_author,
            initials: comment.initials,
            declared_date: comment.declared_date,
            text: comment.text,
            source_location: comment.source_location.into(),
        }
    }
}

impl From<DocxRevision> for DocxRevisionContract {
    fn from(revision: DocxRevision) -> Self {
        Self {
            sequence: revision.sequence,
            kind: docx_revision_kind(revision.kind),
            id: revision.id,
            declared_author: revision.declared_author,
            declared_date: revision.declared_date,
            text: revision.text,
            source_location: revision.source_location.into(),
        }
    }
}

impl From<DocxHyperlink> for DocxHyperlinkContract {
    fn from(hyperlink: DocxHyperlink) -> Self {
        Self {
            sequence: hyperlink.sequence,
            part_name: hyperlink.part_name,
            display_text: hyperlink.display_text,
            relationship_id: hyperlink.relationship_id,
            target: hyperlink.target,
            external: hyperlink.external,
            source_location: hyperlink.source_location.into(),
        }
    }
}

impl From<DocxFinding> for DocxFindingContract {
    fn from(finding: DocxFinding) -> Self {
        Self {
            sequence: finding.sequence,
            kind: docx_finding_kind(finding.kind),
            severity: docx_finding_severity(finding.severity),
            part_name: finding.part_name,
            relationship_sequence: finding.relationship_sequence,
            description: finding.description,
            source_location: finding
                .source_location
                .map(DocxSourceLocationContract::from),
        }
    }
}

impl From<DocxTextChannels> for DocxTextChannelsContract {
    fn from(channels: DocxTextChannels) -> Self {
        Self {
            document_text: channels.document_text,
            headers: channels
                .headers
                .into_iter()
                .map(DocxPartTextChannelContract::from)
                .collect(),
            footers: channels
                .footers
                .into_iter()
                .map(DocxPartTextChannelContract::from)
                .collect(),
            footnotes: channels
                .footnotes
                .into_iter()
                .map(DocxPartTextChannelContract::from)
                .collect(),
            endnotes: channels
                .endnotes
                .into_iter()
                .map(DocxPartTextChannelContract::from)
                .collect(),
            comments: channels
                .comments
                .into_iter()
                .map(DocxPartTextChannelContract::from)
                .collect(),
            deleted_content: channels.deleted_content,
        }
    }
}

impl From<DocxPartTextChannel> for DocxPartTextChannelContract {
    fn from(channel: DocxPartTextChannel) -> Self {
        Self {
            part_name: channel.part_name,
            text: channel.text,
        }
    }
}

impl From<DocxSourceLocation> for DocxSourceLocationContract {
    fn from(location: DocxSourceLocation) -> Self {
        Self {
            part_name: location.part_name,
            decoded_xml_byte_range: location
                .decoded_xml_byte_range
                .map(DocxByteRangeContract::from),
            part_byte_range: location.part_byte_range.map(DocxByteRangeContract::from),
            package_byte_range: location.package_byte_range.map(DocxByteRangeContract::from),
        }
    }
}

impl From<DocxByteRange> for DocxByteRangeContract {
    fn from(range: DocxByteRange) -> Self {
        Self {
            start: range.start,
            end: range.end,
        }
    }
}

impl From<PdfAnalysis> for PdfAnalysisContract {
    fn from(analysis: PdfAnalysis) -> Self {
        Self {
            summary: analysis.summary.into(),
            objects: analysis.objects.into_iter().map(Into::into).collect(),
            revisions: analysis.revisions.into_iter().map(Into::into).collect(),
            metadata: analysis.metadata.into(),
            provenance: analysis
                .provenance
                .into_iter()
                .map(ProvenanceEvidenceContract::from)
                .collect(),
            pages: analysis.pages.into_iter().map(Into::into).collect(),
            images: analysis.images.into_iter().map(Into::into).collect(),
            fonts: analysis.fonts.into_iter().map(Into::into).collect(),
            actions: analysis.actions.into_iter().map(Into::into).collect(),
            annotations: analysis.annotations.into_iter().map(Into::into).collect(),
            external_references: analysis
                .external_references
                .into_iter()
                .map(Into::into)
                .collect(),
            embedded_files: analysis
                .embedded_files
                .into_iter()
                .map(Into::into)
                .collect(),
            forms: analysis.forms.into(),
            signatures: analysis.signatures.into_iter().map(Into::into).collect(),
            text_segments: analysis.text_segments.into_iter().map(Into::into).collect(),
            page_text: analysis.page_text,
            document_text: analysis.document_text,
            text_extraction_status: pdf_text_extraction_status(analysis.text_extraction_status),
            findings: analysis.findings.into_iter().map(Into::into).collect(),
            scan: analysis.scan.into(),
        }
    }
}

impl From<PdfAnalysisError> for PdfAnalysisErrorContract {
    fn from(error: PdfAnalysisError) -> Self {
        let resource_limit = match error {
            PdfAnalysisError::ResourceLimitExceeded(limit) => Some(pdf_resource_limit(limit)),
            _ => None,
        };
        let code = match error {
            PdfAnalysisError::EmptyInput => "empty_input",
            PdfAnalysisError::NonPdfBytes => "non_pdf_bytes",
            PdfAnalysisError::InvalidHeader => "invalid_header",
            PdfAnalysisError::EncryptedPdfUnsupported => "encrypted_pdf_unsupported",
            PdfAnalysisError::PasswordRequired => "password_required",
            PdfAnalysisError::UnsupportedEncryption => "unsupported_encryption",
            PdfAnalysisError::MalformedStructure => "malformed_structure",
            PdfAnalysisError::ResourceLimitExceeded(_) => "resource_limit_exceeded",
        };
        let message = match error {
            PdfAnalysisError::EmptyInput => "The selected PDF is empty.",
            PdfAnalysisError::NonPdfBytes | PdfAnalysisError::InvalidHeader => {
                "The selected bytes do not contain a supported PDF header."
            }
            PdfAnalysisError::EncryptedPdfUnsupported => {
                "The PDF is encrypted and cannot be analyzed without a supplied password."
            }
            PdfAnalysisError::PasswordRequired => {
                "The PDF requires a password and was not opened by TextScope."
            }
            PdfAnalysisError::UnsupportedEncryption => {
                "The PDF uses an encryption handler unsupported by TextScope."
            }
            PdfAnalysisError::MalformedStructure => {
                "The PDF structure could not be analyzed safely."
            }
            PdfAnalysisError::ResourceLimitExceeded(_) => {
                "The PDF exceeds an explicit TextScope resource-safety limit."
            }
        };
        Self {
            code,
            message,
            resource_limit,
        }
    }
}

impl From<PdfDocumentSummary> for PdfDocumentSummaryContract {
    fn from(summary: PdfDocumentSummary) -> Self {
        Self {
            raw_byte_length: summary.raw_byte_length,
            version: summary.version.map(Into::into),
            binary_marker_present: summary.binary_marker_present,
            eof_marker_present: summary.eof_marker_present,
            startxref_count: summary.startxref_count,
            xref_kind: pdf_xref_kind(summary.xref_kind),
            trailer_count: summary.trailer_count,
            object_count: summary.object_count,
            stream_count: summary.stream_count,
            object_stream_count: summary.object_stream_count,
            compressed_object_count: summary.compressed_object_count,
            page_count: summary.page_count,
            font_count: summary.font_count,
            image_count: summary.image_count,
            annotation_count: summary.annotation_count,
            action_count: summary.action_count,
            embedded_file_count: summary.embedded_file_count,
            form_present: summary.form_present,
            metadata_present: summary.metadata_present,
            javascript_present: summary.javascript_present,
            linearized: summary.linearized,
            encryption: pdf_encryption_status(summary.encryption),
        }
    }
}

impl From<PdfVersion> for PdfVersionContract {
    fn from(version: PdfVersion) -> Self {
        Self {
            major: version.major,
            minor: version.minor,
            declared: version.declared,
        }
    }
}

impl From<PdfObjectSummary> for PdfObjectSummaryContract {
    fn from(object: PdfObjectSummary) -> Self {
        Self {
            object_number: object.object_number,
            generation: object.generation,
            kind: object.kind,
            has_stream: object.has_stream,
            compressed_in_object_stream: object.compressed_in_object_stream,
            raw_byte_offset: object.raw_byte_offset,
        }
    }
}

impl From<PdfRevisionSummary> for PdfRevisionSummaryContract {
    fn from(revision: PdfRevisionSummary) -> Self {
        Self {
            sequence: revision.sequence,
            startxref_offset: revision.startxref_offset,
            xref_kind: pdf_xref_kind(revision.xref_kind),
            prev_offset: revision.prev_offset,
            trailer_observed: revision.trailer_observed,
            updated_object_ids: revision
                .updated_object_ids
                .into_iter()
                .map(Into::into)
                .collect(),
        }
    }
}

impl From<PdfPageSummary> for PdfPageSummaryContract {
    fn from(page: PdfPageSummary) -> Self {
        Self {
            page_index: page.page_index,
            object: page.object.into(),
            media_box: page.media_box.map(Into::into),
            crop_box: page.crop_box.map(Into::into),
            rotate: page.rotate,
            resources_present: page.resources_present,
            content_stream_count: page.content_stream_count,
            annotation_count: page.annotation_count,
            text: page.text,
        }
    }
}

impl From<PdfImageRecord> for PdfImageRecordContract {
    fn from(image: PdfImageRecord) -> Self {
        Self {
            sequence: image.sequence,
            page_indices: image.page_indices,
            object: image.object.into(),
            width: image.width,
            height: image.height,
            filter: image.filter,
            color_space: image.color_space,
        }
    }
}

impl From<PdfBox> for PdfBoxContract {
    fn from(pdf_box: PdfBox) -> Self {
        Self {
            values: pdf_box.values,
        }
    }
}

impl From<PdfMetadata> for PdfMetadataContract {
    fn from(metadata: PdfMetadata) -> Self {
        Self {
            title: metadata.title,
            author: metadata.author,
            subject: metadata.subject,
            keywords: metadata.keywords,
            creator: metadata.creator,
            producer: metadata.producer,
            creation_date: metadata.creation_date,
            modification_date: metadata.modification_date,
            trapped: metadata.trapped,
            info_entries: metadata.info_entries.into_iter().map(Into::into).collect(),
            xmp: metadata.xmp.map(Into::into),
            conflicts: metadata.conflicts.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<PdfMetadataEntry> for PdfMetadataEntryContract {
    fn from(entry: PdfMetadataEntry) -> Self {
        Self {
            key: entry.key,
            value: entry.value,
            source: entry.source.into(),
        }
    }
}

impl From<textscope_core::PdfXmpMetadata> for PdfXmpMetadataContract {
    fn from(xmp: textscope_core::PdfXmpMetadata) -> Self {
        Self {
            present: xmp.present,
            well_formed_xml: xmp.well_formed_xml,
            properties: xmp.properties.into_iter().map(Into::into).collect(),
            source: xmp.source.map(Into::into),
        }
    }
}

impl From<PdfMetadataConflict> for PdfMetadataConflictContract {
    fn from(conflict: PdfMetadataConflict) -> Self {
        Self {
            key: conflict.key,
            info_value: conflict.info_value,
            xmp_value: conflict.xmp_value,
            description: conflict.description,
        }
    }
}

impl From<PdfActionRecord> for PdfActionRecordContract {
    fn from(action: PdfActionRecord) -> Self {
        Self {
            sequence: action.sequence,
            kind: pdf_action_kind(action.kind),
            target: action.target,
            source: action.source.into(),
            context: action.context,
        }
    }
}

impl From<PdfAnnotationRecord> for PdfAnnotationRecordContract {
    fn from(annotation: PdfAnnotationRecord) -> Self {
        Self {
            sequence: annotation.sequence,
            page_index: annotation.page_index,
            subtype: annotation.subtype,
            contents: annotation.contents,
            title: annotation.title,
            uri: annotation.uri,
            action: annotation.action.map(pdf_action_kind),
            flags: annotation.flags,
            source: annotation.source.into(),
        }
    }
}

impl From<PdfExternalReference> for PdfExternalReferenceContract {
    fn from(reference: PdfExternalReference) -> Self {
        Self {
            sequence: reference.sequence,
            value: reference.value,
            kind: pdf_external_reference_kind(reference.kind),
            source: reference.source.into(),
        }
    }
}

impl From<PdfEmbeddedFileRecord> for PdfEmbeddedFileRecordContract {
    fn from(file: PdfEmbeddedFileRecord) -> Self {
        Self {
            sequence: file.sequence,
            declared_filename: file.declared_filename,
            description: file.description,
            mime_type: file.mime_type,
            size: file.size,
            object: file.object.into(),
            stream_object: file.stream_object.map(Into::into),
        }
    }
}

impl From<PdfFormSummary> for PdfFormSummaryContract {
    fn from(form: PdfFormSummary) -> Self {
        Self {
            present: form.present,
            field_count: form.field_count,
            field_types: form.field_types,
            need_appearances: form.need_appearances,
            xfa_present: form.xfa_present,
        }
    }
}

impl From<PdfSignatureRecord> for PdfSignatureRecordContract {
    fn from(signature: PdfSignatureRecord) -> Self {
        Self {
            sequence: signature.sequence,
            source: signature.source.into(),
            byte_range_present: signature.byte_range_present,
            contents_present: signature.contents_present,
            sub_filter: signature.sub_filter,
            reason: signature.reason,
            name: signature.name,
            modification_date: signature.modification_date,
        }
    }
}

impl From<PdfTextSegment> for PdfTextSegmentContract {
    fn from(segment: PdfTextSegment) -> Self {
        Self {
            sequence: segment.sequence,
            page_index: segment.page_index,
            text: segment.text,
            font_reference: segment.font_reference.map(Into::into),
            content_stream_object: segment.content_stream_object.map(Into::into),
            operation_index: segment.operation_index,
            source_object: segment.source_object.map(Into::into),
            extraction_status: pdf_text_extraction_status(segment.extraction_status),
            explicit_non_painting: segment.explicit_non_painting,
        }
    }
}

impl From<PdfFinding> for PdfFindingContract {
    fn from(finding: PdfFinding) -> Self {
        Self {
            sequence: finding.sequence,
            kind: pdf_finding_kind(finding.kind),
            severity: pdf_finding_severity(finding.severity),
            description: finding.description,
            source: finding.source.map(Into::into),
        }
    }
}

impl From<PdfFontRecord> for PdfFontRecordContract {
    fn from(font: PdfFontRecord) -> Self {
        Self {
            sequence: font.sequence,
            page_indices: font.page_indices,
            resource_name: font.resource_name,
            source: font.source.into(),
            subtype: font.subtype,
            base_font: font.base_font,
            encoding: font.encoding,
            to_unicode_present: font.to_unicode_present,
            mapping_status: pdf_text_extraction_status(font.mapping_status),
        }
    }
}

impl From<PdfSourceReference> for PdfSourceReferenceContract {
    fn from(source: PdfSourceReference) -> Self {
        Self {
            object_number: source.object_number,
            generation: source.generation,
            raw_byte_offset: source.raw_byte_offset,
            context: source.context,
        }
    }
}

const fn html_encoding_declaration_kind(kind: HtmlEncodingDeclarationKind) -> &'static str {
    match kind {
        HtmlEncodingDeclarationKind::MetaCharset => "meta_charset",
        HtmlEncodingDeclarationKind::HttpEquivContentType => "http_equiv_content_type",
    }
}

const fn html_text_classification(classification: HtmlTextClassification) -> &'static str {
    match classification {
        HtmlTextClassification::VisibleContentCandidate => "visible_content_candidate",
        HtmlTextClassification::Title => "title",
        HtmlTextClassification::Metadata => "metadata",
        HtmlTextClassification::ScriptContent => "script_content",
        HtmlTextClassification::StyleContent => "style_content",
        HtmlTextClassification::TemplateContent => "template_content",
        HtmlTextClassification::NoscriptContent => "noscript_content",
    }
}

const fn html_reference_kind(kind: HtmlReferenceKind) -> &'static str {
    match kind {
        HtmlReferenceKind::Relative => "relative",
        HtmlReferenceKind::Absolute => "absolute",
        HtmlReferenceKind::DataUrl => "data_url",
        HtmlReferenceKind::Fragment => "fragment",
        HtmlReferenceKind::Other => "other",
    }
}

const fn evidence_kind(kind: EvidenceKind) -> &'static str {
    match kind {
        EvidenceKind::MetadataDeclaration => "metadata_declaration",
        EvidenceKind::GeneratorDeclaration => "generator_declaration",
        EvidenceKind::CreatorDeclaration => "creator_declaration",
        EvidenceKind::AuthorDeclaration => "author_declaration",
        EvidenceKind::ProducerDeclaration => "producer_declaration",
        EvidenceKind::CanonicalDeclaration => "canonical_declaration",
        EvidenceKind::EncodingDeclaration => "encoding_declaration",
        EvidenceKind::ClipboardRepresentation => "clipboard_representation",
        EvidenceKind::Attribution => "attribution",
    }
}

const fn evidence_source(source: EvidenceSource) -> &'static str {
    match source {
        EvidenceSource::HtmlMetadata => "html_metadata",
        EvidenceSource::HtmlGenerator => "html_generator",
        EvidenceSource::HtmlCanonicalLink => "html_canonical_link",
        EvidenceSource::HtmlEncoding => "html_encoding",
        EvidenceSource::DocxCoreProperties => "docx_core_properties",
        EvidenceSource::DocxAppProperties => "docx_app_properties",
        EvidenceSource::DocxCustomProperties => "docx_custom_properties",
        EvidenceSource::PdfInfo => "pdf_info",
        EvidenceSource::PdfXmp => "pdf_xmp",
        EvidenceSource::ClipboardPlainText => "clipboard_plain_text",
        EvidenceSource::ClipboardHtml => "clipboard_html",
        EvidenceSource::ClipboardOtherMime => "clipboard_other_mime",
    }
}

const fn clipboard_comparison_state(state: ClipboardComparisonState) -> &'static str {
    match state {
        ClipboardComparisonState::Equivalent => "equivalent",
        ClipboardComparisonState::TechnicallyDifferent => "technically_different",
        ClipboardComparisonState::ContentDifferent => "content_different",
        ClipboardComparisonState::NeedsReview => "needs_review",
    }
}

const fn evidence_level(level: EvidenceLevel) -> &'static str {
    match level {
        EvidenceLevel::Observation => "observation",
        EvidenceLevel::Claim => "claim",
        EvidenceLevel::Verification => "verification",
        EvidenceLevel::Attribution => "attribution",
    }
}

const fn verification_state(state: VerificationState) -> &'static str {
    match state {
        VerificationState::Observed => "observed",
        VerificationState::Declared => "declared",
        VerificationState::Verified => "verified",
        VerificationState::NotVerified => "not_verified",
        VerificationState::Unsupported => "unsupported",
        VerificationState::Conflict => "conflict",
    }
}

const fn html_finding_kind(kind: HtmlFindingKind) -> &'static str {
    match kind {
        HtmlFindingKind::ActiveScriptPresent => "active_script_present",
        HtmlFindingKind::InlineEventHandlerPresent => "inline_event_handler_present",
        HtmlFindingKind::ExternalReferencePresent => "external_reference_present",
        HtmlFindingKind::ExplicitHiddenMarker => "explicit_hidden_marker",
        HtmlFindingKind::EncodingDeclarationConflict => "encoding_declaration_conflict",
        HtmlFindingKind::ParserRecoveryObserved => "parser_recovery_observed",
    }
}

const fn html_finding_severity(severity: HtmlFindingSeverity) -> &'static str {
    match severity {
        HtmlFindingSeverity::Informational => "informational",
        HtmlFindingSeverity::Caution => "caution",
    }
}

const fn html_parser_layer(layer: HtmlParserLayer) -> &'static str {
    match layer {
        HtmlParserLayer::Tokenizer => "tokenizer",
        HtmlParserLayer::TreeBuilder => "tree_builder",
    }
}

const fn html_resource_limit(limit: HtmlResourceLimit) -> &'static str {
    match limit {
        HtmlResourceLimit::SourceBytes => "source_bytes",
        HtmlResourceLimit::Tokens => "tokens",
        HtmlResourceLimit::Elements => "elements",
        HtmlResourceLimit::Attributes => "attributes",
        HtmlResourceLimit::NestingDepth => "nesting_depth",
        HtmlResourceLimit::ParseRecoveries => "parse_recoveries",
        HtmlResourceLimit::Comments => "comments",
        HtmlResourceLimit::AttributeValueBytes => "attribute_value_bytes",
        HtmlResourceLimit::CommentBytes => "comment_bytes",
        HtmlResourceLimit::TextTokenBytes => "text_token_bytes",
        HtmlResourceLimit::ParsedNodes => "parsed_nodes",
    }
}

const fn docx_package_status(status: DocxPackageStatus) -> &'static str {
    match status {
        DocxPackageStatus::ValidWordprocessingPackage => "valid_wordprocessing_package",
        DocxPackageStatus::MalformedPackage => "malformed_package",
        DocxPackageStatus::UnsupportedPackage => "unsupported_package",
    }
}

const fn pdf_xref_kind(kind: PdfXrefKind) -> &'static str {
    match kind {
        PdfXrefKind::Table => "table",
        PdfXrefKind::Stream => "stream",
        PdfXrefKind::Unknown => "unknown",
    }
}

const fn pdf_encryption_status(status: PdfEncryptionStatus) -> &'static str {
    match status {
        PdfEncryptionStatus::Unencrypted => "unencrypted",
        PdfEncryptionStatus::EncryptedOpenableWithEmptyPassword => {
            "encrypted_openable_with_empty_password"
        }
        PdfEncryptionStatus::PasswordRequired => "password_required",
        PdfEncryptionStatus::UnsupportedEncryption => "unsupported_encryption",
    }
}

const fn pdf_action_kind(kind: PdfActionKind) -> &'static str {
    match kind {
        PdfActionKind::Uri => "uri",
        PdfActionKind::GoTo => "go_to",
        PdfActionKind::GoToRemote => "go_to_remote",
        PdfActionKind::Launch => "launch",
        PdfActionKind::JavaScript => "javascript",
        PdfActionKind::SubmitForm => "submit_form",
        PdfActionKind::ImportData => "import_data",
        PdfActionKind::Rendition => "rendition",
        PdfActionKind::Named => "named",
        PdfActionKind::Other => "other",
    }
}

const fn pdf_external_reference_kind(kind: PdfExternalReferenceKind) -> &'static str {
    match kind {
        PdfExternalReferenceKind::AbsoluteUri => "absolute_uri",
        PdfExternalReferenceKind::RelativeReference => "relative_reference",
        PdfExternalReferenceKind::FileLikeTarget => "file_like_target",
        PdfExternalReferenceKind::Other => "other",
    }
}

const fn pdf_text_extraction_status(status: PdfTextExtractionStatus) -> &'static str {
    match status {
        PdfTextExtractionStatus::NoTextOperators => "no_text_operators",
        PdfTextExtractionStatus::TextPresentButMappingUnavailable => {
            "text_present_but_mapping_unavailable"
        }
        PdfTextExtractionStatus::PartiallyExtracted => "partially_extracted",
        PdfTextExtractionStatus::Extracted => "extracted",
    }
}

const fn pdf_finding_kind(kind: PdfFindingKind) -> &'static str {
    match kind {
        PdfFindingKind::IncrementalUpdatesPresent => "incremental_updates_present",
        PdfFindingKind::EncryptionPresent => "encryption_present",
        PdfFindingKind::JavaScriptActionPresent => "javascript_action_present",
        PdfFindingKind::LaunchActionPresent => "launch_action_present",
        PdfFindingKind::ExternalUriPresent => "external_uri_present",
        PdfFindingKind::SubmitFormActionPresent => "submit_form_action_present",
        PdfFindingKind::EmbeddedFilePresent => "embedded_file_present",
        PdfFindingKind::XfaPresent => "xfa_present",
        PdfFindingKind::FormPresent => "form_present",
        PdfFindingKind::SignatureStructurePresent => "signature_structure_present",
        PdfFindingKind::ExplicitNonPaintingTextPresent => "explicit_non_painting_text_present",
        PdfFindingKind::MalformedStructure => "malformed_structure",
        PdfFindingKind::ResourceLimitExceeded => "resource_limit_exceeded",
        PdfFindingKind::TextMappingUnavailable => "text_mapping_unavailable",
        PdfFindingKind::NoTextLayer => "no_text_layer",
        PdfFindingKind::MetadataDeclarationConflict => "metadata_declaration_conflict",
        PdfFindingKind::LinearizationPresent => "linearization_present",
    }
}

const fn pdf_finding_severity(severity: PdfFindingSeverity) -> &'static str {
    match severity {
        PdfFindingSeverity::Informational => "informational",
        PdfFindingSeverity::Caution => "caution",
    }
}

const fn pdf_resource_limit(limit: PdfResourceLimit) -> &'static str {
    match limit {
        PdfResourceLimit::SourceBytes => "source_bytes",
        PdfResourceLimit::Objects => "objects",
        PdfResourceLimit::Pages => "pages",
        PdfResourceLimit::DecodedStreamBytes => "decoded_stream_bytes",
        PdfResourceLimit::IndividualStreamBytes => "individual_stream_bytes",
        PdfResourceLimit::DecompressionRatio => "decompression_ratio",
        PdfResourceLimit::ReferenceDepth => "reference_depth",
        PdfResourceLimit::ContentOperations => "content_operations",
        PdfResourceLimit::TextSegments => "text_segments",
        PdfResourceLimit::Annotations => "annotations",
        PdfResourceLimit::Actions => "actions",
        PdfResourceLimit::EmbeddedFiles => "embedded_files",
        PdfResourceLimit::MetadataBytes => "metadata_bytes",
        PdfResourceLimit::Findings => "findings",
    }
}

const fn docx_part_classification(classification: DocxPartClassification) -> &'static str {
    match classification {
        DocxPartClassification::ContentTypes => "content_types",
        DocxPartClassification::MainDocument => "main_document",
        DocxPartClassification::Relationships => "relationships",
        DocxPartClassification::Styles => "styles",
        DocxPartClassification::Numbering => "numbering",
        DocxPartClassification::Settings => "settings",
        DocxPartClassification::Theme => "theme",
        DocxPartClassification::FontTable => "font_table",
        DocxPartClassification::Header => "header",
        DocxPartClassification::Footer => "footer",
        DocxPartClassification::Footnotes => "footnotes",
        DocxPartClassification::Endnotes => "endnotes",
        DocxPartClassification::Comments => "comments",
        DocxPartClassification::Metadata => "metadata",
        DocxPartClassification::Media => "media",
        DocxPartClassification::EmbeddedObject => "embedded_object",
        DocxPartClassification::CustomXml => "custom_xml",
        DocxPartClassification::ActiveContentRelated => "active_content_related",
        DocxPartClassification::Other => "other",
    }
}

const fn docx_crc_status(status: DocxCrcStatus) -> &'static str {
    match status {
        DocxCrcStatus::DeclaredOnly => "declared_only",
        DocxCrcStatus::Verified => "verified",
    }
}

const fn docx_content_type_kind(kind: DocxContentTypeDeclarationKind) -> &'static str {
    match kind {
        DocxContentTypeDeclarationKind::Default => "default",
        DocxContentTypeDeclarationKind::Override => "override",
    }
}

const fn docx_text_channel(channel: DocxTextChannel) -> &'static str {
    match channel {
        DocxTextChannel::Body => "body",
        DocxTextChannel::Header => "header",
        DocxTextChannel::Footer => "footer",
        DocxTextChannel::Footnote => "footnote",
        DocxTextChannel::Endnote => "endnote",
        DocxTextChannel::Comment => "comment",
        DocxTextChannel::DeletedContent => "deleted_content",
    }
}

const fn docx_text_kind(kind: DocxTextKind) -> &'static str {
    match kind {
        DocxTextKind::Text => "text",
        DocxTextKind::Tab => "tab",
        DocxTextKind::LineBreak => "line_break",
        DocxTextKind::NoBreakHyphen => "no_break_hyphen",
        DocxTextKind::SoftHyphen => "soft_hyphen",
    }
}

const fn docx_revision_kind(kind: DocxRevisionKind) -> &'static str {
    match kind {
        DocxRevisionKind::Inserted => "inserted",
        DocxRevisionKind::Deleted => "deleted",
        DocxRevisionKind::MoveFrom => "move_from",
        DocxRevisionKind::MoveTo => "move_to",
    }
}

const fn docx_finding_kind(kind: DocxFindingKind) -> &'static str {
    match kind {
        DocxFindingKind::UnsafeEntryName => "unsafe_entry_name",
        DocxFindingKind::AmbiguousEntryName => "ambiguous_entry_name",
        DocxFindingKind::PrependedZipDataPresent => "prepended_zip_data_present",
        DocxFindingKind::SymlinkEntryPresent => "symlink_entry_present",
        DocxFindingKind::ExternalRelationshipPresent => "external_relationship_present",
        DocxFindingKind::EmbeddedObjectPresent => "embedded_object_present",
        DocxFindingKind::ActiveContentRelatedPartPresent => "active_content_related_part_present",
        DocxFindingKind::UnexpectedPackagePart => "unexpected_package_part",
        DocxFindingKind::TrackedChangesPresent => "tracked_changes_present",
        DocxFindingKind::CommentsPresent => "comments_present",
        DocxFindingKind::ExplicitHiddenTextPresent => "explicit_hidden_text_present",
        DocxFindingKind::CustomXmlPresent => "custom_xml_present",
        DocxFindingKind::MetadataPresent => "metadata_present",
        DocxFindingKind::MalformedRelationship => "malformed_relationship",
    }
}

const fn docx_finding_severity(severity: DocxFindingSeverity) -> &'static str {
    match severity {
        DocxFindingSeverity::Informational => "informational",
        DocxFindingSeverity::Caution => "caution",
    }
}

const fn docx_resource_limit(limit: DocxResourceLimit) -> &'static str {
    match limit {
        DocxResourceLimit::SourceBytes => "source_bytes",
        DocxResourceLimit::EntryCount => "entry_count",
        DocxResourceLimit::TotalUncompressedBytes => "total_uncompressed_bytes",
        DocxResourceLimit::PartBytes => "part_bytes",
        DocxResourceLimit::CompressionRatio => "compression_ratio",
        DocxResourceLimit::EntryNameBytes => "entry_name_bytes",
        DocxResourceLimit::XmlPartBytes => "xml_part_bytes",
        DocxResourceLimit::Relationships => "relationships",
        DocxResourceLimit::TextSegments => "text_segments",
        DocxResourceLimit::XmlNodes => "xml_nodes",
        DocxResourceLimit::XmlDepth => "xml_depth",
    }
}

const fn finding_kind(kind: FindingKind) -> &'static str {
    match kind {
        FindingKind::InvisibleCharacter => "invisible_character",
        FindingKind::UnusualWhitespace => "unusual_whitespace",
        FindingKind::BidirectionalControl => "bidirectional_control",
        FindingKind::VariationSelector => "variation_selector",
    }
}

const fn finding_severity(severity: FindingSeverity) -> &'static str {
    match severity {
        FindingSeverity::Informational => "informational",
        FindingSeverity::Caution => "caution",
    }
}

const fn clean_profile(profile: CleanProfile) -> &'static str {
    profile.as_str()
}

const fn transformation_operation(operation: TransformationOperation) -> &'static str {
    match operation {
        TransformationOperation::NormalizeNfc => "normalize_nfc",
        TransformationOperation::CanonicalizeLineEnding => "canonicalize_line_ending",
        TransformationOperation::RemoveLeadingByteOrderMark => "remove_leading_byte_order_mark",
        TransformationOperation::ReplaceNonBreakingSpace => "replace_non_breaking_space",
    }
}

const fn content_preservation_status(status: ContentPreservationStatus) -> &'static str {
    match status {
        ContentPreservationStatus::Preserved => "preserved",
        ContentPreservationStatus::NeedsReview => "needs_review",
        ContentPreservationStatus::Unknown => "unknown",
    }
}

const fn verification_status(status: VerificationStatus) -> &'static str {
    match status {
        VerificationStatus::Verified => "verified",
        VerificationStatus::VerifiedWithRemainingFindings => "verified_with_remaining_findings",
        VerificationStatus::NeedsReview => "needs_review",
        VerificationStatus::Failed => "failed",
    }
}

const fn text_source_kind(kind: TextSourceKind) -> &'static str {
    kind.as_str()
}

const fn source_encoding(encoding: SourceEncoding) -> &'static str {
    match encoding {
        SourceEncoding::Utf8 => "utf_8",
        SourceEncoding::Utf16LittleEndian => "utf_16_le",
        SourceEncoding::Utf16BigEndian => "utf_16_be",
    }
}

const fn encoding_evidence(evidence: EncodingEvidence) -> &'static str {
    match evidence {
        EncodingEvidence::EmptyInput => "empty_input",
        EncodingEvidence::StrictUtf8 => "strict_utf_8",
        EncodingEvidence::ByteOrderMark => "byte_order_mark",
    }
}

const fn bom_kind(bom: BomKind) -> &'static str {
    match bom {
        BomKind::None => "none",
        BomKind::Utf8 => "utf_8",
        BomKind::Utf16LittleEndian => "utf_16_le",
        BomKind::Utf16BigEndian => "utf_16_be",
    }
}

const fn decode_status(status: DecodeStatus) -> &'static str {
    match status {
        DecodeStatus::Decoded => "decoded",
    }
}
