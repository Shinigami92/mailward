import { isMap, YAMLMap } from "yaml";
import type { Document, Node } from "yaml";

// The fields and operators the rule DSL accepts (mirrors crates/mailward-core/src/rules/dsl.rs).
export const CONDITION_FIELDS = [
  "subject",
  "fromAddress",
  "fromName",
  "content",
  "folder",
  "age",
] as const;
export const CONDITION_OPS = [
  "regex",
  "includes",
  "endsWith",
  "endsWithAny",
  "equals",
  ">",
  ">=",
] as const;
export const RULE_DECISIONS = ["keep", "markRead", "delete"] as const;

export type ConditionKind = "all" | "any" | "not" | "leaf" | "invalid";

/** Classifies a condition node by its shape (the DSL union is untagged). */
export function conditionKindOf(node: unknown): ConditionKind {
  if (!isMap(node)) return "invalid";
  if (node.has("all")) return "all";
  if (node.has("any")) return "any";
  if (node.has("not")) return "not";
  if (node.has("field")) return "leaf";
  return "invalid";
}

/** A fresh leaf condition node, forced to inline flow style to match the file's hand-written rules. */
// oxlint-disable-next-line typescript/prefer-readonly-parameter-types
export function makeLeafNode(doc: Document.Parsed): Node {
  const node = doc.createNode({ field: "subject", op: "includes", value: "" });
  if (isMap(node)) node.flow = true;
  return node;
}

/** A fresh `{ name, when, then }` rule whose `when` is a single inline leaf. */
// oxlint-disable-next-line typescript/prefer-readonly-parameter-types
export function makeRuleNode(doc: Document.Parsed, name: string): Node {
  // Built field-by-field on an untyped map: a `{ ..., then: ... }` literal trips the no-thenable
  // lint, and `createNode` would fix the map's key type so a later `.set("then", ...)` won't type.
  const rule = new YAMLMap();
  rule.set("name", name);
  rule.set("when", makeLeafNode(doc));
  rule.set("then", "keep");
  return rule;
}
