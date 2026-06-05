//! The `rules.yaml` document: shared base rules plus optional per-account
//! overrides, compiled into ready-to-run [`RuleSet`]s.

use std::collections::HashMap;

use regex::Regex;
use serde::Deserialize;
use serde_yaml_ng::Value;

use super::RuleError;
use super::classifier::Rule;
use super::data::{RawRefs, RulesData};
use super::dsl::{Condition, compile_condition};
use crate::deep_merge::deep_merge;
use crate::message::Decision;

/// One declarative rule entry: `{ name, when, then }`.
#[derive(Debug, Clone, Deserialize)]
pub struct RawRule {
    pub name: String,
    pub when: Condition,
    pub then: Decision,
}

/// The base shape of a rules file (the `accounts:` map is split off before this).
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct RawRulesFile {
    pub refs: RawRefs,
    pub classify: Vec<RawRule>,
    pub cleanup: Vec<RawRule>,
}

/// A compiled, ready-to-run rule bundle for one account (or the shared base).
pub struct RuleSet {
    /// Unread-path rules.
    pub classify: Vec<Rule>,
    /// `--cleanup` prune rules for already-read mail.
    pub cleanup: Vec<Rule>,
    /// The compiled `expiry` pattern, if defined (for the cleanup "expiry" column).
    pub expiry_pattern: Option<Regex>,
}

impl RuleSet {
    /// Compiles a parsed rules file into executable rules.
    pub fn compile(raw: &RawRulesFile) -> Result<Self, RuleError> {
        let data = RulesData::from_refs(&raw.refs)?;
        Ok(Self {
            classify: compile_rules(&raw.classify, &data)?,
            cleanup: compile_rules(&raw.cleanup, &data)?,
            expiry_pattern: data.pattern("expiry").cloned(),
        })
    }
}

fn compile_rules(raw: &[RawRule], data: &RulesData) -> Result<Vec<Rule>, RuleError> {
    raw.iter()
        .map(|rule| {
            Ok(Rule {
                name: rule.name.clone(),
                when: compile_condition(&rule.when, data)?,
                then: rule.then,
            })
        })
        .collect()
}

/// A loaded `rules.yaml`: the shared base plus per-account override blocks. Each
/// account's [`RuleSet`] is the base with its `accounts.<id>` block deep-merged on
/// top (objects merge; arrays/scalars replace).
pub struct RulesDocument {
    base: Value,
    accounts: HashMap<String, Value>,
}

impl RulesDocument {
    /// Parses a `rules.yaml` string, splitting off the optional `accounts:` map.
    pub fn from_yaml_str(yaml: &str) -> Result<Self, RuleError> {
        let mut value: Value = serde_yaml_ng::from_str(yaml)?;
        let accounts = match &mut value {
            Value::Mapping(map) => match map.remove("accounts") {
                Some(Value::Mapping(blocks)) => blocks
                    .into_iter()
                    .filter_map(|(key, block)| Some((key.as_str()?.to_string(), block)))
                    .collect(),
                _ => HashMap::new(),
            },
            _ => HashMap::new(),
        };
        Ok(Self {
            base: value,
            accounts,
        })
    }

    /// The effective rules for one account (or the shared base when `account` is
    /// `None` or has no override block).
    pub fn compile_for(&self, account: Option<&str>) -> Result<RuleSet, RuleError> {
        let merged = match account.and_then(|id| self.accounts.get(id)) {
            Some(override_block) => deep_merge(self.base.clone(), override_block.clone()),
            None => self.base.clone(),
        };
        let raw: RawRulesFile = serde_yaml_ng::from_value(merged)?;
        RuleSet::compile(&raw)
    }
}
