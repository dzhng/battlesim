/** Where each card of the army deck sits, and how a control group's bracket
 *  is drawn round its cards. The deck is one sequence filled top to bottom,
 *  then across, so it never has a gap: a group never forces a new column,
 *  and the slot a part-filled column leaves takes the next card. A bracket is
 *  drawn per card, on each side whose neighbour is not in the same group, so
 *  a group split across columns, or an L, needs no case of its own. */

export type Side = "top" | "right" | "bottom" | "left";

export interface DeckSlot {
  row: number;
  column: number;
  /** The card's group key, or null for an ungrouped card. */
  group: string | null;
  /** The sides of the card its group's bracket runs along, in this order. */
  sides: Side[];
  /** The group's first card: it carries the group's key. */
  keyTab: boolean;
  /** The last card of the group's top row: it carries the group's ×. */
  deleteTab: boolean;
}

/** The slot of each card, given each card's group in deck order and the
 *  deck's row count. */
export function deckSlots(groups: readonly (string | null)[], rows: number): DeckSlot[] {
  const at = (row: number, column: number) =>
    row < 0 || row >= rows || column < 0 ? undefined : groups[column * rows + row];
  return groups.map((group, i) => {
    const row = i % rows;
    const column = Math.floor(i / rows);
    if (group === null) return { row, column, group, sides: [], keyTab: false, deleteTab: false };
    const apart = (r: number, c: number) => at(r, c) !== group;
    const sides = (
      [
        ["top", apart(row - 1, column)],
        ["right", apart(row, column + 1)],
        ["bottom", apart(row + 1, column)],
        ["left", apart(row, column - 1)],
      ] as const
    ).flatMap(([side, open]) => (open ? [side] : []));
    // A group's cards are consecutive: one that runs on into another column
    // reaches the top row there; one that stays in a column tops out where
    // it starts.
    const first = groups.indexOf(group);
    const sameColumn = Math.floor(first / rows) === Math.floor(groups.lastIndexOf(group) / rows);
    const topRow = sameColumn ? first % rows : 0;
    return {
      row,
      column,
      group,
      sides,
      keyTab: first === i,
      deleteTab: row === topRow && apart(row, column + 1),
    };
  });
}
