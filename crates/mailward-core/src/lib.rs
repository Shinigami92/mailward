//! `mailward-core` - the engine shared by every front-end.
//!
//! This crate owns the provider-neutral pieces: the message model, the
//! declarative rule DSL + interpreter, the YAML config model and (in later
//! phases) the IMAP backend and vendor auth strategies.
//!
//! The rule engine is a faithful port of the v1 TypeScript implementation:
//! first-match-wins classification over a small condition DSL whose data
//! (sender lists, regex patterns, age thresholds) lives in `rules.yaml` under a
//! `refs:` section. One deliberate change: a message carries its `age_hours`
//! directly (computed by the IMAP layer) rather than reading a clock inside the
//! DSL, so the engine is pure and its decisions are deterministic.

#![forbid(unsafe_code)]

pub mod auth;
pub mod config;
pub mod deep_merge;
pub mod imap;
pub mod message;
pub mod rules;

pub use config::{
    AuthMethod, ConfigDocument, ConfigError, DefaultsConfig, FileConfig, FoldersConfig, ImapConfig,
    ResolvedAccount, VENDOR_NAMES, VendorDefaults, get_vendor, load_accounts,
};
pub use deep_merge::deep_merge;
pub use message::{Classifier, Decision, MailMessage, Verdict, folder_name};
pub use rules::{
    Condition, Field, Op, Rule, RuleClassifier, RuleError, RuleSet, RulesDocument, first_match,
};

/// Crate version, surfaced by the API for diagnostics.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_reported() {
        assert!(!VERSION.is_empty());
    }
}
