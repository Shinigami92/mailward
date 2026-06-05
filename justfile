# mailward task runner - spans the Cargo (Rust) and pnpm (JS/Vue) toolchains.
# Install just: `winget install Casey.Just` | `cargo install just` | `scoop install just`

# Use PowerShell on Windows; sh elsewhere (CI).
set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

# List available recipes.
default:
    @just --list

# Install all dependencies (JS + fetch Rust crates).
install:
    pnpm install
    cargo fetch

# Run the backend + frontend together (one terminal; Ctrl+C stops both). Open http://localhost:5173.
dev:
    pnpm dev

# Run just the backend (GraphQL API on :8080).
dev-api:
    cargo run -p mailward-api

# Run just the frontend dev server (:5173, proxies /graphql to the API).
dev-web:
    pnpm -C apps/web dev

# Format everything.
fmt:
    pnpm format
    cargo fmt --all

# Quality gate used by the .claude Stop hook: format check + lint + type-check + clippy.
verify:
    pnpm format:check
    pnpm lint
    pnpm -C apps/web ts-check
    cargo fmt --all --check
    cargo clippy --all-targets --all-features -- -D warnings

# Run all tests (Rust + frontend unit).
test:
    cargo test --all
    pnpm -C apps/web test

# End-to-end tests (Playwright).
e2e:
    pnpm -C apps/web test:e2e

# Build release artifacts (SPA + API binary).
build:
    pnpm -C apps/web build
    cargo build --release

# Full CI gate, locally.
ci: verify test build
