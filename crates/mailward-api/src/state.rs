//! Shared application state injected into the GraphQL schema.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use mailward_core::auth::PendingAuth;
use tokio::sync::broadcast;

use crate::settings::Settings;
use crate::types::MessageDecision;

/// Cloneable handle to shared server state.
#[derive(Clone)]
pub struct AppState {
    pub settings: Settings,
    /// In-progress interactive logins, keyed by account id (between authStart/authComplete).
    pub pending: Arc<Mutex<HashMap<String, PendingAuth>>>,
    /// Broadcast of per-message decisions, consumed by the `runProgress` subscription.
    pub events: broadcast::Sender<MessageDecision>,
}

impl AppState {
    pub fn new(settings: Settings) -> Self {
        let (events, _) = broadcast::channel(256);
        Self {
            settings,
            pending: Arc::new(Mutex::new(HashMap::new())),
            events,
        }
    }
}
