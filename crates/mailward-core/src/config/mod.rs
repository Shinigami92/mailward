//! The YAML config model: vendor registry, `config.yaml` (folders/defaults/imap)
//! and `accounts.yaml`, all with per-account overrides where applicable.

mod accounts;
mod file;
mod vendors;

pub use accounts::{ResolvedAccount, load_accounts};
pub use file::{ConfigDocument, DefaultsConfig, FileConfig, FoldersConfig, ImapConfig};
pub use vendors::{AuthMethod, THUNDERBIRD_CLIENT_ID, VENDOR_NAMES, VendorDefaults, get_vendor};

/// Everything that can go wrong while loading config/accounts YAML.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("yaml error: {0}")]
    Yaml(#[from] serde_yaml_ng::Error),

    #[error("accounts.yaml: expected a non-empty list of accounts")]
    EmptyAccounts,

    #[error(
        "accounts.yaml: account #{index} has an invalid \"id\" (need lowercase letters/digits/hyphens): {id:?}"
    )]
    InvalidId { index: usize, id: Option<String> },

    #[error("accounts.yaml: account {id:?} has unknown vendor {vendor:?} (known: {known})")]
    UnknownVendor {
        id: String,
        vendor: Option<String>,
        known: String,
    },

    #[error("accounts.yaml: account {0:?} is missing \"username\"")]
    MissingUsername(String),

    #[error("accounts.yaml: account {id:?} (vendor {vendor}) needs an \"imapHost\"")]
    MissingHost { id: String, vendor: String },

    #[error("accounts.yaml: duplicate account id {0:?}")]
    DuplicateId(String),
}
