import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';
import { parse } from 'yaml';
import { deepMerge } from './deep-merge.js';

const here = dirname(fileURLToPath(import.meta.url));
/** Project root (one level up from src/, or dist/ after a build). */
const projectRoot = resolve(here, '..');
const configPath = resolve(projectRoot, 'config.yaml');

export interface FoldersConfig {
  /** Folders to scan; null = auto-discover all selectable folders. */
  scan: string[] | null;
  /** IMAP special-use flags to exclude from auto-discovery (e.g. '\\Sent'). */
  skipSpecialUse: string[];
  /** Folder names (lower-cased) to exclude from auto-discovery. */
  skipNames: string[];
  /** Folders never scanned/touched, even if named explicitly. */
  ignore: string[];
  /** Sensitive folders: opt-in only, never exposed to any AI. */
  confidential: string[];
}

/** Default targets/sizes for the CLI modes. */
export interface DefaultsConfig {
  /** Folder used by --inspect / --cleanup when --folder is omitted. */
  folder: string;
  /** --inspect default max messages. */
  inspectLimit: number;
  /** --cleanup sweep window size (how many recent messages it scans). */
  cleanupLimit: number;
}

/** IMAP backend tuning. */
export interface ImapConfig {
  /** Folder to move deletions to if no \Trash special-use folder is found. */
  trashFallback: string;
  /** How much decoded body text to keep as the preview (for rule matching). */
  bodyPreviewChars: number;
}

export interface FileConfig {
  folders: FoldersConfig;
  defaults: DefaultsConfig;
  imap: ImapConfig;
}

function asStringArray(value: unknown, fallback: string[]): string[] {
  return Array.isArray(value) ? value.map(String) : fallback;
}

function asString(value: unknown, fallback: string): string {
  return typeof value === 'string' && value !== '' ? value : fallback;
}

function asNumber(value: unknown, fallback: number): number {
  return typeof value === 'number' && Number.isFinite(value) ? value : fallback;
}

interface RawConfig {
  folders?: Partial<FoldersConfig>;
  defaults?: Partial<DefaultsConfig>;
  imap?: Partial<ImapConfig>;
}

/** Normalizes a raw parsed config object into a fully-defaulted {@link FileConfig}. */
function normalize(raw: RawConfig): FileConfig {
  const folders = raw.folders ?? {};
  const defaults = raw.defaults ?? {};
  const imap = raw.imap ?? {};

  return {
    folders: {
      scan: Array.isArray(folders.scan) ? folders.scan.map(String) : null,
      skipSpecialUse: asStringArray(folders.skipSpecialUse, ['\\Sent', '\\Drafts', '\\Trash']),
      skipNames: asStringArray(folders.skipNames, ['outbox', 'notes']).map((n) =>
        n.toLowerCase(),
      ),
      ignore: asStringArray(folders.ignore, []),
      confidential: asStringArray(folders.confidential, []),
    },
    defaults: {
      folder: asString(defaults.folder, 'INBOX'),
      inspectLimit: asNumber(defaults.inspectLimit, 30),
      cleanupLimit: asNumber(defaults.cleanupLimit, 500),
    },
    imap: {
      trashFallback: asString(imap.trashFallback, 'Deleted'),
      bodyPreviewChars: asNumber(imap.bodyPreviewChars, 2000),
    },
  };
}

function parseConfigFile(): { base: RawConfig; accounts: Record<string, unknown> } {
  let raw: string;
  try {
    raw = readFileSync(configPath, 'utf8');
  } catch {
    throw new Error(
      `Missing config.yaml at ${configPath}. Copy config.example.yaml to config.yaml and edit it (see README).`,
    );
  }
  const { accounts, ...base } = (parse(raw) ?? {}) as RawConfig & {
    accounts?: Record<string, unknown>;
  };
  return { base, accounts: accounts ?? {} };
}

const { base: baseRawConfig, accounts: accountConfigOverrides } = parseConfigFile();

/** The shared (account-agnostic) config — used as the back-compat default export. */
export const fileConfig: FileConfig = normalize(baseRawConfig);

/**
 * Effective config for one account: the shared base with that account's `accounts.<id>`
 * overrides deep-merged on top (objects merge; arrays/scalars replace). Falls back to the
 * shared config when the account has no overrides.
 */
export function configFor(accountId: string): FileConfig {
  const override = accountConfigOverrides[accountId];
  return override === undefined ? fileConfig : normalize(deepMerge(baseRawConfig, override));
}
