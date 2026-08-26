use crate::scan_text;

use super::model::{
    BomKind, DecodeStatus, EncodingEvidence, SourceByteRange, SourceCharacterMapping,
    SourceEncoding, TextDecodeResult, TextIngestionError, TextSourceAnalysis, TextSourceKind,
};

const UTF8_BOM: [u8; 3] = [0xEF, 0xBB, 0xBF];
const UTF16_LE_BOM: [u8; 2] = [0xFF, 0xFE];
const UTF16_BE_BOM: [u8; 2] = [0xFE, 0xFF];
const UTF32_LE_BOM: [u8; 4] = [0xFF, 0xFE, 0x00, 0x00];
const UTF32_BE_BOM: [u8; 4] = [0x00, 0x00, 0xFE, 0xFF];

/// Strictly decodes caller-provided bytes and scans the resulting Unicode text.
///
/// This function performs no filesystem, locale, network, or platform access.
/// Source-byte ranges are zero-based and half-open in the original `bytes`
/// slice; scanner UTF-8 offsets retain their decoded-string meaning.
pub fn analyze_text_bytes(
    bytes: &[u8],
    source_kind: TextSourceKind,
) -> Result<TextSourceAnalysis, TextIngestionError> {
    let decode = decode_text_bytes(bytes)?;
    let scan = scan_text(&decode.decoded_text);

    Ok(TextSourceAnalysis {
        raw_byte_length: bytes.len(),
        source_kind,
        decode,
        scan,
    })
}

pub(crate) fn decode_text_bytes(bytes: &[u8]) -> Result<TextDecodeResult, TextIngestionError> {
    if bytes.is_empty() {
        return decode_utf8(bytes, 0, BomKind::None, EncodingEvidence::EmptyInput);
    }

    if bytes.starts_with(&UTF32_LE_BOM) || bytes.starts_with(&UTF32_BE_BOM) {
        return Err(TextIngestionError::UnsupportedEncoding);
    }

    if bytes.starts_with(&UTF8_BOM) {
        return decode_utf8(
            bytes,
            UTF8_BOM.len(),
            BomKind::Utf8,
            EncodingEvidence::ByteOrderMark,
        );
    }

    if bytes.starts_with(&UTF16_LE_BOM) {
        return decode_utf16(
            bytes,
            UTF16_LE_BOM.len(),
            SourceEncoding::Utf16LittleEndian,
            BomKind::Utf16LittleEndian,
            Endianness::Little,
        );
    }

    if bytes.starts_with(&UTF16_BE_BOM) {
        return decode_utf16(
            bytes,
            UTF16_BE_BOM.len(),
            SourceEncoding::Utf16BigEndian,
            BomKind::Utf16BigEndian,
            Endianness::Big,
        );
    }

    if resembles_bomless_utf16(bytes) {
        return Err(TextIngestionError::AmbiguousEncoding);
    }

    decode_utf8(bytes, 0, BomKind::None, EncodingEvidence::StrictUtf8)
}

fn decode_utf8(
    bytes: &[u8],
    payload_start: usize,
    bom: BomKind,
    encoding_evidence: EncodingEvidence,
) -> Result<TextDecodeResult, TextIngestionError> {
    let payload = &bytes[payload_start..];
    let decoded = std::str::from_utf8(payload).map_err(|_| TextIngestionError::InvalidUtf8)?;
    let mut source_mappings = Vec::with_capacity(decoded.chars().count());
    let mut utf16_offset = 0;

    for (code_point_index, (decoded_utf8_byte_offset, character)) in
        decoded.char_indices().enumerate()
    {
        source_mappings.push(SourceCharacterMapping {
            code_point_index,
            source_byte_range: SourceByteRange {
                start: payload_start + decoded_utf8_byte_offset,
                end: payload_start + decoded_utf8_byte_offset + character.len_utf8(),
            },
            decoded_utf8_byte_offset,
            utf16_offset,
        });
        utf16_offset += character.len_utf16();
    }

    Ok(TextDecodeResult {
        encoding: SourceEncoding::Utf8,
        encoding_evidence,
        bom,
        status: DecodeStatus::Decoded,
        decoded_text: decoded.to_owned(),
        source_mappings,
    })
}

