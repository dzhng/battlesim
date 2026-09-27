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
- **The cover search counts the lean (user, 2026-09-27).** Today a spot counts against a seen enemy only if there's a straight line from the spot's own muzzle to him (`take_cover.rs` `line`). So a soldier tucked squarely behind tall cover (a building corner, a tank hull, a trunk dead in line) is rejected: his muzzle is inside the shadow. After this slice, a spot counts if the soldier has a line of fire from the spot or from its lean point. The search and the firing use the same lean function, so the cover he picks is always cover he can fight from.
- **The squad optimises for fire positions (user, 2026-09-27).** A lean point is a claimed place, just as a cover spot is: two soldiers can't lean out of the same edge. When the squad resolves cover, its objective, in order:
  1. **as many soldiers as possible able to engage the enemy**, from a spot or a free lean point. "Engage" means **his round reaches the enemy**: a seen enemy soldier within his weapon's range, and a line from his spot or lean point that no body crosses except his own cover (which he leans round, and which 27c's own-cover rule passes through). It is **not** "the fire code will shoot". Since 27c, rifles and HMGs fire into a breakable blocker such as a neighbouring trunk, and a man who could only chip a tree does not count as engaging. He's moved to a place with a clear line. Share the line test with the fire code, with the own-cover exception, so the two can't drift apart;
  2. then the strongest cover for those who can engage.
  
  A soldier who can't engage looks for other cover from which he can. If there's none, he steps out to the nearest place he can engage from (today's `step_out_m`). He never sits in cover he can't fight from while a place to engage from is in reach.
  
  The worked example is a 3-man squad at a building corner. The man at the corner leans out and fires. The two stacked behind him have no free edge, so each finds other cover with a line of fire, or steps out to fire back if there's none.
- **The squad owns an area; its men move freely inside it (user, 2026-09-27).** It replaces the per-soldier leash, where each man searched `search_m` round his own arranged spot.
  - **The area:** a disc round the squad's **anchor**, with radius half its spread plus `search_m` (about 14 m for 8 men, roughly today's reach, now shared).
  - **Assignment:** the squad assigns fire positions and cover spots inside the area together:
    1. the most soldiers able to engage;
    2. then the strongest cover;
    3. then the least total walking.
    
    A man who can't engage from anywhere in the area steps out, still inside it. The arranged random spots stay as the fallback for men with nothing better.
  - **The anchor only moves on an explicit order.** It's set by a move order's destination, an attack-move's halt point, or a script or player order, and it's never recomputed from where the soldiers stand.
    - Today's re-resolve centres on "where its soldiers stand" (`take_cover.rs` `hold`), so repeated re-resolves can chain and walk the squad across the map with no order. That goes.
    - The anchor lives in unit state and in the digest.
    - The squad's published position for markers and the Space overlay stays the anchor, so the marker never drifts.
- **Buildings as cover:** today the building row has no `cover_tier`, so a soldier outside never takes cover at a building. Give buildings a cover tier (heavy, like a wall) in the fixture, so that corner cover exists. Garrisons are unchanged.
- **Publication:** per own soldier, whether he's leaning and to which side. Enemy soldiers are published at their true positions as today; if leaning changes the drawn position, publish it for seen enemies too.
- **Renderer:** the pose driver slides the soldier to the lean point while he fires (the aim and fire clip), then eases him back. No new clip is needed unless one reads clearly better.

## Verification

- Native tests: soldiers lean out and fire clear of their own cover behind a trunk, a building corner, a tank hull and a wreck; none leans when neither edge gives a line of fire; the cover search now picks spots squarely behind tall cover when the lean gives a line of fire, and still rejects them when it doesn't; the lean state is in replay and digest parity.
- A no-drift test: a squad holding in a long firefight (several minutes, with cover destroyed round it and many re-resolves) keeps its anchor exactly, and every soldier stays inside the area.
- The trunk-row case from 27c: a squad behind a row of trunks facing an enemy in the open. Men whose line crosses a trunk that isn't their cover move until each can engage, by leaning, re-covering or stepping out. No soldier ends the fight chipping a stranger's trunk while a clear line exists within the area.
- A slice-30 scenario for the 3-man corner example, checking that all three end able to engage: one leaning at the corner, two re-covered or stepped out.
- Slice-30 runner scenarios and GIFs: a squad in a wood trades fire with a squad in the open; a squad at a building corner and one behind a parked tank do the same. Men lean out, fire and tuck back, and their own trees take no damage from their own fire.
- A browser scene shot of the same fight at the ground camera, with an unprimed screenshot-critique.
- `bun run check` and `bun run verify`.

## Decision budget

- **Delegated:** the lean distance, the ease timing, and which side when both are open.
- Anything else goes in `choices.md`.
