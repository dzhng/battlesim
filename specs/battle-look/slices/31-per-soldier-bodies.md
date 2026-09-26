# 31 — Sim: per-soldier bodies

**Status:** done (soldiers are bodies with their own position, ground height and collision; seeded arrival arrangements; formation offsets and everything on them deleted; `choices.md` and `decisions.md`, slice 31). **Depends on:** 30. **Lane:** simulation. **Given:** [`movement-unknowns-map.html`](../movement-unknowns-map.html).

## Contract

Soldiers become real bodies with their own world position, **ground height at their own spot** (L4) and collision (L5). The squad anchor is the living soldiers' centroid. Formation offsets and slots are deleted (D1: no set formation), along with everything built on them (L6): garrison exit and collapse scatter, resupply's free slot, reform, side-steps, separation. Range and the engage decision stay per squad, from the anchor (D3).

## API seam

- `Soldier {id, position, velocity, hp, corpse, …}`, with no `offset` and no `formation`. `Unit.position` is derived from the soldiers, and the digest covers each soldier's position.
- On each move order, arrival spots are a seeded random arrangement: within the spread (Q7), at least 2 m apart, clear of props. Cover comes in slice 33.
- Sensing, aim points, hit bodies and publication keep reading per-soldier positions; their shapes are unchanged.

## Verification

- Native tests: soldiers stand on the ground at their own spot on a slope; no soldier ends inside a prop; the arrangement is deterministic per seed and differs per order; garrison exit, collapse and resupply work without slots; replay parity.
- Runner scenarios t0 and t1, reviewed.
- Paired village and endurance reports, logged in `decisions.md`.

- Verify with the slice-30 scenario runner: add this slice's named scenarios, run them, and **review the GIFs yourself** before accepting. Look for jams, lanes through obstacles, cover on the wrong side and twitching. The user looks last, through a non-blocking preview-shots checkpoint.

## Decision budget

- **Delegated:** internal structure.
- **Not delegated:** the decisions in the map.
- Anything else this slice has to invent is a spec gap. Record it in [`choices.md`](../choices.md) and resolve it here; don't improvise.

## Must stay green

Replay and digest parity (with the new state in the digest). Every existing scene and test, except checks this slice deliberately changes, each recorded in `decisions.md`. `bun run check` and `bun run verify` at closeout. Record frame cost in [`frame-cost.md`](../frame-cost.md) (the benchmark's short run) and the endurance report's per-tick time; the soft target is Q12's.
