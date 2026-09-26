# 30 — Movement scenario runner (dots and boxes)

**Status:** done (baseline: today's rigid two-rank formation; soldiers overlap walls, crates and wrecks, squads interpenetrate, heights follow the squad; `choices.md`, slice 30). **Depends on:** 29. **Lane:** simulation (tooling). **Given:** [`movement-unknowns-map.html`](../movement-unknowns-map.html).

## Contract

A scripted, non-interactive scenario runner, modelled on `~/dev/game`'s weave harness (`crates/sim/src/bin/weave_shots.rs` and `tests/mechanics_weave.rs`; read only, copy through the reuse manifest). It is **for the implementing agent to verify pathfinding, cover and pushing** with clean visuals: red dots for soldiers, grey boxes shaded by cover tier, craters as circles, dark boxes for vehicles, and destination rings with facing ticks. The user reviews its GIFs last. It is never interactive (user, 2026-09-26).

## API seam

- `crates/sim/tests/movement_scenarios.rs` defines named scenarios as data (map props, units, orders, duration) and asserts outcomes: arrival, no overlap with props, spacing, cover taken, no jam.
- `cargo run -p sim --release --bin movement_shots` replays **the same** scenarios through the real `Battle` and writes `throwaway/movement/<scenario>/t###.png` plus `<scenario>.gif`. A tiny CPU canvas, as in the weave harness; GIFs via ffmpeg or a small encoder. It is deterministic, so the same GIF bytes come from the same seed.
- Tiered scenarios, from simple to hard:
  - t0: open-ground move; around a single wreck; through a gap;
  - t1: into cover; not enough cover; attack-move halt;
  - t2: two squads crossing; a door or gap jam; a slope;
  - t3: a tank pushing a jeep wreck off a road; a jeep blocked by a fence.
- The first run is **today's formation movement**, as the baseline.
- Carry the throwaway mock's lessons (`throwaway/infantry-mock/`; [`movement-unknowns-map.html`](../movement-unknowns-map.html), unknown knowns).

## What you can run

`cargo test -p sim --test movement_scenarios` asserts the table (pending checks print, never fail); `cargo run -p sim --release --features shots --bin movement_shots [name…]` writes `throwaway/movement/<scenario>/t###.png`, `<scenario>.gif`, `contact-sheet.png` and `report.txt`.

## Verification

- The runner reproduces the tests' outcomes, and every scenario renders.
- A contact sheet of the t0–t1 baselines is reviewed by the agent.

- Verify with the slice-30 scenario runner: add this slice's named scenarios, run them, and **review the GIFs yourself** before accepting. Look for jams, lanes through obstacles, cover on the wrong side and twitching. The user looks last, through a non-blocking preview-shots checkpoint.

## Decision budget

- **Delegated:** the drawing code and the scenario data format.
- **Not delegated:** it stays non-interactive, and it uses the real sim, not a mock.
- Anything else this slice has to invent is a spec gap. Record it in [`choices.md`](../choices.md) and resolve it here; don't improvise.

## Must stay green

Replay and digest parity (with the new state in the digest). Every existing scene and test, except checks this slice deliberately changes, each recorded in `decisions.md`. `bun run check` and `bun run verify` at closeout. Record frame cost in [`frame-cost.md`](../frame-cost.md) (the benchmark's short run) and the endurance report's per-tick time; the soft target is Q12's.
