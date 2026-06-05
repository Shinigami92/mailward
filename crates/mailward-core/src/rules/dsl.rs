//! The declarative condition language. A condition is either a leaf test or a
//! combinator; conditions compile once into a [`Predicate`] (regexes/refs
//! resolved up front) so per-message evaluation is cheap.
//!
//! ```yaml
//! when:
//!   all:
//!     - { field: folder, op: equals, value: Junk }
//!     - { field: subject, op: regex, ref: patterns.spamSubject }
//! ```

use std::borrow::Cow;

use regex::{Regex, RegexBuilder};
use serde::Deserialize;
use serde_yaml_ng::Value;

use super::RuleError;
use super::data::{RefValue, RulesData};
use crate::message::{MailMessage, folder_name};

/// Message fields exposed to the DSL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Field {
    Subject,
    FromAddress,
    FromName,
    /// subject + body preview, for rules that look at content.
    Content,
    /// the LEAF folder name, so `GitHub` matches `Archive/GitHub`.
    Folder,
    /// whole-message age in hours.
    Age,
}

/// Comparison operators. String ops are case-insensitive except `equals` (exact).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Op {
    #[serde(rename = "regex")]
    Regex,
    #[serde(rename = "includes")]
    Includes,
    #[serde(rename = "endsWith")]
    EndsWith,
    #[serde(rename = "endsWithAny")]
    EndsWithAny,
    #[serde(rename = "equals")]
    Equals,
    #[serde(rename = ">")]
    Gt,
    #[serde(rename = ">=")]
    Gte,
}

/// A single leaf test: `{ field, op, value }` or `{ field, op, ref }`.
#[derive(Debug, Clone, Deserialize)]
pub struct LeafCondition {
    pub field: Field,
    pub op: Op,
    #[serde(default)]
    pub value: Option<Value>,
    #[serde(default, rename = "ref")]
    pub reference: Option<String>,
}

/// A condition node: a leaf or a combinator. Deserialized untagged - a map with
/// `all` / `any` / `not` is a combinator, otherwise it's a leaf.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Condition {
    All { all: Vec<Condition> },
    Any { any: Vec<Condition> },
    Not { not: Box<Condition> },
    Leaf(LeafCondition),
}

/// A compiled, ready-to-evaluate condition.
pub type Predicate = Box<dyn Fn(&MailMessage) -> bool + Send + Sync>;

/// Recursively compiles a condition into a single [`Predicate`].
pub fn compile_condition(node: &Condition, data: &RulesData) -> Result<Predicate, RuleError> {
    match node {
        Condition::All { all } => {
            let subs = compile_all(all, data)?;
            Ok(Box::new(move |m| subs.iter().all(|p| p(m))))
        }
        Condition::Any { any } => {
            let subs = compile_all(any, data)?;
            Ok(Box::new(move |m| subs.iter().any(|p| p(m))))
        }
        Condition::Not { not } => {
            let sub = compile_condition(not, data)?;
            Ok(Box::new(move |m| !sub(m)))
        }
        Condition::Leaf(leaf) => compile_leaf(leaf, data),
    }
}

fn compile_all(nodes: &[Condition], data: &RulesData) -> Result<Vec<Predicate>, RuleError> {
    nodes.iter().map(|n| compile_condition(n, data)).collect()
}

fn compile_leaf(leaf: &LeafCondition, data: &RulesData) -> Result<Predicate, RuleError> {
    let field = leaf.field;
    let operand = resolve_operand(leaf, data)?;

    Ok(match leaf.op {
        Op::Equals => {
            ensure_text_field(field, leaf.op)?;
            let target = operand.into_text(leaf.op)?;
            Box::new(move |m| string_value(m, field).as_ref() == target)
        }
        Op::Includes => {
            ensure_text_field(field, leaf.op)?;
            let needle = operand.into_text(leaf.op)?.to_lowercase();
            Box::new(move |m| string_value(m, field).to_lowercase().contains(&needle))
        }
        Op::EndsWith => {
            ensure_text_field(field, leaf.op)?;
            let suffix = operand.into_text(leaf.op)?.to_lowercase();
            Box::new(move |m| string_value(m, field).to_lowercase().ends_with(&suffix))
        }
        Op::EndsWithAny => {
            ensure_text_field(field, leaf.op)?;
            let suffixes: Vec<String> = operand
                .into_list(leaf.op)?
                .into_iter()
                .map(|s| s.to_lowercase())
                .collect();
            Box::new(move |m| {
                let text = string_value(m, field).to_lowercase();
                suffixes.iter().any(|s| text.ends_with(s))
            })
        }
        Op::Regex => {
            ensure_text_field(field, leaf.op)?;
            let regex = operand.into_regex(leaf.op)?;
            Box::new(move |m| regex.is_match(string_value(m, field).as_ref()))
        }
        Op::Gt => {
            ensure_age_field(field, leaf.op)?;
            let threshold = operand.into_num(leaf.op)?;
            Box::new(move |m| number_value(m, field) > threshold)
        }
        Op::Gte => {
            ensure_age_field(field, leaf.op)?;
            let threshold = operand.into_num(leaf.op)?;
            Box::new(move |m| number_value(m, field) >= threshold)
        }
    })
}

