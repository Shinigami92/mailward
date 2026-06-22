//! IMAP backend over TLS (rustls) using `async-imap`, authenticated with SASL
//! XOAUTH2. Fetches unread mail into [`MailMessage`]s and applies decisions
//! (mark read / move to Trash). The fetch layer is where `age_hours` is computed,
//! keeping the rule engine clock-free.

use std::sync::Arc;

use async_imap::imap_proto::NameAttribute;
use async_imap::types::{Flag, Name};
use chrono::{DateTime, FixedOffset, Utc};
use futures::StreamExt;
use mail_parser::MessageParser;
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tokio_rustls::rustls::{ClientConfig, RootCertStore, pki_types::ServerName};
use tokio_util::compat::{Compat, TokioAsyncReadCompatExt};
use utf7_imap::decode_utf7_imap;

use crate::config::FoldersConfig;
use crate::message::{MailMessage, folder_name};

type ImapStream = Compat<tokio_rustls::client::TlsStream<TcpStream>>;

#[derive(Debug, thiserror::Error)]
pub enum ImapError {
    #[error("imap: I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("imap: protocol: {0}")]
    Protocol(#[from] async_imap::error::Error),
    #[error("imap: invalid host name: {0}")]
    InvalidHost(String),
    #[error("imap: no server greeting")]
    NoGreeting,
    #[error("imap: authentication failed: {0}")]
    Auth(async_imap::error::Error),
}

/// SASL XOAUTH2 authenticator: emits the `user=..\x01auth=Bearer ..\x01\x01` string.
struct XOAuth2 {
    user: String,
    access_token: String,
}

impl async_imap::Authenticator for XOAuth2 {
    type Response = String;

    fn process(&mut self, _challenge: &[u8]) -> Self::Response {
        format!(
            "user={}\x01auth=Bearer {}\x01\x01",
            self.user, self.access_token
        )
    }
}

/// An authenticated IMAP session.
pub struct ImapSession {
    session: async_imap::Session<ImapStream>,
}

impl ImapSession {
    /// Connects over TLS and authenticates with XOAUTH2.
    pub async fn connect(
        host: &str,
        port: u16,
        user: &str,
        access_token: &str,
    ) -> Result<Self, ImapError> {
        // Install a default crypto provider once (idempotent across calls).
        let _ = tokio_rustls::rustls::crypto::ring::default_provider().install_default();

        let mut roots = RootCertStore::empty();
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let config = ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
        let connector = TlsConnector::from(Arc::new(config));

        let tcp = TcpStream::connect((host, port)).await?;
        let domain = ServerName::try_from(host.to_string())
            .map_err(|_| ImapError::InvalidHost(host.into()))?;
        let tls = connector.connect(domain, tcp).await?;

        let mut client = async_imap::Client::new(tls.compat());
        // Consume the server greeting before sending AUTHENTICATE.
        if client.read_response().await?.is_none() {
            return Err(ImapError::NoGreeting);
        }

        let session = client
            .authenticate(
                "XOAUTH2",
                XOAuth2 {
                    user: user.to_string(),
                    access_token: access_token.to_string(),
                },
            )
            .await
            .map_err(|(error, _client)| ImapError::Auth(error))?;

        Ok(Self { session })
    }

    /// Fetches up to `limit` of the newest unread messages in `folder`.
    ///
    /// `on_progress(fetched, total)` is called once up front with `(0, total)` and then
    /// after each message arrives, so callers can drive a determinate progress bar over
    /// the (network-bound) fetch.
    pub async fn fetch_unread(
        &mut self,
        folder: &str,
        limit: usize,
        body_preview_chars: usize,
        mut on_progress: impl FnMut(usize, usize),
    ) -> Result<Vec<MailMessage>, ImapError> {
        self.session.select(folder).await?;

        let unseen = self.session.uid_search("UNSEEN").await?;
        let mut uids: Vec<u32> = unseen.into_iter().collect();
        uids.sort_unstable();
        let newest: Vec<u32> = uids.into_iter().rev().take(limit).collect();
        let total = newest.len();
        on_progress(0, total);
        if newest.is_empty() {
            return Ok(Vec::new());
        }
        let set = newest
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(",");

        let mut messages = Vec::new();
        {
            let mut stream = self
                .session
                .uid_fetch(&set, "(UID FLAGS INTERNALDATE BODY.PEEK[])")
                .await?;
            while let Some(item) = stream.next().await {
                let fetch = item?;
                messages.push(to_message(&fetch, folder, body_preview_chars));
                on_progress(messages.len(), total);
            }
        }
        Ok(messages)
    }

