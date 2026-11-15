# Separate farm buildings (placeholder)

**Status:** placeholder. Nothing is sliced or implemented yet.
**Updated:** 2026-11-16.

## Next Agent Prompt

You are picking up a decision the user made but deferred: a farmstead's barn,
house and dovecote stand apart round a yard, so they should be three
buildings, each its own garrison, not one. Parts that really touch (a house
and its wing, sharing a wall) stay one building. When the user starts this,
confirm the approach below with them. It changes generated maps and battle
outcomes, so run `tweak-mechanics` first, then `write-spec` to turn this page
into a sliced plan. Update this section before you end each pass.

- [ ] Confirm the approach (B below) and the "touching" rule.
- [ ] Find every template whose parts stand apart, not only the farmsteads.
- [ ] Slice the asset, generator and re-recording work.

## The problem

A farmstead is one building template with several parts
(`packages/scene-assets/blender/city/farmsteads.py`). The simulation holds a
placed building as one aggregate: one squad garrisons all its parts, and its
integrity is shared, so shelling the barn collapses the house as well. On
screen, a squad in the barn is also in the house fifty metres away, and three
houses that a viewer sees as three are one fighting position. The held-building
outline (`heldOutline` in `packages/battle-renderer/src/garrisonOverlay.ts`)
draws one line round them all, which is honest about the rule but shows how
odd it is.

## Approaches weighed

- **A. Split only the garrison.** Group a building's parts by contact and
  seat one squad per group. Facade edges already name their part
  (`MaterializedEdge::part`), so seats and eyes filter cleanly. Small, but
  integrity stays per farm: shelling the barn still ejects all three squads,
  which a veteran would find ridiculous. A stopgap at best.
- **B. Make each farm building its own building (preferred).** The farmstead
  script exports each building as its own template, and the generator places
  a farmstead as a cluster of buildings round a yard. Garrison, integrity,
  collapse and remains then follow per building with no new simulation
  concept, and "several parts, one building" keeps meaning parts that touch.
  It deletes a special case instead of adding one.

## What B touches

- `farmsteads.py`: three regional families × three farms, each farm's one
  shell split into a shell per building, then the template library re-baked.
- The generator's farmstead placement (`crates/mapgen`): a yard of separate
  placements, keeping today's layouts.
- Other city scripts (`industry.py` and any other multi-part template): check
  for parts that stand apart and treat them the same way.
- Every generated map with a farm changes, so map fixtures, recorded battles
  and the menu reel need re-recording and inspecting (a named decision).

## Open questions for the walk

- Is "touching" exactly sharing a wall, or a small gap (a rule value) under
  which close buildings still count as one?
- Should anything else that stands apart today (outbuildings, compounds)
  split in the same pass?
