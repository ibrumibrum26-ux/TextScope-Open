use crate::{classification::classify_character, special::specific_reveal};

/// Produces a deterministic inspection representation without changing input.
///
/// Normal visible characters are copied. Characters that are normally hidden,
/// control display, or use unusual whitespace are replaced with explicit
/// placeholders in the returned representation.
#[must_use]
pub fn reveal_text(text: &str) -> String {
    let mut revealed = String::with_capacity(text.len());

    for character in text.chars() {
        if let Some(placeholder) = specific_reveal(character) {
            revealed.push_str(placeholder);
            continue;
        }

        let classification = classify_character(character);
        if classification.is_invisible
            || classification.is_control
            || classification.is_unusual_whitespace
        {
            revealed.push_str(&format!("⟦U+{:04X}⟧", character as u32));
        } else {
            revealed.push(character);
        }
    }

    revealed
}
