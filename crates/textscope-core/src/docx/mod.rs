mod analyzer;
mod model;

pub use analyzer::analyze_docx_bytes;
pub use model::{
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
