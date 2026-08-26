use std::{error::Error, fmt};

use crate::{
    ProvenanceEvidence, ScanResult, SourceByteRange, TextDecodeResult, TextIngestionError,
};

/// Maximum HTML source size accepted by both the Core and the local UI.
pub const MAX_HTML_SOURCE_BYTES: usize = 10 * 1024 * 1024;
/// Maximum number of source-observed HTML start tags.
pub const MAX_HTML_SOURCE_ELEMENTS: usize = 100_000;
/// Maximum total number of source-observed attributes.
pub const MAX_HTML_ATTRIBUTES: usize = 500_000;
/// Maximum number of tokenizer output records.
pub const MAX_HTML_TOKENS: usize = 300_000;
/// Maximum source-observed nesting used as a pre-parser safety bound.
pub const MAX_HTML_NESTING_DEPTH: usize = 512;
/// Maximum number of parser recovery observations.
pub const MAX_HTML_PARSE_RECOVERIES: usize = 10_000;
/// Maximum number of source-observed comments.
pub const MAX_HTML_COMMENTS: usize = 50_000;
/// Maximum decoded byte length of one attribute value.
pub const MAX_HTML_ATTRIBUTE_VALUE_BYTES: usize = 1024 * 1024;
/// Maximum decoded byte length of one comment.
pub const MAX_HTML_COMMENT_BYTES: usize = 1024 * 1024;
/// Maximum decoded byte length of one tokenizer text record.
pub const MAX_HTML_TEXT_TOKEN_BYTES: usize = 2 * 1024 * 1024;

/// Zero-based, half-open byte range in the strictly decoded UTF-8 string.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecodedSourceRange {
    pub start: usize,
    pub end: usize,
}

/// Reliable location in decoded source and, when derivable, original bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HtmlSourceRange {
    pub decoded_utf8_byte_range: DecodedSourceRange,
    pub raw_source_byte_range: Option<SourceByteRange>,
}

/// How an HTML charset declaration was expressed in source.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HtmlEncodingDeclarationKind {
    MetaCharset,
    HttpEquivContentType,
}

/// One source-observed HTML charset declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HtmlEncodingDeclaration {
    pub sequence: usize,
    pub kind: HtmlEncodingDeclarationKind,
    pub declared_label: String,
    pub normalized_label: String,
    pub source_range: HtmlSourceRange,
}

/// HTML-specific evidence layered on top of the strict byte decoder.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HtmlEncodingEvidence {
    pub declarations: Vec<HtmlEncodingDeclaration>,
    pub bom_precedence_applied: bool,
    pub declaration_conflict: bool,
}

/// One source-observed document type declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HtmlDoctypeRecord {
    pub name: String,
    pub public_identifier: Option<String>,
    pub system_identifier: Option<String>,
    pub force_quirks: bool,
    pub source_range: HtmlSourceRange,
}

/// One attribute from a source-observed start tag.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HtmlAttributeRecord {
    pub name: String,
    pub value: String,
    pub source_range: HtmlSourceRange,
}

/// One source-observed HTML start tag, not an invented implied DOM element.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HtmlElementRecord {
    pub sequence: usize,
    pub name: String,
    pub attributes: Vec<HtmlAttributeRecord>,
    pub self_closing: bool,
    pub source_range: HtmlSourceRange,
}

/// Origin channel assigned to an HTML text node by the parsed HTML5 tree.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HtmlTextClassification {
    VisibleContentCandidate,
    Title,
    Metadata,
    ScriptContent,
    StyleContent,
    TemplateContent,
    NoscriptContent,
}

/// One parsed text node with conservative source provenance when unambiguous.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HtmlTextSegment {
    pub sequence: usize,
    pub text: String,
    pub context_element: String,
    pub classification: HtmlTextClassification,
    /// Exact representation in decoded HTML source; differs for entities.
    pub source_text: Option<String>,
    /// Unavailable when a parsed node cannot be correlated uniquely to a token.
    pub source_range: Option<HtmlSourceRange>,
}

/// One source-observed HTML comment, kept outside document text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HtmlCommentRecord {
    pub sequence: usize,
    pub text: String,
    pub source_range: HtmlSourceRange,
}

/// One source-observed `meta` declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HtmlMetaDeclaration {
    pub sequence: usize,
    pub charset: Option<String>,
    pub name: Option<String>,
    pub property: Option<String>,
    pub content: Option<String>,
    pub source_range: HtmlSourceRange,
}

/// Directly observed document metadata; no field is externally verified.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct HtmlMetadata {
    pub title: Option<String>,
    pub lang: Option<String>,
    pub dir: Option<String>,
    pub meta_declarations: Vec<HtmlMetaDeclaration>,
    pub canonical_links: Vec<String>,
    pub base_hrefs: Vec<String>,
}

