# 04 — Top-attack view

**Unlocks:** a side-on, in-renderer look at the climb, pitch-over and roof strike, so the
loft and dive can be judged on screen.

**Slice variable:** the silhouette of the flight path only — does it read as "up, over,
down onto the turret" at play camera? Out of scope: missile art, smoke trail look,
explosion, overlay colours.

## Seam

- A top-attack variant in the existing ambush lab (`apps/battle-lab/src/routes/ambush.tsx`)
  or projectiles lab (`apps/battle-lab/src/routes/projectiles.tsx`), whichever already
  frames a guided shot side-on, with a saved encounter and rules pinned to a top-attack
  `atgm` override. No new route.
- Capture through the matching scene (`web/scenes/ambush.mjs` or `web/scenes/projectiles.mjs`).
- The guidance overlay draws a straight line from missile to target while it climbs. Invoke
  [game-ui](../../../.agents/skills/game-ui/SKILL.md) to decide whether that still reads;
  change it only if game-ui says so.

## Verification

1. Capture: launch, apex and impact frames, plus a GIF.
2. [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md): the same shot
   with the field off (the flat TOW path) against on.
3. [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md), unprimed, as
   the last check.
4. Check the enemy side's view: does the missile vanish high in its climb (rounds show only
   over ground the side sees)? Record the observation; change nothing unless it looks broken.
5. Non-blocking review: open with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md),
   about five minutes, then decide on the evidence, record it in [choices.md](../choices.md),
   close the shots.

Copy the accepted frames to `../assets/top-attack/`.

## Delegated

Camera angle, encounter placement, starting `loft_m`/`dive_deg` for the capture (these
become slice 11's starting values).

## Stays green

The scene's existing captures.

## Would change this slice

The user saying the arc is too high or too low on screen changes the starting loft for
slice 11, not this slice.
