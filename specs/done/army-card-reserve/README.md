# Reserve warnings

**Status:** built 2026-11-16 on `feature/army-card-low-ammo`.

## Why

The army deck is where a player manages a large force. Until now a unit's
ammunition was only in its hover panel's weapon rows: the card said health and
whether it was riding or going home, but nothing said a Paladin was down to its
last shells or a supply truck was nearly empty, and a compact info card said it
only once a kind was already empty. Now a reserve running out is a unit state
like suppression: its own row in the info panel (LOW AMMO, OUT OF AMMO, LOW
SUPPLY, OUT OF SUPPLY), its icon by the name on every info card, compact too,
and the same icon in the army card's corner. Presentation only: no command,
observation, digest or replay change.

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
- **A unit state, said once.** The reserve is one `byName` state row
  (`reserveRow`), so the panel's existing rule draws its row and its icon by
  the name on every card, compact included. The army card draws that row's
  icon. One owner, so the hover panel, the compact card and the deck can't
  disagree. It replaced a separate out-of-ammo check by the name.
- **Ammunition, not protection.** Trophy charges are not rounds: a tank whose
  Trophy is spent still fights, so it carries no reserve warning. Its
  protection row still shows its empty charges.
- **One warning tone, the panel's.** A low or empty reserve is the amber `warn`
  every warning row uses. It never becomes the shared "can't" red: that role
  belongs to a refused order, not a weapon running dry.
- **Form before colour.** Low is the magazine with two rounds at its foot; out
  is the crossed magazine; a supply truck uses the crate, and an empty one the
  crossed supply box. Form and word tell the two apart, not a second colour.
- **One mark per army card.** A card's corner already carries a rider's seats
  or a return to base; those outrank the reserve, and a rider's reserve belongs
  to its carrier's card. No second badge, no stack. The info panel has no such
  limit: it lists the reserve row beside every other state.
- **The simulation already says it.** Every mount's rounds and a truck's stock
  are in the observation; the card derives the mark from them. Nothing is
  published or digested differently.

## Invariants

- Unlimited-only units never carry the warning; no row and an empty corner
  are the healthy state.
- The warning appears once the emptiest finite kind (or the stock) is at or
  below `LOW_RESERVE`, a display constant, not a simulation rule.
- The info panel's row, the icon by its name and the army card's mark are one
  `reserveRow`: they always agree.
- Every reserve warning uses the panel's amber warning tone; nothing promotes
  it to the "can't" red.
- An army card already showing seats or returning-to-base never shows the
  reserve mark.
- The mark never changes a command, an observation layout, a digest or a
  replay.

## Pointers

- The amount, the rows and their tone: `LOW_RESERVE`, `Reserve`, `unitReserve`,
  `reserveRow` and the four reserve rows of `STATE_ROWS` in
  `web/src/battle/present/panelRows.ts`.
- The card, priority and accessible name:
  `web/src/battle/present/armyDeck.tsx`.
- The corner slot and tone: `.hud-army-reserve` in `web/src/hud.css`.
- The glyph: `low_ammo` in `packages/scene-assets/src/icons.ts`, written to
  `assets/icons/hud/low_ammo.svg`.
- Tests: `web/tests/unitReserve.test.ts` (the reserve arithmetic),
  `web/tests/unitConditions.test.tsx` (the row and the icon by a compact
  card's name) and `web/tests/armyDeck.test.tsx` (the card's mark is the
  panel's row).
- Visual states: the reserve rows alone and compact in the panel workbench's
  "own states" group (`apps/battle-lab/src/panelSpecimens.ts`), and the
  "Army roster" case in
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

A second follow-up made the reserve a unit state in the info panel: its own
row, and its icon by the name on compact cards, where only an empty kind used
to show. The army card now draws that row's icon. An
unprimed critique of the panel shots read low and out as close, the same amber
with a slash between them; that is deliberate (a weapon running dry is not a
refused order), and the word on the row says which. Out of supply keeps the
existing no-stock box, the icon a served unit already shows when its truck is
empty.
