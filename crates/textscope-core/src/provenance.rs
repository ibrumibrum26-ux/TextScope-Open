//! Shared, format-neutral provenance observations.
//!
//! These records deliberately distinguish an observation from a claim or a
//! verification result.  The core never turns a stored author, creator,
//! generator, or producer value into an attribution.

/// Kind of provenance or metadata evidence observed by an adapter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceKind {
    MetadataDeclaration,
    GeneratorDeclaration,
    CreatorDeclaration,
    AuthorDeclaration,
    ProducerDeclaration,
    CanonicalDeclaration,
    EncodingDeclaration,
    ClipboardRepresentation,
    Attribution,
}

/// Adapter or source channel that supplied the evidence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceSource {
    HtmlMetadata,
    HtmlGenerator,
    HtmlCanonicalLink,
    HtmlEncoding,
    DocxCoreProperties,
    DocxAppProperties,
    DocxCustomProperties,
    PdfInfo,
    PdfXmp,
    ClipboardPlainText,
    ClipboardHtml,
    ClipboardOtherMime,
}

/// Epistemic category of a record.  Attribution is intentionally unsupported
/// unless an independently verified source is added in a future phase.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceLevel {
    Observation,
    Claim,
    Verification,
    Attribution,
}

/// User-facing status of an evidence record.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VerificationState {
    Observed,
    Declared,
    Verified,
    NotVerified,
    Unsupported,
    Conflict,
}

/// One deterministic evidence record shared by HTML, DOCX, PDF, and clipboard
/// adapters.  `subject` identifies the declared field; `value` is the exact
/// observed value.  No attribution or intent is inferred.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProvenanceEvidence {
    pub kind: EvidenceKind,
    pub source: EvidenceSource,
    pub subject: String,
    pub value: String,
    pub level: EvidenceLevel,
    pub verification: VerificationState,
}

impl ProvenanceEvidence {
    #[must_use]
    pub fn declared(
        kind: EvidenceKind,
        source: EvidenceSource,
        subject: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            source,
            subject: subject.into(),
            value: value.into(),
            level: EvidenceLevel::Observation,
            verification: VerificationState::Declared,
        }
    }

    #[must_use]
    pub fn observed(
        kind: EvidenceKind,
        source: EvidenceSource,
        subject: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            source,
            subject: subject.into(),
            value: value.into(),
            level: EvidenceLevel::Observation,
            verification: VerificationState::Observed,
        }
    }

    #[must_use]
    pub fn conflict(
        kind: EvidenceKind,
        source: EvidenceSource,
        subject: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            source,
            subject: subject.into(),
            value: value.into(),
            level: EvidenceLevel::Observation,
            verification: VerificationState::Conflict,
        }
    }

    #[must_use]
    pub fn unsupported_attribution(subject: impl Into<String>) -> Self {
        Self {
            kind: EvidenceKind::Attribution,
            source: EvidenceSource::ClipboardOtherMime,
            subject: subject.into(),
            value: "No independently verifiable attribution evidence is available.".to_owned(),
            level: EvidenceLevel::Attribution,
            verification: VerificationState::Unsupported,
        }
    }
}
