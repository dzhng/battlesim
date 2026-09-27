# 27d — Soldiers lean out round tall cover

**Status:** planned. **Depends on:** 27a, 27b, 27c merged. **Lane:** battle (sim, publication, poses). **Given:** the user's decisions of 2026-09-27: Hollywood realism; a soldier's own rounds never hit his cover (27c).

## Contract

A soldier sheltering behind tall cover (a trunk, a wreck, a wall end, a parked vehicle: anything taller than his muzzle) doesn't fire through it. He **leans out** to its edge, fires from there, and tucks back in. Under Hollywood realism, the picture is a man in a wood stepping out from behind a tree, firing, and ducking back.

## API seam

- **Sim:** a per-soldier lean state. When he fires from tall cover, his round's origin is the lean point: the cover body's nearer edge on the side facing the threat, at muzzle height. Otherwise he stands tucked behind the body. The lean point comes from the body's footprint and the threat bearing. It's in the digest.
- **Publication:** per own soldier, whether he's leaning and to which side. Enemy soldiers are published at their true positions as today; if leaning changes the drawn position, publish it for seen enemies too.
- **Renderer:** the pose driver slides the soldier to the lean point while he fires (the aim and fire clip), then eases him back. No new clip is needed unless one reads clearly better.

## Verification

- Native tests: a squad behind trunks fires back from lean points, and its rounds start clear of its own cover; the lean state is in replay and digest parity.
- A slice-30 runner scenario and GIF: a squad in a wood trades fire with a squad in the open. Men lean out, fire and tuck back, and their own trees take no damage from their own fire.
- A browser scene shot of the same fight at the ground camera, with an unprimed screenshot-critique.
- `bun run check` and `bun run verify`.

## Decision budget

- **Delegated:** the lean distance, the ease timing, and which side when both are open.
- Anything else goes in `choices.md`.
