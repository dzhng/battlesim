# 34 — Sim: weight and push classes, the jeep, new props, pushing

**Status:** planned. **Depends on:** 33. **Lane:** simulation. **Given:** [`movement-unknowns-map.html`](../movement-unknowns-map.html).

## Contract

Vehicle weight class and push class are separate fields (Q3). The jeep is a new light recon unit with 360° sight and a 360° HMG. New props: sandbags, fences and trenches (Q4), plus an **anti-tank wall** (Q18): infantry cross it, every vehicle is blocked, and its weight class is heavy, so only a future super-heavy push class can clear it. It is **a row of individual tooth props, with no special wall concept in the sim**. Each tooth is a small (about 1.2 m) heavy prop that blocks vehicles only in `PropKind::blocks`, like wrecks. Navigation's clearance field then closes the line to every hull, since the gaps leave far under 1 m + half-width. Infantry plan as if the teeth weren't there and dodge them with per-soldier collision (31–32), because the 2 m grid can't reliably plan through 1.5 m gaps. **Each tooth is its own medium-cover spot** for slice 33. Kinematic pushing (Q2) moves lighter props. Navigation classes follow the push classes (Q13). **Vehicles and props are one kind of physical body (Q14): a box, a weight class, and a push class for vehicles. Nothing passes through anything, whatever the side, and one push rule covers every prop; live vehicles are immovable (Q15) and only block. Trees and forest densities are slice 34b.** Knowledge stores each prop's last-seen pose (L1), a learned move bumps the side's revision (L2), prop poses enter the digest (L3), and contact is box against box (L8).

## API seam

- **Body table (Q19):** `props.<kind>.{blocks: {infantry, vehicle…}, occludes, conceals, weight_class, cover_tier, integrity, destroyed}` in the fixture. `PropKind::blocks` and `occludes` leave `contract/map.rs`, and `PropKind` is only the key. Movers: `bodies.<kind>.{weight_class, push_class, loudness}`, and hearing reads loudness instead of `is_vehicle()`.
- Fixture: `bodies.<kind>.{weight_class, push_class}` and `props.<kind>.{weight_class, cover_tier}` (inside the body table). `UnitKind::Jeep` with its hull, speeds, armour, `sensors.sight_shape.jeep = {1,1,1}`, and a turret HMG.
- Vehicle-vs-vehicle contact for every pair, whatever the side (Q14), in the same box-box code as pushing. `movement.rs:262`'s friendly-only check is deleted. A blocked vehicle waits, then the higher id detours after a stall; enemy vehicles use local wait-and-detour only, since they aren't in a side's grid.
- `world::move_prop(id, pose)` keeps the id and bumps `obstacle_revision`.
- `navigation::NavGrid` holds classes = infantry + push classes; a pushable prop costs extra for the classes that can push it.
- A pusher replans when a newly learned prop lies on its route. A moved prop is re-learned after it has moved more than 1 m or come to rest.
- Renderer: moved props update the structures list (slice 24's `setStatics`). Grass not clearing under wrecks is a sharp edge.

## Verification

- Runner scenarios t3 reviewed: infantry cross an anti-tank wall while a tank is stopped and routes around it; a squad ordered to the teeth takes one tooth each; a tank shoves a jeep wreck off the road; a jeep is stopped by a fence and routes around it; two tanks of opposite sides meet on a road and neither passes through; the truck pushes a crate.
- Native tests: an own and an enemy tank meeting head-on never overlap and resolve by wait-then-detour; a column through an enemy wreck field; push-class matrix; no push up-class; the side that didn't see a push still sees the old pose (metamorphic); revision bump; digest parity.

- Verify with the slice-30 scenario runner: add this slice's named scenarios, run them, and **review the GIFs yourself** before accepting. Look for jams, lanes through obstacles, cover on the wrong side and twitching. The user looks last, through a non-blocking preview-shots checkpoint.

## Decision budget

- **Delegated:** push slowdown numbers and turn-on-contact, within the fixture.
- Anything else this slice has to invent is a spec gap. Record it in [`choices.md`](../choices.md) and resolve it here; don't improvise.

## Must stay green

Replay and digest parity (with the new state in the digest). Every existing scene and test, except checks this slice deliberately changes, each recorded in `decisions.md`. `bun run check` and `bun run verify` at closeout. Record frame cost in [`frame-cost.md`](../frame-cost.md) (the benchmark's short run) and the endurance report's per-tick time; the soft target is Q12's.
