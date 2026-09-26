# 34 — Sim: weight and push classes, the jeep, new props, pushing

**Status:** planned. **Depends on:** 33. **Lane:** simulation. **Given:** [`movement-unknowns-map.html`](../movement-unknowns-map.html).

## Contract

Vehicle weight class and push class are separate fields (Q3). The jeep is a new light recon unit with 360° sight and a 360° HMG. New props: sandbags, fences and trenches (Q4). Kinematic pushing (Q2) moves lighter props. Navigation classes follow the push classes (Q13). Knowledge stores each prop's last-seen pose (L1), a learned move bumps the side's revision (L2), prop poses enter the digest (L3), and contact is box against box (L8).

## API seam

- Fixture: `bodies.<kind>.{weight_class, push_class}` and `props.<kind>.{weight_class, cover_tier}`. `UnitKind::Jeep` with its hull, speeds, armour, `sensors.sight_shape.jeep = {1,1,1}`, and a turret HMG.
- `world::move_prop(id, pose)` keeps the id and bumps `obstacle_revision`.
- `navigation::NavGrid` holds classes = infantry + push classes; a pushable prop costs extra for the classes that can push it.
- A pusher replans when a newly learned prop lies on its route. A moved prop is re-learned after it has moved more than 1 m or come to rest.
- Renderer: moved props update the structures list (slice 24's `setStatics`). Grass not clearing under wrecks is a sharp edge.

## Verification

- Runner scenarios t3 reviewed: a tank shoves a jeep wreck off the road; a jeep is stopped by a fence and routes around it; the truck pushes a crate.
- Native tests: push-class matrix; no push up-class; the side that didn't see a push still sees the old pose (metamorphic); revision bump; digest parity.

- Verify with the slice-30 scenario runner: add this slice's named scenarios, run them, and **review the GIFs yourself** before accepting. Look for jams, lanes through obstacles, cover on the wrong side and twitching. The user looks last, through a non-blocking preview-shots checkpoint.

## Decision budget

- **Delegated:** push slowdown numbers and turn-on-contact, within the fixture.
- Anything else this slice has to invent is a spec gap. Record it in [`choices.md`](../choices.md) and resolve it here; don't improvise.

## Must stay green

Replay and digest parity (with the new state in the digest). Every existing scene and test, except checks this slice deliberately changes, each recorded in `decisions.md`. `bun run check` and `bun run verify` at closeout. Record frame cost in [`frame-cost.md`](../frame-cost.md) (the benchmark's short run) and the endurance report's per-tick time; the soft target is Q12's.
