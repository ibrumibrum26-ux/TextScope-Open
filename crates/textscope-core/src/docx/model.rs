use std::{error::Error, fmt};

use crate::{ProvenanceEvidence, ScanResult, SourceEncoding};

/// Maximum caller-provided DOCX package size accepted by the Core and UI.
pub const MAX_DOCX_SOURCE_BYTES: usize = 10 * 1024 * 1024;
/// Maximum number of central-directory entries accepted in one package.
pub const MAX_DOCX_ENTRY_COUNT: usize = 2_048;
/// Maximum declared aggregate uncompressed size across the package.
pub const MAX_DOCX_TOTAL_UNCOMPRESSED_BYTES: u64 = 64 * 1024 * 1024;
/// Maximum declared uncompressed size of one ZIP entry.
pub const MAX_DOCX_PART_BYTES: u64 = 16 * 1024 * 1024;
/// Maximum uncompressed size of one XML part parsed by TextScope.
pub const MAX_DOCX_XML_PART_BYTES: u64 = 8 * 1024 * 1024;
/// Maximum uncompressed-to-compressed ratio for one entry or the package.
pub const MAX_DOCX_COMPRESSION_RATIO: u64 = 200;
/// Maximum decoded byte length of one ZIP entry name.
pub const MAX_DOCX_ENTRY_NAME_BYTES: usize = 1_024;
/// Maximum relationships retained across all relationship parts.
pub const MAX_DOCX_RELATIONSHIPS: usize = 20_000;
/// Maximum text fragments retained across every document channel.
pub const MAX_DOCX_TEXT_SEGMENTS: usize = 200_000;
/// Maximum XML nodes accepted in one parsed part.
pub const MAX_DOCX_XML_NODES: u32 = 250_000;
/// Maximum ancestor depth accepted in one parsed XML part.
pub const MAX_DOCX_XML_DEPTH: usize = 256;

/// Zero-based, half-open byte range.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DocxByteRange {
    pub start: usize,
    pub end: usize,
}

/// Provenance inside one decompressed XML part.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocxSourceLocation {
    pub part_name: String,
    /// Range in the strictly decoded UTF-8 representation parsed as XML.
    pub decoded_xml_byte_range: Option<DocxByteRange>,
    /// Range in the original uncompressed ZIP-entry bytes when provable.
    pub part_byte_range: Option<DocxByteRange>,
    /// Compressed package bytes cannot be mapped to XML nodes reliably.
    pub package_byte_range: Option<DocxByteRange>,
}

/// Package classification after required OOXML validation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DocxPackageStatus {
    ValidWordprocessingPackage,
    MalformedPackage,
    UnsupportedPackage,
}

/// Compact package-level ZIP evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocxPackageSummary {
    pub status: DocxPackageStatus,
    pub raw_byte_length: usize,
    pub entry_count: usize,
    pub compressed_total: u64,
    pub uncompressed_total: u64,
    /// Deterministic ratio multiplied by 1,000; zero when both totals are zero.
    pub compression_ratio_milli: u64,
    pub zip64_present: bool,
    pub archive_offset: u64,
    pub duplicate_entry_names: usize,
    pub invalid_entry_names: usize,
}

/// Semantic classification of one ZIP/OOXML part.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DocxPartClassification {
    ContentTypes,
    MainDocument,
    Relationships,
    Styles,
    Numbering,
    Settings,
    Theme,
    FontTable,
    Header,
    Footer,
    Footnotes,
    Endnotes,
    Comments,
    Metadata,
    Media,
    EmbeddedObject,
    CustomXml,
    ActiveContentRelated,
    Other,
}

/// CRC evidence available for one entry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DocxCrcStatus {
    DeclaredOnly,
    Verified,
}

/// One deterministic package-part inventory record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocxPartRecord {
    pub sequence: usize,
    pub part_name: String,
    pub content_type: Option<String>,
    pub compressed_size: u64,
    pub uncompressed_size: u64,
    pub crc32: u32,
    pub crc_status: DocxCrcStatus,
    pub compression_method: String,
    pub classification: DocxPartClassification,
    pub relationship_count: usize,
    pub encrypted: bool,
    pub directory: bool,
    pub symlink: bool,
    pub local_header_offset: u64,
    pub data_offset: Option<u64>,
    pub xml_encoding: Option<SourceEncoding>,
}

/// How one content-type declaration appears in `[Content_Types].xml`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DocxContentTypeDeclarationKind {
    Default,
    Override,
}

