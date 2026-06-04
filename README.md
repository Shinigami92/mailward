# mailward

A "Pi-hole for your inbox" — a small TypeScript/Node.js tool that signs into a personal mailbox
over **IMAP**, looks at **unread** messages, and automatically **marks them read** or **moves them
to Trash** based on rules richer than a webmail's built-in filters allow.

The engine is **provider-neutral** (any IMAP server); the only vendor-specific piece is
authentication. **Outlook / Hotmail is supported first** (the OAuth flow below); a generic
password / app-password provider for Gmail, Fastmail, self-hosted IMAP, etc. is a planned addition.

Runs manually for now; designed to grow into a cron job / mini-server daemon (and later an
AI classifier and Grafana stats — see [Roadmap](#roadmap)).

## How it works

- **Multiple accounts, one run.** Accounts are declared in **`accounts.yaml`**; `pnpm dev`,
  `--inspect` and `--cleanup` process every account in turn (per-account output sections). Each
  account picks a **vendor** (`microsoft` | `gmail` | `generic`) whose connection + auth defaults
  come from the code **vendor registry** (`src/vendors.ts`) and can be overridden per account.
  Run just one with `--account <id>`.
- Reads/edits each mailbox over **IMAP** using **`imapflow`**, scanning **multiple folders** — Inbox,
  Junk, and custom folders like `GitHub`/`Meetup`. By default it auto-discovers every selectable
  folder except Sent/Drafts/Trash; folder selection (scan list, skips, ignore, confidential) is
  configured in **`config.yaml`**.
- **Auth is the only vendor-specific part**, isolated behind a per-vendor strategy. **Microsoft /
  Outlook is wired today** via **OAuth2 (XOAUTH2)** — personal accounts no longer allow basic-auth
  IMAP — using the **authorization-code flow with manual code paste**: you sign in once in a browser,
  paste back the redirect URL, and a refresh token is cached per account (`.cache/<id>/msal.json`)
  so later runs are unattended. `gmail`/`generic` (password / app-password) are recognized but **not
  implemented yet** — such accounts are skipped with a warning.
- **No app registration / no Azure account / no credit card needed** for Microsoft: we reuse
  **Mozilla Thunderbird's public OAuth client ID** (the same trick `mutt_oauth2.py` uses for personal
  Outlook), so the consent screen shows *"Mozilla Thunderbird"*. Override it per account with your
  own `clientId` if you register an Entra app. See [`src/vendors.ts`](src/vendors.ts).
- A pluggable **`Classifier`** decides `keep` / `markRead` / `delete` per message — a deterministic
  **rule engine** (`rules.yaml`); an AI classifier can drop in behind the same interface later.
- **Dry-run by default** — it only logs what it *would* do until you pass `--apply`.
- `delete` = **move to the Deleted Items / Trash folder** (recoverable), never a hard delete.

## Setup

```sh
pnpm install

# Create your config from the templates (these copies are gitignored):
cp accounts.example.yaml accounts.yaml   # your mailbox(es): id + vendor + username
cp config.example.yaml config.yaml       # folders to scan / ignore / treat as confidential
cp rules.example.yaml rules.yaml         # classification lists, patterns and rules
```

Then edit `accounts.yaml` (your mailboxes), `config.yaml` (your folder names) and `rules.yaml`
(your sender lists). The `*.example.yaml` files are sanitized templates; your real copies stay out
of git. Each carries a `# yaml-language-server: $schema=…` modeline, so with the Red Hat YAML
extension you get hover docs and validation while editing.

`.env` is usually unnecessary — connection/auth defaults come from the vendor registry, overridable
per account in `accounts.yaml`. Use `.env` only for `CACHE_DIR` (token-cache location, handy for
Docker), `MSAL_DEBUG`, or per-account password secrets (referenced from `accounts.yaml` via
`passwordEnv`). Env vars are validated at startup (via `envalid`).

## Usage

```sh
# Dry run (default): logs what it WOULD do, changes nothing.
pnpm dev
```

**First run** prints a sign-in URL per Microsoft account (prefixed with the account id):

1. Open the URL, sign in as that account, and approve the **"Mozilla Thunderbird"** consent.
2. The browser redirects to a `https://localhost/?code=...` page that **fails to load — expected**.
3. **Copy the full address-bar URL** and paste it into the terminal prompt.

The token is then cached at `.cache/<id>/msal.json` (one dir per account), so **subsequent runs
need no sign-in**.

```sh
# Actually apply the decisions:
pnpm dev --apply

# Only process the first N unread messages (handy while tuning / for a first apply):
pnpm dev --limit 5

# Verbose MSAL auth logging, if you ever need to debug sign-in:
MSAL_DEBUG=1 pnpm dev
```

Production build: `pnpm build` then `pnpm start` (same flags, e.g. `node dist/index.js --apply`).

### Read-only inspection

`--inspect` dumps messages from one folder as JSON and **never modifies anything** (fetch uses
`BODY.PEEK`, so it doesn't even mark mail read). Useful for eyeballing a folder or feeding context
to tooling.

```sh
pnpm dev --inspect                              # most recent 30 in INBOX
pnpm dev --inspect --folder Junk --limit 50     # a specific folder
pnpm dev --inspect --folder Archive/GitHub --unread  # only unread
pnpm dev --inspect --search "sale"              # match subject/from/body (incl. read mail)
pnpm dev --inspect --folder INBOX --since 2026-05-01  # only mail on/after a date
```

### Stale-promo cleanup

`--cleanup` prunes **already-read, stale promotional mail** from a folder (INBOX by default) —
expired offers, finished sales, time-limited tickets. It **only touches read mail**, only ever
**moves to Deleted Items** (recoverable), and is governed by a conservative protect-list +
promo-sender list under `cleanup:` in [`rules.yaml`](rules.example.yaml) (edit those to tune).

```sh
pnpm dev --cleanup --since 2026-05-01            # dry-run: list what it WOULD delete
pnpm dev --cleanup --since 2026-05-01 --apply    # actually move stale promos to Deleted Items
```

## Writing rules

Most tuning is **data**, not code: the sender lists, regex patterns and age thresholds the rules
match against live under the **`refs:`** section of **`rules.yaml`** (loaded by `src/rules-data.ts`)
— `refs.lists.*`, `refs.patterns.*`, `refs.thresholds.*`. Add a newsletter domain, a spam pattern,
or change a threshold (`7d`, `30 days`, `1440 min` — any human duration) there, no rebuild of the
rule logic needed. Patterns are compiled case-insensitively; single-quote them so backslashes stay
literal.

> Both `config.yaml` and `rules.yaml` carry a `# yaml-language-server: $schema=…` modeline pointing
> at the JSON Schemas in [`schemas/`](schemas). With the **Red Hat YAML** VSCode extension (recommended
> in `.vscode/extensions.json`) you get hover documentation, key autocomplete, and validation while
> editing — every key is documented in the schema.

The rule *logic* is also in `rules.yaml`, as a small declarative DSL under `classify:` (the unread
path) and `cleanup:` (the `--cleanup` prune). Rules are evaluated top-to-bottom; the **first match
wins**; no match → `keep`. Each rule is `{ name, when: <condition>, then: keep|markRead|delete }`:

```yaml
classify:
  - name: junk-spam-subject
    when:
      all:
        - { field: folder, op: equals, value: Junk }
        - { field: subject, op: regex, ref: patterns.spamSubject }
    then: delete

  - name: known-newsletter
    when: { field: fromAddress, op: endsWithAny, ref: lists.markReadDomains }
    then: markRead
```

A **condition** is either a leaf test or a combinator:

- leaf: `{ field, op, value }` (inline literal) or `{ field, op, ref }`, where `ref` is
  `<bucket>.<name>` naming an entry under the file's `refs:` section — `lists.*` (sender lists),
  `patterns.*` (regexes), or `thresholds.*` (ages in hours), e.g. `ref: patterns.spamSubject`.
- combinator: `{ all: [...] }` (AND), `{ any: [...] }` (OR), `{ not: <condition> }`.

**Fields:** `subject`, `fromAddress`, `fromName`, `content` (subject + body), `folder` (the LEAF
folder name, so `GitHub` matches `Archive/GitHub`), `age` (in hours).
**Ops:** `regex`, `includes`, `endsWith`, `endsWithAny` (list operand), `equals`, `>`, `>=`. String
ops are case-insensitive except `equals` (exact); `regex` is case-insensitive; `>`/`>=` take the
numeric `age` field. The interpreter is `src/classifier/dsl.ts`; loading + compiling (and applying
per-account overrides) is in `src/rules-data.ts`.

**Per-account overrides.** Both `config.yaml` and `rules.yaml` accept an optional top-level
`accounts:` map keyed by the account id from `accounts.yaml`. Each block **deep-merges** over the
shared config (nested objects merge; arrays and scalars replace), so one account can have its own
`folders.confidential`, swap a list/pattern/threshold, or replace the `classify`/`cleanup` arrays —
while everything else stays shared.

> PoC limitations of the IMAP backend: `importance` defaults to `normal` (IMAP envelopes don't carry
> it), so there's no `importance` field yet. `subject`, `fromAddress`, `fromName`, `content` and
> `age` are populated. More fields can be added to the DSL as the backend is enriched.

## Project layout

```
src/
  index.ts                  orchestrator: for each account → auth → fetch → classify → dry-run/apply
  accounts.ts               loads accounts.yaml → ResolvedAccount[] (vendor defaults + overrides)
  vendors.ts                vendor registry: per-provider connection + auth defaults
  auth.ts                   authenticate(account): MSAL XOAUTH2 (wired) + password/google stubs
  imap.ts                   IMAP backend: withMailbox(conn) → listUnread / markRead / moveToDeleted
  env.ts                    typed/validated env (envalid): CACHE_DIR / MSAL_DEBUG + secret() reader
  types.ts                  MailMessage, Decision, Classifier interface
  classifier/
    rule-classifier.ts      rule engine (first-match-wins) + folderName / ageInHours helpers
    dsl.ts                  declarative condition DSL → compiles rules.yaml entries into predicates
  rules-data.ts             loads rules.yaml → compiled rules per account (rulesFor) + shared base
  file-config.ts            loads config.yaml → FileConfig per account (configFor) + shared base
  deep-merge.ts             deep-merge util for per-account overrides (objects merge; arrays replace)
  logger.ts
```

## Security notes

- `.cache/<id>/` holds each account's refresh token — treat it like a password. The whole `.cache/`
  dir is **gitignored**.
- `accounts.yaml` (mailbox addresses), `config.yaml`, `rules.yaml` and `.env` are gitignored too.

## Roadmap

- **AI classifier** — a Claude-API-backed `Classifier` for fuzzy spam/importance scoring, behind the
  same interface.
- **Triggers** — run via cron, or IMAP IDLE / polling for near-real-time handling on a mini-server.
- **Stats** — emit per-run counters to Prometheus/Grafana for a "blocked vs kept" dashboard.
- **Backend enrichment** — fetch body preview + importance headers; allow/deny lists; decision audit log.
- **More vendors** — wire Gmail / generic-IMAP auth (password / app-password, or Google OAuth)
  behind the existing per-vendor auth strategy; the engine is already provider-neutral.
- **Own app registration** — if you ever get an Entra directory, register your own client and set
  an account's `clientId` in `accounts.yaml` so the consent screen shows your app instead of Thunderbird.
