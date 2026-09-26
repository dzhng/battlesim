# 32 — Sim: per-soldier movement

**Status:** planned. **Depends on:** 31. **Lane:** simulation. **Given:** [`movement-unknowns-map.html`](../movement-unknowns-map.html).

## Contract

Company of Heroes-style movement (Q6): one squad A* corridor, then each soldier walks his own lane: the corridor plus his offset, wander and speed variation, with soft personal space (Q10, about 0.8 m) and his own short A* for the final stretch to his spot. The lane falls back toward the corridor wherever the offset would cross an obstacle (mock lesson). Stagger and variation are seeded.

## API seam

- `movement::soldier_steer` and `movement::final_leg`. The corridor stays `navigation::plan`, one search per squad.
- **Two resolutions (Q27):** the 2 m grid marks a cell passable for infantry if its 0.5 m sub-cells hold a gap a 0.3 m-radius soldier fits through, so squads route through lines of teeth and gaps between wrecks. Each soldier's lane steering and final-leg A* use the exact body boxes on a local 0.5 m window, about 40 m, and that is how he walks around a wreck, crate or tooth and squeezes through a 1-man gap.
- The spread, wander, speed variation, personal space and stagger numbers go in `fixtures/village.json` (`infantry_movement`).
- Rerouting on a new or moved prop follows Q13's rules: each soldier's final stretch replans on the side's revision change. **Props are in soldiers' planning like everything else (Q26):** the final-leg A* and lane avoidance use the same grid and body table.
- **Vehicles (Q23):** live vehicles of either side are moving bodies to soldiers. Soldiers steer around them, and yield sideways when a vehicle's ~2 s path crosses them. Vehicles never stop for soldiers; a soldier who can't clear is shoved aside without damage. The squad side-steps (`DEFLECTIONS_DEG`) are deleted.

## Verification

- Runner scenarios: a squad threads a line of dragon's teeth one man at a time; soldiers walk around a wreck and a crate stack rather than through them.
- Runner scenarios: a tank drives through a resting squad and they yield; a crate is pushed across a soldier's lane and he replans.
- Runner scenarios t0–t2 reviewed: no lanes through wrecks, no jams at gaps, staggered and varied arrival, no twitching.
- Native tests: route searches per order ≤ 1 + (soldiers), and no rigid soldier jam in the gap scenario.
- The endurance report is within the Q12 soft target (+15%).

- Verify with the slice-30 scenario runner: add this slice's named scenarios, run them, and **review the GIFs yourself** before accepting. Look for jams, lanes through obstacles, cover on the wrong side and twitching. The user looks last, through a non-blocking preview-shots checkpoint.

## Decision budget

- **Delegated:** the steering formulation and the numbers within the fixture.
- Anything else this slice has to invent is a spec gap. Record it in [`choices.md`](../choices.md) and resolve it here; don't improvise.

## Must stay green

Replay and digest parity (with the new state in the digest). Every existing scene and test, except checks this slice deliberately changes, each recorded in `decisions.md`. `bun run check` and `bun run verify` at closeout. Record frame cost in [`frame-cost.md`](../frame-cost.md) (the benchmark's short run) and the endurance report's per-tick time; the soft target is Q12's.
