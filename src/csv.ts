/** Minimal RFC-4180-ish CSV writer (dependency-free). */

function field(value: string): string {
  if (/[",\r\n]/.test(value)) {
    return `"${value.replace(/"/g, '""')}"`;
  }
  return value;
}

/**
 * Serializes rows to CSV. `columns` defines the header order; each row is looked
 * up by column key (missing keys become empty). A UTF-8 BOM is prepended so Excel
 * on Windows renders umlauts correctly. Lines use CRLF.
 */
export function toCsv(
  columns: readonly string[],
  rows: ReadonlyArray<Record<string, string>>,
): string {
  const header = columns.map(field).join(',');
  const body = rows.map((row) =>
    columns.map((col) => field(row[col] ?? '')).join(','),
  );
  return `﻿${[header, ...body].join('\r\n')}\r\n`;
}