#[derive(Clone, Copy)]
enum Endianness {
    Little,
    Big,
}

fn decode_utf16(
    bytes: &[u8],
    payload_start: usize,
    encoding: SourceEncoding,
    bom: BomKind,
    endianness: Endianness,
) -> Result<TextDecodeResult, TextIngestionError> {
    let payload = &bytes[payload_start..];
    if !payload.len().is_multiple_of(2) {
        return Err(TextIngestionError::InvalidUtf16);
    }

    let (pairs, remainder) = payload.as_chunks::<2>();
    if !remainder.is_empty() {
        return Err(TextIngestionError::InvalidUtf16);
    }
    let units: Vec<u16> = pairs
        .iter()
        .map(|pair| match endianness {
            Endianness::Little => u16::from_le_bytes([pair[0], pair[1]]),
            Endianness::Big => u16::from_be_bytes([pair[0], pair[1]]),
        })
        .collect();
    let mut decoded_text = String::with_capacity(payload.len());
    let mut source_mappings = Vec::with_capacity(units.len());
    let mut unit_index = 0;

    while unit_index < units.len() {
        let first = units[unit_index];
        let source_start = payload_start + (unit_index * 2);
        let decoded_utf8_byte_offset = decoded_text.len();
        let code_point_index = source_mappings.len();

        let (character, consumed_units) = if (0xD800..=0xDBFF).contains(&first) {
            let Some(&second) = units.get(unit_index + 1) else {
                return Err(TextIngestionError::InvalidUtf16);
            };
            if !(0xDC00..=0xDFFF).contains(&second) {
                return Err(TextIngestionError::InvalidUtf16);
            }

            let high = u32::from(first) - 0xD800;
            let low = u32::from(second) - 0xDC00;
            let code_point = 0x1_0000 + ((high << 10) | low);
            let Some(character) = char::from_u32(code_point) else {
                return Err(TextIngestionError::InvalidUtf16);
            };
            (character, 2)
        } else if (0xDC00..=0xDFFF).contains(&first) {
            return Err(TextIngestionError::InvalidUtf16);
        } else {
            let Some(character) = char::from_u32(u32::from(first)) else {
                return Err(TextIngestionError::InvalidUtf16);
            };
            (character, 1)
        };

        decoded_text.push(character);
        source_mappings.push(SourceCharacterMapping {
            code_point_index,
            source_byte_range: SourceByteRange {
                start: source_start,
                end: source_start + (consumed_units * 2),
            },
            decoded_utf8_byte_offset,
            utf16_offset: unit_index,
        });
        unit_index += consumed_units;
    }

    Ok(TextDecodeResult {
        encoding,
        encoding_evidence: EncodingEvidence::ByteOrderMark,
        bom,
        status: DecodeStatus::Decoded,
        decoded_text,
        source_mappings,
    })
}

/// Blocks only a strong alternating-NUL pattern with at least two complete
/// code units. It does not select an endian or decode the bytes.
fn resembles_bomless_utf16(bytes: &[u8]) -> bool {
    if bytes.len() < 4 || !bytes.len().is_multiple_of(2) {
        return false;
    }

    let (pairs, remainder) = bytes.as_chunks::<2>();
    if !remainder.is_empty() {
        return false;
    }
    let little_endian_pattern = pairs
        .iter()
        .all(|pair| is_single_byte_text(pair[0]) && pair[1] == 0);
    let big_endian_pattern = pairs
        .iter()
        .all(|pair| pair[0] == 0 && is_single_byte_text(pair[1]));

    little_endian_pattern || big_endian_pattern
}

const fn is_single_byte_text(byte: u8) -> bool {
    matches!(byte, b'\t' | b'\n' | b'\r' | 0x20..=0xFF)
}
