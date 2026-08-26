//! Deterministic comparison rules for browser clipboard representations.

/// Maximum total clipboard bytes accepted by the UI before crossing WASM.
pub const MAX_CLIPBOARD_BYTES: usize = 10 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClipboardRepresentationKind {
    PlainText,
    Html,
    OtherMime,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClipboardRepresentation {
    pub kind: ClipboardRepresentationKind,
    pub mime_type: String,
    pub byte_length: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClipboardMetadata {
    pub advertised_types: Vec<String>,
    pub representation_order: Vec<ClipboardRepresentationKind>,
    pub plain_text_present: bool,
    pub html_present: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClipboardFindingKind {
    UnsupportedMime,
    MissingReadableRepresentation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClipboardFinding {
    pub kind: ClipboardFindingKind,
    pub message: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClipboardComparisonState {
    Equivalent,
    TechnicallyDifferent,
    ContentDifferent,
    NeedsReview,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClipboardRepresentationComparison {
    pub state: ClipboardComparisonState,
    pub plain_byte_length: Option<usize>,
    pub html_text_byte_length: Option<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClipboardAnalysis {
    pub representations: Vec<ClipboardRepresentation>,
    pub metadata: ClipboardMetadata,
    pub findings: Vec<ClipboardFinding>,
    pub comparison: ClipboardRepresentationComparison,
}

/// Compares the plain-text representation with text extracted from HTML.
/// Exact equality is equivalent; a difference only in line-ending spelling is
/// technical; all other deterministic differences are content differences.
/// A missing side is retained as a review state rather than silently discarded.
#[must_use]
pub fn compare_clipboard_representations(
    plain_text: Option<&str>,
    html_text: Option<&str>,
) -> ClipboardRepresentationComparison {
    let state = match (plain_text, html_text) {
        (Some(plain), Some(html)) if plain == html => ClipboardComparisonState::Equivalent,
        (Some(plain), Some(html))
            if normalize_line_endings(plain) == normalize_line_endings(html) =>
        {
            ClipboardComparisonState::TechnicallyDifferent
        }
        (Some(_), Some(_)) => ClipboardComparisonState::ContentDifferent,
        _ => ClipboardComparisonState::NeedsReview,
    };
    ClipboardRepresentationComparison {
        state,
        plain_byte_length: plain_text.map(str::len),
        html_text_byte_length: html_text.map(str::len),
    }
}

fn normalize_line_endings(value: &str) -> String {
    value.replace("\r\n", "\n").replace('\r', "\n")
}

#[cfg(test)]
mod tests {
    use super::{ClipboardComparisonState, compare_clipboard_representations};

    #[test]
    fn comparison_is_deterministic_and_content_preserving() {
        assert_eq!(
            compare_clipboard_representations(Some("A😀B"), Some("A😀B")).state,
            ClipboardComparisonState::Equivalent
        );
        assert_eq!(
            compare_clipboard_representations(Some("A\r\nB"), Some("A\nB")).state,
            ClipboardComparisonState::TechnicallyDifferent
        );
        assert_eq!(
            compare_clipboard_representations(Some("A"), Some("B")).state,
            ClipboardComparisonState::ContentDifferent
        );
        assert_eq!(
            compare_clipboard_representations(Some("A"), None).state,
            ClipboardComparisonState::NeedsReview
        );
    }
}
