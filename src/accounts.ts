import { readFileSync } from 'node:fs';
import { isAbsolute, resolve } from 'node:path';
import { parse } from 'yaml';
import { env, projectRoot } from './env.js';
import { getVendor, VENDOR_NAMES, type AuthMethod } from './vendors.js';

const accountsPath = resolve(projectRoot, 'accounts.yaml');

/** A fully-resolved account: vendor defaults merged with the account's own overrides. */
export interface ResolvedAccount {
  id: string;
  vendor: string;
  username: string;
  imapHost: string;
  imapPort: number;
  authMethod: AuthMethod;
  clientId?: string;
  authority?: string;
  redirectUri?: string;
  scopes?: string[];
  /** Absolute path to this account's cache directory (`<CACHE_DIR>/<id>/`). */
  cacheDir: string;
  /** Name of the env var holding this account's password/app-password (password vendors). */
  passwordEnv?: string;
}

interface RawAccount {
  id?: unknown;
  vendor?: unknown;
  username?: unknown;
  imapHost?: unknown;
  imapPort?: unknown;
  clientId?: unknown;
  authority?: unknown;
  redirectUri?: unknown;
  scopes?: unknown;
  passwordEnv?: unknown;
}

const ID_PATTERN = /^[a-z0-9-]+$/;

function str(value: unknown): string | undefined {
  return typeof value === 'string' && value !== '' ? value : undefined;
}

/** Resolves a possibly-relative path against the project root. */
function resolvePath(p: string): string {
  return isAbsolute(p) ? p : resolve(projectRoot, p);
}

function resolveAccount(raw: RawAccount, index: number): ResolvedAccount {
  const id = str(raw.id);
  if (!id || !ID_PATTERN.test(id)) {
    throw new Error(
      `accounts.yaml: account #${index} has an invalid "id" (need lowercase letters/digits/hyphens): ${JSON.stringify(raw.id)}.`,
    );
  }
  const vendorName = str(raw.vendor);
  const vendor = vendorName ? getVendor(vendorName) : undefined;
  if (!vendor) {
    throw new Error(
      `accounts.yaml: account "${id}" has unknown vendor ${JSON.stringify(raw.vendor)} (known: ${VENDOR_NAMES.join(', ')}).`,
    );
  }
  const username = str(raw.username);
  if (!username) {
    throw new Error(`accounts.yaml: account "${id}" is missing "username".`);
  }

  const d = vendor.defaults;
  const imapHost = str(raw.imapHost) ?? d.imapHost;
  if (!imapHost) {
    throw new Error(`accounts.yaml: account "${id}" (vendor ${vendorName}) needs an "imapHost".`);
  }
  const imapPort =
    typeof raw.imapPort === 'number' && Number.isFinite(raw.imapPort) ? raw.imapPort : d.imapPort;

  // Per-account cache directory `<CACHE_DIR>/<id>/`, anchored to the project root (so it doesn't
  // depend on the process's current working directory). Auth strategies put their files here.
  const cacheDir = resolvePath(`${env.CACHE_DIR}/${id}`);

  return {
    id,
    vendor: vendorName as string,
    username,
    imapHost,
    imapPort,
    authMethod: d.authMethod,
    clientId: str(raw.clientId) ?? d.clientId,
    authority: str(raw.authority) ?? d.authority,
    redirectUri: str(raw.redirectUri) ?? d.redirectUri,
    scopes: Array.isArray(raw.scopes) ? raw.scopes.map(String) : d.scopes,
    cacheDir,
    passwordEnv: str(raw.passwordEnv),
  };
}

function loadAccounts(): ResolvedAccount[] {
  let raw: string;
  try {
    raw = readFileSync(accountsPath, 'utf8');
  } catch {
    throw new Error(
      `Missing accounts.yaml at ${accountsPath}. Copy accounts.example.yaml to accounts.yaml and edit it (see README).`,
    );
  }

  const parsed = parse(raw) as unknown;
  if (!Array.isArray(parsed) || parsed.length === 0) {
    throw new Error('accounts.yaml: expected a non-empty list of accounts.');
  }

  const resolved = parsed.map((entry, i) => resolveAccount((entry ?? {}) as RawAccount, i));

  const seen = new Set<string>();
  for (const account of resolved) {
    if (seen.has(account.id)) {
      throw new Error(`accounts.yaml: duplicate account id "${account.id}".`);
    }
    seen.add(account.id);
  }
  return resolved;
}

export const accounts: ResolvedAccount[] = loadAccounts();
