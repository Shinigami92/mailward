//! The `config.yaml` model - folder policy, CLI defaults and IMAP tuning - with
//! per-account overrides and full defaulting.

use std::collections::HashMap;

use serde::Deserialize;
use serde_yaml_ng::Value;

use super::ConfigError;
use crate::deep_merge::deep_merge;

/// Folder selection + privacy policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoldersConfig {
    /// Folders to scan; `None` = auto-discover all selectable folders.
    pub scan: Option<Vec<String>>,
    /// IMAP special-use flags to exclude from auto-discovery (e.g. `\Sent`).
    pub skip_special_use: Vec<String>,
    /// Folder names (lower-cased) to exclude from auto-discovery.
    pub skip_names: Vec<String>,
    /// Folders never scanned/touched, even if named explicitly.
    pub ignore: Vec<String>,
    /// Sensitive folders: opt-in only, never exposed to any AI.
    pub confidential: Vec<String>,
}

impl FoldersConfig {
    /// Folder is off-limits entirely (never scan/inspect/clean, even if named).
    pub fn is_ignored(&self, folder: &str) -> bool {
        self.ignore.iter().any(|f| f == folder)
    }

    /// Folder holds sensitive data: opt-in only, never exposed to any AI/external service.
    pub fn is_confidential(&self, folder: &str) -> bool {
        self.confidential.iter().any(|f| f == folder)
    }
}

/// Default targets/sizes for the CLI modes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefaultsConfig {
    /// Folder used by inspect / cleanup when `--folder` is omitted.
    pub folder: String,
    /// inspect default max messages.
    pub inspect_limit: u32,
    /// cleanup sweep window size (recent messages scanned).
    pub cleanup_limit: u32,
}

/// IMAP backend tuning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImapConfig {
    /// Folder to move deletions to if no `\Trash` special-use folder is found.
    pub trash_fallback: String,
    /// How much decoded body text to keep as the preview (for rule matching).
    pub body_preview_chars: usize,
}

/// Fully-defaulted operational config for one account (or the shared base).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileConfig {
    pub folders: FoldersConfig,
    pub defaults: DefaultsConfig,
    pub imap: ImapConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct RawFileConfig {
    folders: RawFolders,
    defaults: RawDefaults,
    imap: RawImap,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct RawFolders {
    scan: Option<Vec<String>>,
    skip_special_use: Option<Vec<String>>,
    skip_names: Option<Vec<String>>,
    ignore: Option<Vec<String>>,
    confidential: Option<Vec<String>>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct RawDefaults {
    folder: Option<String>,
    inspect_limit: Option<u32>,
    cleanup_limit: Option<u32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct RawImap {
    trash_fallback: Option<String>,
    body_preview_chars: Option<usize>,
}

fn non_empty(value: Option<String>, fallback: &str) -> String {
    value
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

fn owned(values: &[&str]) -> Vec<String> {
    values.iter().map(|s| s.to_string()).collect()
}

/// Applies defaults to a raw parsed config (mirrors the v1 `normalize`).
fn normalize(raw: RawFileConfig) -> FileConfig {
    let folders = raw.folders;
    FileConfig {
        folders: FoldersConfig {
            scan: folders.scan,
            skip_special_use: folders
                .skip_special_use
                .unwrap_or_else(|| owned(&["\\Sent", "\\Drafts", "\\Trash"])),
            skip_names: folders
                .skip_names
                .unwrap_or_else(|| owned(&["outbox", "notes"]))
                .iter()
                .map(|n| n.to_lowercase())
                .collect(),
            ignore: folders.ignore.unwrap_or_default(),
            confidential: folders.confidential.unwrap_or_default(),
        },
        defaults: DefaultsConfig {
            folder: non_empty(raw.defaults.folder, "INBOX"),
            inspect_limit: raw.defaults.inspect_limit.unwrap_or(30),
            cleanup_limit: raw.defaults.cleanup_limit.unwrap_or(500),
        },
        imap: ImapConfig {
            trash_fallback: non_empty(raw.imap.trash_fallback, "Deleted"),
            body_preview_chars: raw.imap.body_preview_chars.unwrap_or(2000),
        },
    }
}

/// A loaded `config.yaml`: the shared base plus per-account override blocks (keyed
/// by account id), each deep-merged over the base.
pub struct ConfigDocument {
    base: Value,
    accounts: HashMap<String, Value>,
}

impl ConfigDocument {
    /// Parses a `config.yaml` string, splitting off the optional `accounts:` map.
    pub fn from_yaml_str(yaml: &str) -> Result<Self, ConfigError> {
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

    /// The effective config for one account (or the shared base when `account` is
    /// `None` or has no override block).
    pub fn config_for(&self, account: Option<&str>) -> Result<FileConfig, ConfigError> {
        let merged = match account.and_then(|id| self.accounts.get(id)) {
            Some(override_block) => deep_merge(self.base.clone(), override_block.clone()),
            None => self.base.clone(),
        };
        let raw: RawFileConfig = serde_yaml_ng::from_value(merged)?;
        Ok(normalize(raw))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: &str = include_str!("../../../../config.example.yaml");

    #[test]
    fn example_config_normalizes_with_expected_values() {
        let doc = ConfigDocument::from_yaml_str(EXAMPLE).unwrap();
        let config = doc.config_for(None).unwrap();

        assert_eq!(config.folders.scan, None);
        assert_eq!(
            config.folders.skip_special_use,
            owned(&["\\Sent", "\\Drafts", "\\Trash"])
        );
        assert_eq!(config.folders.skip_names, owned(&["outbox", "notes"]));
        assert!(config.folders.is_confidential("Archive/Banking"));
        assert!(config.folders.is_ignored("Archive/Old Photos"));
        assert!(!config.folders.is_confidential("INBOX"));

        assert_eq!(config.defaults.folder, "INBOX");
        assert_eq!(config.defaults.inspect_limit, 30);
        assert_eq!(config.defaults.cleanup_limit, 500);
        assert_eq!(config.imap.trash_fallback, "Deleted");
        assert_eq!(config.imap.body_preview_chars, 2000);
    }

    #[test]
    fn empty_config_uses_all_defaults() {
        let doc = ConfigDocument::from_yaml_str("{}").unwrap();
        let config = doc.config_for(None).unwrap();
        assert_eq!(config.defaults.folder, "INBOX");
        assert_eq!(config.imap.trash_fallback, "Deleted");
        assert!(config.folders.scan.is_none());
    }

    #[test]
    fn per_account_override_merges_over_base() {
        let yaml = r#"
defaults:
  folder: INBOX
folders:
  confidential: [Archive/Banking]
accounts:
  work:
    defaults:
      folder: Inbox/Work
    folders:
      confidential: [Finance]
"#;
        let doc = ConfigDocument::from_yaml_str(yaml).unwrap();
        let base = doc.config_for(None).unwrap();
        let work = doc.config_for(Some("work")).unwrap();

        assert_eq!(base.defaults.folder, "INBOX");
        assert_eq!(base.folders.confidential, owned(&["Archive/Banking"]));
        // Override replaces the scalar and the array, but inherits everything else.
        assert_eq!(work.defaults.folder, "Inbox/Work");
        assert_eq!(work.folders.confidential, owned(&["Finance"]));
        assert_eq!(work.imap.trash_fallback, "Deleted");
    }
}
