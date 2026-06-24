import { makeRuleNode } from "@/lib/rulesAst";
import { describe, expect, it } from "vitest";
import { parseDocument } from "yaml";

const STRINGIFY = { lineWidth: 0 } as const;

// A representative slice of rules.yaml: header + section comments, a flow-style leaf, and a
// rule with a comment above it - the shapes whose round-trip fidelity actually matters.
const FIXTURE = `# yaml-language-server: $schema=./schemas/rules.schema.json
# mailward classification rules.
refs:
  lists:
    # Low-priority senders -> mark read.
    markReadDomains:
      - newsletter.example.com
      - notifications.example.org
  patterns:
    spamSubject: '(casino|free spins|\\bbonus\\b|jackpot)'
classify:
  # Mark bot notifications read but leave human activity unread.
  - name: github-bot
    when:
      all:
        - { field: folder, op: equals, value: GitHub }
        - { field: fromName, op: includes, value: '[bot]' }
    then: markRead
`;

describe("rules YAML round-trip", () => {
  it("preserves comments and is stable across parse -> stringify", () => {
    const once = parseDocument(FIXTURE).toString(STRINGIFY);
    const twice = parseDocument(once).toString(STRINGIFY);

    // Idempotent: re-parsing the serialized output reproduces it byte-for-byte.
    expect(twice).toBe(once);
    // Header, section, and per-rule comments all survive.
    expect(once).toContain("# yaml-language-server: $schema=./schemas/rules.schema.json");
    expect(once).toContain("# Low-priority senders -> mark read.");
    expect(once).toContain("# Mark bot notifications read but leave human activity unread.");
    // The hand-written inline (flow) leaf style is kept, not expanded to a block.
    expect(once).toContain("{ field: folder, op: equals, value: GitHub }");
  });

  it("applies a node-level edit without dropping comments", () => {
    const doc = parseDocument(FIXTURE);
    doc.setIn(["classify", 0, "then"], "delete");
    const out = doc.toString(STRINGIFY);

    expect(out).toMatch(/name: github-bot[\s\S]*?then: delete/u);
    expect(out).toContain("# Mark bot notifications read but leave human activity unread.");
    expect(out).toContain("# yaml-language-server: $schema=./schemas/rules.schema.json");
  });

  it("makeRuleNode appends a keep rule with an inline (flow) leaf", () => {
    const doc = parseDocument("classify: []");
    doc.addIn(["classify"], makeRuleNode(doc, "new-rule"));
    const out = doc.toString(STRINGIFY);

    expect(out).toContain("name: new-rule");
    expect(out).toContain("then: keep");
    // The `when` leaf is serialized inline, matching the file's hand-written style.
    expect(out).toMatch(/when: \{ field: subject, op: includes/u);
  });
});
