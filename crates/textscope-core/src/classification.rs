use unicode_properties::{GeneralCategory, UnicodeGeneralCategory};
use unicode_script::UnicodeScript;

use crate::special::{is_bidi_control, is_variation_selector, special_name};

pub(crate) struct Classification {
    pub(crate) unicode_category: &'static str,
    pub(crate) script: &'static str,
    pub(crate) is_whitespace: bool,
    pub(crate) is_control: bool,
    pub(crate) is_format: bool,
    pub(crate) is_invisible: bool,
    pub(crate) is_unusual_whitespace: bool,
    pub(crate) is_bidi_control: bool,
    pub(crate) is_variation_selector: bool,
}

pub(crate) fn classify_character(character: char) -> Classification {
    let category = character.general_category();
    let is_control = category == GeneralCategory::Control;
    let is_format = category == GeneralCategory::Format;
    let is_variation_selector = is_variation_selector(character);
    let is_whitespace = character.is_whitespace();
    let is_unusual_whitespace = is_whitespace && !matches!(character, ' ' | '\t' | '\r' | '\n');
    let expected_text_control = matches!(character, '\t' | '\r' | '\n');
    let is_invisible = is_format || is_variation_selector || (is_control && !expected_text_control);

    Classification {
        unicode_category: category_abbreviation(category),
        script: character.script().full_name(),
        is_whitespace,
        is_control,
        is_format,
        is_invisible,
        is_unusual_whitespace,
        is_bidi_control: is_bidi_control(character),
        is_variation_selector,
    }
}

pub(crate) fn code_point_label(character: char) -> String {
    let name = special_name(character)
        .map(str::to_owned)
        .or_else(|| unicode_names2::name(character).map(|name| name.to_string()));

    match name {
        Some(name) => format!("U+{:04X} {name}", character as u32),
        None => format!("U+{:04X}", character as u32),
    }
}

fn category_abbreviation(category: GeneralCategory) -> &'static str {
    match category {
        GeneralCategory::UppercaseLetter => "Lu",
        GeneralCategory::LowercaseLetter => "Ll",
        GeneralCategory::TitlecaseLetter => "Lt",
        GeneralCategory::ModifierLetter => "Lm",
        GeneralCategory::OtherLetter => "Lo",
        GeneralCategory::NonspacingMark => "Mn",
        GeneralCategory::SpacingMark => "Mc",
        GeneralCategory::EnclosingMark => "Me",
        GeneralCategory::DecimalNumber => "Nd",
        GeneralCategory::LetterNumber => "Nl",
        GeneralCategory::OtherNumber => "No",
        GeneralCategory::ConnectorPunctuation => "Pc",
        GeneralCategory::DashPunctuation => "Pd",
        GeneralCategory::OpenPunctuation => "Ps",
        GeneralCategory::ClosePunctuation => "Pe",
        GeneralCategory::InitialPunctuation => "Pi",
        GeneralCategory::FinalPunctuation => "Pf",
        GeneralCategory::OtherPunctuation => "Po",
        GeneralCategory::MathSymbol => "Sm",
        GeneralCategory::CurrencySymbol => "Sc",
        GeneralCategory::ModifierSymbol => "Sk",
        GeneralCategory::OtherSymbol => "So",
        GeneralCategory::SpaceSeparator => "Zs",
        GeneralCategory::LineSeparator => "Zl",
        GeneralCategory::ParagraphSeparator => "Zp",
        GeneralCategory::Control => "Cc",
        GeneralCategory::Format => "Cf",
        GeneralCategory::Surrogate => "Cs",
        GeneralCategory::PrivateUse => "Co",
        GeneralCategory::Unassigned => "Cn",
    }
}
