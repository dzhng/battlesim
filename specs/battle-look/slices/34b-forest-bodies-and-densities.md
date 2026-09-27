# 34b — Sim: forests as bodies, with densities

**Status:** done (forests generate jittered trunks from `forests.densities`; foliage per 8 m fog cell from the concealing bodies; heavy push classes knock trees down and clear a lane in the world and the ground layer's `cleared` channel; sides learn fallen trees by sight; the renderer draws exactly the trunks and fog marches the foliage grid; `choices.md` and `decisions.md`, slice 34b). **Depends on:** 34. **Lane:** simulation. **Given:** [`movement-unknowns-map.html`](../movement-unknowns-map.html) (Q14, Q16).

## Contract

Tree trunks become physical bodies like every other prop (Q14). A heavy push class knocks a tree down, and the lane it clears **stops being forest** (user 2026-09-26). It becomes open ground: no forest speed penalty, no concealment, no canopy, no foliage attenuation. Flattened trees leave no obstacle and no cover, only crushed-ground marks in the ground layer, so any vehicle can use the lane. Medium and light vehicles route around standing trees. Forests gain a **density** (Q16): light, medium, dense. Density sets trunk spacing, so light forest leaves vehicle routes. It also sets concealment: light forest hides infantry far less, and vehicles in it are spotted from much farther. The cleared lane is open ground, so a tank carves a lane both visible and exposed.

## API seam

- **Map data.** `ForestDefinition.density` in the map JSON. The fixture holds `forests.densities.<name> {trunk_spacing_m, trunk_jitter, concealment_infantry, concealment_vehicle, attenuation_per_m, canopy}`.
- **Placement.** Trunks are generated deterministically from density and seed. They are authoritative sim props, and the renderer (slice 19's placement) draws exactly these trunks.
- **Concealment (Q21).** It is generalised from bodies: every 8 m fog cell precomputes the product of the `conceals` factors of the bodies standing in it, refreshed on prop change. A sight ray's effective range shrinks by the cells it crosses. This replaces `forest_depth` in `sensing.rs` and `visibility.rs`. The per-class concealment strength (infantry against vehicle) stays a hard-coded game rule. Forest cover is trunks as bodies (Q20). The forest rect and its density are only authoring input that generates trunks; at runtime a forest is its bodies plus the `cleared` channel. A knocked tree updates that cell and bumps `obstacle_revision`.
- **Knocking down.** A heavy push class meeting a trunk removes its body. The ground it clears, the vehicle's swept footprint through the trees, is recorded in the ground layer as a `cleared` channel. Every forest query reads it: the surface speed (`ground_surface_at`), `forest_depth`, sensing attenuation, cover, the fog canopy mask, and the terrain, grass and tree drawing. There is no pushing of standing trees and no fallen-trunk prop.
- **Knowledge.** Sides learn knocked trees by sight or contact, like other prop changes (L1–L2). The digest covers trunk states.

## Verification

- Scenario runner, reviewed by the agent:
  - a tank carves a lane through medium forest, and a jeep follows it at open-ground speed, spotted as if in the open;
  - a jeep routes through light forest;
  - infantry spotted at different ranges in light versus dense forest.
- Native tests:
  - density → spacing;
  - concealment by density; a cleared lane reads as open ground to speed, sensing, cover and fog;
  - trunks never block infantry, and do block vehicles below heavy push;
  - digest and replay parity.
- The fog scene's agreement check stays within 5% (the canopy mask follows standing trees).
- Paired village and endurance reports, logged.

## Decision budget

- **Delegated:** the density numbers, within the fixture, and the cleared lane's width beyond the hull.
- **Not delegated:** the rules above.
- Anything else this slice has to invent is a spec gap. Record it in [`choices.md`](../choices.md) and resolve it here; don't improvise.

## Must stay green

Replay and digest parity. Every existing scene and test. `bun run check` and `bun run verify` at closeout. Record frame cost (the benchmark's short run) and the endurance per-tick time against Q12's soft target.
