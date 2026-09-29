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
