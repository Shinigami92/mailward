//! The `accounts.yaml` model: a list of mailboxes, each resolved against its
//! vendor's defaults into a fully-specified [`ResolvedAccount`].

use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_yaml_ng::Value;

use super::ConfigError;
use super::vendors::{AuthMethod, VENDOR_NAMES, get_vendor};

/// A fully-resolved account: vendor defaults merged with the account's overrides.
#[derive(Debug, Clone)]
pub struct ResolvedAccount {
    pub id: String,
    pub vendor: String,
    pub username: String,
    pub imap_host: String,
    pub imap_port: u16,
    pub auth_method: AuthMethod,
    pub client_id: Option<String>,
    pub authority: Option<String>,
    pub redirect_uri: Option<String>,
    pub scopes: Vec<String>,
    /// This account's cache directory (`<cache_root>/<id>/`); auth files live here.
    pub cache_dir: PathBuf,
    /// Name of the env var holding this account's password (password vendors).
    pub password_env: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct RawAccount {
    id: Option<String>,
    vendor: Option<String>,
    username: Option<String>,
    imap_host: Option<String>,
    imap_port: Option<u16>,
    client_id: Option<String>,
    authority: Option<String>,
    redirect_uri: Option<String>,
    scopes: Option<Vec<String>>,
    password_env: Option<String>,
}

fn is_valid_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn clean(value: Option<String>) -> Option<String> {
    value.filter(|s| !s.is_empty())
}

fn resolve_account(
    raw: RawAccount,
    index: usize,
    cache_root: &Path,
) -> Result<ResolvedAccount, ConfigError> {
    let id = match &raw.id {
        Some(s) if is_valid_id(s) => s.clone(),
        other => {
            return Err(ConfigError::InvalidId {
                index,
                id: other.clone(),
            });
        }
    };

    let vendor_name = clean(raw.vendor);
    let defaults =
        vendor_name
            .as_deref()
            .and_then(get_vendor)
            .ok_or_else(|| ConfigError::UnknownVendor {
                id: id.clone(),
                vendor: vendor_name.clone(),
                known: VENDOR_NAMES.join(", "),
            })?;
    let vendor = vendor_name.expect("vendor name present when defaults resolved");

    let username = clean(raw.username).ok_or_else(|| ConfigError::MissingUsername(id.clone()))?;

    let imap_host = clean(raw.imap_host).unwrap_or_else(|| defaults.imap_host.to_string());
    if imap_host.is_empty() {
        return Err(ConfigError::MissingHost {
            id: id.clone(),
            vendor: vendor.clone(),
        });
    }

    Ok(ResolvedAccount {
        cache_dir: cache_root.join(&id),
        id,
        vendor,
        username,
        imap_host,
        imap_port: raw.imap_port.unwrap_or(defaults.imap_port),
        auth_method: defaults.auth_method,
        client_id: clean(raw.client_id).or_else(|| defaults.client_id.map(String::from)),
        authority: clean(raw.authority).or_else(|| defaults.authority.map(String::from)),
        redirect_uri: clean(raw.redirect_uri).or_else(|| defaults.redirect_uri.map(String::from)),
        scopes: raw
            .scopes
            .unwrap_or_else(|| defaults.scopes.iter().map(|s| s.to_string()).collect()),
        password_env: clean(raw.password_env),
    })
}

/// Parses `accounts.yaml` (a non-empty list) into resolved accounts. `cache_root`
/// is the base dir for per-account cache directories (`<cache_root>/<id>/`).
pub fn load_accounts(yaml: &str, cache_root: &Path) -> Result<Vec<ResolvedAccount>, ConfigError> {
    let value: Value = serde_yaml_ng::from_str(yaml)?;
    match &value {
        Value::Sequence(seq) if !seq.is_empty() => {}
        _ => return Err(ConfigError::EmptyAccounts),
    }
    let raws: Vec<RawAccount> = serde_yaml_ng::from_value(value)?;

    let mut resolved = Vec::with_capacity(raws.len());
    for (index, raw) in raws.into_iter().enumerate() {
        resolved.push(resolve_account(raw, index, cache_root)?);
    }

    let mut seen = std::collections::HashSet::new();
    for account in &resolved {
        if !seen.insert(account.id.as_str()) {
            return Err(ConfigError::DuplicateId(account.id.clone()));
        }
    }
    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(yaml: &str) -> Result<Vec<ResolvedAccount>, ConfigError> {
        load_accounts(yaml, Path::new(".cache"))
    }

    #[test]
    fn resolves_vendor_defaults_and_overrides() {
        let accounts = load(
            r#"
- id: personal
  vendor: microsoft
  username: you@hotmail.com
- id: work-gmail
  vendor: gmail
  username: you@gmail.com
  passwordEnv: WORK_GMAIL_PASSWORD
  imapPort: 1993
"#,
        )
        .unwrap();

        assert_eq!(accounts.len(), 2);

        let personal = &accounts[0];
        assert_eq!(personal.imap_host, "outlook.office365.com");
        assert_eq!(personal.imap_port, 993);
        assert_eq!(personal.auth_method, AuthMethod::Xoauth2Msal);
        assert_eq!(
            personal.client_id.as_deref(),
            Some(super::super::vendors::THUNDERBIRD_CLIENT_ID)
        );
        assert_eq!(personal.cache_dir, Path::new(".cache").join("personal"));

        let work = &accounts[1];
        assert_eq!(work.imap_host, "imap.gmail.com");
        assert_eq!(work.imap_port, 1993); // overridden
        assert_eq!(work.auth_method, AuthMethod::Password);
        assert_eq!(work.password_env.as_deref(), Some("WORK_GMAIL_PASSWORD"));
    }

    #[test]
    fn rejects_invalid_id() {
        let err = load("- id: Personal\n  vendor: microsoft\n  username: a@b.com\n").unwrap_err();
        assert!(matches!(err, ConfigError::InvalidId { .. }));
    }

    #[test]
    fn rejects_unknown_vendor() {
        let err = load("- id: x\n  vendor: fastmail\n  username: a@b.com\n").unwrap_err();
        assert!(matches!(err, ConfigError::UnknownVendor { .. }));
    }

    #[test]
    fn rejects_duplicate_ids() {
        let err = load(
            "- id: x\n  vendor: microsoft\n  username: a@b.com\n- id: x\n  vendor: gmail\n  username: c@d.com\n",
        )
        .unwrap_err();
        assert!(matches!(err, ConfigError::DuplicateId(_)));
    }

    #[test]
    fn rejects_empty_file() {
        assert!(matches!(
            load("[]").unwrap_err(),
            ConfigError::EmptyAccounts
        ));
    }

    #[test]
    fn generic_vendor_requires_host() {
        let err = load("- id: x\n  vendor: generic\n  username: a@b.com\n").unwrap_err();
        assert!(matches!(err, ConfigError::MissingHost { .. }));
    }
}
