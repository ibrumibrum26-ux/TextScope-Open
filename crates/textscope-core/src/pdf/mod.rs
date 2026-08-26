mod analyzer;
mod model;

pub use analyzer::analyze_pdf_bytes;
pub use model::{
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
    PdfXmpMetadata, PdfXrefKind,
};
