use super::model::{
    CleanProfile, CleanProfileDefinition, CleanRule, RuleCategory, RuleId, RulePolicy, RuleVersion,
    TransformationOperation,
};

const TCS_VERSION: &str = "0.1.0";
const RULE_VERSION: RuleVersion = RuleVersion::new("0.1.0");

const RULES: [CleanRule; 4] = [
    CleanRule {
        id: RuleId::UnicodeNfc,
        version: RULE_VERSION,
        title: "Unicode NFC normalization",
        description: "Replace canonically equivalent sequences with their deterministic NFC form.",
        category: RuleCategory::Normalization,
        default_policy: RulePolicy::Preserve,
        operation: TransformationOperation::NormalizeNfc,
    },
    CleanRule {
        id: RuleId::LineEndings,
        version: RULE_VERSION,
        title: "Canonical LF line endings",
        description: "Represent CRLF and lone CR line breaks as LF without changing line count.",
        category: RuleCategory::LineEndings,
        default_policy: RulePolicy::Apply,
        operation: TransformationOperation::CanonicalizeLineEnding,
    },
    CleanRule {
        id: RuleId::LeadingByteOrderMark,
        version: RULE_VERSION,
        title: "Unambiguous initial byte-order marker removal",
        description: "Remove one U+FEFF at absolute input start, but preserve repeated leading U+FEFF as ambiguous.",
        category: RuleCategory::Control,
        default_policy: RulePolicy::Preserve,
        operation: TransformationOperation::RemoveLeadingByteOrderMark,
    },
    CleanRule {
        id: RuleId::NonBreakingSpace,
        version: RULE_VERSION,
        title: "Plain-text NBSP replacement",
        description: "Replace U+00A0 with U+0020 only in the intervention-oriented plain-text profile.",
        category: RuleCategory::Whitespace,
        default_policy: RulePolicy::Preserve,
        operation: TransformationOperation::ReplaceNonBreakingSpace,
    },
];

const SAFE_RULES: [RuleId; 1] = [RuleId::LineEndings];
const CANONICAL_RULES: [RuleId; 3] = [
    RuleId::UnicodeNfc,
    RuleId::LineEndings,
    RuleId::LeadingByteOrderMark,
];
const PLAIN_TEXT_RULES: [RuleId; 4] = [
    RuleId::UnicodeNfc,
    RuleId::LineEndings,
    RuleId::LeadingByteOrderMark,
    RuleId::NonBreakingSpace,
];
const NO_RULES: [RuleId; 0] = [];

const PROFILES: [CleanProfileDefinition; 5] = [
    CleanProfileDefinition {
        profile: CleanProfile::SafeClean,
        label: "Safe Clean",
        description: "Preserves ambiguous Unicode and only canonicalizes line-ending representation.",
        supported: true,
        active_rules: &SAFE_RULES,
    },
    CleanProfileDefinition {
        profile: CleanProfile::CanonicalClean,
        label: "Canonical Clean",
        description: "Applies NFC, LF line endings, and removal of one unambiguous leading U+FEFF marker.",
        supported: true,
        active_rules: &CANONICAL_RULES,
    },
    CleanProfileDefinition {
        profile: CleanProfile::PurePlainText,
        label: "Pure Plain Text",
        description: "Adds explicit NBSP-to-space replacement and may require content review.",
        supported: true,
        active_rules: &PLAIN_TEXT_RULES,
    },
    CleanProfileDefinition {
        profile: CleanProfile::DocumentClean,
        label: "Document Clean",
        description: "Reserved until document context and metadata rules are specified.",
        supported: false,
        active_rules: &NO_RULES,
    },
    CleanProfileDefinition {
        profile: CleanProfile::DeveloperClean,
        label: "Developer Clean",
        description: "Reserved until source-code-specific preservation rules are specified.",
        supported: false,
        active_rules: &NO_RULES,
    },
];

/// Returns the formal TextScope Clean Standard version.
#[must_use]
pub const fn tcs_version() -> &'static str {
    TCS_VERSION
}

/// Returns the deterministic TCS 0.1.0 rule registry.
#[must_use]
pub const fn tcs_rules() -> &'static [CleanRule] {
    &RULES
}

/// Returns supported and explicitly reserved profile metadata.
#[must_use]
pub const fn available_clean_profiles() -> &'static [CleanProfileDefinition] {
    &PROFILES
}

pub(crate) fn profile_definition(profile: CleanProfile) -> &'static CleanProfileDefinition {
    PROFILES
        .iter()
        .find(|definition| definition.profile == profile)
        .expect("every CleanProfile variant has static metadata")
}

pub(crate) fn rule_definition(rule_id: RuleId) -> &'static CleanRule {
    RULES
        .iter()
        .find(|rule| rule.id == rule_id)
        .expect("every internal rule identifier has static metadata")
}

pub(crate) fn profile_uses_rule(profile: CleanProfile, rule_id: RuleId) -> bool {
    profile_definition(profile).active_rules.contains(&rule_id)
}
