mod decoder;
mod model;

pub use decoder::analyze_text_bytes;
pub(crate) use decoder::decode_text_bytes;
pub use model::{
    BomKind, DecodeStatus, EncodingEvidence, SourceByteRange, SourceCharacterMapping,
    SourceEncoding, TextDecodeResult, TextIngestionError, TextSourceAnalysis, TextSourceKind,
};
