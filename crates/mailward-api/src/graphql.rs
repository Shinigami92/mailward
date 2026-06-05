//! The GraphQL schema: queries to read config/accounts, mutations to edit the
//! YAML files + drive the OAuth login + trigger runs, and a subscription that
//! streams live per-message decisions.

use async_graphql::{Context, Object, Schema, Subscription};
use futures::{Stream, StreamExt};
use mailward_core::auth::{begin_auth, finish_auth, load_cached_refresh, save_tokens};
use mailward_core::{ConfigDocument, ResolvedAccount, RulesDocument, load_accounts};
use tokio_stream::wrappers::BroadcastStream;

use crate::run::{list_account_folders, run};
use crate::state::AppState;
use crate::types::{AccountInfo, AuthStart, InspectedMessage, MessageDecision, RunMode, RunResult};

pub type ApiSchema = Schema<Query, Mutation, Subscription>;

pub fn build_schema(state: AppState) -> ApiSchema {
    Schema::build(Query, Mutation, Subscription)
        .data(state)
        .finish()
}

fn to_err<E: std::fmt::Display>(error: E) -> async_graphql::Error {
    async_graphql::Error::new(error.to_string())
}

fn read_file(state: &AppState, name: &str) -> async_graphql::Result<String> {
    std::fs::read_to_string(state.settings.config_file(name)).map_err(to_err)
}

fn load_account(state: &AppState, id: &str) -> async_graphql::Result<ResolvedAccount> {
    let accounts = load_accounts(
        &read_file(state, "accounts.yaml")?,
        &state.settings.cache_dir,
    )
    .map_err(to_err)?;
    accounts
        .into_iter()
        .find(|a| a.id == id)
        .ok_or_else(|| async_graphql::Error::new(format!("unknown account {id}")))
}

pub struct Query;

#[Object]
impl Query {
    /// Engine version.
    async fn version(&self) -> &'static str {
        mailward_core::VERSION
    }

    /// Configured accounts (no secrets), with whether each is signed in.
    async fn accounts(&self, ctx: &Context<'_>) -> async_graphql::Result<Vec<AccountInfo>> {
        let state = ctx.data::<AppState>()?;
        let accounts = load_accounts(
            &read_file(state, "accounts.yaml")?,
            &state.settings.cache_dir,
        )
        .map_err(to_err)?;
        Ok(accounts
            .iter()
            .map(|a| AccountInfo {
                id: a.id.clone(),
                vendor: a.vendor.clone(),
                username: a.username.clone(),
                imap_host: a.imap_host.clone(),
                auth_method: format!("{:?}", a.auth_method),
                authenticated: load_cached_refresh(a).is_some(),
            })
            .collect())
    }

    /// Raw `accounts.yaml` (for the editor).
    async fn accounts_yaml(&self, ctx: &Context<'_>) -> async_graphql::Result<String> {
        read_file(ctx.data::<AppState>()?, "accounts.yaml")
    }

    /// Raw `config.yaml` (for the editor).
    async fn config_yaml(&self, ctx: &Context<'_>) -> async_graphql::Result<String> {
        read_file(ctx.data::<AppState>()?, "config.yaml")
    }

    /// Raw `rules.yaml` (for the editor).
    async fn rules_yaml(&self, ctx: &Context<'_>) -> async_graphql::Result<String> {
        read_file(ctx.data::<AppState>()?, "rules.yaml")
    }

    /// The folders a classify run would scan for an account (auto-discovered, live).
    async fn folders(
        &self,
        ctx: &Context<'_>,
        account: String,
    ) -> async_graphql::Result<Vec<String>> {
        let state = ctx.data::<AppState>()?;
        list_account_folders(state, &account).await.map_err(to_err)
    }

    /// Folders available for read-only inspection (all selectable except confidential/ignored).
    async fn inspectable_folders(
        &self,
        ctx: &Context<'_>,
        account: String,
    ) -> async_graphql::Result<Vec<String>> {
        let state = ctx.data::<AppState>()?;
        crate::run::inspectable_folders(state, &account)
            .await
            .map_err(to_err)
    }

    /// Read-only view of a folder's messages. Uses BODY.PEEK - never marks mail read.
    async fn inspect(
        &self,
        ctx: &Context<'_>,
        account: String,
        folder: String,
        #[graphql(default = 50)] limit: i32,
        #[graphql(default = false)] unread_only: bool,
    ) -> async_graphql::Result<Vec<InspectedMessage>> {
        let state = ctx.data::<AppState>()?;
        let messages = crate::run::inspect_messages(
            state,
            &account,
            &folder,
            limit.max(0) as usize,
            unread_only,
        )
        .await
        .map_err(to_err)?;
        Ok(messages
            .into_iter()
            .map(InspectedMessage::from_message)
            .collect())
    }
}

