# mailward

A **"Pi-hole for your inbox"** - it signs into your mailbox(es) over **IMAP**, looks at unread mail,
and automatically **marks it read** or **moves it to Trash** based on rules richer than a webmail's
built-in filters allow. You configure accounts, folders and rules from a **web UI**; the engine runs
on a small server and keeps your inbox quiet.

> ### 🚧 v2 rewrite
>
> mailward began as a single-account TypeScript CLI ([published history](https://github.com/Shinigami92/mailward/commits/main)).
> It has been rebuilt as a **self-hostable app**: a Rust engine + GraphQL API and a Vue web UI,
> shipped as a Docker image. It is functional today - classify / cleanup / inspect, dry-run by
> default - and runs from a single container (see [Run with Docker](#run-with-docker)). The v1 rule
> behavior and privacy guarantees were ported 1:1.

## Architecture

| Piece                               | Tech                                                                                                                                             |
| ----------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Engine** (`crates/mailward-core`) | Rust - IMAP (`async-imap`, IDLE-ready), OAuth/password auth, rule DSL, YAML config model                                                         |
| **API** (`crates/mailward-api`)     | Rust - `axum` + `async-graphql` (GraphQL with WebSocket **subscriptions** for live run progress); also serves the built SPA                      |
| **Web UI** (`apps/web`)             | Vue 3 + Vite SPA - Tailwind 4, Reka UI, urql, hsml templates; responsive for iPhone / iPad / desktop                                             |
| **Config**                          | Plain YAML files (`accounts.yaml` / `config.yaml` / `rules.yaml`) on a mounted volume - the UI reads & writes them; they are the source of truth |
| **Deploy**                          | One container (the API binary serves the SPA), via Docker Compose                                                                                |

The whole thing is **provider-neutral IMAP**; only authentication is vendor-specific (Microsoft /
Outlook is wired first via the Thunderbird-OAuth trick - no Azure app, no credit card).

### Principles (carried over from v1)

- **Dry-run by default** - nothing is changed until you opt in.
- **`delete` = move to Trash**, never a hard delete (recoverable).
- **Confidential folders are never touched** and never fed to any classifier.
- **Static rules first.** Any future AI classifier is a last-resort, **local/offline** option.

## Run with Docker

```sh
mkdir -p config
# Put your accounts.yaml / config.yaml / rules.yaml in ./config - copy the *.example.yaml
# templates and rename them, or just create them later from the web UI.
docker compose up -d
```

Open **http://localhost:8080**. The web UI reads and writes the YAML files in `./config` (the
source of truth); OAuth refresh tokens persist in a named volume. Released images are published to
`ghcr.io/shinigami92/mailward` (tagged per release).

## Develop

```sh
just install   # pnpm install + cargo fetch
just dev       # API (:8080) + web UI (:5173) together; open http://localhost:5173
```

Run them separately with `just dev-api` / `just dev-web` if you prefer two terminals. See
**[CONTRIBUTING.md](CONTRIBUTING.md)** for prerequisites, the repo layout and the full `just` task list.

## Roadmap

- [x] **v1**: single-account TypeScript CLI (IMAP triage, rule DSL, YAML config) - _published_
- [x] **v2 - Phase 0**: monorepo skeleton (Cargo + pnpm workspaces, `just`, CI, tooling)
- [x] **Phase 1**: ported the rule engine (DSL + refs + classifier + per-account overrides) to `mailward-core`, with snapshot parity tests
- [x] **Phase 2**: IMAP + OAuth in Rust (rustls + `async-imap` XOAUTH2, `oauth2` manual-paste); dry-run verified against a real mailbox via `examples/dry_run.rs`
- [x] **Phase 3**: `mailward-api` - axum + async-graphql (queries, mutations incl. `triggerRun`/auth, a `runProgress` subscription), serves the SPA; verified live end-to-end
- [x] **Phase 4**: web UI (hsml templates via `vite-plugin-vue-hsml`) - nav, run view (Classify/Cleanup, live `runProgress` subscription, dry-run by default + Apply), read-only **Inspect** view (`BODY.PEEK` - never marks read; confidential folders excluded), accounts + sign-in flow, config/rules/accounts YAML editors. Backend reached v1 parity: folder auto-discovery, `\Trash` discovery, cleanup + inspect modes (confidential folders never scanned)
- [x] **Phase 5**: Docker image (multi-stage) + Compose + a GHCR publish workflow (build on PRs, push on `v*` tags)
- [ ] **Later**: IMAP IDLE / cron triggers, Prometheus/Grafana stats, optional local-LLM classifier ← _here_
- [ ] **Backlog**: building-block rule builder UI (visual editor over the DSL - dedicated PR; the YAML editors cover it for now)
- [ ] **Backlog**: live run experience - stream `runProgress` decisions into the table as they arrive (with a progress indicator and per-account grouping) instead of only showing the final result
- [ ] **Backlog**: dashboard / overview home - a Pi-hole-style landing page with accounts sign-in status, last-run summary/counts, and quick actions

## License

[MIT](LICENSE) © Christopher Quadflieg
