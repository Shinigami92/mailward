import { compileRules } from './classifier/dsl.js';
import type { Rule } from './classifier/rule-classifier.js';
import { rawRules, resolveRef, rulesData } from './rules-data.js';
import type { MailMessage } from './types.js';

/**
 * Rules for `--cleanup` (pruning already-READ INBOX mail). Declared in rules.yaml under
 * `cleanup:` and compiled here (see src/classifier/dsl.ts). Policy differs from the unread
 * path: only keep | delete, and deliberately conservative —
 *   1. a protect-list (senders + subject keywords) ALWAYS wins → never deleted;
 *   2. only known promotional senders are eligible for stale-deletion;
 *   3. 'delete' = move to Deleted Items (recoverable; Outlook purges after ~30 days).
 */
export const cleanupRules: readonly Rule[] = compileRules(rawRules.cleanup, resolveRef);

const contentOf = (m: MailMessage): string => `${m.subject} ${m.bodyPreview}`;

/** The expiry phrase that makes a message look stale (for display), or '' if none. */
export function expirySignal(m: MailMessage): string {
  return contentOf(m).match(rulesData.patterns.expiry)?.[0] ?? '';
}
