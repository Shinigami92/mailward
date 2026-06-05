/** The GraphQL `DecisionKind` enum values. */
export type DecisionKind = "KEEP" | "MARK_READ" | "DELETE";

/** Human label for a decision. */
export function decisionLabel(decision: DecisionKind): string {
  switch (decision) {
    case "KEEP":
      return "keep";
    case "MARK_READ":
      return "mark read";
    case "DELETE":
      return "delete";
  }
}

/** Tailwind badge classes per decision (green = kept, amber = read, red = delete). */
export function decisionClasses(decision: DecisionKind): string {
  switch (decision) {
    case "KEEP":
      return "bg-emerald-100 text-emerald-700 ring-emerald-200";
    case "MARK_READ":
      return "bg-amber-100 text-amber-700 ring-amber-200";
    case "DELETE":
      return "bg-rose-100 text-rose-700 ring-rose-200";
  }
}
