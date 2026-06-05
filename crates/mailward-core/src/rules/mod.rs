//! The declarative rule engine: data refs, the condition DSL, the classifier
//! and the YAML document loader (with per-account overrides).

mod classifier;
mod data;
mod document;
mod dsl;

pub use classifier::{Rule, RuleClassifier, first_match};
pub use data::RulesData;
pub use document::{RawRule, RawRulesFile, RuleSet, RulesDocument};
pub use dsl::{Condition, Field, LeafCondition, Op, Predicate, compile_condition};

/// Everything that can go wrong while compiling rules from YAML.
#[derive(Debug, thiserror::Error)]
pub enum RuleError {
    #[error("yaml error: {0}")]
    Yaml(#[from] serde_yaml_ng::Error),

    #[error("rules: refs.patterns.{key} is not a valid regex: {source}")]
    BadRegex {
        key: String,
        #[source]
        source: regex::Error,
    },

    #[error("rules: refs.thresholds.{key} is not a valid duration: \"{value}\"")]
    BadDuration { key: String, value: String },

    #[error("rules: unknown ref \"{0}\"")]
    UnknownRef(String),

    #[error("rules: op \"{op}\" needs {expected} field, got \"{field}\"")]
    WrongField {
        op: String,
        expected: String,
        field: String,
    },

    #[error("rules: leaf {{field: {field}, op: {op}}} needs a value or ref")]
    MissingOperand { field: String, op: String },

    #[error("rules: op \"{op}\" needs {expected} operand")]
    OperandType { op: String, expected: String },
}
