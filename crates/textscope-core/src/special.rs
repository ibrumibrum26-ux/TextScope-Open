pub(crate) fn special_name(character: char) -> Option<&'static str> {
    match character {
        '\u{00AD}' => Some("SOFT HYPHEN"),
        '\u{00A0}' => Some("NO-BREAK SPACE"),
        '\u{061C}' => Some("ARABIC LETTER MARK"),
        '\u{200B}' => Some("ZERO WIDTH SPACE"),
        '\u{200C}' => Some("ZERO WIDTH NON-JOINER"),
        '\u{200D}' => Some("ZERO WIDTH JOINER"),
        '\u{200E}' => Some("LEFT-TO-RIGHT MARK"),
        '\u{200F}' => Some("RIGHT-TO-LEFT MARK"),
        '\u{202A}' => Some("LEFT-TO-RIGHT EMBEDDING"),
        '\u{202B}' => Some("RIGHT-TO-LEFT EMBEDDING"),
        '\u{202C}' => Some("POP DIRECTIONAL FORMATTING"),
        '\u{202D}' => Some("LEFT-TO-RIGHT OVERRIDE"),
        '\u{202E}' => Some("RIGHT-TO-LEFT OVERRIDE"),
        '\u{2060}' => Some("WORD JOINER"),
        '\u{2066}' => Some("LEFT-TO-RIGHT ISOLATE"),
        '\u{2067}' => Some("RIGHT-TO-LEFT ISOLATE"),
        '\u{2068}' => Some("FIRST STRONG ISOLATE"),
        '\u{2069}' => Some("POP DIRECTIONAL ISOLATE"),
        '\u{FEFF}' => Some("ZERO WIDTH NO-BREAK SPACE"),
        _ => None,
    }
}

pub(crate) fn specific_reveal(character: char) -> Option<&'static str> {
    match character {
        '\t' => Some("⟦TAB⟧"),
        '\n' => Some("⟦LF⟧"),
        '\r' => Some("⟦CR⟧"),
        '\u{00AD}' => Some("⟦SHY⟧"),
        '\u{00A0}' => Some("⟦NBSP⟧"),
        '\u{061C}' => Some("⟦ALM⟧"),
        '\u{200B}' => Some("⟦ZWSP⟧"),
        '\u{200C}' => Some("⟦ZWNJ⟧"),
        '\u{200D}' => Some("⟦ZWJ⟧"),
        '\u{200E}' => Some("⟦LRM⟧"),
        '\u{200F}' => Some("⟦RLM⟧"),
        '\u{202A}' => Some("⟦LRE⟧"),
        '\u{202B}' => Some("⟦RLE⟧"),
        '\u{202C}' => Some("⟦PDF⟧"),
        '\u{202D}' => Some("⟦LRO⟧"),
        '\u{202E}' => Some("⟦RLO⟧"),
        '\u{2060}' => Some("⟦WJ⟧"),
        '\u{2066}' => Some("⟦LRI⟧"),
        '\u{2067}' => Some("⟦RLI⟧"),
        '\u{2068}' => Some("⟦FSI⟧"),
        '\u{2069}' => Some("⟦PDI⟧"),
        '\u{FEFF}' => Some("⟦ZWNBSP/BOM⟧"),
        _ => None,
    }
}

pub(crate) fn is_bidi_control(character: char) -> bool {
    matches!(
        character,
        '\u{061C}'
            | '\u{200E}'
            | '\u{200F}'
            | '\u{202A}'
            | '\u{202B}'
            | '\u{202C}'
            | '\u{202D}'
            | '\u{202E}'
            | '\u{2066}'
            | '\u{2067}'
            | '\u{2068}'
            | '\u{2069}'
    )
}

pub(crate) fn is_variation_selector(character: char) -> bool {
    matches!(character as u32, 0xFE00..=0xFE0F | 0xE0100..=0xE01EF)
}

pub(crate) fn is_join_control(character: char) -> bool {
    matches!(character, '\u{200C}' | '\u{200D}')
}