    /// Fetches the newest `limit` messages in `folder` regardless of read state
    /// (used by `--cleanup`, which prunes already-read stale mail).
    pub async fn fetch_recent(
        &mut self,
        folder: &str,
        limit: usize,
        body_preview_chars: usize,
        mut on_progress: impl FnMut(usize, usize),
    ) -> Result<Vec<MailMessage>, ImapError> {
        let mailbox = self.session.select(folder).await?;
        let exists = mailbox.exists;
        if exists == 0 || limit == 0 {
            on_progress(0, 0);
            return Ok(Vec::new());
        }
        let start = exists.saturating_sub(limit as u32).saturating_add(1).max(1);
        let set = format!("{start}:{exists}");
        let total = (exists - start + 1) as usize;
        on_progress(0, total);

        let mut messages = Vec::new();
        {
            let mut stream = self
                .session
                .fetch(&set, "(UID FLAGS INTERNALDATE BODY.PEEK[])")
                .await?;
            while let Some(item) = stream.next().await {
                let fetch = item?;
                messages.push(to_message(&fetch, folder, body_preview_chars));
                on_progress(messages.len(), total);
            }
        }
        Ok(messages)
    }

    /// Marks the given UIDs `\Seen`.
    pub async fn mark_read(&mut self, uids: &[String]) -> Result<(), ImapError> {
        if uids.is_empty() {
            return Ok(());
        }
        let mut stream = self
            .session
            .uid_store(uids.join(","), "+FLAGS (\\Seen)")
            .await?;
        while let Some(item) = stream.next().await {
            item?;
        }
        Ok(())
    }

    /// Moves the given UIDs to `trash_folder` (recoverable; never a hard delete).
    pub async fn move_to_trash(
        &mut self,
        uids: &[String],
        trash_folder: &str,
    ) -> Result<(), ImapError> {
        if uids.is_empty() {
            return Ok(());
        }
        self.session.uid_mv(uids.join(","), trash_folder).await?;
        Ok(())
    }

    /// Lists every mailbox on the server with its selectability + special-use flags.
    pub async fn list_folders(&mut self) -> Result<Vec<Folder>, ImapError> {
        let mut stream = self.session.list(None, Some("*")).await?;
        let mut folders = Vec::new();
        while let Some(item) = stream.next().await {
            folders.push(folder_from_name(&item?));
        }
        Ok(folders)
    }

    /// Logs out, closing the session.
    pub async fn logout(mut self) -> Result<(), ImapError> {
        self.session.logout().await?;
        Ok(())
    }
}

fn to_message(
    fetch: &async_imap::types::Fetch,
    folder: &str,
    body_preview_chars: usize,
) -> MailMessage {
    let uid = fetch.uid.unwrap_or(fetch.message);
    let raw = fetch.body().or_else(|| fetch.text()).unwrap_or(&[]);
    let parsed = MessageParser::default().parse(raw);

    let subject = parsed
        .as_ref()
        .and_then(|m| m.subject())
        .unwrap_or_default()
        .to_string();
    let (from_address, from_name) = parsed
        .as_ref()
        .and_then(|m| m.from())
        .and_then(|a| a.first())
        .map(|addr| {
            (
                addr.address().unwrap_or_default().to_lowercase(),
                addr.name().unwrap_or_default().to_string(),
            )
        })
        .unwrap_or_default();
    let body_preview = parsed
        .as_ref()
        .and_then(|m| m.body_text(0))
        .map(|text| text.chars().take(body_preview_chars).collect())
        .unwrap_or_default();
    let age_hours = fetch.internal_date().map(age_in_hours).unwrap_or(0.0);
    let is_read = fetch.flags().any(|f| matches!(f, Flag::Seen));

    MailMessage {
        id: uid.to_string(),
        folder: folder.to_string(),
        subject,
        from_address,
        from_name,
        body_preview,
        age_hours,
        is_read,
    }
}

fn age_in_hours(received: DateTime<FixedOffset>) -> f64 {
    let seconds = (Utc::now() - received.with_timezone(&Utc)).num_seconds();
    seconds as f64 / 3600.0
}

/// A mailbox as returned by LIST.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Folder {
    /// Raw IMAP name (modified UTF-7) - use this for SELECT / MOVE.
    pub name: String,
    /// Decoded UTF-8 name - use this for matching config and for display.
    pub display_name: String,
    pub selectable: bool,
    /// RFC 6154 special-use flags (e.g. `\Sent`, `\Trash`) as `\Name` strings.
    pub special_use: Vec<String>,
}

