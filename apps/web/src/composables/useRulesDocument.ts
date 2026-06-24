import { computed, inject, ref, shallowRef } from "vue";
import type { ComputedRef, InjectionKey, Ref, ShallowRef } from "vue";
import { parseDocument } from "yaml";
import type { Document } from "yaml";

// `lineWidth: 0` is mandatory: it stops the serializer from reflowing long regex/list lines, which
// would otherwise produce huge spurious diffs and move the user's cursor.
const STRINGIFY = { lineWidth: 0 } as const;

// Where the latest change came from, so the host can decide whether to push the re-serialized text
// back into the code editor: only GUI edits need to (text edits already ARE the text).
export type EditOrigin = "seed" | "text" | "gui";

export interface RulesStore {
  /** The comment-preserving YAML AST - the single source of truth for both tabs. */
  doc: ShallowRef<Document.Parsed>;
  /** Bumped on every mutation; drives the GUI projection and the `text` computed. */
  docVersion: Ref<number>;
  /** Non-null when the current text failed to parse (the last good `doc` is retained). */
  parseError: Ref<string | null>;
  origin: Ref<EditOrigin>;
  /** Canonical serialization of `doc` (comment-preserving, never reflowed). */
  text: ComputedRef<string>;
  /** Load a fresh document (initial server load). */
  seed: (raw: string) => void;
  /** Re-parse after a raw-text edit; on parse error keep the last good `doc`. */
  ingestText: (raw: string) => void;
  /** Mutate the AST in place (node-level, so untouched comments/formatting survive). */
  // oxlint-disable-next-line typescript/prefer-readonly-parameter-types
  commitGuiEdit: (mutator: (doc: Document.Parsed) => void) => void;
}

/** Injection key so the recursive condition editor can reach the store without prop drilling. */
export const rulesStoreKey: InjectionKey<RulesStore> = Symbol("rulesStore");

/** Inject the rules store provided by `RulesEditor`, throwing if used outside it. */
export function injectRulesStore(): RulesStore {
  const store = inject(rulesStoreKey);
  if (store == null) throw new Error("rules store not provided");
  return store;
}

// oxlint-disable-next-line typescript/prefer-readonly-parameter-types
function firstError(parsed: Document.Parsed): string | null {
  return parsed.errors.length > 0 ? parsed.errors[0].message : null;
}

export function useRulesDocument(): RulesStore {
  const doc = shallowRef<Document.Parsed>(parseDocument(""));
  const docVersion = ref(0);
  const parseError = ref<string | null>(null);
  const origin = ref<EditOrigin>("seed");

  const text = computed(() => {
    void docVersion.value;
    return doc.value.toString(STRINGIFY);
  });

  function seed(raw: string): void {
    doc.value = parseDocument(raw);
    parseError.value = firstError(doc.value);
    origin.value = "seed";
    docVersion.value += 1;
  }

  function ingestText(raw: string): void {
    const next = parseDocument(raw);
    parseError.value = firstError(next);
    if (next.errors.length > 0) return;
    doc.value = next;
    origin.value = "text";
    docVersion.value += 1;
  }

  // oxlint-disable-next-line typescript/prefer-readonly-parameter-types
  function commitGuiEdit(mutator: (target: Document.Parsed) => void): void {
    mutator(doc.value);
    origin.value = "gui";
    docVersion.value += 1;
  }

  return { doc, docVersion, parseError, origin, text, seed, ingestText, commitGuiEdit };
}
