/**
 * Folder-level privacy & handling policy. Static for the PoC; in the shipped product
 * this is intended to move to a human-editable, volume-mounted config file (no DB).
 * Matching is by exact IMAP folder path.
 */

import { fileConfig } from './file-config.js';

/**
 * Folder lists now come from config.yaml (see file-config.ts):
 *  - ignore       → never scanned or touched, even when named explicitly.
 *  - confidential → excluded from the default auto-scan + CSV export; opt-in only via
 *    explicit --folder; content must NEVER be sent to an AI (the future local-LLM
 *    fallback MUST refuse these, enforced via isConfidential()).
 */
const IGNORE = new Set<string>(fileConfig.folders.ignore);
const CONFIDENTIAL = new Set<string>(fileConfig.folders.confidential);

/** Folder is off-limits entirely (never scan/inspect/clean, even if named). */
export function isIgnored(folder: string): boolean {
  return IGNORE.has(folder);
}

/** Folder holds sensitive data: opt-in only, and never exposed to any AI/external service. */
export function isConfidential(folder: string): boolean {
  return CONFIDENTIAL.has(folder);
}
