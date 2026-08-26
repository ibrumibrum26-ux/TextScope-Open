mod analyzer;
mod model;

pub use analyzer::analyze_html_bytes;
pub use model::{
    DecodedSourceRange, HtmlAnalysis, HtmlAnalysisError, HtmlAttributeRecord, HtmlCommentRecord,
    HtmlDoctypeRecord, HtmlDocumentSummary, HtmlElementRecord, HtmlEncodingDeclaration,
    HtmlEncodingDeclarationKind, HtmlEncodingEvidence, HtmlExternalReference, HtmlFinding,
    HtmlFindingKind, HtmlFindingSeverity, HtmlMetaDeclaration, HtmlMetadata, HtmlParserLayer,
    HtmlParserRecovery, HtmlReferenceKind, HtmlResourceLimit, HtmlSourceRange, HtmlTextChannels,
    HtmlTextClassification, HtmlTextSegment, MAX_HTML_ATTRIBUTE_VALUE_BYTES, MAX_HTML_ATTRIBUTES,
    MAX_HTML_COMMENT_BYTES, MAX_HTML_COMMENTS, MAX_HTML_NESTING_DEPTH, MAX_HTML_PARSE_RECOVERIES,
    MAX_HTML_SOURCE_BYTES, MAX_HTML_SOURCE_ELEMENTS, MAX_HTML_TEXT_TOKEN_BYTES, MAX_HTML_TOKENS,
};
