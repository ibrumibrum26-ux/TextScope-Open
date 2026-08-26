use std::{error::Error, fmt};

use crate::{ProvenanceEvidence, ScanResult};

/// Maximum caller-provided PDF size accepted by the Core and the browser UI.
pub const MAX_PDF_SOURCE_BYTES: usize = 10 * 1024 * 1024;
/// Maximum number of materialized indirect objects retained by the adapter.
pub const MAX_PDF_OBJECTS: usize = 100_000;
/// Maximum number of pages traversed from the page tree.
pub const MAX_PDF_PAGES: usize = 10_000;
/// Maximum decoded bytes accepted for one PDF stream or page content budget.
pub const MAX_PDF_DECODED_STREAM_BYTES: usize = 8 * 1024 * 1024;
/// Maximum decoded bytes accepted from any individual content stream.
pub const MAX_PDF_INDIVIDUAL_STREAM_BYTES: usize = 8 * 1024 * 1024;
/// Maximum decoded-to-encoded ratio accepted where both sizes are measurable.
pub const MAX_PDF_DECOMPRESSION_RATIO: usize = 200;
/// Maximum decoded bytes accepted while resolving one font mapping stream.
pub const MAX_PDF_FONT_STREAM_BYTES: usize = 2 * 1024 * 1024;
/// Maximum nesting/reference depth used by the forensic walker.
pub const MAX_PDF_REFERENCE_DEPTH: usize = 256;
/// Maximum content operations inspected across one page.
pub const MAX_PDF_CONTENT_OPERATIONS: usize = 200_000;
/// Maximum extracted text segments retained across the document.
pub const MAX_PDF_TEXT_SEGMENTS: usize = 200_000;
/// Maximum annotations retained across the document.
pub const MAX_PDF_ANNOTATIONS: usize = 50_000;
/// Maximum actions retained across the document.
pub const MAX_PDF_ACTIONS: usize = 50_000;
/// Maximum embedded-file records retained across the document.
pub const MAX_PDF_EMBEDDED_FILES: usize = 10_000;
/// Maximum metadata bytes inspected from one Info/XMP declaration.
pub const MAX_PDF_METADATA_BYTES: usize = 1024 * 1024;
/// Maximum number of structural findings retained.
pub const MAX_PDF_FINDINGS: usize = 50_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PdfVersion {
    pub major: u8,
    pub minor: u8,
    pub declared: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PdfEncryptionStatus {
    Unencrypted,
    EncryptedOpenableWithEmptyPassword,
    PasswordRequired,
    UnsupportedEncryption,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PdfXrefKind {
    Table,
    Stream,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PdfSourceReference {
    pub object_number: Option<u32>,
    pub generation: Option<u16>,
    pub raw_byte_offset: Option<usize>,
    pub context: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PdfObjectSummary {
    pub object_number: u32,
    pub generation: u16,
    pub kind: String,
    pub has_stream: bool,
    pub compressed_in_object_stream: bool,
    pub raw_byte_offset: Option<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PdfRevisionSummary {
    pub sequence: usize,
    pub startxref_offset: Option<usize>,
    pub xref_kind: PdfXrefKind,
    pub prev_offset: Option<usize>,
    pub trailer_observed: bool,
    pub updated_object_ids: Vec<PdfSourceReference>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PdfBox {
    /// Values are retained as deterministic decimal representations because
    /// PDF numbers may be integers or real numbers and are not a text layout
    /// guarantee.
    pub values: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PdfPageSummary {
    pub page_index: usize,
    pub object: PdfSourceReference,
    pub media_box: Option<PdfBox>,
    pub crop_box: Option<PdfBox>,
    pub rotate: Option<i32>,
    pub resources_present: bool,
    pub content_stream_count: usize,
    pub annotation_count: usize,
    pub text: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PdfImageRecord {
    pub sequence: usize,
    pub page_indices: Vec<usize>,
    pub object: PdfSourceReference,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub filter: Option<String>,
    pub color_space: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PdfMetadataEntry {
    pub key: String,
    pub value: String,
    pub source: PdfSourceReference,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PdfXmpMetadata {
    pub present: bool,
    pub well_formed_xml: bool,
    pub properties: Vec<PdfMetadataEntry>,
    pub source: Option<PdfSourceReference>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PdfMetadataConflict {
    pub key: String,
    pub info_value: Option<String>,
    pub xmp_value: Option<String>,
    pub description: &'static str,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PdfMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub keywords: Option<String>,
    pub creator: Option<String>,
    pub producer: Option<String>,
    pub creation_date: Option<String>,
    pub modification_date: Option<String>,
    pub trapped: Option<String>,
    pub info_entries: Vec<PdfMetadataEntry>,
    pub xmp: Option<PdfXmpMetadata>,
    pub conflicts: Vec<PdfMetadataConflict>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PdfActionKind {
    Uri,
    GoTo,
    GoToRemote,
    Launch,
    JavaScript,
    SubmitForm,
    ImportData,
    Rendition,
    Named,
    Other,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PdfActionRecord {
    pub sequence: usize,
    pub kind: PdfActionKind,
    pub target: Option<String>,
    pub source: PdfSourceReference,
    pub context: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PdfAnnotationRecord {
    pub sequence: usize,
    pub page_index: usize,
    pub subtype: String,
    pub contents: Option<String>,
    pub title: Option<String>,
    pub uri: Option<String>,
    pub action: Option<PdfActionKind>,
    pub flags: Option<i64>,
    pub source: PdfSourceReference,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PdfExternalReferenceKind {
    AbsoluteUri,
    RelativeReference,
    FileLikeTarget,
    Other,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PdfExternalReference {
    pub sequence: usize,
    pub value: String,
    pub kind: PdfExternalReferenceKind,
    pub source: PdfSourceReference,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PdfEmbeddedFileRecord {
    pub sequence: usize,
    pub declared_filename: Option<String>,
    pub description: Option<String>,
    pub mime_type: Option<String>,
    pub size: Option<u64>,
    pub object: PdfSourceReference,
    pub stream_object: Option<PdfSourceReference>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PdfFormSummary {
    pub present: bool,
    pub field_count: usize,
    pub field_types: Vec<String>,
    pub need_appearances: Option<bool>,
    pub xfa_present: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PdfSignatureRecord {
    pub sequence: usize,
    pub source: PdfSourceReference,
    pub byte_range_present: bool,
    pub contents_present: bool,
    pub sub_filter: Option<String>,
    pub reason: Option<String>,
    pub name: Option<String>,
    pub modification_date: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PdfTextExtractionStatus {
    NoTextOperators,
    TextPresentButMappingUnavailable,
    PartiallyExtracted,
    Extracted,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PdfTextSegment {
    pub sequence: usize,
    pub page_index: usize,
    pub text: String,
    pub font_reference: Option<PdfSourceReference>,
    pub content_stream_object: Option<PdfSourceReference>,
    pub operation_index: usize,
    pub source_object: Option<PdfSourceReference>,
    pub extraction_status: PdfTextExtractionStatus,
    pub explicit_non_painting: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PdfFindingKind {
    IncrementalUpdatesPresent,
    EncryptionPresent,
    JavaScriptActionPresent,
    LaunchActionPresent,
    ExternalUriPresent,
    SubmitFormActionPresent,
    EmbeddedFilePresent,
    XfaPresent,
    FormPresent,
    SignatureStructurePresent,
    ExplicitNonPaintingTextPresent,
    MalformedStructure,
    ResourceLimitExceeded,
    TextMappingUnavailable,
    NoTextLayer,
    MetadataDeclarationConflict,
    LinearizationPresent,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PdfFindingSeverity {
    Informational,
    Caution,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PdfFinding {
    pub sequence: usize,
    pub kind: PdfFindingKind,
    pub severity: PdfFindingSeverity,
    pub description: &'static str,
    pub source: Option<PdfSourceReference>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PdfDocumentSummary {
    pub raw_byte_length: usize,
    pub version: Option<PdfVersion>,
    pub binary_marker_present: bool,
    pub eof_marker_present: bool,
    pub startxref_count: usize,
    pub xref_kind: PdfXrefKind,
    pub trailer_count: usize,
    pub object_count: usize,
    pub stream_count: usize,
    pub object_stream_count: usize,
    pub compressed_object_count: usize,
    pub page_count: usize,
    pub font_count: usize,
    pub image_count: usize,
    pub annotation_count: usize,
    pub action_count: usize,
    pub embedded_file_count: usize,
    pub form_present: bool,
    pub metadata_present: bool,
    pub javascript_present: bool,
    pub linearized: bool,
    pub encryption: PdfEncryptionStatus,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PdfAnalysis {
    pub summary: PdfDocumentSummary,
    pub objects: Vec<PdfObjectSummary>,
    pub revisions: Vec<PdfRevisionSummary>,
    pub metadata: PdfMetadata,
    /// Unified evidence records; Info and XMP values remain declarations.
    pub provenance: Vec<ProvenanceEvidence>,
    pub pages: Vec<PdfPageSummary>,
    pub images: Vec<PdfImageRecord>,
    pub fonts: Vec<PdfFontRecord>,
    pub actions: Vec<PdfActionRecord>,
    pub annotations: Vec<PdfAnnotationRecord>,
    pub external_references: Vec<PdfExternalReference>,
    pub embedded_files: Vec<PdfEmbeddedFileRecord>,
    pub forms: PdfFormSummary,
    pub signatures: Vec<PdfSignatureRecord>,
    pub text_segments: Vec<PdfTextSegment>,
    pub page_text: Vec<String>,
    pub document_text: String,
    pub text_extraction_status: PdfTextExtractionStatus,
    pub findings: Vec<PdfFinding>,
    /// Unicode scan of `document_text` only. Metadata, annotations, actions,
    /// and attachments are deliberately not mixed into this channel.
    pub scan: ScanResult,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PdfFontRecord {
    pub sequence: usize,
    pub page_indices: Vec<usize>,
    pub resource_name: String,
    pub source: PdfSourceReference,
    pub subtype: Option<String>,
    pub base_font: Option<String>,
    pub encoding: Option<String>,
    pub to_unicode_present: bool,
    pub mapping_status: PdfTextExtractionStatus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PdfResourceLimit {
    SourceBytes,
    Objects,
    Pages,
    DecodedStreamBytes,
    IndividualStreamBytes,
    DecompressionRatio,
    ReferenceDepth,
    ContentOperations,
    TextSegments,
    Annotations,
    Actions,
    EmbeddedFiles,
    MetadataBytes,
    Findings,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PdfAnalysisError {
    EmptyInput,
    NonPdfBytes,
    InvalidHeader,
    EncryptedPdfUnsupported,
    PasswordRequired,
    UnsupportedEncryption,
    MalformedStructure,
    ResourceLimitExceeded(PdfResourceLimit),
}

impl fmt::Display for PdfAnalysisError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyInput => formatter.write_str("The selected PDF is empty."),
            Self::NonPdfBytes | Self::InvalidHeader => {
                formatter.write_str("The selected bytes do not contain a supported PDF header.")
            }
            Self::EncryptedPdfUnsupported => formatter.write_str(
                "The PDF is encrypted and cannot be analyzed without a supplied password.",
            ),
            Self::PasswordRequired => {
                formatter.write_str("The PDF requires a password and was not opened by TextScope.")
            }
            Self::UnsupportedEncryption => {
                formatter.write_str("The PDF uses an encryption handler unsupported by TextScope.")
            }
            Self::MalformedStructure => {
                formatter.write_str("The PDF structure could not be analyzed safely.")
            }
            Self::ResourceLimitExceeded(_) => {
                formatter.write_str("The PDF exceeds an explicit TextScope resource-safety limit.")
            }
        }
    }
}

impl Error for PdfAnalysisError {}
