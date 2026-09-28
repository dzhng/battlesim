# C01: buildings aggregate

**Depends on:** G0. **Kind:** slice.

## Question
Can one building own several box parts in the sim, with one integrity, one garrison and one ruin, without moving the village?

## Contract it unlocks
```rust
MapDefinition.buildings: Vec<BuildingDefinition {
  parts: Vec<BoxPart { center, yaw, half_xy }>,
  exposed: Vec<ExposedEdge>,          // per S5: which part edges are outer walls
  height_m, floors, ground_floor_m, upper_floor_m,
  archetype: String, seed: u64,
}>
```
- At `WorldGeometry::new`, building parts become props **first**, before map props, bridges and forest trees (`world/mod.rs:118-160`). The village's 3 houses become one-part buildings and keep PropIds 0–2 (S-agg).
- `WorldGeometry::building_of(PropId)` and an owner prop per building.
- `Structures` (`structures.rs`) keys integrity by the owner, so the digest layout is unchanged.
- A hit on any part wears the building. Destruction replaces **every** part atomically in `destroy_prop` (`battle.rs:970-1026`), and `KnownProp.replaces` holds per part.
- One garrison per building. Slots sit on exposed edges only; capacity and bands are unchanged until C40.
- `village::scenario`'s `initial_garrisons` (`village/mod.rs:140-148`) indexes buildings.
- The prop export row gains an owner column, described in `layout_json`; the browser decoder cuts over in the same commit.
- Every lab map fixture migrates in the same commit (Compat).

## API seam
`contract::map`, `sim::world`, `sim::structures`, `sim::garrison` (slot source only), `world/export.rs`, `web/src/battle/sim/observation.ts`.

## What the human can run or see
`movement_shots` GIFs: a rocket hits part B and the building's integrity drops; a 3-part building collapses all at once.

## Verification
- Native tests:
  - a hit on any part wears one integrity;
  - collapse swaps every part atomically;
  - no slot lies inside another part or on a non-exposed edge.
- Village digest and replay parity (0 battles).
- `bun run --cwd web scene -- village` unchanged.

## Delegated to the implementer
The owner-id representation; the owner-column encoding. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
All village digests and replays; every lab scene.
