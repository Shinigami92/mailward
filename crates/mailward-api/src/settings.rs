//! Runtime settings from the environment (with sensible defaults for dev).

use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Settings {
    /// Directory holding `accounts.yaml` / `config.yaml` / `rules.yaml`.
    pub config_dir: PathBuf,
    /// Base directory for per-account token caches (`<cache_dir>/<id>/`).
    pub cache_dir: PathBuf,
    /// Directory of the built SPA to serve (created in Phase 4 / the Docker build).
    pub web_dir: PathBuf,
    /// Address to bind the HTTP server to.
    pub bind_addr: String,
}

fn var_or(key: &str, default: &str) -> String {
    std::env::var(key)
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| default.to_string())
}

impl Settings {
    pub fn from_env() -> Self {
        Self {
            config_dir: var_or("MAILWARD_CONFIG_DIR", ".").into(),
            cache_dir: var_or("MAILWARD_CACHE_DIR", ".cache").into(),
            web_dir: var_or("MAILWARD_WEB_DIR", "apps/web/dist").into(),
            bind_addr: var_or("MAILWARD_BIND", "127.0.0.1:8080"),
        }
    }

    pub fn config_file(&self, name: &str) -> PathBuf {
        self.config_dir.join(name)
    }
}
