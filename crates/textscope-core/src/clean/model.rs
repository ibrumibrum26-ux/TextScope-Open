use crate::Finding;

/// Stable identifier for one TextScope Clean Standard rule.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum RuleId {
    /// Canonically normalize text to Unicode NFC.
    UnicodeNfc,
    /// Canonicalize CRLF and lone CR line endings to LF.
    LineEndings,
    /// Remove one leading U+FEFF unless another U+FEFF makes it ambiguous.
    LeadingByteOrderMark,
    /// Replace U+00A0 with U+0020 in the intervention-oriented profile.
    NonBreakingSpace,
}

impl RuleId {
    /// Returns the stable, human-readable TCS identifier.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UnicodeNfc => "TCS-NORMALIZATION-001",
            Self::LineEndings => "TCS-LINE-ENDINGS-001",
            Self::LeadingByteOrderMark => "TCS-CONTROL-001",
            Self::NonBreakingSpace => "TCS-WHITESPACE-001",
        }
    }
}

/// SemVer identifier for one rule definition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuleVersion(&'static str);

impl RuleVersion {
    pub(crate) const fn new(value: &'static str) -> Self {
        Self(value)
    }

    /// Returns the rule's SemVer identifier.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

/// High-level category used to organize TCS rules.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuleCategory {
    /// Unicode normalization rule.
    Normalization,
    /// Line-ending representation rule.
    LineEndings,
    /// Technical control or marker rule.
    Control,
    /// Whitespace representation rule.
    Whitespace,
}

/// Conservative default when no explicit profile enables a rule.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RulePolicy {
    /// Apply the transformation.
    Apply,
    /// Preserve the original representation.
    Preserve,
}

/// Deterministic operation performed by a TCS rule.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransformationOperation {
    /// Normalize a changed source range to NFC.
    NormalizeNfc,
    /// Replace CRLF or CR with LF.
    CanonicalizeLineEnding,
    /// Remove an initial U+FEFF technical marker.
    RemoveLeadingByteOrderMark,
    /// Replace NBSP with an ordinary space.
    ReplaceNonBreakingSpace,
}

/// Immutable metadata for one rule in the deterministic registry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CleanRule {
    pub id: RuleId,
    pub version: RuleVersion,
    pub title: &'static str,
    pub description: &'static str,
    pub category: RuleCategory,
    pub default_policy: RulePolicy,
    pub operation: TransformationOperation,
}

/// Cleaning profiles recognized by TCS 0.1.0.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CleanProfile {
    SafeClean,
    CanonicalClean,
    PurePlainText,
    DocumentClean,
    DeveloperClean,
}

impl CleanProfile {
    /// Stable profile identifier used across the Rust/WASM boundary.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SafeClean => "safe_clean",
            Self::CanonicalClean => "canonical_clean",
            Self::PurePlainText => "pure_plain_text",
            Self::DocumentClean => "document_clean",
            Self::DeveloperClean => "developer_clean",
        }
    }

    /// Parses a stable profile identifier without locale-sensitive behavior.
    #[must_use]
    pub fn from_id(value: &str) -> Option<Self> {
        match value {
            "safe_clean" => Some(Self::SafeClean),
            "canonical_clean" => Some(Self::CanonicalClean),
            "pure_plain_text" => Some(Self::PurePlainText),
            "document_clean" => Some(Self::DocumentClean),
            "developer_clean" => Some(Self::DeveloperClean),
            _ => None,
        }
    }
}

/// Public metadata for an available or reserved clean profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CleanProfileDefinition {
    pub profile: CleanProfile,
    pub label: &'static str,
    pub description: &'static str,
    pub supported: bool,
    pub active_rules: &'static [RuleId],
}

/// Explicit request model for one local pasted-text clean operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CleanRequest<'a> {
    pub text: &'a str,
    pub profile: CleanProfile,
}

/// Half-open range whose coordinate space is identified by its field name.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TextRange {
    pub start: usize,
    pub end: usize,
}

/// One declared source-to-replacement operation, measured against original input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Transformation {
    pub sequence: usize,
    pub rule_id: RuleId,
    pub rule_version: RuleVersion,
    pub operation: TransformationOperation,
    /// Half-open range in Unicode scalar indexes of the original input.
    pub original_code_point_range: TextRange,
    /// Half-open range in UTF-8 bytes of the original input.
    pub original_utf8_byte_range: TextRange,
    pub replacement: String,
    pub reason: &'static str,
    pub before: String,
    pub after: String,
}

/// Complete, ordered audit log for transformations actually applied.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransformationManifest {
    pub tcs_version: &'static str,
    pub profile: CleanProfile,
    pub transformations: Vec<Transformation>,
}

/// Mechanical difference between original and clean representations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TechnicalDiff {
    pub changed: bool,
    pub original_utf8_bytes: usize,
    pub clean_utf8_bytes: usize,
    pub original_code_points: usize,
    pub clean_code_points: usize,
    pub transformation_count: usize,
}

/// Conservative assessment of whether transformations preserved content.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContentPreservationStatus {
    Preserved,
    NeedsReview,
    Unknown,
}

/// Content-focused assessment kept separate from the mechanical diff.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContentPreservationDiff {
    pub status: ContentPreservationStatus,
    pub reason: &'static str,
}

/// Compact result of a real scanner invocation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CleanScanSummary {
    pub characters: usize,
    pub findings: usize,
    pub invisible: usize,
    pub unusual_whitespace: usize,
    pub bidi_controls: usize,
    pub variation_selectors: usize,
}

/// Individually auditable verification checks.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VerificationChecks {
    pub manifest_reproduces_clean_text: bool,
    pub rules_allowed_by_profile: bool,
    pub no_undeclared_transformations: bool,
    pub clean_rescan_completed: bool,
    pub expected_findings_reassessed: bool,
    pub preserved_findings_visible: bool,
    pub deterministic: bool,
    pub idempotent: bool,
}

impl VerificationChecks {
    #[must_use]
    pub(crate) const fn all_passed(self) -> bool {
        self.manifest_reproduces_clean_text
            && self.rules_allowed_by_profile
            && self.no_undeclared_transformations
            && self.clean_rescan_completed
            && self.expected_findings_reassessed
            && self.preserved_findings_visible
            && self.deterministic
            && self.idempotent
    }
}

/// Overall status after independent rescanning and policy verification.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VerificationStatus {
    Verified,
    VerifiedWithRemainingFindings,
    NeedsReview,
    Failed,
}

/// Auditable result of the independent rescan and verification pipeline.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationReport {
    pub original_scan_summary: CleanScanSummary,
    pub clean_scan_summary: CleanScanSummary,
    pub applied_transformations: usize,
    pub remaining_findings: Vec<Finding>,
    pub content_preservation_status: ContentPreservationStatus,
    pub verification_status: VerificationStatus,
    pub checks: VerificationChecks,
}

/// Complete deterministic output for one pasted-text clean request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CleanResult {
    pub original_text: String,
    pub clean_text: String,
    pub profile: CleanProfile,
    pub tcs_version: &'static str,
    pub manifest: TransformationManifest,
    pub technical_diff: TechnicalDiff,
    pub content_preservation_diff: ContentPreservationDiff,
    pub verification: VerificationReport,
}
