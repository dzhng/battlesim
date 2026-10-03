/** Changed value paths, independent of authored JSON whitespace and key order. */
export function changedPaths(before: unknown, after: unknown, path: string[] = []): string[][] {
  if (JSON.stringify(before) === JSON.stringify(after)) return [];
  if (
    before &&
    after &&
    typeof before === "object" &&
    typeof after === "object" &&
    Array.isArray(before) === Array.isArray(after)
  ) {
    const oldValues = before as Record<string, unknown>;
    const newValues = after as Record<string, unknown>;
    return [...new Set([...Object.keys(before), ...Object.keys(after)])].flatMap((key) =>
      changedPaths(oldValues[key], newValues[key], [...path, key]),
    );
  }
  return [path];
}

export function valueAtPath(value: unknown, path: string[]): unknown {
  for (const key of path) {
    if (!value || typeof value !== "object") return undefined;
    value = (value as Record<string, unknown>)[key];
  }
  return value;
}
