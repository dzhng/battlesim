# Air-war flight (placeholder)

**Status:** placeholder. Nothing is sliced or implemented yet.
**Updated:** 2026-10-13.

## Next Agent Prompt

You are picking up the flight behaviour [air war](../done/air-war/README.md) specified but the simulation does not yet deliver. The air-war follow-up gave each air fixture its own lab scene, and those scenes show that fixed-wing aircraft don't fly the way the spec describes. When the user starts this work, run `explore-unknowns`, then `write-spec`, and turn this page into a sliced plan. Load `tweak-mechanics` before changing any rule: judge each behaviour by picturing the real moment on the battlefield. Update this section before you end each pass.

- [ ] Map the unknowns with the user (see the open questions below).
- [ ] Slice: jets fly at their own altitude, not the helicopters'.
- [ ] Slice: jets circle (orbit) a goal instead of hovering on it.
- [ ] Slice: ground attacks are attack runs.
- [ ] Slice: dedicated simulation tests for dogfights and the afterburner.
- [ ] Revisit what follows from altitude (routing clearance, sight, AA reach) once jets fly high.

## Why it exists

The air-war spec says fixed-wing units occupy `HighAir` at an authored cruise height, and enter from a friendly edge, circle, attack, dogfight, burn fuel and withdraw. The simulation and the air lab scenes show four gaps against that:

1. **Jets fly at the helicopters' height.** Fixed-wing aircraft fly at the same 30 m cruise height as helicopters and route on the same low-air grid around tall buildings. That's why the air routing clearance is "the largest airframe any battle may field", which is now a jet (`crates/sim/src/navigation/air.rs`, `hull_limits.air` in `fixtures/game.json`).
2. **Jets hover at their goal.** Given a move or attack goal, a jet stops over it like a helicopter instead of circling.
3. **Ground attacks aren't runs.** A jet attacks a ground target from wherever it is, rather than lining up, diving or passing, releasing and pulling away.
4. **Dogfights and the afterburner have no dedicated simulation tests.** Dogfight joining and breaking, and afterburner fuel burn, are held only by browser checkpoints. Afterburner speed and ground-attack accuracy have no behavioural test at all (`specs/done/air-war/README.md`).

## What already holds, to build on

- **Air layers:** `HighAir` sensing and fog rules, layered ground AA, SEAD, radar contacts, flares, fuel, the afterburner toggle, and the dogfight state (`crates/sim/src/movement/air.rs`, `flight/`, `sight.rs`, `weapons.rs`).
- **Lab scenes:** each air fixture has its own scenario and lab scene, which record the claims above as they stand today. They are the browser checkpoints to turn green.
- **Airframe sizes:** jets carry real hull dimensions from their art, and `hull_limits.air` is sized to the largest (Su-34).
- **Sound:** the air alarm and the positional jet loop play in battle, now that jets are classed as fixed-wing in Rust (`crates/presentation/src/sound/`).

## Panel follow-ups

These are smaller UI items, left from the air panels pass (`web/scenes/baselines/panels/panels-aircraft-states.png`):
- At the panels' real size the radar dish icon is a small blob; it only reads as a dish when zoomed. The UNKNOWN, RADAR CONTACT and BEARING rows all share it, and in red on UNKNOWN it loses most of its detail.
- The UNKNOWN row starts a few pixels left of the rows below it.
- With no context, the dogfight icon (two chevrons meeting nose to nose) can pass for a bow tie.

## Open questions for the walk

- Cruise height: one `HighAir` height for every jet, or per type? Do jets descend for attack runs, and if so, do they then enter low-air routing and AA reach?
- Orbit: what radius and direction, and does it follow the target or stay on the goal?
- Attack run: one pass per order, or repeated passes until the target dies or ammunition runs out? How does a run interact with flares, AA and dogfights?
- Once jets fly high, should the low-air routing clearance fall back to the largest low-flying airframe (the helicopters)?