/// Conservative classification of one source-declared reference value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HtmlReferenceKind {
    Relative,
    Absolute,
    DataUrl,
    Fragment,
    Other,
}

/// One reference-bearing HTML attribute; TextScope never resolves it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HtmlExternalReference {
    pub sequence: usize,
    pub element: String,
    pub attribute: String,
    pub value: String,
    pub kind: HtmlReferenceKind,
    pub source_range: HtmlSourceRange,
}

/// Objective HTML-specific observation taxonomy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HtmlFindingKind {
    ActiveScriptPresent,
    InlineEventHandlerPresent,
    ExternalReferencePresent,
    ExplicitHiddenMarker,
    EncodingDeclarationConflict,
    ParserRecoveryObserved,
}

/// Conservative severity for an HTML structural observation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HtmlFindingSeverity {
    Informational,
    Caution,
}

/// One neutral finding tied to source evidence where available.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HtmlFinding {
    pub sequence: usize,
    pub kind: HtmlFindingKind,
    pub severity: HtmlFindingSeverity,
    pub element_sequence: Option<usize>,
    pub attribute: Option<String>,
    pub description: &'static str,
    pub source_range: Option<HtmlSourceRange>,
}

/// Parser layer that reported a recoverable HTML condition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HtmlParserLayer {
    Tokenizer,
    TreeBuilder,
}

/// Recoverable parser observation, distinct from an analysis failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HtmlParserRecovery {
    pub sequence: usize,
    pub layer: HtmlParserLayer,
    pub message: String,
    pub source_range: Option<HtmlSourceRange>,
}

/// Compact deterministic summary of source and parsed structure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HtmlDocumentSummary {
    pub doctype: Option<HtmlDoctypeRecord>,
    pub quirks_mode: String,
    pub parsed_elements: usize,
    pub source_start_tags: usize,
    pub attributes: usize,
    pub text_segments: usize,
    pub comments: usize,
    pub script_blocks: usize,
    pub style_blocks: usize,
    pub noscript_blocks: usize,
    pub template_blocks: usize,
    pub external_references: usize,
    pub tokenizer_recoveries: usize,
    pub tree_builder_recoveries: usize,
    pub html_present: bool,
    pub head_present: bool,
    pub body_present: bool,
    pub title_present: bool,
    pub meta_elements: usize,
    pub link_elements: usize,
}

/// Explicit text channels derived from the parsed tree.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct HtmlTextChannels {
    pub document_text: String,
    pub metadata_text: String,
    pub active_content_text: String,
    pub style_text: String,
    pub template_text: String,
    pub noscript_text: String,
}

/// Complete deterministic forensic analysis of caller-provided HTML bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HtmlAnalysis {
    pub raw_byte_length: usize,
    pub decode: TextDecodeResult,
    pub encoding: HtmlEncodingEvidence,
    pub summary: HtmlDocumentSummary,
    pub metadata: HtmlMetadata,
    /// Unified evidence records; stored declarations are not attribution.
    pub provenance: Vec<ProvenanceEvidence>,
    pub elements: Vec<HtmlElementRecord>,
    pub text_segments: Vec<HtmlTextSegment>,
    pub comments: Vec<HtmlCommentRecord>,
    pub external_references: Vec<HtmlExternalReference>,
    pub findings: Vec<HtmlFinding>,
    pub parser_recoveries: Vec<HtmlParserRecovery>,
    pub text_channels: HtmlTextChannels,
    /// Unicode scan of `text_channels.document_text` only.
    pub scan: ScanResult,
}

/// Explicit resource bound responsible for a controlled rejection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HtmlResourceLimit {
    SourceBytes,
    Tokens,
    Elements,
    Attributes,
    NestingDepth,
    ParseRecoveries,
    Comments,
    AttributeValueBytes,
    CommentBytes,
    TextTokenBytes,
    ParsedNodes,
}

/// Controlled HTML ingestion or analysis failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HtmlAnalysisError {
    ByteIngestion(TextIngestionError),
    UnsupportedDeclaredEncoding,
    ResourceLimitExceeded(HtmlResourceLimit),
    ParserFailure,
}

impl From<TextIngestionError> for HtmlAnalysisError {
    fn from(error: TextIngestionError) -> Self {
        Self::ByteIngestion(error)
    }
}

impl fmt::Display for HtmlAnalysisError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ByteIngestion(error) => error.fmt(formatter),
            Self::UnsupportedDeclaredEncoding => formatter.write_str(
                "The HTML declares an encoding that TextScope does not support in this phase.",
            ),
            Self::ResourceLimitExceeded(_) => {
                formatter.write_str("The HTML exceeds an explicit TextScope resource-safety limit.")
            }
            Self::ParserFailure => {
                formatter.write_str("The HTML parser could not complete the analysis.")
            }
        }
    }
}

impl Error for HtmlAnalysisError {}
