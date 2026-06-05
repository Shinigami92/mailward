//! The `refs:` data the rules point at - sender `lists`, regex `patterns`, age
//! `thresholds` - plus ref resolution and human-duration parsing.

use std::collections::HashMap;
use std::sync::OnceLock;

use regex::{Regex, RegexBuilder};
use serde::Deserialize;
use serde_yaml_ng::Value;

use super::RuleError;

/// Raw `refs:` section as read from YAML (patterns are still strings here).
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct RawRefs {
    pub lists: HashMap<String, Vec<String>>,
    pub patterns: HashMap<String, String>,
    pub thresholds: HashMap<String, Value>,
}

/// Compiled refs: lists as-is, patterns as case-insensitive regexes, thresholds
/// normalised to hours. Resolution is generic (no hard-coded key names), so any
/// `refs.<bucket>.<name>` the rules use just works.
#[derive(Debug)]
pub struct RulesData {
    lists: HashMap<String, Vec<String>>,
    patterns: HashMap<String, Regex>,
    thresholds: HashMap<String, f64>,
}

/// A resolved ref operand.
pub enum RefValue<'a> {
    List(&'a [String]),
    Pattern(&'a Regex),
    Threshold(f64),
}

impl RulesData {
    pub fn from_refs(refs: &RawRefs) -> Result<Self, RuleError> {
        let mut patterns = HashMap::with_capacity(refs.patterns.len());
        for (key, source) in &refs.patterns {
            let regex = RegexBuilder::new(source)
                .case_insensitive(true)
                .build()
                .map_err(|source| RuleError::BadRegex {
                    key: key.clone(),
                    source,
                })?;
            patterns.insert(key.clone(), regex);
        }

        let mut thresholds = HashMap::with_capacity(refs.thresholds.len());
        for (key, value) in &refs.thresholds {
            let raw = scalar_to_string(value).ok_or_else(|| RuleError::BadDuration {
                key: key.clone(),
                value: format!("{value:?}"),
            })?;
            let hours = parse_hours(&raw).ok_or_else(|| RuleError::BadDuration {
                key: key.clone(),
                value: raw.clone(),
            })?;
            thresholds.insert(key.clone(), hours);
        }

        Ok(Self {
            lists: refs.lists.clone(),
            patterns,
            thresholds,
        })
    }

    /// Resolves a dotted ref path like `lists.markReadDomains`, `patterns.phishing`
    /// or `thresholds.monthly`. Errors on an unknown bucket or key.
    pub fn resolve(&self, path: &str) -> Result<RefValue<'_>, RuleError> {
        let resolved = path.split_once('.').and_then(|(bucket, key)| match bucket {
            "lists" => self.lists.get(key).map(|v| RefValue::List(v.as_slice())),
            "patterns" => self.patterns.get(key).map(RefValue::Pattern),
            "thresholds" => self.thresholds.get(key).map(|n| RefValue::Threshold(*n)),
            _ => None,
        });
        resolved.ok_or_else(|| RuleError::UnknownRef(path.to_string()))
    }

    /// A compiled pattern by name, if present (used for the cleanup expiry column).
    pub fn pattern(&self, key: &str) -> Option<&Regex> {
        self.patterns.get(key)
    }
}

/// Renders a scalar YAML value (string or number) to a string for duration parsing.
fn scalar_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

/// Parses a human duration (`7d`, `30 days`, `1440 min`, `36h`, `2 weeks`, or a
/// compound like `1d 12h`) into hours. Returns `None` if no unit token is found.
fn parse_hours(input: &str) -> Option<f64> {
    static TOKEN: OnceLock<Regex> = OnceLock::new();
    let token = TOKEN.get_or_init(|| Regex::new(r"(?i)(\d+(?:\.\d+)?)\s*([a-z]+)").unwrap());

    let mut total = 0.0;
    let mut matched = false;
    for capture in token.captures_iter(input) {
        let amount: f64 = capture[1].parse().ok()?;
        let hours_per_unit = unit_to_hours(&capture[2].to_lowercase())?;
        total += amount * hours_per_unit;
        matched = true;
    }
    matched.then_some(total)
}

fn unit_to_hours(unit: &str) -> Option<f64> {
    Some(match unit {
        "ms" => 1.0 / 3_600_000.0,
        "s" | "sec" | "secs" | "second" | "seconds" => 1.0 / 3600.0,
        "m" | "min" | "mins" | "minute" | "minutes" => 1.0 / 60.0,
        "h" | "hr" | "hrs" | "hour" | "hours" => 1.0,
        "d" | "day" | "days" => 24.0,
        "w" | "wk" | "week" | "weeks" => 168.0,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_common_durations_to_hours() {
        assert_eq!(parse_hours("7d"), Some(168.0));
        assert_eq!(parse_hours("30 days"), Some(720.0));
        assert_eq!(parse_hours("1d"), Some(24.0));
        assert_eq!(parse_hours("36h"), Some(36.0));
        assert_eq!(parse_hours("90 min"), Some(1.5));
        assert_eq!(parse_hours("2 weeks"), Some(336.0));
        assert_eq!(parse_hours("1d 12h"), Some(36.0));
        assert_eq!(parse_hours("nonsense"), None);
    }
}
