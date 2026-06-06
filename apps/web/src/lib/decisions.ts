/** The GraphQL `DecisionKind` enum values. */
export type DecisionKind = "KEEP" | "MARK_READ" | "DELETE";

/** Human label for a decision. */
const DECISION_LABELS: Record<DecisionKind, string> = {
  KEEP: "keep",
  MARK_READ: "mark read",
  DELETE: "delete",
};

export function decisionLabel(decision: DecisionKind): string {
  return DECISION_LABELS[decision];
}

/** Tailwind badge classes per decision (green = kept, amber = read, red = delete). */
const DECISION_CLASSES: Record<DecisionKind, string> = {
  KEEP: "bg-emerald-100 text-emerald-700 ring-emerald-200 dark:bg-emerald-950 dark:text-emerald-300 dark:ring-emerald-900",
  MARK_READ:
    "bg-amber-100 text-amber-700 ring-amber-200 dark:bg-amber-950 dark:text-amber-300 dark:ring-amber-900",
  DELETE:
    "bg-rose-100 text-rose-700 ring-rose-200 dark:bg-rose-950 dark:text-rose-300 dark:ring-rose-900",
};

export function decisionClasses(decision: DecisionKind): string {
  return DECISION_CLASSES[decision];
}
