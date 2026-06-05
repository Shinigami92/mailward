//! GraphQL output types shared by the schema and the run engine.

use async_graphql::{Enum, SimpleObject};
use mailward_core::{Decision, MailMessage};

/// What the engine decided for a message (GraphQL enum: KEEP / MARK_READ / DELETE).
#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum DecisionKind {
    Keep,
    MarkRead,
    Delete,
}

/// Which rule set a run applies (GraphQL enum: CLASSIFY / CLEANUP).
#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum RunMode {
    /// Unread-path classification across scanned folders (mark read / delete).
    Classify,
    /// Prune already-read stale mail from the default folder (delete only).
    Cleanup,
}

impl From<Decision> for DecisionKind {
    fn from(decision: Decision) -> Self {
        match decision {
            Decision::Keep => DecisionKind::Keep,
            Decision::MarkRead => DecisionKind::MarkRead,
            Decision::Delete => DecisionKind::Delete,
        }
    }
}

/// One classified message and what would (or did) happen to it.
#[derive(SimpleObject, Clone, Debug)]
pub struct MessageDecision {
    pub account: String,
    pub folder: String,
    pub uid: String,
    pub subject: String,
    pub from: String,
    pub decision: DecisionKind,
    pub reason: String,
}

/// The outcome of a classify run.
#[derive(SimpleObject, Debug)]
pub struct RunResult {
    pub scanned: i32,
    pub actioned: i32,
    /// `false` for a dry run (nothing changed), `true` if decisions were applied.
    pub applied: bool,
    pub decisions: Vec<MessageDecision>,
}

/// Summary of a configured account (no secrets).
#[derive(SimpleObject, Debug)]
pub struct AccountInfo {
    pub id: String,
    pub vendor: String,
    pub username: String,
    pub imap_host: String,
    pub auth_method: String,
    /// Whether a cached login (refresh token) exists for this account.
    pub authenticated: bool,
}

/// The sign-in URL to open for an interactive login.
#[derive(SimpleObject, Debug)]
pub struct AuthStart {
    pub authorize_url: String,
}

/// A message as shown by the read-only inspect view (never modifies the mailbox).
#[derive(SimpleObject, Debug)]
pub struct InspectedMessage {
    pub folder: String,
    pub uid: String,
    pub subject: String,
    pub from: String,
    pub from_name: String,
    pub is_read: bool,
    pub age_hours: f64,
    pub body_preview: String,
}

impl InspectedMessage {
    pub fn from_message(message: MailMessage) -> Self {
        Self {
            folder: message.folder,
            uid: message.id,
            subject: message.subject,
            from: message.from_address,
            from_name: message.from_name,
            is_read: message.is_read,
            age_hours: message.age_hours,
            body_preview: message.body_preview,
        }
    }
}
