// The menu's pages: each one is an address under the one menu shell
// (`MenuShell.tsx`), with its title, and whether home lists it as a tile.
// This is plain data, so the battle's links (`battleLinks.ts`), the router and
// the browser scenes all read the same paths.

/** One page of the menu. */
export interface MenuPageRow {
  id: string;
  path: `/${string}`;
  title: string;
  /** Home lists the page as a tile. Otherwise something else opens it. */
  tile: boolean;
}

export const MENU_PAGES = [
  { id: "multiplayer", path: "/multiplayer", title: "Multiplayer", tile: true },
  { id: "skirmish", path: "/skirmish", title: "Skirmish", tile: true },
  { id: "tutorial", path: "/tutorial", title: "Tutorial", tile: true },
  { id: "armory", path: "/armory", title: "Armory", tile: true },
  { id: "settings", path: "/settings", title: "Settings", tile: true },
  // Opened by home's quiet developer link, not by a tile.
  { id: "developer", path: "/developer", title: "Developer", tile: false },
] as const satisfies readonly MenuPageRow[];

/** A room is an addressed visit, not an entry in home's navigation. */
const ROOM_PAGE = {
  id: "room",
  path: "/room",
  title: "Room lobby",
  tile: false,
} as const satisfies MenuPageRow;

export type MenuPageId = (typeof MENU_PAGES)[number]["id"] | typeof ROOM_PAGE.id;

/** The row of page `id`. */
export function menuPage(id: MenuPageId): MenuPageRow {
  return id === ROOM_PAGE.id ? ROOM_PAGE : MENU_PAGES.find((row) => row.id === id)!;
}
