# 10: Swamp terrain

**Lane:** sim. Starts on day one, parallel with the render lane.
**Depends on:** nothing (slice 00 for the human view).
**Visual variable:** none. The dip shows as bare ground until slice 13.

## Contract

- **Map contract:** `MapDefinition.swamps: Vec<Swamp>`, where
  `Swamp { shape: GroundShape, depth_m }`, marked
  `#[serde(default, skip_serializing_if = "Vec::is_empty")]` so maps without
  swamps keep their hash. It is a map list of its own, like `forests`, not a
  `SurfaceKind`. Update the struct literal in `with_buildings`.
- **One lookup:** `SurfaceIndex` (`crates/sim/src/world/surfaces.rs`) answers
  "is this point in a swamp, and how far inside".
- **One dip:** `Carve` (`world/carve.rs`) carves the dip. It runs on maps with
  no river (today `height()` returns early) and registers each swamp's
  regions, for both terrain variation and navigation. Roads are never carved;
  a road surface ends the cut (decision 4). The edge spans several height
  cells (decision 21). Depth is about 0.6 m (decision 30), as data.
- **Fields:** `Surface.swamp` and the navigation `Cell.swamp`
  (`navigation/cells.rs`), copied in `base.rs` and kept through `update.rs`.
- **One standing-water predicate:** `Surface::standing_water()`, true for a
  river or a swamp.
  - `ground.rs` `burst` and `wear` both read it, so swamps take no craters or
    track wear (decisions 7 and 23).
  - `water_hit` (`world/mod.rs`) includes swamps, so rounds and sight stop at
    the water's surface, as they do for rivers.
  - The sim has no prone posture, and a standing soldier's sight and aim
    points sit above 0.6 m of water, so this adds no concealment
    (decision 32).
- **Admission** (contract `admit_world`):
  - minimum size of about three height cells across (decision 22);
  - edge grade under `steepest_grade`, with depth checked against edge width;
  - off buildings and bridge approaches, with a gap from river edges.
- **Export:** a swamp stride with each swamp's index, for presentation.
- **The `wetlands` map gains its swamp.**

## Seam tests

New file `crates/sim/tests/swamps.rs`, using test maps:

- a swamp carves its dip on a map with no river;
- the planner and movement see the same swamp (`Cell.swamp` matches
  `Surface.swamp` at sample points);
- a road across a swamp is never carved;
- a swamp takes no crater or track wear, but its dry rim does;
- a round falling into a swamp stops at its surface (mirroring the river test
  in `rivers.rs`);
- a soldier standing in a swamp is seen and can be hit;
- the sight mechanic (decision 21), on a test dip deeper than the tuned one: a
  far observer on flat ground loses a unit in the dip, and a near or high
  observer sees it;
- one admission test per rule.

## The human sees

The water scene, once its encounter walks a unit into the swamp: its feet
drop.

## Builds on PR #17

[PR #17](https://github.com/dzhng/battlesim/pull/17), merged into `staging`, made
`GroundLayer::wear` take the world and mark only ground a mover stands on, so
nothing wears ground under a bridge deck or in water. This slice widens the
"water" in that check to `Surface::standing_water()`. Branch from a `staging` that includes it.

## Native

None. Native gets everything through presentation.

## Delegated to the implementer

- the edge falloff curve;
- the exact gap from river edges and the minimum size;
- the swamp shape's representation, within `GroundShape`.

## Must stay green

Every existing map hash, battle digest and replay. Only maps that contain
swamps change, which is a named decision.
