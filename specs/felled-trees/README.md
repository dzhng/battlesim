# Felled trees

**Status:** all three slices implemented and committed; whole-spec review and
close remain.
**Updated:** 2026-10-05.

## Next Agent Prompt

You are finishing felled trees on branch `worktree-felled-trees`. Read the repo
README, the [renderer](../../.agents/skills/renderer/SKILL.md) skill, then this
file.

Pickup: the whole-spec review (refactor-clean, code-review, write-docs over the
branch's full diff against `main`), the choices ledger consolidation, then
close-spec. Nothing else is open.

- [x] Slice 1: the simulation records falls and publishes them per side.
- [x] Slice 2: the publication group and the browser decoder.
- [x] Slice 3: the felled-tree pass (fall, landing, lying shape, stump).
- [ ] Whole-spec review, choices ledger consolidation, close-spec.

## Goal

A tree knocked down by a tank or felled by fire visibly falls the way it was
pushed and lies by its stump, drawn as its own species. It is presentation only:
it gives no cover, blocks nothing and hides nobody, so lying its crown is pressed
low. The cover-giving `log` prop stays the only lying wood that is a body.

## Contracts

- **Fall record (`FallenBody`, `crates/contract/src/observation.rs`).** Every
  body whose row `topples` leaves one record when it goes down: its prop id,
  where its foot stood, the horizontal unit direction it fell, and the tick it
  fell. Pushed by a vehicle it falls along the hull's travel; felled by fire,
  along the round's flight, or away from the burst (a pass uses the shooter's
  line). One rule for every toppling body; presentation draws trees and ignores
  the rest today.
- **Per-side knowledge (L1).** The pushing side knows a fall at once; any other
  side once its fog sweep sees where the body stood. A late learner gets the
  original tick, so the tree lies already fallen rather than replaying.
- **Digest.** The fall log and each side's learned set are battle state, folded
  only once non-empty: a battle where nothing topples keeps its digest. Parity
  streams' digests are unchanged.
- **Publication.** A `fallenBodies` group and a trailing `fallenBodyCount`
  header word; unchanged rows reuse their view reference like corpses.
- **Frame input.** `BattleFrame.setFelled(trees)`: prop id, direction, and the
  presentation second the fall began (`(tick − 1) / tick_hz`). The scenery hides
  the standing tree by trunk id (`SceneryPlacement.forestIds`) and draws the
  fall on the frame clock.
- **Drawn shape (`scenery/felled.ts`, mirrored by the felled vertex stage).** The
  tree tips about its stump's top, gathering speed; in the second half of the
  fall it lands: its foot kicks off the stump onto the ground, its top comes to
  rest on the ground, and its leaves (and bark above the crown's base) press
  under `rest_height_m` and narrow to `rest_spread`. The trunk keeps its girth.
  What stood below the cut closes onto it: the stump stands there throughout.
- **Tunables.** `biome.trees.felled`: fall and settle times, rock-back angle,
  the lying height bound and crown spread, stump height and cut colour.

## Evidence

- Sim: `forest::a_knocked_tree_falls_along_the_hull_it_met`,
  `forest::a_side_learns_a_fall_only_by_seeing_where_the_tree_stood`,
  `destruction::a_tree_felled_by_a_burst_falls_away_from_it` (each falsified once).
- Wire: publication parity reblessed with digests unchanged; codec vector
  carries a fallen body with an id past 2²⁴ (`web/tests/observation.test.ts`).
- Renderer: `web/tests/felled.test.ts` (fall curve, landing, lying height bound,
  girth, stump clearance, stump from the trunk's rings),
  `web/tests/scenery.test.ts` (each forest tree names its trunk).
- Browser: the `consequences` scene checks a burst fells a tree blue sees and
  that the scenery draws it falling, then lying with its stump; it saves
  overhead and game-camera shots of the lying tree.
- Visual review: two unprimed critiques over burst and tank captures. Fixed from
  them: the crown pressed to paper, the log hiding its stump, the stump only
  appearing on landing, a too-bright cut face. Accepted as designed: the lying
  crown reads as a low mat (the user asked it be flat and small so it never
  hides units); soldiers and tanks pass through it (it has no body); inside a
  wood the neighbours' crowns hide most of it from the game camera; the log is
  the trunk's own low-poly mesh.
