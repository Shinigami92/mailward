//! Live dry-run diagnostic: sign in to each configured account, fetch unread
//! mail from a few folders, run the rule engine, and print what it WOULD do.
//! Nothing is ever modified (no mark-read, no move). Run it from the repo root,
//! where your `accounts.yaml` / `config.yaml` / `rules.yaml` live:
//!
//! ```sh
//! cargo run -p mailward-core --example dry_run
//! ```
//!
//! Microsoft accounts trigger a one-time browser sign-in: open the printed URL,
//! approve, then paste the `https://localhost/?code=...` redirect URL back here.

use std::error::Error;
use std::io::Write;
use std::path::Path;

use mailward_core::auth::{self, AuthError};
use mailward_core::config::ConfigDocument;
use mailward_core::imap::ImapSession;
use mailward_core::{RulesDocument, first_match, load_accounts};

/// Folders scanned when `config.yaml` doesn't pin an explicit `folders.scan` list.
const DEFAULT_FOLDERS: &[&str] = &["INBOX", "Junk"];

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let cache_root = Path::new(".cache");
    let accounts = load_accounts(&read("accounts.yaml")?, cache_root)?;
    let config_doc = ConfigDocument::from_yaml_str(&read("config.yaml")?)?;
    let rules_doc = RulesDocument::from_yaml_str(&read("rules.yaml")?)?;

    for account in &accounts {
        println!(
            "\n══════ Account: {} ({}) ══════",
            account.id, account.vendor
        );

        let token = match auth::authenticate(account, paste_prompt).await {
            Ok(token) => token,
            Err(AuthError::VendorNotImplemented { vendor, .. }) => {
                println!("  ↪ skipped: vendor '{vendor}' auth is not implemented yet");
                continue;
            }
            Err(error) => {
                eprintln!("  ✗ auth failed: {error}");
                continue;
            }
        };

        let config = config_doc.config_for(Some(&account.id))?;
        let rules = rules_doc.compile_for(Some(&account.id))?;

        let folders: Vec<String> = config
            .folders
            .scan
            .clone()
            .unwrap_or_else(|| DEFAULT_FOLDERS.iter().map(|s| s.to_string()).collect());

        let mut imap = ImapSession::connect(
            &account.imap_host,
            account.imap_port,
            &account.username,
            &token,
        )
        .await?;

        let mut acted = 0usize;
        let mut total = 0usize;
        for folder in folders {
            if config.folders.is_ignored(&folder) {
                continue;
            }
            let messages = imap
                .fetch_unread(
                    &folder,
                    config.defaults.inspect_limit as usize,
                    config.imap.body_preview_chars,
                )
                .await?;

            for message in &messages {
                total += 1;
                let verdict = first_match(&rules.classify, message);
                let mark = match verdict.decision {
                    mailward_core::Decision::Keep => "keep    ",
                    mailward_core::Decision::MarkRead => "markRead",
                    mailward_core::Decision::Delete => "delete  ",
                };
                if verdict.decision != mailward_core::Decision::Keep {
                    acted += 1;
                }
                println!(
                    "  [{mark}] {:<14} {:<46} {}  · {}",
                    truncate(&folder, 14),
                    truncate(&message.subject, 46),
                    truncate(&message.from_address, 30),
                    verdict.reason,
                );
            }
        }

        println!("  - {total} unread scanned, {acted} would be acted on (DRY RUN)");
        imap.logout().await?;
    }

    println!("\nDry run complete - nothing was modified.");
    Ok(())
}

/// Shows the sign-in URL and reads the pasted redirect URL from stdin.
fn paste_prompt(authorize_url: &str) -> std::io::Result<String> {
    println!("\n  Open this URL, sign in, approve, then paste the redirect URL:\n");
    println!("    {authorize_url}\n");
    print!("  redirect URL> ");
    std::io::stdout().flush()?;
    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;
    Ok(line)
}

fn read(path: &str) -> Result<String, Box<dyn Error>> {
    std::fs::read_to_string(path).map_err(|e| {
        format!("could not read {path}: {e} (copy the matching *.example.yaml)").into()
    })
}

fn truncate(s: &str, max: usize) -> String {
    let trimmed: String = s.chars().take(max).collect();
    format!("{trimmed:<max$}")
}
