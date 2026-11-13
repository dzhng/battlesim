# 08 · Armory page: the sheet and layout A

Map rows: U1–U7, U9, D2, L10, L11, L13. Depends on 01 (paths), 05 (`derived`)
and 06 (groups).

## Contract

**The address.** K1 gains `/armory` and `/armory?unit=<card id>`, and
`MENU_PAGES` gains the armory row with `tile: true`.

**Layout A:**
- on the left, faction tabs, then the nine group tabs, then one tile per
  family;
- on the right, the full sheet;
- the centre is kept for the stage. Until slice 11 it shows the held menu
  backdrop.

**The selection** lives in the page's own state. It is mirrored to the address
with `history.replaceState(history.state, "", url)`, the
`routes/airWar.tsx:246-251` pattern, so a pick never remounts the page.

**The tile** gets key art. The game's own owners decide everything on the sheet.

## Seam

### The sheet model

`web/src/battle/present/unitSheet.ts` is pure:

```ts
export interface UnitSheet {
  card: UnitCard; planned: boolean;                 // L10: no type → planned
  body: "squad" | "hull" | null;
  men?: number; hpPerMan?: [number, number];        // U2: a range when soldiers differ
  hp?: number; armor?: Record<"front"|"side"|"rear"|"roof", number>;
  movement: SheetStat[]; senses: SheetStat[]; capabilities: string[];
  mounts: { name: string; kinds: SheetAmmo[] }[];    // D2: by mount, ammunition kinds beneath
}
export interface SheetAmmo { weapon: string; name: string; description: string;
  ammo: number | "unlimited"; firesRegardless: boolean; reach: Reach[]; stats: SheetStat[] }
export interface SheetStat { key: string; value: number; scale: number /* 0..1 of the roster's best, same body */ }
export function unitSheet(catalog: SessionCatalog, card: UnitCard): UnitSheet;
```

- **Mount names.** `equipmentName` (`panelRows.ts:357`, private today) is
  exported and reused. It stays the one owner of mount names.
- **Published facts.** `firesRegardless` and `reach` are read from
  `WeaponRow.derived` and never recomputed.
- **Bars** scale against the best in the roster of the same body: squads
  against squads, hulls against hulls.

### The page

`apps/battle-lab/src/armory/ArmoryPage.tsx`, with
`useArmorySelection(): [Selection, set]` and
`Selection = { faction; group: UnitGroup; card: string }`. An unknown or absent
card picks the first tile of the first non-empty group and replaces the
address.

**The list:**
- It holds the cards whose `roster.factions` contains the open tab, so a
  shared card appears under each of its factions.
- Cards are grouped by `unitGroup`.
- There is one tile per `card.family`; variant chips sit on the sheet.
- Tiles carry `data-cue-hover="off"`.

**The sheet:**
- The fires-regardless icon (no "default" text), with the tooltip "Fires at
  anything it sees, even what it can't hurt".
- Ammunition as a count or ∞.
- Reach per band: ground, low air, high air.
- A class limit appears only where `reach.only` is set.
- Squads show Men and HP per man (U2).
- No raw class lists, and no internal fields (U3).

**Planned cards** (L10) are greyed tiles with their silhouette and a "planned"
sheet that lists their planned weapons.

**Key art.** `assets/source/menu/armory.png` → `assets/runtime/menu/armory.jpg`,
with an entry in `provenance.json` like the existing three. Add
`data-art="armory"` in `web/src/menu.css`. This is menu art, not bake output,
so L1 doesn't hold it.

**The fixture.** `armory`, route `/armory`, with no `ROUTES` entry, so it stays
a menu path. Its scene is `web/scenes/armory.mjs`. It is pinned in
`uiCoverage.test.ts`.

Run `game-ui` before building.

## The human runs or sees

- The menu's new Armory tile.
- `/armory`, then a faction, a group, a family and a variant.
- `/armory?unit=…` opened directly and refreshed.
- Back and Escape.

## Verification

Write these first.

`web/tests/unitSheet.test.ts`:
- `every card in the game catalog yields a sheet` (generative);
- `each mount lists exactly its weapons' kinds under its equipment name`;
- `the fires-regardless mark follows the published fact, not default`: a
  fabricated finite default gun has no mark;
- `squad HP per man is a range when soldiers differ` (fabricated);
- `the best unit of a body kind fills its bar, and squads scale against squads`;
- `a card with no type yields a planned sheet with its planned weapons`.

`web/tests/armoryPage.test.tsx`:
- `a pick replaces the address and never remounts the page` (mount count 1,
  `history.length` unchanged);
- `an unknown unit falls back to the first tile and rewrites the address`;
- `each faction tab lists exactly the cards that faction fields` (generative);
- `each group tab shows only its own group's cards` (generative).

`web/scenes/ui.mjs`:
- an `armory` baseline at 1280 and 1920 under `contained`, with fixed test
  content, never the growing roster;
- the `menu` baseline changes only inside the new tile's box (Q4 item 6):
  `compare-screenshots` with that box masked must report identity.

**Visual gate.**
- **Shot 1.**
  - The one variable: layout A's legibility, list and sheet.
  - Crop: the left list and the right sheet, with the centre masked.
  - Compare against `assets/mock/a-tank.png`, the chosen layout.
  - Include one squad sheet, one weapon mount row, and one planned card.
- **Shot 2.**
  - The one variable: the tile art sits in the set.
  - Crop: the tile row.
- `screenshot-critique` last on each.
- Then the non-blocking `preview-shots` checkpoint.

## Delegated to the implementer

- Tile density, and which stats get bars.
- The order of sheet sections.
- The icon glyph and the style of the variant chips.
- Tooltip wording, except U7's sentence above, which is given.
- The composition of the key art.

## Must stay green

- Slices 01 and 06.
- `router.test.ts` and `navigation.test.tsx`.
- `panels.mjs`: `equipmentName` is only exported, not changed.
- The `infoPanel` and `purchasePicker` tests.

## Feedback that would change it

- **Sheet density, panel widths or tile size:** layout only.
- **A stat the user wants added:** goes to `unitSheet`.
- **"Show the class list":** reopens U7, which is the user's call.

## Reslice if too big

Split it into the sheet model with its tests, and then the page with its
visuals.
