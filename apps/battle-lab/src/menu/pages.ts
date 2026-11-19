// The menu's pages, each an address under the one menu shell
// (`MenuShell.tsx`) with its title, and home's tiles, each opening a page or
// an address outside the menu. This is plain data, so the battle's links
// (`battleLinks.ts`), the router and the browser scenes all read the same
// paths; it imports nothing, since the scenes load it in Node.

/** One page of the menu. */
export interface MenuPageRow {
  id: string;
  path: `/${string}`;
  title: string;
}

export const MENU_PAGES = [
  { id: "multiplayer", path: "/multiplayer", title: "Multiplayer" },
  { id: "skirmish", path: "/skirmish", title: "Skirmish" },
  { id: "armory", path: "/armory", title: "Armory" },
  { id: "settings", path: "/settings", title: "Settings" },
  // Opened by home's quiet developer link, not by a tile.
  { id: "developer", path: "/developer", title: "Developer" },
] as const satisfies readonly MenuPageRow[];

/** A room is an addressed visit, not an entry in home's navigation. */
const ROOM_PAGE = {
  id: "room",
  path: "/room",
  title: "Room lobby",
} as const satisfies MenuPageRow;

export type MenuPageId = (typeof MENU_PAGES)[number]["id"] | typeof ROOM_PAGE.id;

/** The row of page `id`. */
export function menuPage(id: MenuPageId): MenuPageRow {
  return id === ROOM_PAGE.id ? ROOM_PAGE : MENU_PAGES.find((row) => row.id === id)!;
}

/** One of home's tiles: where it leads, under its title. Its id names its
 *  art and the tile focused on coming back. */
export interface MenuTile {
  id: string;
  title: string;
  path: string;
}

const pageTile = (id: MenuPageId): MenuTile => ({
  id,
  title: menuPage(id).title,
  path: menuPage(id).path,
});

/** Home's tiles, in order. The tutorial is a battle: Operation Ferryman,
 *  coached as it is played, at its `scenarioHref` address (the menu routes
 *  test reads it back as a scenario battle). */
export const MENU_TILES: readonly MenuTile[] = [
  pageTile("multiplayer"),
  pageTile("skirmish"),
  {
    id: "tutorial",
    title: "Tutorial",
    path: "/battle?scenario=ferryman-valley/operation-ferryman",
  },
  pageTile("armory"),
  pageTile("settings"),
];
