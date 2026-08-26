/// A per-scalar record produced by [`crate::scan_text`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CharacterRecord {
    /// Zero-based position measured in Unicode scalar values.
    pub code_point_index: usize,
    /// Zero-based position measured in UTF-16 code units.
    pub utf16_offset: usize,
    /// Zero-based position measured in UTF-8 bytes.
    pub utf8_byte_offset: usize,
    /// The Unicode scalar value as a Rust `char`.
    pub character: char,
    /// The Unicode scalar value as an unambiguous integer code point.
    pub code_point: u32,
    /// A label containing `U+XXXX` and the Unicode name when available.
    pub code_point_label: String,
    /// The exact UTF-8 bytes encoding this scalar value.
    pub utf8_bytes: Vec<u8>,
    /// The two-letter Unicode General Category abbreviation, such as `Lu`.
    pub unicode_category: &'static str,
    /// The full Unicode Script property name, such as `Latin` or `Common`.
    pub script: &'static str,
    /// Whether Rust classifies the scalar as Unicode whitespace.
    pub is_whitespace: bool,
    /// Whether its General Category is `Cc` (Control).
    pub is_control: bool,
    /// Whether its General Category is `Cf` (Format).
    pub is_format: bool,
    /// Whether it is normally non-rendering and merits technical inspection.
    ///
    /// This flag does not mean malicious, erroneous, or removable.
    pub is_invisible: bool,
    /// Whether it is whitespace other than ordinary space, TAB, CR, or LF.
    pub is_unusual_whitespace: bool,
    /// Whether it is a bidirectional control covered by the scanner contract.
    pub is_bidi_control: bool,
    /// Whether it is in a Unicode variation-selector range.
    pub is_variation_selector: bool,
}

/// The kind of conservative technical observation made by the scanner.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FindingKind {
    /// A normally non-rendering character merits inspection.
    InvisibleCharacter,
    /// Whitespace differs from ordinary space, TAB, CR, or LF.
    UnusualWhitespace,
    /// A bidirectional control can affect display order.
    BidirectionalControl,
    /// A variation selector can affect presentation of adjacent content.
    VariationSelector,
}

/// Conservative importance assigned to a finding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FindingSeverity {
    /// An observation that is often legitimate and needs context.
    Informational,
    /// An observation that deserves deliberate review because display may vary.
    Caution,
}

/// A technical observation tied to one Unicode scalar in the input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Finding {
    /// Taxonomy entry for the observation.
    pub kind: FindingKind,
    /// Conservative importance of the observation.
    pub severity: FindingSeverity,
    /// Unicode scalar index of the observed character.
    pub code_point_index: usize,
    /// UTF-16 offset of the observed character.
    pub utf16_offset: usize,
    /// UTF-8 byte offset of the observed character.
    pub utf8_byte_offset: usize,
    /// Unicode scalar value associated with the finding.
    pub code_point: u32,
    /// Neutral explanation that does not claim intent or recommend removal.
    pub description: &'static str,
}

/// Reports whether each Unicode normalization form would alter the input.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NormalizationInspection {
    /// `true` when NFC output would differ from the input.
    pub nfc_would_change: bool,
    /// `true` when NFD output would differ from the input.
    pub nfd_would_change: bool,
    /// `true` when NFKC output would differ from the input.
    pub nfkc_would_change: bool,
    /// `true` when NFKD output would differ from the input.
    pub nfkd_would_change: bool,
}

/// Deterministic result of scanning one valid Rust string slice.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScanResult {
    /// One record per Unicode scalar value, in input order.
    pub characters: Vec<CharacterRecord>,
    /// Conservative observations, in input order.
    pub findings: Vec<Finding>,
    /// Read-only normalization analysis of the original input.
    pub normalization: NormalizationInspection,
}
