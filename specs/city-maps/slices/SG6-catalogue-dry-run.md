# SG6: catalogue dry run

**Depends on:** C09. **Kind:** slice.

## Question
Can every map move to `fixtures/maps/<id>/` in one cutover with stable scene ids and no aliases (Q-G9, L-G9)?

## Contract it unlocks
Throwaway: work in a scratch worktree, merge nothing, and write `specs/city-maps/spikes/SG6.md` (numbers, one verdict row per question, and the kill check). Delete the worktree after.

## API seam
A script moves the 12 map-bearing fixtures (the village, the 11 labs, and the endurance map inline at `crates/sim/src/endurance.rs:73-75`), drafts each `meta.json`, rewrites imports and runs `bun run check`.

## What the human can run or see
A table of every route → map, plus files touched and failing tests.

## Verification
- **Kill if** a scene id must change, or an alias or compat import is needed.
- If hud-chrome's router branch is live and touches `fixtures.json` or `router.tsx`, C61 waits for it.

## Delegated to the implementer
The script. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Everything; nothing merges.
