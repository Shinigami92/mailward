import { describe, expect, it } from "vitest";
import { type DecisionKind, decisionClasses, decisionLabel } from "./decisions";

describe("decisions", () => {
  it("labels each decision", () => {
    expect(decisionLabel("KEEP")).toBe("keep");
    expect(decisionLabel("MARK_READ")).toBe("mark read");
    expect(decisionLabel("DELETE")).toBe("delete");
  });

  it("maps every decision to non-empty badge classes", () => {
    const kinds: DecisionKind[] = ["KEEP", "MARK_READ", "DELETE"];
    for (const kind of kinds) {
      expect(decisionClasses(kind).length).toBeGreaterThan(0);
    }
  });
});
