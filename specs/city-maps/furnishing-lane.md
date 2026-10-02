# Furnishing lane: streets with things in them, and country round the old maps

A separate session works this lane in parallel with the others. It is placement logic in the generator, proved by rules and counts. The models for what it places belong to the visual lanes; this lane places the bodies the simulation already has.

## The contract

- **A generated town's streets hold street furniture** (parked cars, wrecks, barriers, street trees) placed by rule, as bodies that give cover and block as their catalog rows say, without closing a street: infantry and vehicle routes through the town survive, entrances stay clear, and the open approach stays open ([C46](slices/C46-street-placement.md), over the bodies of [C44](slices/C44-street-bodies.md)).
- **Every existing map has surroundings.** The village, the labs and the benchmark fields are small arenas with nothing beyond their edge. Each gets generated country round a reserved arena whose own battle does not change: the same digests inside, real ground outside ([C56](slices/C56-fixture-surroundings.md) → [C34 village](slices/C34-village-surroundings.md) · [C35 labs](slices/C35-lab-surroundings.md) · [C36 benchmarks](slices/C36-benchmark-surroundings.md)).

## Ownership

| This lane owns | Others own (stay out) |
|---|---|
| `crates/mapgen/src/parcels/` and new placement modules in `crates/mapgen/` | `crates/mapgen/src/joints.rs` and the layout's roads and towns (the map lane is changing them) |
| `fixtures/maps/` for the surroundings work; the placement rows of `fixtures/map-presets.json` | `fixtures/game.json` rules, `fixtures/props/` (rows exist; a missing property is a small named commit) |
| Its tests under `crates/mapgen/tests/` | `crates/sim/src/` (the [scale lane](scale-lane.md)), the renderer and `packages/scene-assets` (the visual lanes), the menu and preparation code |
| This file's Status section | `README.md`'s Next Agent Prompt and TODO |

The generator's output is pinned: a change bumps the generator version, re-blesses `fixtures/parity/map-layout/` and `encounter/`, and re-saves `fixtures/maps/market-town` with the commands in the [fixtures guide](../../fixtures/README.md). No repo-wide renames.

## Work, in order

1. **[C46 street placement](slices/C46-street-placement.md).** Picture the street before writing the rule (invoke `tweak-mechanics`): what a squad crossing it would use for cover, where a tank must still pass. Prove on a seed sweep that routes survive and entrances stay clear.
2. **[C56 reservations](slices/C56-fixture-surroundings.md):** the inventory of every shipped map and the one reservation contract.
3. **[C34](slices/C34-village-surroundings.md), [C35](slices/C35-lab-surroundings.md), [C36](slices/C36-benchmark-surroundings.md):** surroundings for each, with the arena's battle digests unchanged, or each change named.

## How to work

Read [`AGENTS.md`](../../AGENTS.md): narrow checks only, no full gate, no frozen-record tests. The battle digest is the proof that an arena did not change. `mapgen inspect` draws a plan as a picture; look at a few before trusting a count. Branch from main, merge main often, push small green passes. Add an Outcome to each slice file and decisions to [`choices.md`](choices.md) under a heading for the slice. Scratch output goes in gitignored `throwaway/`.

**One GPU, shared.** Several sessions are working at once. Run every scene through the GPU lock in the main checkout (the README's Checks section has the command), never two at a time, and keep heavy jobs (a sweep, a full Rust test run) to one at a time.

## Status

Not started. Update this section, not the README, at the end of each pass: what landed, the counts, what is next, and anything the other lanes need to know.
