//! First-match-wins rule execution.

use super::dsl::Predicate;
use crate::message::{Classifier, Decision, MailMessage, Verdict};

/// A single ordered rule. The first rule whose `when` predicate matches wins and
/// its `then` decision is applied - this is where logic a webmail's built-in
/// filters can't express lives.
pub struct Rule {
    /// Short identifier, surfaced as the decision reason.
    pub name: String,
    pub when: Predicate,
    pub then: Decision,
}

/// Evaluates rules top-to-bottom; the first match wins, otherwise `keep`.
pub fn first_match(rules: &[Rule], message: &MailMessage) -> Verdict {
    for rule in rules {
        if (rule.when)(message) {
            return Verdict {
                decision: rule.then,
                reason: rule.name.clone(),
            };
        }
    }
    Verdict {
        decision: Decision::Keep,
        reason: "no rule matched".to_string(),
    }
}

/// A [`Classifier`] backed by an ordered list of rules.
pub struct RuleClassifier {
    rules: Vec<Rule>,
}

impl RuleClassifier {
    pub fn new(rules: Vec<Rule>) -> Self {
        Self { rules }
    }
}

impl Classifier for RuleClassifier {
    fn classify(&self, message: &MailMessage) -> Verdict {
        first_match(&self.rules, message)
    }
}
