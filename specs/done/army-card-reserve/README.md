# Army-card reserve marks

**Status:** built 2026-11-16 on `feature/army-card-low-ammo`.

## Why

The army deck is where a player manages a large force. Until now a unit's
ammunition was only in its hover panel: the card said health and whether it was
riding or going home, but nothing said a Paladin was down to its last shells or
a supply truck was nearly empty. The card now repeats the panel's own warning:
the emptiest finite ammunition kind, or a truck's stock, in the panel's amber.
Presentation only: no command, observation, digest or replay change.

## Why this shape

- **The emptiest kind, as the panel lists them.** `unitReserve` takes the
  lowest share of any finite ammunition kind, whatever its magazine size.
  Unlimited kinds are skipped; a unit with only unlimited ammunition has no
  reserve to warn about. A tank with full HE and an empty AP rack still warns,
  because that is the row the panel marks spent — averaging the two would hide
  it.
- **A truck's stock is its reserve.** A unit with no ammunition kinds falls
  back to its finite supply stock; that is the amount a supply truck spends
  itself.
- **One warning tone, the panel's.** A low or empty reserve is the same amber
  `warn` the panel's out-of-ammo mark uses. It never becomes the shared "can't"
  red: that role belongs to a refused order, not a weapon running dry.
- **Form before colour.** Low is the magazine with two rounds at its foot; out
  is the crossed magazine; a supply truck uses the crate, and an empty one the
  crossed supply box. Form tells the two apart, not a second colour.
- **One mark per card.** A card's corner already carries a rider's seats or a
  return to base; those outrank the reserve, and a rider's reserve belongs to
  its carrier's card. No second badge, no stack.
- **The simulation already says it.** Every mount's rounds and a truck's stock
  are in the observation; the card derives the mark from them. Nothing is
  published or digested differently.

## Invariants

- Unlimited-only units never carry the mark; an empty corner is the healthy
  state.
- A mark appears at or below a quarter of the emptiest finite kind (or the
  stock); the threshold is a display constant (`LOW_AMMO`), not a simulation
  rule.
- An empty kind, or an empty stock, is the same out state the unit's panel
  marks by its name: `outOfAmmo` and `unitReserve` agree.
- Every reserve mark uses the panel's amber warning tone; the card never
  promotes it to the "can't" red.
- A card already showing seats or returning-to-base never shows the reserve
  mark.
- The mark never changes a command, an observation layout, a digest or a
  replay.

## Pointers

- The amount, out state and tone: `LOW_AMMO`, `Reserve`, `unitReserve` and
  `outOfAmmo` in `web/src/battle/present/panelRows.ts`.
- The card, priority and accessible name:
  `web/src/battle/present/armyDeck.tsx`.
- The corner slot and tone: `.hud-army-ammo` in `web/src/hud.css`.
- The glyph: `low_ammo` in `packages/scene-assets/src/icons.ts`, written to
  `assets/icons/hud/low_ammo.svg`.
- Tests: `web/tests/unitReserve.test.ts` (the reserve arithmetic and its
  agreement with the panel) and `web/tests/armyDeck.test.tsx` (the card's
  marks).
- Visual states: the "Army roster" case in
  `apps/battle-lab/src/routes/panels.tsx`, captured by
  `web/scenes/_battleDeck.mjs`. Its pictures are the `army-*-army-roster*`
  baselines; the shared corner size also re-pins the seat and return marks on
  the `army-*-control-groups*` and `army-*-returning-to-base*` pictures.

## Visual provenance

`assets/reserve-states.png` is the twelve-card "Army roster" workbench deck at
3×: full, low and out tanks, a low rifle section, an out gun jeep, low and out
supply trucks, an unlimited-only recon team and AT team, a low rider and a low
withdrawing tank. `assets/supply-low.png` and `assets/supply-out.png` are the
two supply states at the same scale.

An unprimed critique of the first shot read the solid-fill magazine as a
battery and found the supply states differed only by colour. The glyph now
shows two rounds at the magazine's foot; an empty supply truck uses the crossed
crate; and the corner mark is 1.1 em at a 3 px inset, clear of the 25 px role
icon (the first version clipped it by about 2 px). The user approved the
smaller, higher mark.

A follow-up made the card defer to the unit panel's indicator: the emptiest
kind decides the mark (not the average), and every reserve mark uses the
panel's amber warning instead of a second red. The old red drew a gun jeep
empty of its only cannon and a tank empty of AP and HE but still holding an
HMG in two different colours, while both panels showed the same amber
out-of-ammo mark.
