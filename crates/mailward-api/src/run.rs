//! The run engine: load config, authenticate (non-interactive, via cached refresh
//! token), and either **classify** unread mail across the scanned folders or
//! **cleanup** already-read stale mail in one folder. Decisions are broadcast for
//! the `runProgress` subscription and optionally applied (mark read / move to
//! Trash). Shared by the `triggerRun` mutation and the `folders` query.

use std::fs;

use mailward_core::auth::{AuthError, load_cached_refresh, refresh, save_tokens};
use mailward_core::config::ConfigError;
use mailward_core::imap::{Folder, ImapError, ImapSession, discover_scan_folders, find_trash};
use mailward_core::rules::RuleError;
use mailward_core::{
    ConfigDocument, Decision, FileConfig, MailMessage, ResolvedAccount, RuleSet, RulesDocument,
    Verdict, first_match, load_accounts,
};

use crate::state::AppState;
use crate::types::{MessageDecision, RunMode, RunProgress, RunResult};

#[derive(Debug, thiserror::Error)]
pub enum RunError {
    #[error("could not read {path}: {source}")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Rules(#[from] RuleError),
    #[error(transparent)]
    Auth(#[from] AuthError),
    #[error(transparent)]
    Imap(#[from] ImapError),
    #[error("account {0:?} not found in accounts.yaml")]
    UnknownAccount(String),
    #[error("account {0} is not signed in - call authStart/authComplete first")]
    NeedsLogin(String),
    #[error("folder {0:?} not found on the server")]
    UnknownFolder(String),
    #[error("folder {0:?} is confidential or excluded and cannot be inspected")]
    FolderNotInspectable(String),
}

fn read_config(state: &AppState, name: &str) -> Result<String, RunError> {
    let path = state.settings.config_file(name);
    fs::read_to_string(&path).map_err(|source| RunError::Read {
        path: path.display().to_string(),
        source,
    })
}

/// Loaded, parsed config documents for a run.
struct Loaded {
    accounts: Vec<ResolvedAccount>,
    config_doc: ConfigDocument,
    rules_doc: RulesDocument,
}

fn load(state: &AppState) -> Result<Loaded, RunError> {
    Ok(Loaded {
        accounts: load_accounts(
            &read_config(state, "accounts.yaml")?,
            &state.settings.cache_dir,
        )?,
        config_doc: ConfigDocument::from_yaml_str(&read_config(state, "config.yaml")?)?,
        rules_doc: RulesDocument::from_yaml_str(&read_config(state, "rules.yaml")?)?,
    })
}

/// Refreshes an account's access token from its cached refresh token (no prompt).
async fn access_token(account: &ResolvedAccount) -> Result<String, RunError> {
    let refresh_token =
        load_cached_refresh(account).ok_or_else(|| RunError::NeedsLogin(account.id.clone()))?;
    let tokens = refresh(account, &refresh_token).await?;
    save_tokens(account, &tokens)?;
    Ok(tokens.access_token)
}

fn make_decision(
    account: &str,
    folder: &str,
    message: &MailMessage,
    verdict: &Verdict,
) -> MessageDecision {
    MessageDecision {
        account: account.to_string(),
        folder: folder.to_string(),
        uid: message.id.clone(),
        subject: message.subject.clone(),
        from: message.from_address.clone(),
        decision: verdict.decision.into(),
        reason: verdict.reason.clone(),
    }
}

/// Runs `mode` over one account (or all). Broadcasts each decision (tagged with `run_id`
/// so clients only render their own run's events); applies them unless `dry_run`.
///
/// Holds `state.run_lock` for the whole run, so concurrent triggers (mode switch, view
/// navigation, a second tab) serialize instead of racing the shared IMAP/token state.
pub async fn run(
    state: &AppState,
    account_filter: Option<&str>,
    mode: RunMode,
    dry_run: bool,
    run_id: String,
) -> Result<RunResult, RunError> {
    let _run_guard = state.run_lock.lock().await;
    let loaded = load(state)?;
    if let Some(id) = account_filter
        && !loaded.accounts.iter().any(|a| a.id == id)
    {
        return Err(RunError::UnknownAccount(id.to_string()));
    }

    let mut decisions = Vec::new();
    let mut actioned = 0i32;

    for account in &loaded.accounts {
        if account_filter.is_some_and(|id| id != account.id) {
            continue;
        }

        let token = access_token(account).await?;
        let config = loaded.config_doc.config_for(Some(&account.id))?;
        let rules = loaded.rules_doc.compile_for(Some(&account.id))?;

        let mut imap = ImapSession::connect(
            &account.imap_host,
            account.imap_port,
            &account.username,
            &token,
        )
        .await?;

        let listed = imap.list_folders().await?;
        let trash = find_trash(&listed).unwrap_or_else(|| config.imap.trash_fallback.clone());

        match mode {
            RunMode::Classify => {
                let folders = scan_folders(&config, &listed);
                let total = folders.len() as i32;
                for (index, folder) in folders.iter().enumerate() {
                    // Stream per-message fetch progress so the bar advances within each
                    // folder (and names the folder being scanned), not just folder-to-folder.
                    let position = index as i32 + 1;
                    let events = state.events.clone();
                    let label = folder.display_name.clone();
                    let progress_run_id = run_id.clone();
                    let messages = imap
                        .fetch_unread(
                            &folder.name,
                            config.defaults.classify_limit as usize,
                            config.imap.body_preview_chars,
                            move |fetched, fetch_total| {
                                let _ = events.send(RunProgress::new_folder(
                                    progress_run_id.clone(),
                                    label.clone(),
                                    position,
                                    total,
                                    fetched as i32,
                                    fetch_total as i32,
                                ));
                            },
                        )
                        .await?;
                    let (acted, decided) = classify_folder(
                        state,
                        &run_id,
                        &account.id,
                        &folder.display_name,
                        &messages,
                        &rules,
                    );
                    actioned += acted.mark_read.len() as i32 + acted.delete.len() as i32;
                    decisions.extend(decided);
                    if !dry_run {
                        imap.mark_read(&acted.mark_read).await?;
                        imap.move_to_trash(&acted.delete, &trash).await?;
                    }
                }
            }
            RunMode::Cleanup => {
                let folder = config.defaults.folder.clone();
                // Single folder, but a big fetch: stream per-message progress so the bar
                // climbs over the scan instead of jumping to 100% and stalling.
                let events = state.events.clone();
                let label = folder.clone();
                let progress_run_id = run_id.clone();
                let messages = imap
                    .fetch_recent(
                        &folder,
                        config.defaults.cleanup_limit as usize,
                        config.imap.body_preview_chars,
                        move |fetched, fetch_total| {
                            let _ = events.send(RunProgress::new_folder(
                                progress_run_id.clone(),
                                label.clone(),
                                1,
                                1,
                                fetched as i32,
                                fetch_total as i32,
                            ));
                        },
                    )
                    .await?;
                let mut to_delete = Vec::new();
                for message in &messages {
                    // Cleanup only ever touches already-read mail.
                    if !message.is_read {
                        continue;
                    }
                    let verdict = first_match(&rules.cleanup, message);
                    let decision = make_decision(&account.id, &folder, message, &verdict);
                    let _ = state
                        .events
                        .send(RunProgress::new_decision(run_id.clone(), decision.clone()));
                    if verdict.decision == Decision::Delete {
                        to_delete.push(message.id.clone());
                        actioned += 1;
                    }
                    decisions.push(decision);
                }
                if !dry_run {
                    imap.move_to_trash(&to_delete, &trash).await?;
                }
            }
        }

        imap.logout().await?;
    }

    Ok(RunResult {
        run_id,
        scanned: decisions.len() as i32,
        actioned,
        applied: !dry_run,
        decisions,
    })
}

/// UIDs to act on after classifying a folder.
struct Actions {
    mark_read: Vec<String>,
    delete: Vec<String>,
}

fn classify_folder(
    state: &AppState,
    run_id: &str,
    account: &str,
    folder: &str,
    messages: &[MailMessage],
    rules: &RuleSet,
) -> (Actions, Vec<MessageDecision>) {
    let mut actions = Actions {
        mark_read: Vec::new(),
        delete: Vec::new(),
    };
    let mut decisions = Vec::new();
    for message in messages {
        let verdict = first_match(&rules.classify, message);
        let decision = make_decision(account, folder, message, &verdict);
        let _ = state.events.send(RunProgress::new_decision(
            run_id.to_string(),
            decision.clone(),
        ));
        match verdict.decision {
            Decision::MarkRead => actions.mark_read.push(message.id.clone()),
            Decision::Delete => actions.delete.push(message.id.clone()),
            Decision::Keep => {}
        }
        decisions.push(decision);
    }
    (actions, decisions)
}

/// The folders a classify run scans: the configured `scan` list (minus ignored), or
/// auto-discovery when `scan` is null.
fn scan_folders(config: &FileConfig, listed: &[Folder]) -> Vec<Folder> {
    match &config.folders.scan {
        Some(scan) => listed
            .iter()
            .filter(|f| scan.iter().any(|s| s == &f.display_name || s == &f.name))
            .filter(|f| !config.folders.is_ignored(&f.display_name))
            .cloned()
            .collect(),
        None => discover_scan_folders(&config.folders, listed),
    }
}

/// Lists the folders a classify run would scan for an account (for the UI).
pub async fn list_account_folders(
    state: &AppState,
    account_id: &str,
) -> Result<Vec<String>, RunError> {
    let loaded = load(state)?;
    let account = loaded
        .accounts
        .iter()
        .find(|a| a.id == account_id)
        .ok_or_else(|| RunError::UnknownAccount(account_id.to_string()))?;

    let token = access_token(account).await?;
    let config = loaded.config_doc.config_for(Some(&account.id))?;
    let mut imap = ImapSession::connect(
        &account.imap_host,
        account.imap_port,
        &account.username,
        &token,
    )
    .await?;
    let listed = imap.list_folders().await?;
    let folders = scan_folders(&config, &listed)
        .into_iter()
        .map(|f| f.display_name)
        .collect();
    imap.logout().await?;
    Ok(folders)
}

/// All folders a user may inspect: every selectable mailbox (incl. Sent/Junk/Trash)
/// EXCEPT confidential and ignored ones - those are never offered for inspection.
pub async fn inspectable_folders(
    state: &AppState,
    account_id: &str,
) -> Result<Vec<String>, RunError> {
    let loaded = load(state)?;
    let account = loaded
        .accounts
        .iter()
        .find(|a| a.id == account_id)
        .ok_or_else(|| RunError::UnknownAccount(account_id.to_string()))?;

    let token = access_token(account).await?;
    let config = loaded.config_doc.config_for(Some(&account.id))?;
    let mut imap = ImapSession::connect(
        &account.imap_host,
        account.imap_port,
        &account.username,
        &token,
    )
    .await?;
    let listed = imap.list_folders().await?;
    imap.logout().await?;

    Ok(listed
        .into_iter()
        .filter(|f| f.selectable)
        .filter(|f| !config.folders.is_confidential(&f.display_name))
        .filter(|f| !config.folders.is_ignored(&f.display_name))
        .map(|f| f.display_name)
        .collect())
}

/// Read-only inspection of one folder. Uses `BODY.PEEK`, so it NEVER marks mail
/// read, and it refuses confidential/ignored folders server-side (defense in depth,
/// even if a client asks).
pub async fn inspect_messages(
    state: &AppState,
    account_id: &str,
    folder: &str,
    limit: usize,
    unread_only: bool,
) -> Result<Vec<MailMessage>, RunError> {
    let loaded = load(state)?;
    let account = loaded
        .accounts
        .iter()
        .find(|a| a.id == account_id)
        .ok_or_else(|| RunError::UnknownAccount(account_id.to_string()))?;
    let config = loaded.config_doc.config_for(Some(&account.id))?;

    let token = access_token(account).await?;
    let mut imap = ImapSession::connect(
        &account.imap_host,
        account.imap_port,
        &account.username,
        &token,
    )
    .await?;
    let listed = imap.list_folders().await?;

    // Resolve the requested (display) name to the raw IMAP name, and refuse
    // confidential / ignored folders.
    let (raw, display) = {
        let target = listed
            .iter()
            .find(|f| f.display_name == folder || f.name == folder)
            .ok_or_else(|| RunError::UnknownFolder(folder.to_string()))?;
        if config.folders.is_confidential(&target.display_name)
            || config.folders.is_ignored(&target.display_name)
        {
            return Err(RunError::FolderNotInspectable(target.display_name.clone()));
        }
        (target.name.clone(), target.display_name.clone())
    };

    let mut messages = if unread_only {
        imap.fetch_unread(&raw, limit, config.imap.body_preview_chars, |_, _| {})
            .await?
    } else {
        imap.fetch_recent(&raw, limit, config.imap.body_preview_chars, |_, _| {})
            .await?
    };
    imap.logout().await?;

    // Label with the decoded folder name for display.
    for message in &mut messages {
        message.folder = display.clone();
    }
    Ok(messages)
}
