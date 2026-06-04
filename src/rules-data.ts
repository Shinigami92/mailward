import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';
import { parse as parseYaml } from 'yaml';
import parseDuration from 'parse-duration';

const here = dirname(fileURLToPath(import.meta.url));
/** Project root (one level up from src/, or dist/ after a build). */
const projectRoot = resolve(here, '..');
const rulesPath = resolve(projectRoot, 'rules.yaml');

/** Sender-domain lists (matched as address suffixes). */
export interface RulesLists {
  markReadDomains: string[];
  spamDomains: string[];
  protectSenders: string[];
  promoSenders: string[];
  monthlySenders: string[];
}

/** Named regexes, compiled case-insensitively. */
export interface RulesPatterns {
  spamSubject: RegExp;
  phishing: RegExp;
  moneyScam: RegExp;
  emotionalScam: RegExp;
  impersonation: RegExp;
  fakeParcel: RegExp;
  protectSubject: RegExp;
  digestSubject: RegExp;
  expiry: RegExp;
}

/** Age thresholds, all expressed in hours (parsed from human durations). */
export interface RulesThresholds {
  stalePromo: number;
  monthly: number;
  digest: number;
  promoExpiredMin: number;
}

export interface RulesData {
  lists: RulesLists;
  patterns: RulesPatterns;
  thresholds: RulesThresholds;
}

/** Raw, uncompiled declarative rule arrays (compiled by src/classifier/dsl.ts). */
export interface RawRules {
  classify: unknown;
  cleanup: unknown;
}

interface RawRulesFile {
  lists?: Record<string, unknown>;
  patterns?: Record<string, unknown>;
  thresholds?: Record<string, unknown>;
  classify?: unknown;
  cleanup?: unknown;
}

/** Reads a required string[] list, failing loudly if it is missing/mistyped. */
function list(source: Record<string, unknown> | undefined, key: string): string[] {
  const value = source?.[key];
  if (!Array.isArray(value)) {
    throw new Error(`rules.yaml: lists.${key} must be a list of strings.`);
  }
  return value.map(String);
}

/** Compiles a required pattern string into a case-insensitive RegExp. */
function pattern(source: Record<string, unknown> | undefined, key: string): RegExp {
  const value = source?.[key];
  if (typeof value !== 'string') {
    throw new Error(`rules.yaml: patterns.${key} must be a regex string.`);
  }
  try {
    return new RegExp(value, 'i');
  } catch (error) {
    throw new Error(`rules.yaml: patterns.${key} is not a valid regex: ${String(error)}`);
  }
}

/** Parses a human duration (e.g. "7d", "30 days", "1440 min") into hours. */
function hours(source: Record<string, unknown> | undefined, key: string): number {
  const value = source?.[key];
  if (typeof value !== 'string' && typeof value !== 'number') {
    throw new Error(`rules.yaml: thresholds.${key} must be a duration like "7d".`);
  }
  const result = parseDuration(String(value), 'h');
  if (result == null) {
    throw new Error(`rules.yaml: thresholds.${key} is not a valid duration: "${String(value)}".`);
  }
  return result;
}

function parseRulesFile(): RawRulesFile {
  let raw: string;
  try {
    raw = readFileSync(rulesPath, 'utf8');
  } catch {
    throw new Error(
      `Missing rules.yaml at ${rulesPath}. Copy rules.example.yaml to rules.yaml and edit it (see README).`,
    );
  }
  return (parseYaml(raw) ?? {}) as RawRulesFile;
}

function loadRulesData(parsed: RawRulesFile): RulesData {
  return {
    lists: {
      markReadDomains: list(parsed.lists, 'markReadDomains'),
      spamDomains: list(parsed.lists, 'spamDomains'),
      protectSenders: list(parsed.lists, 'protectSenders'),
      promoSenders: list(parsed.lists, 'promoSenders'),
      monthlySenders: list(parsed.lists, 'monthlySenders'),
    },
    patterns: {
      spamSubject: pattern(parsed.patterns, 'spamSubject'),
      phishing: pattern(parsed.patterns, 'phishing'),
      moneyScam: pattern(parsed.patterns, 'moneyScam'),
      emotionalScam: pattern(parsed.patterns, 'emotionalScam'),
      impersonation: pattern(parsed.patterns, 'impersonation'),
      fakeParcel: pattern(parsed.patterns, 'fakeParcel'),
      protectSubject: pattern(parsed.patterns, 'protectSubject'),
      digestSubject: pattern(parsed.patterns, 'digestSubject'),
      expiry: pattern(parsed.patterns, 'expiry'),
    },
    thresholds: {
      stalePromo: hours(parsed.thresholds, 'stalePromo'),
      monthly: hours(parsed.thresholds, 'monthly'),
      digest: hours(parsed.thresholds, 'digest'),
      promoExpiredMin: hours(parsed.thresholds, 'promoExpiredMin'),
    },
  };
}

const parsedFile = parseRulesFile();

export const rulesData: RulesData = loadRulesData(parsedFile);

/** The raw declarative rule arrays, compiled into predicates by the DSL layer. */
export const rawRules: RawRules = {
  classify: parsedFile.classify,
  cleanup: parsedFile.cleanup,
};

/**
 * Resolves a dotted `ref` path (e.g. "lists.markReadDomains", "patterns.spamSubject",
 * "thresholds.monthly") against the loaded {@link rulesData}. Throws on an unknown path.
 */
export function resolveRef(path: string): unknown {
  return path.split('.').reduce<unknown>((node, key) => {
    if (node !== null && typeof node === 'object' && key in node) {
      return (node as Record<string, unknown>)[key];
    }
    throw new Error(`rules.yaml: unknown ref "${path}".`);
  }, rulesData);
}
