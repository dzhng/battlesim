# 38 — Sim: a released guided missile coasts, then goes to ground

**Status:** planned. **Depends on:** none (independent sim rule). **Lane:** simulation. **Given:** the user's rule of 2026-09-26.

## Contract

Rule P06 changes. Today, when a guided missile loses support (the launcher moved, died, stopped, or lost its sighting), it is released and "keeps flying to the last point, fixed on the ground beneath" (`battle.rs:1113–1165`), so it still homes onto where the target was. **New rule (user):** a released missile flies straight on in its current direction for `guided.release_coast_s` (0.5 s, in the fixture), then goes to ground. So a tank that sees a missile coming can pop smoke (a future concealing body, Q28). If the missile is far away, losing sight makes it miss even if the tank doesn't move; if it's close, its straight line still carries it into the target.

## API seam

- On release, the commanded point becomes the missile's position plus its current velocity × `release_coast_s`, then dropped to the ground height beneath. The missile keeps its turn limit, so it runs straight and then dives. There is no reacquisition, which is unchanged from P06.
- The fixture gets `guided.release_coast_s` (0.5). The digest already covers guidance state.
- It releases on loss of sighting whatever the cause, including concealment, so smoke works once smoke bodies exist.

## Verification

- Native tests:
  - A far missile released by losing sight misses a stationary target, and its impact lies about the coast distance past the release point, on the ground.
  - A close missile released the same way still hits.
  - A launcher that moves releases it, and the missile coasts and dives.
  - Replay parity.
  - Break the rule (fly to the old point) and the tests go red.
- Paired village and endurance reports, logged in `decisions.md`, since AT outcomes shift.

## Decision budget

- **Delegated:** how the dive is shaped within the turn limit.
- Anything else this slice has to invent is a spec gap. Record it in [`choices.md`](../choices.md) and resolve it here; don't improvise.

## Must stay green

Replay and digest parity (digests change by design; record it). Every existing scene and test, except pinned P06 results, which are updated deliberately and recorded. `bun run check` and `bun run verify` at closeout.
