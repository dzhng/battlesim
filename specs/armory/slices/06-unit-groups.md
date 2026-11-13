# 06 · Nine unit groups, in the picker first

Map rows: U9, L12. No dependencies. The Armory (08) imports an owner that this
slice has already proven in the Reinforcements picker.

## Contract

K6. The nine groups are derived for presentation only, category first. One
table owns their order and labels:

| Group | Cards it holds |
|---|---|
| REC | `rec` |
| INF | `inf`, including AT and MANPADS teams |
| MBT | `veh` + `mbt` |
| IFV·APC | the rest of `veh` |
| ARTY | `sup` + artillery |
| AA | `sup` + air defence |
| LOG | `sup` + logistics |
| HELI | `hel` |
| JETS | `air` |

A card with no unit type (the planned drones) lands in its category's first
group.

`Category` stays the game rule: objectives (`objectives.rs:222`) and the AI's
purchases (`skirmish_ai/purchase.rs:58`) read it, never a group.

## Seam

`web/src/battle/present/purchaseFilter.ts`:

```ts
export type UnitGroup = "rec"|"inf"|"mbt"|"ifv_apc"|"arty"|"aa"|"log"|"hel"|"air";
export const UNIT_GROUPS: readonly { id: UnitGroup; label: string; category: UnitCategory }[];
export function unitGroup(card: UnitCard, unit: UnitType | null): UnitGroup;
```

- **The picker.** `purchasePicker.tsx` drops `CATEGORIES` (`:34`) and the
  uppercased ids (`:177,233`), and reads `UNIT_GROUPS`. Its default tab is the
  first group.
- **Roles.** They live on `UnitType`, not on `UnitCard`. The picker gets the
  type through the session catalog (`catalog.units.has(card.id)`, as
  `panelRows.ts:530` does).
- **The tutorial.**
  - Update the prose that names the tabs (`apps/battle-lab/src/Tutorial.tsx:152-155,178`).
  - Re-shoot only the stills that show the picker strip, with
    `web/tutorial-shots.mjs`, into `assets/runtime/tutorial-media/` (renamed in
    slice 01).

## The human runs or sees

The skirmish picker's nine tabs at 1280 px, with search, for each faction.

## Verification

Write these first, in `web/tests/purchaseFilter.test.ts`. Each one reads the
table and pins no roster:
- `every game card lands in exactly one group`;
- `every group holds one category's cards`;
- `a card with no type lands in its category's first group`;
- `search still spans every group`.

Then rewrite the tab names in `web/tests/purchasePicker.test.tsx`, and update
the scenes that click tabs by label:
- `web/scenes/ui.mjs:109,135`;
- `unit-roster.mjs:79,116,150`;
- `_appJourney.mjs:185`;
- `web/tutorial-shots.mjs:219,240`.

Add a containment check: nine tabs plus the 120 px search fit at 1280 px,
through `contained` / `_spills.mjs`. That is the map's "Confirm before coding"
item 4.

**Visual gate.**
- **The one variable:** the tab strip's legibility and fit.
- **Crop:** the tab row, the search and the first row of cards.
- `compare-screenshots`: old picker against new, with the crop as the mask.
- Re-approve `purchase-*` and `menu-tutorial` on macOS.
- `screenshot-critique` last.
- Then the non-blocking `preview-shots` checkpoint.

## Delegated to the implementer

- The label typography within `game-ui`.
- Whether the search collapses to an icon at narrow widths. That applies only
  if the nine tabs don't fit; record it in `decisions.md`.

## Must stay green

- `purchasePlacement` and `skirmishPurchases`.
- The `unit-roster` and `lobby` scenes.
- `catalog.rs`'s 10-families-per-category check, which stays per category.

## Feedback that would change it

- **A label or order change:** a table edit.
- **A different split:** a table edit plus new fabricated test cards.
