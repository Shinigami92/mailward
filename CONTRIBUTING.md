# Contributing to mailward

Thanks for your interest! mailward is a polyglot monorepo: a **Rust** engine + API and a
**TypeScript / Vue** web UI, orchestrated with [`just`](https://just.systems).

## Prerequisites

| Tool            | Version                                | Notes                                                                     |
| --------------- | -------------------------------------- | ------------------------------------------------------------------------- |
| **Rust**        | 1.96 (pinned in `rust-toolchain.toml`) | `rustup` auto-selects it; includes `rustfmt` + `clippy`                   |
| **Node.js**     | ≥ 26                                   |                                                                           |
| **pnpm**        | 11 (pinned via `packageManager`)       | `npm install -g pnpm@11.5.2` (Node 26 removed corepack)                   |
| **just**        | latest                                 | `winget install Casey.Just` · `cargo install just` · `scoop install just` |
| **cargo-watch** | latest (optional)                      | `cargo install cargo-watch` - gives the backend hot-reload in `just dev`  |

## Repo layout

```
crates/
  mailward-core/   engine: IMAP, auth, rule DSL, YAML config model (library)
  mailward-api/    axum + async-graphql server; serves the SPA (binary)
apps/
  web/             Vue 3 + Vite SPA (Tailwind 4, Reka UI, urql, hsml templates); e2e/ = Playwright
schemas/           JSON Schemas for the YAML config files
*.example.yaml     sanitized config templates (copy to the gitignored real files)
```

Workspaces: `Cargo.toml` (`[workspace]`) for Rust, `pnpm-workspace.yaml` for JS.

## Tasks (`just`)

| Command                         | What it does                                                                                            |
| ------------------------------- | ------------------------------------------------------------------------------------------------------- |
| `just install`                  | `pnpm install` + `cargo fetch`                                                                          |
| `just dev`                      | run API + web UI together (one terminal; Ctrl+C stops both); needs `cargo-watch`                        |
| `just dev-api` / `just dev-web` | run just the API / just the Vue dev server in separate terminals                                        |
| `just fmt`                      | format everything (`oxfmt` + `cargo fmt`)                                                               |
| `just verify`                   | the quality gate: `oxfmt --check`, `oxlint`, `vue-tsc`, `cargo fmt --check`, `cargo clippy -D warnings` |
| `just test`                     | `cargo test` + frontend unit tests (vitest)                                                             |
| `just e2e`                      | Playwright end-to-end tests                                                                             |
| `just build`                    | build the SPA + the release API binary                                                                  |
| `just ci`                       | `verify` + `test` + `build`                                                                             |

`run any recipe's underlying command directly if you don't have just installed - see the justfile.`

## Code style & quality

- **JS/TS/Vue**: [`oxfmt`](https://oxc.rs) (format) + [`oxlint`](https://oxc.rs) (lint). Type-checked
  with `vue-tsc` / `tsc`.
- **Rust**: `cargo fmt` + `cargo clippy` (warnings are denied in CI).
- **No git hooks.** Quality is enforced by **CI** and, locally, by a project **`.claude` Stop hook**
  that runs `just verify` after each Claude Code turn (advisory - it reports problems, doesn't block).
- Tests: Rust uses `cargo test` + [`insta`](https://insta.rs) snapshots; the frontend uses
  **vitest** (unit) and **Playwright** (e2e). Mock data comes from
  [`@faker-js/faker`](https://fakerjs.dev).

## Commits

This repo follows **[Conventional Commits](https://www.conventionalcommits.org/)** (`feat:`, `fix:`,
`chore:`, `docs:`, `refactor:`, ...). It is **enforced in CI** via commitlint, so PRs with
non-conforming commit messages will fail. Dependencies are kept current by **Renovate**.

## Working on the engine

The v1 TypeScript implementation is the behavioral reference; it lives in git history (the
`feat: init`-era commits on `main`). When porting a rule or behavior to `mailward-core`, add an
`insta` snapshot test that reproduces the v1 decision so parity is provable.

> **Privacy is non-negotiable.** Confidential folders are never inspected or sent to any classifier;
> `delete` always means _move to Trash_; everything is dry-run until explicitly applied. Don't
> regress these invariants.