/// The literal (`value`) or resolved (`ref`) operand of a leaf, as owned data the
/// compiled predicate can capture.
enum Operand {
    Text(String),
    Num(f64),
    List(Vec<String>),
    Pattern(Regex),
}

impl Operand {
    fn into_text(self, op: Op) -> Result<String, RuleError> {
        match self {
            Operand::Text(s) => Ok(s),
            Operand::Num(n) => Ok(format!("{n}")),
            _ => Err(operand_type(op, "a text")),
        }
    }

    fn into_list(self, op: Op) -> Result<Vec<String>, RuleError> {
        match self {
            Operand::List(v) => Ok(v),
            _ => Err(operand_type(op, "a list")),
        }
    }

    fn into_num(self, op: Op) -> Result<f64, RuleError> {
        match self {
            Operand::Num(n) => Ok(n),
            Operand::Text(s) => s.parse().map_err(|_| operand_type(op, "a numeric")),
            _ => Err(operand_type(op, "a numeric")),
        }
    }

    fn into_regex(self, op: Op) -> Result<Regex, RuleError> {
        match self {
            Operand::Pattern(re) => Ok(re),
            Operand::Text(s) => RegexBuilder::new(&s)
                .case_insensitive(true)
                .build()
                .map_err(|source| RuleError::BadRegex {
                    key: format!("(inline {op:?})"),
                    source,
                }),
            _ => Err(operand_type(op, "a regex")),
        }
    }
}

fn resolve_operand(leaf: &LeafCondition, data: &RulesData) -> Result<Operand, RuleError> {
    if let Some(path) = &leaf.reference {
        return Ok(match data.resolve(path)? {
            RefValue::List(v) => Operand::List(v.to_vec()),
            RefValue::Pattern(re) => Operand::Pattern(re.clone()),
            RefValue::Threshold(n) => Operand::Num(n),
        });
    }
    if let Some(value) = &leaf.value {
        return value_to_operand(value);
    }
    Err(RuleError::MissingOperand {
        field: format!("{:?}", leaf.field),
        op: format!("{:?}", leaf.op),
    })
}

fn value_to_operand(value: &Value) -> Result<Operand, RuleError> {
    Ok(match value {
        Value::String(s) => Operand::Text(s.clone()),
        Value::Bool(b) => Operand::Text(b.to_string()),
        Value::Number(n) => Operand::Num(n.as_f64().unwrap_or_default()),
        Value::Sequence(seq) => Operand::List(seq.iter().filter_map(value_scalar).collect()),
        other => {
            return Err(RuleError::OperandType {
                op: "value".into(),
                expected: format!("a scalar or list (got {other:?})"),
            });
        }
    })
}

fn value_scalar(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// The text of a field for the current message (case folding happens at the call site).
fn string_value(message: &MailMessage, field: Field) -> Cow<'_, str> {
    match field {
        Field::Subject => Cow::Borrowed(&message.subject),
        Field::FromAddress => Cow::Borrowed(&message.from_address),
        Field::FromName => Cow::Borrowed(&message.from_name),
        Field::Content => Cow::Owned(format!("{} {}", message.subject, message.body_preview)),
        Field::Folder => Cow::Borrowed(folder_name(&message.folder)),
        // Guarded by `ensure_text_field`; never reached.
        Field::Age => Cow::Borrowed(""),
    }
}

fn number_value(message: &MailMessage, field: Field) -> f64 {
    match field {
        Field::Age => message.age_hours,
        // Guarded by `ensure_age_field`; never reached.
        _ => 0.0,
    }
}

fn ensure_text_field(field: Field, op: Op) -> Result<(), RuleError> {
    if field == Field::Age {
        return Err(RuleError::WrongField {
            op: format!("{op:?}"),
            expected: "a text".into(),
            field: format!("{field:?}"),
        });
    }
    Ok(())
}

fn ensure_age_field(field: Field, op: Op) -> Result<(), RuleError> {
    if field != Field::Age {
        return Err(RuleError::WrongField {
            op: format!("{op:?}"),
            expected: "a numeric".into(),
            field: format!("{field:?}"),
        });
    }
    Ok(())
}

fn operand_type(op: Op, expected: &str) -> RuleError {
    RuleError::OperandType {
        op: format!("{op:?}"),
        expected: expected.into(),
    }
}
