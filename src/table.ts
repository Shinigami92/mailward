/** Renders a GitHub-flavored markdown table, aligned by terminal display width. */

/** Approximate display width of a single code point (CJK/emoji = 2, combining = 0). */
function charWidth(cp: number): number {
  if (cp === 0) return 0;
  // Zero-width: combining marks, ZWJ, variation selectors, zero-width space.
  if (
    (cp >= 0x0300 && cp <= 0x036f) ||
    cp === 0x200b ||
    cp === 0x200d ||
    (cp >= 0xfe00 && cp <= 0xfe0f)
  ) {
    return 0;
  }
  // Wide: CJK, Hangul, kana, fullwidth forms, and emoji/symbol blocks.
  if (
    (cp >= 0x1100 && cp <= 0x115f) ||
    (cp >= 0x2600 && cp <= 0x27bf) || // misc symbols + dingbats (⚽ ✅ …)
    (cp >= 0x2b00 && cp <= 0x2bff) ||
    (cp >= 0x2e80 && cp <= 0xa4cf) || // CJK
    (cp >= 0xac00 && cp <= 0xd7a3) || // Hangul syllables
    (cp >= 0xf900 && cp <= 0xfaff) ||
    (cp >= 0xfe30 && cp <= 0xfe4f) ||
    (cp >= 0xff00 && cp <= 0xff60) ||
    (cp >= 0xffe0 && cp <= 0xffe6) ||
    (cp >= 0x1f000 && cp <= 0x1ffff) // emoji (SMP)
  ) {
    return 2;
  }
  return 1;
}

function stringWidth(value: string): number {
  let width = 0;
  for (const ch of value) {
    width += charWidth(ch.codePointAt(0) ?? 0);
  }
  return width;
}

/** Truncates by code point (never splits a surrogate pair / emoji), adding an ellipsis. */
export function truncate(value: string, max: number): string {
  const chars = Array.from(value);
  return chars.length <= max ? value : `${chars.slice(0, max - 1).join('')}…`;
}

function cell(value: string): string {
  return value.replace(/\r?\n/g, ' ').replace(/\|/g, '/').trim();
}

export function markdownTable(
  headers: readonly string[],
  rows: ReadonlyArray<readonly string[]>,
): string {
  const cells = rows.map((row) => headers.map((_, i) => cell(row[i] ?? '')));
  const headerCells = headers.map(cell);
  const widths = headerCells.map((h, i) =>
    Math.max(stringWidth(h), ...cells.map((r) => stringWidth(r[i] ?? ''))),
  );
  const pad = (value: string, i: number): string =>
    value + ' '.repeat(Math.max(0, (widths[i] ?? 0) - stringWidth(value)));
  const renderRow = (row: readonly string[]): string =>
    `| ${row.map((v, i) => pad(v, i)).join(' | ')} |`;
  const separator = `| ${widths.map((w) => '-'.repeat(Math.max(3, w))).join(' | ')} |`;

  return [renderRow(headerCells), separator, ...cells.map(renderRow)].join('\n');
}
