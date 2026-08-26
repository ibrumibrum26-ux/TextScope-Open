#![forbid(unsafe_code)]
//! Deterministic, platform-independent domain core for TextScope.
//!
//! The core observes Unicode text and applies explicitly versioned cleaning
//! policies. Findings are conservative technical observations and never imply
//! intent or automatic removal.

mod classification;
mod clean;
mod clipboard;
mod docx;
mod html;
mod ingestion;
mod model;
mod normalization;
mod pdf;
mod provenance;
mod reveal;
mod scanner;
mod special;

pub use clean::{
    CleanError, CleanProfile, CleanProfileDefinition, CleanRequest, CleanResult, CleanRule,
    CleanScanSummary, ContentPreservationDiff, ContentPreservationStatus, RuleCategory, RuleId,
    RulePolicy, RuleVersion, TechnicalDiff, TextRange, Transformation, TransformationManifest,
    TransformationOperation, VerificationChecks, VerificationReport, VerificationStatus,
    available_clean_profiles, clean_text, tcs_rules, tcs_version,
};
pub use clipboard::{
    ClipboardAnalysis, ClipboardComparisonState, ClipboardFinding, ClipboardFindingKind,
    ClipboardMetadata, ClipboardRepresentation, ClipboardRepresentationComparison,
    ClipboardRepresentationKind, MAX_CLIPBOARD_BYTES, compare_clipboard_representations,
};
pub use docx::{
    DocxAnalysis, DocxAnalysisError, DocxByteRange, DocxComment, DocxContentTypeDeclaration,
    DocxContentTypeDeclarationKind, DocxCrcStatus, DocxDocumentSummary, DocxFinding,
    DocxFindingKind, DocxFindingSeverity, DocxHyperlink, DocxMetadata, DocxMetadataProperty,
    DocxPackageStatus, DocxPackageSummary, DocxPartClassification, DocxPartRecord,
    DocxPartTextChannel, DocxRelationship, DocxResourceLimit, DocxRevision, DocxRevisionKind,
    DocxSourceLocation, DocxTextChannel, DocxTextChannels, DocxTextKind, DocxTextSegment,
    MAX_DOCX_COMPRESSION_RATIO, MAX_DOCX_ENTRY_COUNT, MAX_DOCX_ENTRY_NAME_BYTES,
    MAX_DOCX_PART_BYTES, MAX_DOCX_RELATIONSHIPS, MAX_DOCX_SOURCE_BYTES, MAX_DOCX_TEXT_SEGMENTS,
    MAX_DOCX_TOTAL_UNCOMPRESSED_BYTES, MAX_DOCX_XML_DEPTH, MAX_DOCX_XML_NODES,
    MAX_DOCX_XML_PART_BYTES, analyze_docx_bytes,
};
pub use html::{
    DecodedSourceRange, HtmlAnalysis, HtmlAnalysisError, HtmlAttributeRecord, HtmlCommentRecord,
    HtmlDoctypeRecord, HtmlDocumentSummary, HtmlElementRecord, HtmlEncodingDeclaration,
    HtmlEncodingDeclarationKind, HtmlEncodingEvidence, HtmlExternalReference, HtmlFinding,
    HtmlFindingKind, HtmlFindingSeverity, HtmlMetaDeclaration, HtmlMetadata, HtmlParserLayer,
    HtmlParserRecovery, HtmlReferenceKind, HtmlResourceLimit, HtmlSourceRange, HtmlTextChannels,
    HtmlTextClassification, HtmlTextSegment, MAX_HTML_ATTRIBUTE_VALUE_BYTES, MAX_HTML_ATTRIBUTES,
    MAX_HTML_COMMENT_BYTES, MAX_HTML_COMMENTS, MAX_HTML_NESTING_DEPTH, MAX_HTML_PARSE_RECOVERIES,
    MAX_HTML_SOURCE_BYTES, MAX_HTML_SOURCE_ELEMENTS, MAX_HTML_TEXT_TOKEN_BYTES, MAX_HTML_TOKENS,
    analyze_html_bytes,
};
pub use ingestion::{
    BomKind, DecodeStatus, EncodingEvidence, SourceByteRange, SourceCharacterMapping,
    SourceEncoding, TextDecodeResult, TextIngestionError, TextSourceAnalysis, TextSourceKind,
    analyze_text_bytes,
};
pub use model::{
    CharacterRecord, Finding, FindingKind, FindingSeverity, NormalizationInspection, ScanResult,
};
pub use pdf::{
    MAX_PDF_ACTIONS, MAX_PDF_ANNOTATIONS, MAX_PDF_CONTENT_OPERATIONS, MAX_PDF_DECODED_STREAM_BYTES,
    MAX_PDF_DECOMPRESSION_RATIO, MAX_PDF_EMBEDDED_FILES, MAX_PDF_FINDINGS,
    MAX_PDF_FONT_STREAM_BYTES, MAX_PDF_INDIVIDUAL_STREAM_BYTES, MAX_PDF_METADATA_BYTES,
    MAX_PDF_OBJECTS, MAX_PDF_PAGES, MAX_PDF_REFERENCE_DEPTH, MAX_PDF_SOURCE_BYTES,
    MAX_PDF_TEXT_SEGMENTS, PdfActionKind, PdfActionRecord, PdfAnalysis, PdfAnalysisError,
    PdfAnnotationRecord, PdfBox, PdfDocumentSummary, PdfEmbeddedFileRecord, PdfEncryptionStatus,
    PdfExternalReference, PdfExternalReferenceKind, PdfFinding, PdfFindingKind, PdfFindingSeverity,
    PdfFontRecord, PdfFormSummary, PdfImageRecord, PdfMetadata, PdfMetadataConflict,
    PdfMetadataEntry, PdfObjectSummary, PdfPageSummary, PdfResourceLimit, PdfRevisionSummary,
    PdfSignatureRecord, PdfSourceReference, PdfTextExtractionStatus, PdfTextSegment, PdfVersion,
    PdfXmpMetadata, PdfXrefKind, analyze_pdf_bytes,
};
pub use provenance::{
    EvidenceKind, EvidenceLevel, EvidenceSource, ProvenanceEvidence, VerificationState,
};
pub use reveal::reveal_text;
pub use scanner::scan_text;
