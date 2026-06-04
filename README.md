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

- Reads/edits the mailbox over **IMAP** (defaults to `outlook.office365.com`, configurable via
  `IMAP_HOST`/`IMAP_PORT`) using **`imapflow`**, scanning
  **multiple folders** — Inbox, Junk, and custom folders like `GitHub`/`Meetup`. By default it
  auto-discovers every selectable folder except Sent/Drafts/Trash; folder selection (scan list,
  skips, ignore, confidential) is configured in **`config.yaml`**.
- Authenticates with **OAuth2 (XOAUTH2)** — personal accounts no longer allow basic-auth IMAP.
  It uses the **authorization-code flow with manual code paste**: you sign in once in a browser,
  paste back the redirect URL, and a refresh token is cached on disk so later runs are unattended.
- **No app registration / no Azure account / no credit card needed.** Microsoft now requires a
  paid-signup or gated dev-program directory to register your own app, so we reuse **Mozilla
  Thunderbird's public OAuth client ID** (the same trick `mutt_oauth2.py` uses for personal
  Outlook). Consequence: the consent screen shows *"Mozilla Thunderbird"*. See [`config.ts`](src/config.ts).
- A pluggable **`Classifier`** decides `keep` / `markRead` / `delete` per message. The PoC ships a
  deterministic **rule engine** (`src/rules.config.ts`); an AI classifier can drop in behind the
  same interface later.
- **Dry-run by default** — it only logs what it *would* do until you pass `--apply`.
- `delete` = **move to the Deleted Items / Trash folder** (recoverable), never a hard delete.

## Setup

```sh
pnpm install

# Create your config from the templates (these copies are gitignored):
cp config.example.yaml config.yaml   # folders to scan / ignore / treat as confidential
cp rules.example.yaml rules.yaml     # classification lists, patterns and rules
```

Then edit `config.yaml` (your folder names) and `rules.yaml` (your sender lists). The
`*.example.yaml` files are sanitized templates; your real `config.yaml`/`rules.yaml` stay out of
git. Both carry a `# yaml-language-server: $schema=…` modeline, so with the Red Hat YAML extension
you get hover docs and validation while editing.

No `.env` is required — connection defaults live in `src/env.ts`. Optionally copy `.env.example`
to `.env` to override `CLIENT_ID` / `AUTHORITY` / `IMAP_HOST` / `IMAP_PORT` / `REDIRECT_URI` /
`TOKEN_CACHE_PATH`. Environment variables are validated at startup (via `envalid`), so a malformed
value fails fast with a clear message instead of surfacing later as a confusing connection error.

## Usage

```sh
# Dry run (default): logs what it WOULD do, changes nothing.
pnpm dev
```

**First run** prints a sign-in URL:

1. Open the URL, sign in as your Hotmail account, and approve the **"Mozilla Thunderbird"** consent.
2. The browser redirects to a `https://localhost/?code=...` page that **fails to load — expected**.
3. **Copy the full address-bar URL** and paste it into the terminal prompt.

The token is then cached in `.cache/msal.json`, so **subsequent runs need no sign-in**.

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
promo-sender list in [`src/cleanup.config.ts`](src/cleanup.config.ts) (edit those to tune).

```sh
pnpm dev --cleanup --since 2026-05-01            # dry-run: list what it WOULD delete
pnpm dev --cleanup --since 2026-05-01 --apply    # actually move stale promos to Deleted Items
```

## Writing rules

Most tuning is **data**, not code: the sender lists, regex patterns and age thresholds the rules
match against live in **`rules.yaml`** (loaded by `src/rules-data.ts`). Add a newsletter domain, a
spam pattern, or change a threshold (`7d`, `30 days`, `1440 min` — any human duration) there, no
rebuild of the rule logic needed. Patterns are compiled case-insensitively; single-quote them so
backslashes stay literal.

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

- leaf: `{ field, op, value }` (inline literal) or `{ field, op, ref }` (dotted path into
  `lists`/`patterns`/`thresholds` in the same file).
- combinator: `{ all: [...] }` (AND), `{ any: [...] }` (OR), `{ not: <condition> }`.

**Fields:** `subject`, `fromAddress`, `fromName`, `content` (subject + body), `folder` (the LEAF
folder name, so `GitHub` matches `Archive/GitHub`), `age` (in hours).
**Ops:** `regex`, `includes`, `endsWith`, `endsWithAny` (list operand), `equals`, `>`, `>=`. String
ops are case-insensitive except `equals` (exact); `regex` is case-insensitive; `>`/`>=` take the
numeric `age` field. The interpreter is `src/classifier/dsl.ts` (~50 lines); the thin wiring is in
`src/rules.config.ts` and `src/cleanup.config.ts`.

> PoC limitations of the IMAP backend: `importance` defaults to `normal` (IMAP envelopes don't carry
> it), so there's no `importance` field yet. `subject`, `fromAddress`, `fromName`, `content` and
> `age` are populated. More fields can be added to the DSL as the backend is enriched.

## Project layout

```
src/
  index.ts                  orchestrator: auth → fetch unread → classify → dry-run/apply → summary
  auth.ts                   MSAL auth-code flow (manual paste) + on-disk token cache
  imap.ts                   IMAP backend: withInbox() → listUnread / markRead / moveToDeleted
  env.ts                    typed/validated env (envalid): CLIENT_ID/AUTHORITY/IMAP_HOST/IMAP_PORT/MSAL_DEBUG
  config.ts                 client ID / authority / IMAP host+port / scope / token-cache path
  types.ts                  MailMessage, Decision, Classifier interface
  classifier/
    rule-classifier.ts      rule engine (first-match-wins) + folderName / ageInHours helpers
    dsl.ts                  declarative condition DSL → compiles rules.yaml entries into predicates
  rules-data.ts             loads rules.yaml → lists / patterns / thresholds + raw rule arrays + resolveRef
  rules.config.ts           compiles the `classify:` rules from rules.yaml
  cleanup.config.ts         compiles the `cleanup:` rules from rules.yaml + expirySignal()
  logger.ts
```

## Security notes

- `.cache/msal.json` contains a refresh token — treat it like a password. It's **gitignored**.
- `.env` is gitignored too (though usually unnecessary now).

## Roadmap

- **AI classifier** — a Claude-API-backed `Classifier` for fuzzy spam/importance scoring, behind the
  same interface.
- **Triggers** — run via cron, or IMAP IDLE / polling for near-real-time handling on a mini-server.
- **Stats** — emit per-run counters to Prometheus/Grafana for a "blocked vs kept" dashboard.
- **Backend enrichment** — fetch body preview + importance headers; allow/deny lists; decision audit log.
- **Own app registration** — if you ever get an Entra directory, register your own client and set
  `CLIENT_ID` so the consent screen shows your app instead of Thunderbird.