/// One source-observed OPC content-type declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocxContentTypeDeclaration {
    pub sequence: usize,
    pub kind: DocxContentTypeDeclarationKind,
    pub extension: Option<String>,
    pub part_name: Option<String>,
    pub content_type: String,
    pub source_location: DocxSourceLocation,
}

/// One relationship declared in an OOXML `.rels` part.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocxRelationship {
    pub sequence: usize,
    pub relationship_part: String,
    pub source_part: Option<String>,
    pub id: String,
    pub relationship_type: String,
    pub target: String,
    pub external: bool,
    pub resolved_internal_part: Option<String>,
    pub source_location: DocxSourceLocation,
}

/// One directly stored metadata declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocxMetadataProperty {
    pub sequence: usize,
    pub name: String,
    pub value: String,
    pub value_type: Option<String>,
    pub source_location: DocxSourceLocation,
}

/// Observed DOCX metadata; values are declarations, not verified attribution.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DocxMetadata {
    pub core_properties: Vec<DocxMetadataProperty>,
    pub app_properties: Vec<DocxMetadataProperty>,
    pub custom_properties: Vec<DocxMetadataProperty>,
}

/// Explicit document channel assigned to one text fragment.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DocxTextChannel {
    Body,
    Header,
    Footer,
    Footnote,
    Endnote,
    Comment,
    DeletedContent,
}

/// Whether text came from XML character data or a structural OOXML element.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DocxTextKind {
    Text,
    Tab,
    LineBreak,
    NoBreakHyphen,
    SoftHyphen,
}

/// One text fragment with structural and XML-part provenance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocxTextSegment {
    pub sequence: usize,
    pub text: String,
    pub part_name: String,
    pub channel: DocxTextChannel,
    pub kind: DocxTextKind,
    pub paragraph_index: Option<usize>,
    pub run_index: Option<usize>,
    pub table_index: Option<usize>,
    pub row_index: Option<usize>,
    pub cell_index: Option<usize>,
    pub xml_space_preserve: bool,
    pub explicit_hidden: bool,
    pub revision_kind: Option<DocxRevisionKind>,
    pub source_location: DocxSourceLocation,
}

/// One named text channel derived from a separate OOXML part.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocxPartTextChannel {
    pub part_name: String,
    pub text: String,
}

/// Text channels that must not be silently conflated.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DocxTextChannels {
    pub document_text: String,
    pub headers: Vec<DocxPartTextChannel>,
    pub footers: Vec<DocxPartTextChannel>,
    pub footnotes: Vec<DocxPartTextChannel>,
    pub endnotes: Vec<DocxPartTextChannel>,
    pub comments: Vec<DocxPartTextChannel>,
    pub deleted_content: String,
}

/// One comment stored in `word/comments.xml`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocxComment {
    pub sequence: usize,
    pub id: Option<String>,
    pub declared_author: Option<String>,
    pub initials: Option<String>,
    pub declared_date: Option<String>,
    pub text: String,
    pub source_location: DocxSourceLocation,
}

/// Tracked-change category observed in WordprocessingML.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DocxRevisionKind {
    Inserted,
    Deleted,
    MoveFrom,
    MoveTo,
}

/// One tracked revision observation; TextScope never accepts or rejects it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocxRevision {
    pub sequence: usize,
    pub kind: DocxRevisionKind,
    pub id: Option<String>,
    pub declared_author: Option<String>,
    pub declared_date: Option<String>,
    pub text: String,
    pub source_location: DocxSourceLocation,
}

/// One hyperlink observation linked to package relationships when available.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocxHyperlink {
    pub sequence: usize,
    pub part_name: String,
    pub display_text: String,
    pub relationship_id: Option<String>,
    pub target: Option<String>,
    pub external: bool,
    pub source_location: DocxSourceLocation,
}

/// Compact WordprocessingML structural summary.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DocxDocumentSummary {
    pub main_document_present: bool,
    pub styles_present: bool,
    pub numbering_present: bool,
    pub paragraphs: usize,
    pub runs: usize,
    pub tables: usize,
    pub table_rows: usize,
    pub table_cells: usize,
    pub headers: usize,
    pub footers: usize,
    pub footnotes: usize,
    pub endnotes: usize,
    pub comments: usize,
    pub tracked_changes: usize,
    pub explicit_hidden_runs: usize,
    pub hyperlinks: usize,
    pub external_relationships: usize,
    pub media_parts: usize,
    pub embedded_object_parts: usize,
    pub custom_xml_parts: usize,
}

