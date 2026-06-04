import { ageInHours, folderName, type Rule } from './rule-classifier.js';
import type { Decision, MailMessage } from '../types.js';

/**
 * A tiny declarative condition language for rules, so the rule logic itself can live in
 * rules.yaml instead of TypeScript. A condition is either a leaf test or a combinator:
 *
 *   leaf:        { field, op, value }   or   { field, op, ref }
 *   combinator:  { all: [...] } | { any: [...] } | { not: <condition> }
 *
 * `value` is an inline literal; `ref` is a dotted path resolved within rules.yaml's data
 * (e.g. `lists.markReadDomains`, `patterns.spamSubject`, `thresholds.monthly`).
 *
 * Conditions are compiled once into a predicate (regexes/refs resolved up front), so
 * per-message evaluation is cheap. Compilation throws on any malformed rule.
 */

/** Message fields exposed to the DSL. */
const STRING_FIELDS = {
  subject: (m: MailMessage) => m.subject,
  fromAddress: (m: MailMessage) => m.fromAddress,
  fromName: (m: MailMessage) => m.fromName,
  /** subject + body, for rules that should look at content too. */
  content: (m: MailMessage) => `${m.subject} ${m.bodyPreview}`,
  /** The LEAF folder name, so `GitHub` matches `Archive/GitHub`. */
  folder: (m: MailMessage) => folderName(m),
} as const;

const NUMBER_FIELDS = {
  /** Whole-message age in hours (thresholds are stored in hours too). */
  age: (m: MailMessage) => ageInHours(m),
} as const;

export type LeafCondition = {
  field: string;
  op: string;
  value?: unknown;
  ref?: string;
};
export type Condition =
  | LeafCondition
  | { all: Condition[] }
  | { any: Condition[] }
  | { not: Condition };

export type ResolveRef = (path: string) => unknown;

type Predicate = (message: MailMessage) => boolean;

function stringField(field: string, op: string): (m: MailMessage) => string {
  const get = (STRING_FIELDS as Record<string, (m: MailMessage) => string>)[field];
  if (!get) {
    throw new Error(`rules.yaml: op "${op}" needs a text field, got "${field}".`);
  }
  return get;
}

function numberField(field: string, op: string): (m: MailMessage) => number {
  const get = (NUMBER_FIELDS as Record<string, (m: MailMessage) => number>)[field];
  if (!get) {
    throw new Error(`rules.yaml: op "${op}" needs a numeric field, got "${field}".`);
  }
  return get;
}

/** The literal (`value`) or resolved (`ref`) operand of a leaf condition. */
function operand(leaf: LeafCondition, resolveRef: ResolveRef): unknown {
  if (leaf.ref !== undefined) {
    return resolveRef(leaf.ref);
  }
  if (leaf.value === undefined) {
    throw new Error(`rules.yaml: leaf {field:${leaf.field}, op:${leaf.op}} needs a value or ref.`);
  }
  return leaf.value;
}

function asStringList(value: unknown, op: string): string[] {
  if (!Array.isArray(value)) {
    throw new Error(`rules.yaml: op "${op}" needs a list operand.`);
  }
  return value.map((v) => String(v).toLowerCase());
}

/** Compiles a leaf test into a predicate. String ops are case-insensitive; equals is exact. */
function compileLeaf(leaf: LeafCondition, resolveRef: ResolveRef): Predicate {
  const { field, op } = leaf;
  const value = operand(leaf, resolveRef);

  switch (op) {
    case 'equals': {
      const get = stringField(field, op);
      const target = String(value);
      return (m) => get(m) === target;
    }
    case 'includes': {
      const get = stringField(field, op);
      const needle = String(value).toLowerCase();
      return (m) => get(m).toLowerCase().includes(needle);
    }
    case 'endsWith': {
      const get = stringField(field, op);
      const suffix = String(value).toLowerCase();
      return (m) => get(m).toLowerCase().endsWith(suffix);
    }
    case 'endsWithAny': {
      const get = stringField(field, op);
      const suffixes = asStringList(value, op);
      return (m) => {
        const text = get(m).toLowerCase();
        return suffixes.some((s) => text.endsWith(s));
      };
    }
    case 'regex': {
      const get = stringField(field, op);
      const re = value instanceof RegExp ? value : new RegExp(String(value), 'i');
      return (m) => re.test(get(m));
    }
    case '>': {
      const get = numberField(field, op);
      const n = Number(value);
      return (m) => get(m) > n;
    }
    case '>=': {
      const get = numberField(field, op);
      const n = Number(value);
      return (m) => get(m) >= n;
    }
    default:
      throw new Error(`rules.yaml: unknown op "${op}".`);
  }
}

/** Recursively compiles any condition node into a single predicate. */
export function compileCondition(node: Condition, resolveRef: ResolveRef): Predicate {
  if (node && typeof node === 'object') {
    if ('all' in node) {
      const subs = node.all.map((n) => compileCondition(n, resolveRef));
      return (m) => subs.every((p) => p(m));
    }
    if ('any' in node) {
      const subs = node.any.map((n) => compileCondition(n, resolveRef));
      return (m) => subs.some((p) => p(m));
    }
    if ('not' in node) {
      const sub = compileCondition(node.not, resolveRef);
      return (m) => !sub(m);
    }
    if ('field' in node && 'op' in node) {
      return compileLeaf(node, resolveRef);
    }
  }
  throw new Error(`rules.yaml: malformed condition: ${JSON.stringify(node)}`);
}

const DECISIONS = new Set<Decision>(['keep', 'markRead', 'delete']);

interface RawRule {
  name?: unknown;
  when?: unknown;
  then?: unknown;
}

/** Compiles the declarative rule array from rules.yaml into executable {@link Rule}s. */
export function compileRules(raw: unknown, resolveRef: ResolveRef): Rule[] {
  if (!Array.isArray(raw)) {
    throw new Error('rules.yaml: expected a list of rules.');
  }
  return raw.map((entry, index) => {
    const { name, when, then } = entry as RawRule;
    if (typeof name !== 'string') {
      throw new Error(`rules.yaml: rule #${index} is missing a string "name".`);
    }
    if (!DECISIONS.has(then as Decision)) {
      throw new Error(`rules.yaml: rule "${name}" has invalid "then": ${JSON.stringify(then)}.`);
    }
    if (when === undefined) {
      throw new Error(`rules.yaml: rule "${name}" is missing "when".`);
    }
    return { name, when: compileCondition(when as Condition, resolveRef), then: then as Decision };
  });
}
