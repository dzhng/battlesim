# Army-card reserve marks

**Status:** built 2026-11-16 on `feature/army-card-low-ammo`.

## Why

The army deck is where a player manages a large force. Until now a unit's
ammunition was only in its hover panel: the card said health and whether it was
riding or going home, but nothing said a Paladin was down to its last shells or
a supply truck was nearly empty. A unit that empties a finite reserve either
loses its reason to exist (artillery, a supply truck, a missile team) or keeps
fighting on an unlimited gun (a tank's HMG, a rifle squad's rifles); the card
should say which. Presentation only: no command, observation, digest or replay
change.

## Why this shape

- **One reserve, not one number per gun.** `unitReserve` averages every finite
  ammunition kind once, whatever its magazine size, so a tank's AP and HE weigh
  the same and a helicopter's autocannon does not drown its missiles. Unlimited
  kinds are skipped; a unit with only unlimited ammunition has no reserve to
  warn about.
- **A truck's stock is its reserve.** A unit with no ammunition kinds falls
  back to its finite supply stock; that is the amount a supply truck spends
  itself.
- **Amber while it can still fight, light red only once it cannot.** An
  unlimited ammunition kind left means the unit still has a weapon
  (`stillArmed`), so an empty finite reserve stays a warning. With no unlimited
  kind left, the empty reserve takes the shared "can't" red.
- **Form before colour.** Low is the magazine with two rounds at its foot; out
  is the crossed magazine; a supply truck uses the crate, and an empty one the
  crossed supply box. The colours repeat the existing `warn`/`bad` roles rather than
  adding a hue.
- **One mark per card.** A card's corner already carries a rider's seats or a
  return to base; those outrank the reserve, and a rider's reserve belongs to
  its carrier's card. No second badge, no stack.
- **The simulation already says it.** Every mount's rounds and a truck's stock
  are in the observation; the card derives the mark from them. Nothing is
  published or digested differently.

## Invariants

- Unlimited-only units never carry the mark; an empty corner is the healthy
  state.
- A mark appears at or below a quarter of the finite reserve; the threshold is
  a display constant (`LOW_AMMO`), not a simulation rule.
- A card already showing seats or returning-to-base never shows the reserve
  mark.
- The mark never changes a command, an observation layout, a digest or a
  replay.

## Pointers

- The amount and tone: `LOW_AMMO`, `Reserve`, `unitReserve` in
  `web/src/battle/present/panelRows.ts`.
- The card, priority and accessible name:
  `web/src/battle/present/armyDeck.tsx`.
- The corner slot and tones: `.hud-army-ammo` in `web/src/hud.css`.
- The glyph: `low_ammo` in `packages/scene-assets/src/icons.ts`, written to
  `assets/icons/hud/low_ammo.svg`.
- Tests: `web/tests/unitReserve.test.ts` (the reserve arithmetic).
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
