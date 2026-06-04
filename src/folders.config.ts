/**
 * Folder-level privacy & handling policy, evaluated against a (possibly per-account) folders
 * config. Matching is by exact IMAP folder path.
 *
 *  - ignore       → never scanned or touched, even when named explicitly.
 *  - confidential → excluded from the default auto-scan + CSV export; opt-in only via an
 *    explicit scan list. Content must NEVER be sent to an AI (the future local-LLM fallback
 *    MUST refuse these, enforced via isConfidential()).
 */

import type { FoldersConfig } from './file-config.js';

/** Folder is off-limits entirely (never scan/inspect/clean, even if named). */
export function isIgnored(folder: string, folders: FoldersConfig): boolean {
  return folders.ignore.includes(folder);
}

/** Folder holds sensitive data: opt-in only, and never exposed to any AI/external service. */
export function isConfidential(folder: string, folders: FoldersConfig): boolean {
  return folders.confidential.includes(folder);
}
