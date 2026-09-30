# Original producer ownership

This is evidence at the revision in `receipt.json`; the live route owner remains
`apps/battle-lab/src/fixtures.json`. The inventory probe confirms every registered
route has a scene module and every scene has a route. No catalogue IDs are inferred
from this table. Exact original source bytes/hashes are in `inventory.json`.

| Physical source | Actual routes / purpose | Source vs encounter owner |
|---|---|---|
| Embedded village map | Village, replay, watch, benchmark, fog, fog-look, lean; ground's village mode | `crates/sim/src/village/mod.rs::scenario` composes the fixture map with deployment; `village/scripts.rs` owns comparison-script building discovery. Browser factories use `useBuiltScenario`; native reports call the same scenario/trial owners. |
| geometry-lab | Geometry surface/ray probes; authority credit/tick exercises; ballistics flight arena | Saved geometry is shared; each route supplies units, event emitters and scripts through `apps/battle-lab/src/scenarios.ts::labScenario`. |
| sensors-lab | Occlusion/identification arena; contacts evidence arena | One saved physical arena, distinct units/scripts for sensors and contacts. |
| movement-lab | Routing, terrain costs, injected obstruction | Saved physical map; route owns dynamic event/order composition. |
| weapons-lab | Weapon mounts and evidence under cover | Saved physical map; route owns deployment/order composition. |
| consequences-lab | Blast, suppression, wreck and corpse consequences | Saved forest/wall arena; scenario owns starting units and firing events. |
| deployment-lab | Deployment progress/reversal | Saved small road/obstacle arena; scenario owns supply setup. |
| ambush-lab | Guided AT missile tactical cases | Saved cover geometry; variant/unit orders remain encounter composition. |
| garrison-lab | Enter, share, leave, facade visibility and collapse | Saved houses/wall; soldiers and timed orders remain encounter composition. |
| supply-lab | Recovery/stock exercise | Saved flat arena; damaged units and stock remain encounter composition. |
| ground-lab | Learned craters/tracks/marks and side switching | Saved flat arena; authored marks, emitters and moving units remain encounter composition. |
| readouts-lab | Production weapon/action-state readouts | Saved cover geometry; unit situations remain encounter composition. |
| Endurance code recipe | Native `endurance_report`, browser endurance route | `crates/sim/src/endurance.rs::scenario` owns the physical field/house base. Seeded late wrecks are physical augmentation; corpses/units/orders are encounter state. Separate wave/remains RNG streams and the wreck→corpse draw continuation must be preserved during extraction. |
| Foundation render-only patch | Gaussian raised ground, primitive camera/pick/reset exercise | `apps/battle-lab/src/routes/foundation.tsx` owns its negative-coordinate analytic ground. Current MapDefinition cannot exactly express that frame/height recipe; C56 remains pending. |
| Workbench render-only ground | Model/socket/hitbox/feed inspection | `apps/battle-lab/src/workbench/benchWorld.ts::benchWorld` owns its negative-coordinate flat ground. C56 remains pending; no invented replacement physical map. |
| No physical map | Panels and offline sound routes | Presentation/rules/units are inputs, but no physical world is produced. |

## Loaded contract and consumers

Current saved map decoding is `contract::map::MapDefinition`. Scenario composition
is `ScenarioDefinition`, with `Rules` and unit catalogue resolved independently.
`crates/sim/src/fixtures.rs::village` supplies the native rules/unit-catalogue fixture;
`crates/game-wasm/src/lib.rs::{village_scenario,endurance_scenario}` expose the same
builders. The authority receives the composed scenario and creates one Battle.
`apps/battle-lab/src/useStaticWorld.ts::useStaticWorld` currently constructs the
main-thread query/export world from the same map; removing that duplication is C33.

Asset-fit inputs also bypass acquisition today: `packages/scene-assets/src/authority.ts::fixtureAuthority`,
`web/asset.mjs` and `benchWorld.ts::{footprint,placedProp}` read fixture-derived body,
canopy or placed geometry facts. They must receive resolved geometry through the
approved built-WASM prerequisite without adding IO/WASM to scene-assets. Scene
checks such as `web/scenes/village.mjs` read physical map data directly; report and
analytic/resource test callers are enumerated in the frozen caller logs. Rules,
presentation and biome imports must not be mistaken for physical map producers.

## Cutover risks that remain open

The core has not migrated any of these callers. Actual prepared physical f64 values,
unit IDs, seeded draw order, scripts, replay outcomes and scene checks still need
factory-level preservation. In particular, a Value/JSON/factory/decoder round trip
is a different numerical boundary from raw saved token decoding; the earlier
publication80 corpus does not prove every shipping Village prop bit. C01's missed
ScriptedBlue building-discovery consumer was independently identified and corrected
by the coordinator; that encounter omission was the measured ground-scene drift,
not evidence that this unresolved numeric boundary is harmless.

The next pass must implement the actual adapters and source/encounter split through
the common resolver, consume C72 shared shape admission, and preserve source/art
readiness as a separate gate. Foundation/workbench remain pending rather than
being counted as migrated maps. Original scenes and all original evidence remain
at their original identities; metadata and source relocation cannot repin them.