/// Objective DOCX finding taxonomy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DocxFindingKind {
    UnsafeEntryName,
    AmbiguousEntryName,
    PrependedZipDataPresent,
    SymlinkEntryPresent,
    ExternalRelationshipPresent,
    EmbeddedObjectPresent,
    ActiveContentRelatedPartPresent,
    UnexpectedPackagePart,
    TrackedChangesPresent,
    CommentsPresent,
    ExplicitHiddenTextPresent,
    CustomXmlPresent,
    MetadataPresent,
    MalformedRelationship,
}

/// Conservative severity for a package or document observation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DocxFindingSeverity {
    Informational,
    Caution,
}

/// One neutral finding tied to a package part or XML location when available.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocxFinding {
    pub sequence: usize,
    pub kind: DocxFindingKind,
    pub severity: DocxFindingSeverity,
    pub part_name: Option<String>,
    pub relationship_sequence: Option<usize>,
    pub description: &'static str,
    pub source_location: Option<DocxSourceLocation>,
}

/// Complete deterministic analysis of caller-provided DOCX bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocxAnalysis {
    pub package: DocxPackageSummary,
    pub parts: Vec<DocxPartRecord>,
    pub content_types: Vec<DocxContentTypeDeclaration>,
    pub relationships: Vec<DocxRelationship>,
    pub metadata: DocxMetadata,
    /// Unified evidence records; stored declarations are not attribution.
    pub provenance: Vec<ProvenanceEvidence>,
    pub summary: DocxDocumentSummary,
    pub text_segments: Vec<DocxTextSegment>,
    pub comments: Vec<DocxComment>,
    pub revisions: Vec<DocxRevision>,
    pub hyperlinks: Vec<DocxHyperlink>,
    pub findings: Vec<DocxFinding>,
    pub text_channels: DocxTextChannels,
    /// Unicode scan of `text_channels.document_text` only.
    pub scan: ScanResult,
}

/// Explicit resource bound responsible for controlled rejection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DocxResourceLimit {
    SourceBytes,
    EntryCount,
    TotalUncompressedBytes,
    PartBytes,
    CompressionRatio,
    EntryNameBytes,
    XmlPartBytes,
    Relationships,
    TextSegments,
    XmlNodes,
    XmlDepth,
}

/// Controlled ZIP, OPC, or OOXML analysis failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DocxAnalysisError {
    InvalidZip,
    DuplicateEntryNames,
    EncryptedEntry,
    UnsupportedCompression,
    ResourceLimitExceeded(DocxResourceLimit),
    MissingContentTypes,
    MissingRootRelationships,
    MissingMainDocument,
    MalformedXml,
    MalformedRelationships,
    UnsupportedPackage,
    PartReadFailure,
}

impl fmt::Display for DocxAnalysisError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidZip => {
                formatter.write_str("The selected file is not a valid ZIP package.")
            }
            Self::DuplicateEntryNames => formatter.write_str(
                "The DOCX package contains duplicate entry names and is structurally ambiguous.",
            ),
            Self::EncryptedEntry => {
                formatter.write_str("The DOCX package contains an encrypted entry.")
            }
            Self::UnsupportedCompression => formatter.write_str(
                "The DOCX package uses a compression method unsupported by this TextScope phase.",
            ),
            Self::ResourceLimitExceeded(_) => formatter
                .write_str("The DOCX package exceeds an explicit TextScope resource-safety limit."),
            Self::MissingContentTypes => {
                formatter.write_str("The package is missing [Content_Types].xml.")
            }
            Self::MissingRootRelationships => {
                formatter.write_str("The package is missing _rels/.rels.")
            }
            Self::MissingMainDocument => {
                formatter.write_str("The package is missing word/document.xml.")
            }
            Self::MalformedXml => {
                formatter.write_str("An OOXML part is not well-formed supported XML.")
            }
            Self::MalformedRelationships => {
                formatter.write_str("A required OOXML relationship declaration is malformed.")
            }
            Self::UnsupportedPackage => formatter
                .write_str("The ZIP package is not a supported WordprocessingML DOCX package."),
            Self::PartReadFailure => {
                formatter.write_str("TextScope could not read a required DOCX package part.")
            }
        }
    }
}

impl Error for DocxAnalysisError {}
