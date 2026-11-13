# Armory: choices

## Slice 06

- **A group's roles live in the table, with `null` for "the rest of its
  category".** `unitGroup` takes the first of the card's category's groups
  that claims one of its type's roles, else that category's first group. So
  IFV·APC is MBT's catch-all, a role no sup group claims lands in ARTY, and a
  label, order or split change is a table edit. *Why:* one rule reads the
  table, with no per-group code. *Alternative:* a switch per category.
  *Verdict:* sound.
- **`unitGroup` takes `Pick<UnitType, "roles"> | null`,** not a whole
  `UnitType`, so tests fabricate a role without a type. *Why:* roles are all
  it reads. *Alternative:* the spec's `UnitType | null`, which any caller still
  satisfies. *Verdict:* sound.
- **The group is per card, not per family.** No shipped family mixes a planned
  (typeless) variant with a typed one in a split category (veh, sup), so no
  family shows under two tabs today. A future family that did would show its
  planned variant under the category's first group. *Why:* the slice's seam is
  per card, and family-level machinery has no consumer yet. *Alternative:* the
  family's group from its first typed variant. *Verdict:* provisional; slice
  08 groups tiles by family and should decide.
- **"Search still spans every group" is a picker test,** not a
  `purchaseFilter` test. `filterCards` never sees groups, so the pure test
  would be vacuous; the picker test proves a search from the MBT tab finds
  another group of the same category and another category. *Verdict:* sound.
- **The picker widens from 720 to 840 px and its tabs sit 6 px apart (were
  12).** The nine tabs measured about 579 px plus about 163 px for the search
  and its gap, against the old 694 px content box. Now there are about 58 px
  to spare at 1280 px. The diamonds and the tab typography are unchanged, so
  the tabs keep every menu choice's look. *Alternative:* drop the diamonds or
  shrink the type. *Verdict:* sound.
- **The tutorial's buy still now crops the tab row in** (`.hud-purchase-filter`
  joins the stack and cards in its fit), so the still shows the tabs the
  prose names. The old still predated the tabs moving under the cards and
  still showed three of them above. *Verdict:* sound.
- **The UI gallery's picker cards** gained an APC (veh, one variant, dearer
  than the short wallet) and moved the Supply Truck to sup. The Jeep's HMG
  variant is now the test Gun Jeep type, so the Jeep family stays in one
  group. *Why:* the scene's hover and click checks need two families on one
  tab, now IFV·APC, and the unsold-variant checks moved to the MBT tab.
  *Verdict:* sound.
