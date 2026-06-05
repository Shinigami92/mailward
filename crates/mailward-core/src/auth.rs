//! Vendor authentication. Only Microsoft/Outlook personal accounts are wired
//! today, via the OAuth2 **authorization-code flow with manual redirect-URL
//! paste** (the Thunderbird public client, no MSAL). The flow is split into
//! [`begin_auth`] / [`finish_auth`] / [`refresh`] so the GraphQL API can drive
//! the browser dance over two mutations; [`authenticate`] ties them together for
//! a terminal (the dry-run example) using a caller-supplied paste prompt.

use std::fs;
use std::path::Path;

use oauth2::basic::BasicClient;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, CsrfToken, PkceCodeChallenge, PkceCodeVerifier,
    RedirectUrl, RefreshToken, Scope, TokenResponse, TokenUrl,
};
use serde::{Deserialize, Serialize};

use crate::config::{AuthMethod, ResolvedAccount};

/// Tokens obtained from a successful auth or refresh.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenSet {
    pub access_token: String,
    /// Present after an interactive login (we request `offline_access`); a refresh
    /// may or may not return a new one.
    pub refresh_token: Option<String>,
    /// Unix seconds when the access token expires, if the server told us.
    pub expires_at: Option<i64>,
}

/// An in-progress interactive login: show `authorize_url`, collect the pasted
/// redirect URL, then pass this back to [`finish_auth`].
pub struct PendingAuth {
    pub authorize_url: String,
    verifier: PkceCodeVerifier,
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("auth: vendor {vendor} ({method:?}) is not implemented yet")]
    VendorNotImplemented { vendor: String, method: AuthMethod },
    #[error("auth: account {0} is missing OAuth config (clientId/authority/redirectUri)")]
    MissingOAuthConfig(String),
    #[error("auth: no cached login for account {0} - interactive sign-in required")]
    NeedsLogin(String),
    #[error("auth: the pasted redirect URL has no ?code= parameter")]
    NoAuthCode,
    #[error("auth: invalid URL: {0}")]
    Url(#[from] url::ParseError),
    #[error("auth: token request failed: {0}")]
    Token(String),
    #[error("auth: cache I/O at {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("auth: cache parse: {0}")]
    Parse(#[from] serde_json::Error),
}

/// The MSAL/OAuth endpoints + client derived from an account, or an error if the
/// account isn't an OAuth (xoauth2-msal) account or is missing config.
struct OAuthConfig {
    client_id: String,
    auth_url: String,
    token_url: String,
    redirect_uri: String,
    scopes: Vec<String>,
}

fn oauth_config(account: &ResolvedAccount) -> Result<OAuthConfig, AuthError> {
    if account.auth_method != AuthMethod::Xoauth2Msal {
        return Err(AuthError::VendorNotImplemented {
            vendor: account.vendor.clone(),
            method: account.auth_method,
        });
    }
    let authority = account
        .authority
        .as_deref()
        .ok_or_else(|| AuthError::MissingOAuthConfig(account.id.clone()))?;
    let client_id = account
        .client_id
        .clone()
        .ok_or_else(|| AuthError::MissingOAuthConfig(account.id.clone()))?;
    let redirect_uri = account
        .redirect_uri
        .clone()
        .ok_or_else(|| AuthError::MissingOAuthConfig(account.id.clone()))?;

    // `offline_access` is required to receive a refresh token for unattended runs.
    let mut scopes = account.scopes.clone();
    if !scopes.iter().any(|s| s == "offline_access") {
        scopes.push("offline_access".to_string());
    }

    Ok(OAuthConfig {
        client_id,
        auth_url: format!("{authority}/oauth2/v2.0/authorize"),
        token_url: format!("{authority}/oauth2/v2.0/token"),
        redirect_uri,
        scopes,
    })
}

/// Builds the OAuth client. The concrete typestate is unwieldy, so callers build
/// it inline and use it immediately rather than storing it.
fn build_client(
    config: &OAuthConfig,
) -> Result<
    BasicClient<
        oauth2::EndpointSet,
        oauth2::EndpointNotSet,
        oauth2::EndpointNotSet,
        oauth2::EndpointNotSet,
        oauth2::EndpointSet,
    >,
    AuthError,
> {
    Ok(BasicClient::new(ClientId::new(config.client_id.clone()))
        .set_auth_uri(AuthUrl::new(config.auth_url.clone())?)
        .set_token_uri(TokenUrl::new(config.token_url.clone())?)
        .set_redirect_uri(RedirectUrl::new(config.redirect_uri.clone())?))
}

fn http_client() -> Result<oauth2::reqwest::Client, AuthError> {
    oauth2::reqwest::ClientBuilder::new()
        // Don't follow the redirect to https://localhost - we read the code from it.
        .redirect(oauth2::reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| AuthError::Token(e.to_string()))
}

/// Step 1: produce the sign-in URL (+ PKCE verifier) to show the user.
pub fn begin_auth(account: &ResolvedAccount) -> Result<PendingAuth, AuthError> {
    let config = oauth_config(account)?;
    let client = build_client(&config)?;
    let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();

    let mut request = client.authorize_url(CsrfToken::new_random);
    for scope in &config.scopes {
        request = request.add_scope(Scope::new(scope.clone()));
    }
    let (url, _csrf) = request.set_pkce_challenge(challenge).url();

    Ok(PendingAuth {
        authorize_url: url.to_string(),
        verifier,
    })
}

/// Step 2: exchange the code from the pasted redirect URL for tokens.
pub async fn finish_auth(
    account: &ResolvedAccount,
    pending: PendingAuth,
    redirect_url: &str,
) -> Result<TokenSet, AuthError> {
    let config = oauth_config(account)?;
    let client = build_client(&config)?;

    let code = url::Url::parse(redirect_url)?
        .query_pairs()
        .find(|(k, _)| k == "code")
        .map(|(_, v)| v.into_owned())
        .ok_or(AuthError::NoAuthCode)?;

    let http = http_client()?;
    let token = client
        .exchange_code(AuthorizationCode::new(code))
        .set_pkce_verifier(pending.verifier)
        .request_async(&http)
        .await
        .map_err(|e| AuthError::Token(e.to_string()))?;

    Ok(token_set(&token))
}

/// Exchanges a stored refresh token for a fresh access token.
pub async fn refresh(
    account: &ResolvedAccount,
    refresh_token: &str,
) -> Result<TokenSet, AuthError> {
    let config = oauth_config(account)?;
    let client = build_client(&config)?;
    let http = http_client()?;

    let token = client
        .exchange_refresh_token(&RefreshToken::new(refresh_token.to_string()))
        .request_async(&http)
        .await
        .map_err(|e| AuthError::Token(e.to_string()))?;

    let mut set = token_set(&token);
    // A refresh response often omits a new refresh token; keep the old one.
    if set.refresh_token.is_none() {
        set.refresh_token = Some(refresh_token.to_string());
    }
    Ok(set)
}

fn token_set(token: &oauth2::basic::BasicTokenResponse) -> TokenSet {
    let expires_at = token.expires_in().map(|d| {
        let secs = d.as_secs() as i64;
        chrono::Utc::now().timestamp() + secs
    });
    TokenSet {
        access_token: token.access_token().secret().clone(),
        refresh_token: token.refresh_token().map(|r| r.secret().clone()),
        expires_at,
    }
}

fn cache_path(account: &ResolvedAccount) -> std::path::PathBuf {
    account.cache_dir.join("oauth.json")
}

/// Loads the cached refresh token for an account, if any.
pub fn load_cached_refresh(account: &ResolvedAccount) -> Option<String> {
    let raw = fs::read_to_string(cache_path(account)).ok()?;
    let cached: TokenSet = serde_json::from_str(&raw).ok()?;
    cached.refresh_token
}

/// Persists tokens to `<cache_dir>/oauth.json`. The refresh token is
/// password-equivalent, so the directory and file are created owner-only
/// (`0700`/`0600`) on Unix - with the mode set at creation, leaving no window
/// where the file is world-readable. On non-Unix platforms the OS default applies.
pub fn save_tokens(account: &ResolvedAccount, tokens: &TokenSet) -> Result<(), AuthError> {
    let dir = &account.cache_dir;
    create_private_dir(dir).map_err(|source| AuthError::Io {
        path: dir.display().to_string(),
        source,
    })?;
    let path = cache_path(account);
    let json = serde_json::to_string_pretty(tokens)?;
    write_private(&path, &json).map_err(|source| AuthError::Io {
        path: path.display().to_string(),
        source,
    })
}

/// Creates a directory (recursively) owner-only on Unix.
fn create_private_dir(dir: &Path) -> std::io::Result<()> {
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(dir)
}

/// Writes a file owner-only on Unix, with the mode applied at creation time.
fn write_private(path: &Path, contents: &str) -> std::io::Result<()> {
    use std::io::Write;
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(contents.as_bytes())
}

/// High-level: return a usable access token, reusing the cached refresh token when
/// possible and otherwise running the interactive flow via `paste` (which is shown
/// the authorize URL and returns the pasted redirect URL).
pub async fn authenticate<F>(account: &ResolvedAccount, paste: F) -> Result<String, AuthError>
where
    F: FnOnce(&str) -> std::io::Result<String>,
{
    // Ensure this is an implemented vendor before doing any I/O.
    let _ = oauth_config(account)?;

    // Reuse the cached refresh token when it still works; otherwise fall through
    // to an interactive login.
    if let Some(refresh_token) = load_cached_refresh(account)
        && let Ok(tokens) = refresh(account, &refresh_token).await
    {
        save_tokens(account, &tokens)?;
        return Ok(tokens.access_token);
    }

    let pending = begin_auth(account)?;
    let redirect = paste(&pending.authorize_url).map_err(|source| AuthError::Io {
        path: "<stdin>".to_string(),
        source,
    })?;
    let tokens = finish_auth(account, pending, redirect.trim()).await?;
    save_tokens(account, &tokens)?;
    Ok(tokens.access_token)
}
