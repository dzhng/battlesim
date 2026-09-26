# 08 — Sim: ground delivery and resync

**Status:** done. **Depends on:** 07. **Lane:** simulation.

## Contract

Each side learns ground cells by sight: a 1 m cell counts as seen when its 8 m fog cell is seen (decision). It receives only changed cells, with a full resync on reset or side switch. Publication stays bounded.

## API seam

- Side knowledge owns the learned cells.
- The transport sends `GroundPatch {epoch, side, base_revision, revision, full, cells[]}` in the publication.
- The client's `GroundView` applies patches before returning credits, and rejects stale epochs.
- Transport cursors stay outside the digest. Learned cells are digested.

## What you can run or see

A paused village ground inspector in `/lab/ground`.

## Verification

Tests:
- a repeated publish is idempotent;
- exhausted credits;
- unchanged cells are never re-sent;
- side switch and reset send a full snapshot, then deltas;
- deltas equal the full state;
- hidden cells never leak (metamorphic);
- endurance bytes per tick recorded.

Record frame cost in [`frame-cost.md`](../frame-cost.md) for this slice.

## Decision budget

- **Delegated:** Patch packing.
- Anything else this slice has to invent is a spec gap. Record it in [`choices.md`](../choices.md) and resolve it here; don't improvise.

## Must stay green

Replay and digest parity. Every existing scene and test, except the pixel checks this slice retunes: each retune is recorded in `decisions.md`, never loosened silently. `bun run check` and `bun run verify` at closeout.

## Feedback that changes this slice

None expected.
