import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';
import { bool, cleanEnv, str } from 'envalid';

const here = dirname(fileURLToPath(import.meta.url));
/** Project root (one level up from src/, or dist/ after a build). */
export const projectRoot = resolve(here, '..');

// Load .env (if present) into process.env before validation. A missing file is fine —
// connection/auth config now lives in accounts.yaml + the vendor registry, so .env only
// holds optional global flags and any per-account secrets (referenced via `passwordEnv`).
try {
  process.loadEnvFile(resolve(projectRoot, '.env'));
} catch {
  // no .env file; rely on the ambient environment
}

/** Typed, validated global environment. */
export const env = cleanEnv(process.env, {
  CACHE_DIR: str({
    desc: 'Directory for per-account OAuth token caches (mount this in Docker to persist logins).',
    default: '.cache',
  }),
  MSAL_DEBUG: bool({
    desc: "Log MSAL's auth/token traffic (helpful for diagnosing Microsoft sign-in hangs).",
    default: false,
  }),
});

/**
 * Reads a dynamically-named secret from the environment (used for an account's
 * `passwordEnv`). Returns undefined if unset/empty. Not part of the validated `env`
 * schema because the variable name is chosen per account in accounts.yaml.
 */
export function secret(name: string): string | undefined {
  const value = process.env[name];
  return value && value.trim() !== '' ? value : undefined;
}
