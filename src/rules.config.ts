import { compileRules } from './classifier/dsl.js';
import type { Rule } from './classifier/rule-classifier.js';
import { rawRules, resolveRef } from './rules-data.js';

/**
 * Unread-path classification rules. The rules themselves — conditions and the resulting
 * keep/markRead/delete — are declared in rules.yaml under `classify:` and compiled here
 * into predicates (see src/classifier/dsl.ts). The data they match against (sender lists,
 * regex patterns) lives in the same file under `lists:`/`patterns:`.
 *
 * Evaluated top-to-bottom; first match wins; no match → 'keep'. Design principle: be
 * PRECISE about deletes — Junk is not blanket-deleted, only well-known spam/scam clusters.
 */
export const rules: readonly Rule[] = compileRules(rawRules.classify, resolveRef);
