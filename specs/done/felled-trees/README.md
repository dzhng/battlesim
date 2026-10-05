# Felled trees

A tree knocked down by a tank or felled by fire falls the way it was pushed,
lands beside its stump and lies there, drawn as its own species. The fallen tree
is presentation only: it has no body, gives no cover, blocks nobody and conceals
nothing. The cover-giving `log` prop (`fixtures/props/forest/log.json`) stays the
only lying wood that is a body.

## Why it works this way

- **The simulation records, the renderer draws.** The battle already knew when a
  tree went down; before this, the only trace it published was seen-cleared
  ground, so a tree simply vanished. The simulation now records each fall as a
  fact (where, which way, when) and each side learns it under the same sight
  rule as everything else. No game rule reads the record; drawing it changes
  nothing a battle decides.
- **One rule for every toppling body.** A fall is recorded for any body whose
  catalog row `topples`, however it went down: pushed by a hull, or destroyed
  by fire whatever its destroyed state becomes. Trees are not special-cased;
  today the renderer draws only trees and ignores the rest (walls and railings
  could use the same records later).
- **Which way it falls is what pushed it.** A vehicle's travel; a hitting or
  passing round's flight; away from a burst. A burst at the foot itself (no
  direction) falls back to one of eight compass directions by prop id, with no
  trigonometry, so native and WebAssembly agree bit for bit.
- **Lying, it must not read as cover.** A war-film fallen tree is a big obstacle
  to hide behind, but the simulation gives it none, so the drawing must not
  promise it. The crown is pressed low and narrowed; the trunk keeps its girth
  so it still reads as a trunk. A soldier standing in a fallen crown is taller
  than all of it.
- **Felled trees are drawn by their own small pass**, not by tilting the
  forest's instances: tilt would break the forest's static far buffers, its
  chunk bounds and its upright fog probe, and felled trees are few.

## Invariants

- **Per-side knowledge (L1).** The side whose vehicle pushed a body over knows
  the fall at once; a fall by fire is learned by every side, the firing side
  included, only once its fog sweep sees where the body stood. A late learner
  gets the original tick: past the fall's end it is drawn already lying, never
  replayed; sooner, it sees the rest of the fall.
- **Digest.** The fall log and each side's learned set are battle state; an
  empty log folds nothing, so a battle where nothing topples digests exactly as
  before. Battles where something topples changed digest when this shipped (a
  named decision). Nothing that decides a battle reads the log: only sight
  learning, the observation and the digest do.
- **A forest tree leaves the drawn forest only by its fall record** (by trunk
  id), never by seen-cleared ground, so a tree never vanishes without its stump
  and log. Seen-cleared ground still clears shrubs and the forest-floor
  dressing; the dressing is laid again when a fall arrives.
- **The lying bound.** Lying, no leaf rises above `trees.felled.rest_height_m`;
  the trunk's girth near its foot sets the floor for that bound (~0.9 m for the
  current trunks), which is why it is 1 m.
- **The vertex stage and its CPU mirror agree.** `felledVertex`
  (`packages/battle-renderer/src/frame/sceneryLayer.ts`) and `felledPoint`
  (`packages/battle-renderer/src/scenery/felled.ts`) compute the same pose;
  tests hold the drawn shape to the rules through the mirror, so a change to
  one is a change to both.
- **Presentation reads tunables from data**: `trees.felled` in
  `fixtures/biomes/summer.json`, validated in `validateBiome`
  (`packages/battle-renderer/src/terrain/biome.ts`).

## Where it lives

- **Record and its rule:** `FallenBody` and `ObservationFrame::fallen_bodies`
  (`crates/contract/src/observation.rs`); `Battle::fell`, `knock_down` and
  `destroy_part` (`crates/sim/src/battle.rs`); the push direction on `Shove`
  (`crates/sim/src/movement/push.rs`), `StructuralHit`
  (`crates/sim/src/damage.rs`) and `Pass::along`
  (`crates/sim/src/flight/mod.rs`); `SideKnowledge::learn_fallen`
  (`crates/sim/src/knowledge.rs`).
