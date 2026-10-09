# Unit models follow-up

Three pieces of work the user decided on 2026-10-08 after the
[unit models](../done/unit-models/README.md) spec closed. Read that README and its
[choices](../done/unit-models/choices.md) first: every invariant there still
holds (frames don't move to fit art, tolerances aren't widened, no number is a
requirement, exports are byte-reproducible, tests use test units).

## Next Agent Prompt

**Status, 2026-10-08:** planned; slices run in parallel worktrees
(`um2/<slice>`), merged by the coordinator.

TODO:

- [x] [01 Dragoon hull and M10 turret](slices/01-dragoon-m10.md): the user's
  report that the Stryker Dragoon's sides don't follow its sloped hull; the
  M10 reads as a small Abrams.
- [x] [02 Wreck debris scatters and fades](slices/02-wreck-debris.md): "yes,
  maybe debris that scattered can just disappear?"
- [x] [03 Disabled cards at the roster bar: air](slices/03-disabled-air.md):
  all 45 cards rebuilt to their photos (jets' layouts, planforms and stores;
  every rotorcraft's cabin and glass), with markings and the shared detail.
- [ ] [04 Disabled cards at the roster bar: ground, support, drones, infantry](slices/04-disabled-ground.md):
  "Disable cards needs to be same detail bar - they will be implemented next".

Update this section before you end a pass. When every slice lands, run the
narrowest checks plus `asset check`, push, and close this spec with close-spec.
