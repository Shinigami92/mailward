import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';
import { bool, cleanEnv, host, port, str, url } from 'envalid';

const here = dirname(fileURLToPath(import.meta.url));
/** Project root (one level up from src/, or dist/ after a build). */
const projectRoot = resolve(here, '..');

// Load .env (if present) into process.env before validation. A missing file is fine —
// env vars may already be set by the shell/cron, and every var below has a default.
try {
  process.loadEnvFile(resolve(projectRoot, '.env'));
} catch {
  // no .env file; rely on the ambient environment
}

/**
 * Mozilla Thunderbird's public OAuth client ID. Personal Microsoft accounts can no
 * longer register their own app without an Entra directory (which needs a paid Azure
 * signup or a gated dev program), so we reuse this well-known public client — the same
 * one Thunderbird and tools like mutt_oauth2.py use for personal Outlook/Hotmail.
 * Consequence: the first-run consent screen shows "Mozilla Thunderbird". Override via
 * CLIENT_ID if you ever register your own app.
 */
const THUNDERBIRD_CLIENT_ID = '9e5f94bc-e8a4-4e73-b8be-63364c29d753';

/** Typed, validated environment. Invalid/missing-without-default values fail fast at startup. */
export const env = cleanEnv(process.env, {
  CLIENT_ID: str({
    desc: "OAuth client ID. Defaults to Mozilla Thunderbird's public client.",
    default: THUNDERBIRD_CLIENT_ID,
  }),
  AUTHORITY: url({
    // "common" (not "consumers") is required for the borrowed multi-tenant client to
    // accept a personal account through the auth-code flow.
    desc: 'OAuth authority. Must be the /common tenant for the borrowed client to accept personal accounts.',
    default: 'https://login.microsoftonline.com/common',
  }),
  REDIRECT_URI: url({
    // We never run a server on it — after sign-in the browser lands on
    // https://localhost/?code=... (which fails to load, that's fine) and the user
    // pastes that URL back. Must match the redirect URI registered for CLIENT_ID.
    desc: 'OAuth redirect URI. The default works for the borrowed Thunderbird client.',
    default: 'https://localhost',
  }),
  TOKEN_CACHE_PATH: str({
    desc: 'Where the MSAL token cache (refresh token etc.) is persisted. Override to mount it in Docker.',
    default: resolve(projectRoot, '.cache', 'msal.json'),
  }),
  IMAP_HOST: host({
    desc: 'IMAP server host.',
    default: 'outlook.office365.com',
  }),
  IMAP_PORT: port({
    desc: 'IMAP server port.',
    default: 993,
  }),
  MSAL_DEBUG: bool({
    desc: "Log MSAL's auth/token traffic (helpful for diagnosing sign-in hangs).",
    default: false,
  }),
});
