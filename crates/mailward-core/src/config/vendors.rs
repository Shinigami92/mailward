//! Vendor registry: per-provider connection + auth defaults. An account picks a
//! `vendor` and inherits these; any field can be overridden per account.
//!
//! Only `xoauth2-msal` (Microsoft/Outlook personal) is wired today. `password`
//! (app-password / generic IMAP) and `xoauth2-google` are declared but not yet
//! implemented - such accounts are skipped with a warning by the auth layer.

use serde::Serialize;

/// How an account authenticates to its IMAP server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AuthMethod {
    Xoauth2Msal,
    Xoauth2Google,
    Password,
}

/// Connection + auth defaults for a vendor.
#[derive(Debug, Clone, Copy)]
pub struct VendorDefaults {
    pub imap_host: &'static str,
    pub imap_port: u16,
    pub auth_method: AuthMethod,
    /// OAuth (MSAL) specifics - only meaningful for `xoauth2-msal`.
    pub client_id: Option<&'static str>,
    pub authority: Option<&'static str>,
    pub redirect_uri: Option<&'static str>,
    pub scopes: &'static [&'static str],
}

/// Mozilla Thunderbird's public OAuth client ID. Personal Microsoft accounts can
/// no longer register their own app without an (paid/gated) Entra directory, so we
/// reuse this well-known public client - the same one Thunderbird and
/// `mutt_oauth2.py` use. The first-run consent screen therefore shows "Mozilla
/// Thunderbird"; override via an account's `clientId` if you register your own app.
pub const THUNDERBIRD_CLIENT_ID: &str = "9e5f94bc-e8a4-4e73-b8be-63364c29d753";

/// Known vendor names, in registry order.
pub const VENDOR_NAMES: &[&str] = &["microsoft", "gmail", "generic"];

/// Looks up a vendor's defaults by name.
pub fn get_vendor(name: &str) -> Option<VendorDefaults> {
    Some(match name {
        "microsoft" => VendorDefaults {
            imap_host: "outlook.office365.com",
            imap_port: 993,
            auth_method: AuthMethod::Xoauth2Msal,
            client_id: Some(THUNDERBIRD_CLIENT_ID),
            // "common" (not "consumers") is required for the borrowed multi-tenant
            // client to accept a personal account through the auth-code flow.
            authority: Some("https://login.microsoftonline.com/common"),
            redirect_uri: Some("https://localhost"),
            scopes: &["https://outlook.office.com/IMAP.AccessAsUser.All"],
        },
        "gmail" => VendorDefaults {
            imap_host: "imap.gmail.com",
            imap_port: 993,
            // App-password over IMAP for now; a Google-OAuth path can come later.
            auth_method: AuthMethod::Password,
            client_id: None,
            authority: None,
            redirect_uri: None,
            scopes: &[],
        },
        "generic" => VendorDefaults {
            // No sensible host default - a generic account must set imapHost itself.
            imap_host: "",
            imap_port: 993,
            auth_method: AuthMethod::Password,
            client_id: None,
            authority: None,
            redirect_uri: None,
            scopes: &[],
        },
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn microsoft_defaults_use_the_thunderbird_client() {
        let v = get_vendor("microsoft").unwrap();
        assert_eq!(v.auth_method, AuthMethod::Xoauth2Msal);
        assert_eq!(v.client_id, Some(THUNDERBIRD_CLIENT_ID));
        assert_eq!(v.imap_port, 993);
    }

    #[test]
    fn unknown_vendor_is_none() {
        assert!(get_vendor("fastmail").is_none());
    }
}
