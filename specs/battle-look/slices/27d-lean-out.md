# 27d — Soldiers lean out round tall cover

**Status:** planned. **Depends on:** 27a, 27b, 27c merged. **Lane:** battle (sim, publication, poses). **Given:** the user's decisions of 2026-09-27: Hollywood realism; a soldier's own rounds never hit his cover (27c).

## Contract

A soldier sheltering behind tall cover (a trunk, a wreck, a wall end, a parked vehicle: anything taller than his muzzle) doesn't fire through it. He **leans out** to its edge, fires from there, and tucks back in. Under Hollywood realism, the picture is a man in a wood stepping out from behind a tree, firing, and ducking back.

## API seam

- **Sim:** a per-soldier lean state. When he fires from tall cover, his round's origin is the lean point, at muzzle height. Otherwise he stands tucked behind the body. It's in the digest.
- **One rule for every body shape (user, 2026-09-27):** the lean point comes only from the cover body's footprint and the threat's position, never from the body's kind. A trunk's disc, a building's corner, a tank's or wreck's hull corner and a wall's end all work the same way.
  - Take the footprint's silhouette edge (the tangent point, or the box corner) nearest the soldier, from which the threat is in line of fire.
  - Step him just past it, clear of the footprint by his body radius.
  - If neither edge gives a line of fire, he doesn't lean. He holds, and the cover search may move him.
- **Buildings as cover:** today the building row has no `cover_tier`, so a soldier outside never takes cover at a building. Give buildings a cover tier (heavy, like a wall) in the fixture, so that corner cover exists. Garrisons are unchanged.
- **Publication:** per own soldier, whether he's leaning and to which side. Enemy soldiers are published at their true positions as today; if leaning changes the drawn position, publish it for seen enemies too.
- **Renderer:** the pose driver slides the soldier to the lean point while he fires (the aim and fire clip), then eases him back. No new clip is needed unless one reads clearly better.

## Verification

- Native tests: soldiers lean out and fire clear of their own cover behind a trunk, a building corner, a tank hull and a wreck; none leans when neither edge gives a line of fire; the lean state is in replay and digest parity.
- Slice-30 runner scenarios and GIFs: a squad in a wood trades fire with a squad in the open; a squad at a building corner and one behind a parked tank do the same. Men lean out, fire and tuck back, and their own trees take no damage from their own fire.
- A browser scene shot of the same fight at the ground camera, with an unprimed screenshot-critique.
- `bun run check` and `bun run verify`.

## Decision budget

- **Delegated:** the lean distance, the ease timing, and which side when both are open.
- Anything else goes in `choices.md`.
