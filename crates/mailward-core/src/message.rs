//! The message model and the pluggable classifier interface.

use serde::{Deserialize, Serialize};

/// A single mail message, projected from IMAP into the fields the rule DSL needs.
///
/// `age_hours` is precomputed by the fetch layer (`now - received`) so the engine
/// stays clock-free and deterministic. `importance` is omitted for now - IMAP
/// envelopes don't carry it (a v1 PoC limitation noted in the docs).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MailMessage {
    pub id: String,
    /// IMAP folder the message lives in, e.g. `INBOX`, `Junk`, `Archive/GitHub`.
    pub folder: String,
    pub subject: String,
    /// Sender email address, lower-cased, or "" if absent.
    pub from_address: String,
    /// Sender display name, or "" if absent.
    pub from_name: String,
    /// First chunk of the body, plain text.
    pub body_preview: String,
    /// Whole-message age in hours (computed upstream from the received date).
    pub age_hours: f64,
    pub is_read: bool,
}

/// What the engine decides to do with a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Decision {
    Keep,
    MarkRead,
    Delete,
}

/// A decision plus a human-readable reason (usually the matching rule's name).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Verdict {
    pub decision: Decision,
    pub reason: String,
}

/// Pluggable decision engine. The rule-based [`crate::RuleClassifier`] ships today;
/// an AI-backed classifier can drop in behind the same trait later.
pub trait Classifier {
    fn classify(&self, message: &MailMessage) -> Verdict;
}

/// The leaf name of a (possibly nested) IMAP folder path, so rules can match
/// `Archive/GitHub` as just `GitHub`. Uses the IMAP `/` delimiter.
pub fn folder_name(folder: &str) -> &str {
    folder.rsplit('/').next().unwrap_or(folder)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folder_name_takes_the_leaf() {
        assert_eq!(folder_name("Archive/GitHub"), "GitHub");
        assert_eq!(folder_name("INBOX"), "INBOX");
        assert_eq!(folder_name("a/b/c"), "c");
    }
}
