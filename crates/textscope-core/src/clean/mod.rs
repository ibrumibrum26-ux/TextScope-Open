mod engine;
mod model;
mod registry;

pub use engine::{CleanError, clean_text};
pub use model::{
    CleanProfile, CleanProfileDefinition, CleanRequest, CleanResult, CleanRule, CleanScanSummary,
    ContentPreservationDiff, ContentPreservationStatus, RuleCategory, RuleId, RulePolicy,
    RuleVersion, TechnicalDiff, TextRange, Transformation, TransformationManifest,
    TransformationOperation, VerificationChecks, VerificationReport, VerificationStatus,
};
pub use registry::{available_clean_profiles, tcs_rules, tcs_version};
