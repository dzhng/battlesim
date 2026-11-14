# 04 · Lobby picture: numbered pointed tags

**Depends on:** 01. **Changes digests:** no.

**Slice variable:** the entry mark's shape and digit legibility at lobby size.
Occupancy (open lanes, your lane) is slice 05; map colours, roads and objectives
are out of scope.

## Contract

`skirmish_picture(map, sites)` (`crates/mapgen/src/inspect/lobby.rs`) draws each
entry as look D (D5): a side-coloured body with an arrow tip pointing into the
map and a dark digit in the body. Each entry's group carries
`data-entry="<side>"` and `data-lane="N"`, with N from `sim::entries::lane_number`.
The function's signature is unchanged and it reads no players.

To keep all geometry in Rust, the group also carries the own-lane mark (the
white band), hidden by default; slice 05 only reveals it.

## What a human can see

`/labs/map-picture` cells and the lobby on 1v1 (one road, lane 1), a Medium
two-road map, and a 3v3.

## Verification

- `crates/mapgen/tests/picture.rs`: on admitted maps of each size, each side's
  `data-lane` values are 1..n and increase down the picture (picture y), on
  both sides. Compare positions, not markup strings.
- Visual gates: `game-ui` first. `compare-screenshots`: the lobby before shot
  ([assets/lobby-before-3v3.png](../assets/lobby-before-3v3.png)) against
  after, and after against the chosen mock (option D in
  [lane-tag-looks.html](../visualizations/lane-tag-looks.html)). Crop: the
  left and right 15% of the map frame, where the tags sit. Check that a tag
  never reads as an objective's white circle. `screenshot-critique` last.
  `preview-shots`, non-blocking.

## Docs in this slice

Rewrite [I20](../../done/team-skirmish/choices.md) ("entries are unlabelled …")
to describe numbered lane tags.

## Delegated

Tag proportions, digit weight and halo, within the mock's look and the
picture's `WEIGHTS` scaling.

## Out of scope (even if visibly wrong)

Terrain colours, roads, objective rings, the vignette.

## Must stay green

The picture test's existing checks (every entry and objective drawn, Blue on
the left); digests.
