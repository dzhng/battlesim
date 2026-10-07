// Presentation tables keyed by a published kind (a weapon row, a round, a
// vehicle), each with a `default` row that every unlisted kind takes: the
// effects' and the sound's looks read their rows the same way.

/** `table`'s row for `key`, or its `default`. */
export const pick = <T>(table: Record<string, T>, key: string): T => table[key] ?? table.default;

/** Throws unless each of `tables` (named `at.<name>`) has a `default` row. */
export function requireDefaults<K extends string>(
  at: string,
  owner: Record<K, Record<string, unknown>>,
  tables: readonly K[],
) {
  for (const t of tables) if (!owner[t].default) throw new Error(`${at}.${t} needs a default`);
}

/** `table` with a row for every weapon of `weapons` (rows as `game.json`
 *  holds them, with their `extends`) that has none of its own: its nearest
 *  ancestor's. A weapon derived from the rifle then looks and sounds like
 *  one; one with no styled ancestor keeps the default. */
export function inheritRows<T>(
  table: Record<string, T>,
  weapons: Record<string, object>,
): Record<string, T> {
  const out = { ...table };
  for (const kind of Object.keys(weapons)) {
    let at: string | undefined = kind;
    const seen = new Set<string>();
    while (at && !(at in table) && !seen.has(at)) {
      seen.add(at);
      const parent: unknown = (weapons[at] as { extends?: unknown } | undefined)?.extends;
      at = typeof parent === "string" ? parent : undefined;
    }
    if (at && at in table && !(kind in table)) out[kind] = table[at];
  }
  return out;
}