- **Wire:** the `fallenBodies` group and `fallenBodyCount` header word
  (`crates/sim/src/publication.rs`, `GROUP_ROW_WIDTHS`); `FallenBodyView` in the
  decoder (`web/src/battle/sim/observation.ts`).
- **Frame input:** `BattleFrame.setFelled` (`packages/battle-renderer/src/scene.ts`),
  fed by the battle session from `fallenBodies`, each fall starting at its
  tick's start, `(tick − 1) / tick_hz`, as combat effects do
  (`apps/battle-lab/src/useBattleSession.ts`).
- **Drawing:** `SceneryPlacement.forestIds` names each placed forest tree's
  trunk (`scenery/placement.ts`); `scenery/felled.ts` owns the fall
  (`fellPose`, `restAngle`, `fallAngle`, `lyingShare`), the stump (`trunkAt`,
  `stumpMesh`) and the records; the scenery layer packs and draws them through
  the caster, depth and colour passes.
- **Tests:** `forest::a_knocked_tree_falls_along_the_hull_it_met`,
  `forest::a_side_learns_a_fall_only_by_seeing_where_the_tree_stood`,
  `destruction::a_tree_felled_by_a_burst_falls_away_from_it`,
  `destruction::any_toppling_body_felled_by_fire_publishes_its_fall`
  (`crates/sim/tests/`); the publication parity records and codec vectors;
  `web/tests/felled.test.ts`, `web/tests/scenery.test.ts`
  (each forest tree names its trunk); the `consequences` browser scene (a burst
  fells a tree blue sees; it is drawn falling, then lying with its stump).

## Dead ends

- **Pressing the whole lying tree at the hinge** left a paper-thin crown and a
  plank of a trunk floating at stump height. Only leaves (and bark above the
  crown's base) are pressed now, bounded where the crown actually lies, near
  the ground at the far end.
- **Lying with its foot on the stump** hid the stump under the log's end. On
  landing the foot kicks two trunk radii along the fall and drops to the
  ground, so the stump stands bare.
- **Cutting the stump from bark vertices in a height band** found nothing: a
  probe of the shipped tree appearances showed their trunks are rings with no
  vertices between the foot and ~1–2 m. The stump's radius interpolates
  between the foot ring and the next one; `trunkAt` throws on a tree without
  those rings, which the browser scene's load would surface.
- **Pressing as the tree lands by angle cubed** popped the crown flat in the
  last fraction of a second; landing now eases over the fall's second half.
- **Hiding forest trees by seen-cleared ground as well as by fall** could drop a
  tree without its stump at a cell boundary; falls alone own it now.

## Visual provenance

There was no reference image: the standard was the request. The player should
see the tree go down and leave a stump. The lying tree should look like its
species but be flatter and smaller than a standing crown, so it never hides
units, since it gives no cover. Two unprimed visual critiques (fresh agents
shown only the captures, during the build) judged shots like those below.
Fixed from them: the paper crown, the stump hidden by the log, the stump
appearing only on landing, and a too-bright cut face. Accepted as designed:
- the lying crown reads as a low mat;
- soldiers and tanks pass through it;
- inside a wood the neighbours' crowns hide most of it from the game camera;
- the log is the trunk's own low-poly mesh.

- [Mid-fall, from a raised angle, surrounding trees not drawn](assets/falling-side.png):
  a shell-felled tree tipping away from the burst over its stump.
- [Lying, the same view](assets/lying-side.png): stump with its cut face, the log
  on the ground beside it, the crown pressed to a low mat at the far end (the
  red bar across it is the lab's tracer overlay).
- [A tank's lane, from above](assets/tank-lane-top.png): the stump and log of a
  tree the tank pushed over, beside the tank (drawn see-through under trees).

Choices made where the plan was silent are in [choices.md](choices.md).
