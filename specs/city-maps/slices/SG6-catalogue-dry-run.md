# SG6: catalogue dry run

**Depends on:** none. **Kind:** throwaway spike.

## Question
Can every map move to `fixtures/maps/<id>/` in one cutover with stable scene ids and no aliases (Q-G9, L-G9)?

## Contract it unlocks
Throwaway: work in a scratch worktree, merge nothing, and write `specs/city-maps/spikes/SG6.md` (numbers, one verdict row per question, and the kill check). Delete the worktree after.

## API seam
Inventory every map producer (fixtures/routes, endurance and synthetic benchmark worlds), then prototype C09's saved-map resolution cutover and metadata in scratch. Rewrite consumers and use narrow checks while iterating, with the closeout gate once after the dry run. The inventory is discovered, not a historical fixed count. C09 consumes this verdict; SG6 does not require C09 to have landed.

## What the human can run or see
A table of every route → map, plus files touched and failing tests.

## Verification
- **Kill if** a scene id must change, or an alias or compat import is needed.
- If hud-chrome's router branch is live and touches `fixtures.json` or `router.tsx`, C61 waits for it.

## Delegated to the implementer
The script. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Everything; nothing merges.

## Feedback that would change this slice
An unaccounted map producer or route invalidates inventory coverage and requires another dry-run row.
