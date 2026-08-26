use std::{error::Error, fmt};

use crate::ScanResult;

/// Literal-text source adapters supported by the byte-ingestion layer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextSourceKind {
    PlainText,
    Markdown,
}

impl TextSourceKind {
    /// Stable identifier used by interoperability boundaries.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PlainText => "plain_text",
            Self::Markdown => "markdown",
        }
    }

    /// Parses a stable source-kind identifier without consulting file content.
    #[must_use]
    pub fn from_id(value: &str) -> Option<Self> {
        match value {
            "plain_text" => Some(Self::PlainText),
            "markdown" => Some(Self::Markdown),
            _ => None,
        }
    }
}

/// Character encoding selected from deterministic evidence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceEncoding {
    Utf8,
    Utf16LittleEndian,
    Utf16BigEndian,
}

/// Evidence that authorized the encoding decision.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EncodingEvidence {
    /// Zero bytes are defined as empty UTF-8 text.
    EmptyInput,
    /// The complete BOM-less byte sequence passed strict UTF-8 validation.
    StrictUtf8,
    /// A recognized byte-order mark selected the encoding.
    ByteOrderMark,
}

/// Recognized encoding signature at the beginning of the source bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BomKind {
    None,
    Utf8,
    Utf16LittleEndian,
    Utf16BigEndian,
}

/// Status of a successful strict decode.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecodeStatus {
    Decoded,
}

/// Zero-based, half-open range in the original source-byte coordinate space.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceByteRange {
    pub start: usize,
    pub end: usize,
}

/// Correlates one decoded Unicode scalar with its original source bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceCharacterMapping {
    pub code_point_index: usize,
    pub source_byte_range: SourceByteRange,
    /// Offset in the decoded Rust string's UTF-8 representation.
    pub decoded_utf8_byte_offset: usize,
    /// Offset in UTF-16 code units of the decoded Unicode text.
    pub utf16_offset: usize,
}

/// Strict decoding result and its source-byte provenance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextDecodeResult {
    pub encoding: SourceEncoding,
    pub encoding_evidence: EncodingEvidence,
    pub bom: BomKind,
    pub status: DecodeStatus,
    pub decoded_text: String,
    pub source_mappings: Vec<SourceCharacterMapping>,
}

/// Complete analysis of caller-provided bytes without filesystem access.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextSourceAnalysis {
    pub raw_byte_length: usize,
    pub source_kind: TextSourceKind,
    pub decode: TextDecodeResult,
    pub scan: ScanResult,
}

/// Controlled failures produced by strict byte ingestion.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextIngestionError {
    /// A recognized signature identifies an encoding outside the current scope.
    UnsupportedEncoding,
    /// A strong BOM-less UTF-16 pattern prevents a safe UTF-8 interpretation.
    AmbiguousEncoding,
    /// The bytes selected for UTF-8 decoding are not strictly valid UTF-8.
    InvalidUtf8,
    /// BOM-selected UTF-16 contains invalid length or surrogate structure.
    InvalidUtf16,
}

impl fmt::Display for TextIngestionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedEncoding => formatter.write_str(
                "The source has evidence of an encoding that TextScope does not support yet.",
            ),
            Self::AmbiguousEncoding => formatter.write_str(
                "The source resembles BOM-less UTF-16 and cannot be decoded without guessing.",
            ),
            Self::InvalidUtf8 => formatter.write_str("The source is not strictly valid UTF-8."),
            Self::InvalidUtf16 => formatter.write_str("The source contains invalid UTF-16."),
        }
    }
}

impl Error for TextIngestionError {}