pub struct Mutation;

#[Object]
impl Mutation {
    /// Validate and save `accounts.yaml`.
    async fn update_accounts_yaml(
        &self,
        ctx: &Context<'_>,
        content: String,
    ) -> async_graphql::Result<bool> {
        let state = ctx.data::<AppState>()?;
        // Validate by fully resolving before persisting.
        load_accounts(&content, &state.settings.cache_dir).map_err(to_err)?;
        std::fs::write(state.settings.config_file("accounts.yaml"), &content).map_err(to_err)?;
        Ok(true)
    }

    /// Validate and save `config.yaml`.
    async fn update_config_yaml(
        &self,
        ctx: &Context<'_>,
        content: String,
    ) -> async_graphql::Result<bool> {
        let state = ctx.data::<AppState>()?;
        ConfigDocument::from_yaml_str(&content)
            .and_then(|doc| doc.config_for(None))
            .map_err(to_err)?;
        std::fs::write(state.settings.config_file("config.yaml"), &content).map_err(to_err)?;
        Ok(true)
    }

    /// Validate (and compile) then save `rules.yaml`.
    async fn update_rules_yaml(
        &self,
        ctx: &Context<'_>,
        content: String,
    ) -> async_graphql::Result<bool> {
        let state = ctx.data::<AppState>()?;
        RulesDocument::from_yaml_str(&content)
            .and_then(|doc| doc.compile_for(None))
            .map_err(to_err)?;
        std::fs::write(state.settings.config_file("rules.yaml"), &content).map_err(to_err)?;
        Ok(true)
    }

    /// Begin an interactive sign-in: returns the URL to open. Pair with `authComplete`.
    async fn auth_start(
        &self,
        ctx: &Context<'_>,
        account: String,
    ) -> async_graphql::Result<AuthStart> {
        let state = ctx.data::<AppState>()?;
        let resolved = load_account(state, &account)?;
        let pending = begin_auth(&resolved).map_err(to_err)?;
        let authorize_url = pending.authorize_url.clone();
        state
            .pending
            .lock()
            .expect("pending-auth lock")
            .insert(account, pending);
        Ok(AuthStart { authorize_url })
    }

    /// Complete sign-in with the pasted redirect URL; caches the refresh token.
    async fn auth_complete(
        &self,
        ctx: &Context<'_>,
        account: String,
        redirect_url: String,
    ) -> async_graphql::Result<bool> {
        let state = ctx.data::<AppState>()?;
        let resolved = load_account(state, &account)?;
        let pending = state
            .pending
            .lock()
            .expect("pending-auth lock")
            .remove(&account)
            .ok_or_else(|| {
                async_graphql::Error::new(format!(
                    "no pending login for account {account}; call authStart first"
                ))
            })?;
        let tokens = finish_auth(&resolved, pending, redirect_url.trim())
            .await
            .map_err(to_err)?;
        save_tokens(&resolved, &tokens).map_err(to_err)?;
        Ok(true)
    }

    /// Run the unread-classify path. Dry run by default; pass `dryRun: false` to apply.
    async fn trigger_run(
        &self,
        ctx: &Context<'_>,
        account: Option<String>,
        #[graphql(default_with = "RunMode::Classify")] mode: RunMode,
        #[graphql(default = true)] dry_run: bool,
    ) -> async_graphql::Result<RunResult> {
        let state = ctx.data::<AppState>()?;
        run(state, account.as_deref(), mode, dry_run)
            .await
            .map_err(to_err)
    }
}

pub struct Subscription;

#[Subscription]
impl Subscription {
    /// Live stream of per-message decisions emitted while a run executes.
    async fn run_progress(&self, ctx: &Context<'_>) -> impl Stream<Item = MessageDecision> {
        let receiver = ctx.data_unchecked::<AppState>().events.subscribe();
        BroadcastStream::new(receiver).filter_map(|result| async move { result.ok() })
    }
}
