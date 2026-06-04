/** True for plain objects (maps) — not arrays, not null. */
function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

/**
 * Deep-merges `override` onto `base` and returns a new value (inputs are not mutated):
 * nested plain objects merge recursively, while arrays and scalars in `override` REPLACE the
 * corresponding base value. Used to apply per-account overrides over the shared YAML config.
 */
export function deepMerge<T>(base: T, override: unknown): T {
  if (override === undefined) {
    return base;
  }
  if (!isPlainObject(base) || !isPlainObject(override)) {
    return override as T;
  }
  const out: Record<string, unknown> = { ...base };
  for (const [key, value] of Object.entries(override)) {
    out[key] = deepMerge(out[key], value);
  }
  return out as T;
}