fn folder_from_name(name: &Name) -> Folder {
    let mut selectable = true;
    let mut special_use = Vec::new();
    for attribute in name.attributes() {
        match attribute {
            NameAttribute::NoSelect => selectable = false,
            NameAttribute::Sent => special_use.push("\\Sent".to_string()),
            NameAttribute::Drafts => special_use.push("\\Drafts".to_string()),
            NameAttribute::Trash => special_use.push("\\Trash".to_string()),
            NameAttribute::Junk => special_use.push("\\Junk".to_string()),
            NameAttribute::Archive => special_use.push("\\Archive".to_string()),
            NameAttribute::All => special_use.push("\\All".to_string()),
            NameAttribute::Flagged => special_use.push("\\Flagged".to_string()),
            NameAttribute::Extension(flag) => {
                special_use.push(format!("\\{}", flag.trim_start_matches('\\')));
            }
            // NoInferiors / Marked / Unmarked and any future attributes are irrelevant here.
            _ => {}
        }
    }
    let raw = name.name().to_string();
    Folder {
        display_name: decode_utf7_imap(raw.clone()),
        name: raw,
        selectable,
        special_use,
    }
}

/// The folders a default classify run should scan: selectable, minus the configured
/// special-use / name / ignore / confidential exclusions. Mirrors the v1 auto-discovery.
///
/// Matching uses each folder's **decoded** `display_name`, so a config entry written
/// in UTF-8 (e.g. a confidential folder whose name has non-ASCII characters) correctly
/// excludes the folder even though IMAP reports it in modified UTF-7.
pub fn discover_scan_folders(config: &FoldersConfig, folders: &[Folder]) -> Vec<Folder> {
    folders
        .iter()
        .filter(|folder| folder.selectable)
        .filter(|folder| {
            !folder.special_use.iter().any(|su| {
                config
                    .skip_special_use
                    .iter()
                    .any(|skip| skip.eq_ignore_ascii_case(su))
            })
        })
        .filter(|folder| {
            let leaf = folder_name(&folder.display_name).to_lowercase();
            !config.skip_names.iter().any(|name| name == &leaf)
        })
        .filter(|folder| !config.is_ignored(&folder.display_name))
        .filter(|folder| !config.is_confidential(&folder.display_name))
        .cloned()
        .collect()
}

/// The server's `\Trash` mailbox, if it advertises one.
pub fn find_trash(folders: &[Folder]) -> Option<String> {
    folders
        .iter()
        .find(|folder| {
            folder
                .special_use
                .iter()
                .any(|su| su.eq_ignore_ascii_case("\\Trash"))
        })
        .map(|folder| folder.name.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(name: &str, selectable: bool, special: &[&str]) -> Folder {
        Folder {
            name: name.to_string(),
            display_name: decode_utf7_imap(name.to_string()),
            selectable,
            special_use: special.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn folders_config() -> FoldersConfig {
        FoldersConfig {
            scan: None,
            skip_special_use: vec!["\\Sent".into(), "\\Drafts".into(), "\\Trash".into()],
            skip_names: vec!["outbox".into(), "notes".into()],
            ignore: vec!["Archive/Old".into()],
            confidential: vec!["Archive/Banking".into()],
        }
    }

    #[test]
    fn discovery_applies_all_exclusions() {
        let config = folders_config();
        let listed = vec![
            folder("INBOX", true, &[]),
            folder("Junk", true, &["\\Junk"]),
            folder("Sent", true, &["\\Sent"]), // skip: special-use
            folder("Deleted", true, &["\\Trash"]), // skip: special-use
            folder("Notes", true, &[]),        // skip: name
            folder("[Gmail]", false, &["\\NoSelect"]), // skip: not selectable
            folder("Archive/Old", true, &[]),  // skip: ignore
            folder("Archive/Banking", true, &[]), // skip: confidential
            folder("Archive/GitHub", true, &[]), // keep
        ];
        let scanned = discover_scan_folders(&config, &listed);
        let names: Vec<&str> = scanned.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["INBOX", "Junk", "Archive/GitHub"]);
    }

    #[test]
    fn confidential_excludes_non_ascii_folder_via_decoded_name() {
        // IMAP reports the folder in modified UTF-7 ("ä" → "&AOQ-"); config lists UTF-8.
        let listed = vec![folder("Archive/Reisepl&AOQ-ne", true, &[])];
        assert_eq!(listed[0].display_name, "Archive/Reisepläne");

        let mut config = folders_config();
        config.confidential = vec!["Archive/Reisepläne".into()];
        assert!(discover_scan_folders(&config, &listed).is_empty());
    }

    #[test]
    fn find_trash_uses_special_use() {
        let listed = vec![
            folder("INBOX", true, &[]),
            folder("Deleted Items", true, &["\\Trash"]),
        ];
        assert_eq!(find_trash(&listed).as_deref(), Some("Deleted Items"));
        assert_eq!(find_trash(&[folder("INBOX", true, &[])]), None);
    }
}
