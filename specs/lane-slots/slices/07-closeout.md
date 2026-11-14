# 07 · Docs and the one full run

**Depends on:** 01–06.

## Contract

The docs describe lanes as designed, and the whole system passes once.

## Work

- `development.md#game-rules`: the skirmish line that says a side's players
  share its roads gains the lane clause (a slot holds a numbered lane; arrivals
  stay shared and quickest).
- `crates/sim/README.md`: the entries paragraph names `sim::entries` as the
  owner of lane numbers, slot-to-lane and the opening entry.
- `apps/battle-lab` README (lobby section), if it describes the status line,
  size reason or region picker.
- Confirm I17 and I20 were rewritten (slices 01 and 04) and record slice 02's
  value and probe summary in this spec's notes.
- `review` (shape, diff, docs) before pushing.
- Pull the LFS files the full run needs, then run everything once.

## Done when

All slices ticked in the README, the full run passes, and the spec is ready for
`close-spec`.
