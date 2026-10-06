# Courtyards: paved, lived-in block interiors

A generated town's dense districts are built as real block interiors: each building stands in its own paved yard, bounded by its region's wall, railing or chain-link where there is room, car parks line the streets, and the ground between is lawn crossed by paths and dressed with benches, trees, playgrounds and each region's signature pieces (China bike sheds, concrete ping-pong tables and outdoor gyms; New York chain-link half-courts, garages and dumpsters; Paris kiosks and pétanque). Garden suburbs and villages keep their lawns and gain sheds, hedges, fences, washing lines and tables. Every piece is an ordinary body of the catalog, so it blocks, hides and covers by what it is made of. The [choices ledger](choices.md) records the decisions the plan left open.

## Why

The user's complaint ([before](assets/before/)): the ground between a town's buildings was one unbroken lawn in every region, "super empty", where a real town has "stone or concrete blocks with actual stuff in them". The fix had to keep three things the game already promised: the generator stays deterministic, the simulation stays the one authority (props are physics, not decoration), and units can still go where a player sends them.

That last constraint shaped everything. Each attempt to fill a court tightened what vehicles could reach, and the move probe (`crates/mapgen/examples/battle_sweep.rs`, every type × size × region) is what held the line: where filling courts broke placement, the fix went into the simulation's rules for where a unit stops (below), not into emptier courts. It ships at 41 of 81 maps with an unplaced destination, against 38 before courtyards and 57 at its worst (ledger).

## Principles and invariants

- **A court is infantry ground; a vehicle is owed a way in, not a way everywhere.** Each yard front, car-park aisle and sizeable lawn keeps one way in from a street for the widest hull; squads reach every door, garden, gate, car park and lawn point they could before; no body stands on a door's way out. The reach claims live in `routes_survive_the_furniture_on_every_type_and_size_of_map` and the door's way out in `yards_are_bounded_with_gates_and_car_parks_keep_their_aisles` (`crates/mapgen/tests/street_props.rs`).
- **One paved kind for hard ground that is no way through.** `SurfaceKind::Paving` (`crates/contract/src/map.rs`, formerly `Sidewalk`) holds courts, yards, car parks, paths and industrial aprons; `is_road()` names the carriageways explicitly. The terrain shader holds exactly four paved kinds, so material is a look per region, never a new kind.
- **The map names its region once.** `MapDefinition.regional_family`, set from `parcels::family()` and validated against every building; the request may ask for one (`GenerationRequest.region`, refused at `$.region` if unlisted). The renderer, the paving finish and the page's appearance request (`familyLooks`) read only this field.
- **One legality check, one fence routine.** Every body goes through `Field::legal` (`crates/mapgen/src/street_props/field.rs`); every fence, wall and railing through `fence_round` and its layout `side_panels` (`street_props/mod.rs`), which presets validation also calls.
- **Courts and gardens are dressed last.** `place_courts_and_gardens` runs after `open_country::cover` and keeps off the woods, because the sight certificate's copses need their ground. At the authored-part limit courts take their share first (`street_props.courts.share`, `split`); gardens get the rest.
- **Dressing is data.** Groups, boundaries per region, car parks, lawns and budget split live in `street_props` and each district's `props.courts` / `props.gardens` in `fixtures/map-presets.json`; validation refuses unknown kinds, groups and families, fences whose corners stay open, a `beside` group wider than its spacing, and a lawn with no street way.
- **A shared piece has regional looks; a different body is a different kind.** Appearances tagged `regional_family` are fetched only for that region's maps, gzipped and counted against `MAP_DOWNLOAD_MAX_BYTES` (`fetchedOnRequest`, `packages/scene-assets/src/schema.ts`); where a region has its own look for a kind, only its looks are candidates (`PropAppearances`, `packages/battle-renderer/src/models/propAppearance.ts`). Each court and garden piece is at most 1 MiB raw in every look, held by `downloadBudget.test.ts`.
- **A unit stops where it can stand and get to.** A vehicle's destination (`NavGrid::destination_point`, `crates/sim/src/navigation.rs`, reachability in `navigation/pockets.rs`) is the nearest point within a hull's length where its whole hull stands clear of known bodies and that it can reach on its side's known grid, or, where that ground is cut off, the nearest reachable one within 48 m; where reaching that room is a long way round, it stops short on its way instead (`StopShort`, `crates/sim/src/units.rs`, weighed against `navigation.stop_short_detour_ratio`); a cover post (`unwedged`, `crates/sim/src/movement/take_cover.rs`) keeps a soldier half a metre clear of every body, not only his cover.

## Pointers

- Court geometry: laid by `crates/mapgen/src/parcels/courts.rs`; `CourtPlan` and `CourtKind` (yard, parking, path, lane) are in `crates/mapgen/src/lib.rs`; lawn is the district ground under no paving. Placement: `crates/mapgen/src/street_props/courts.rs` (`try_groups`), gardens in `street_props/mod.rs`.
- Catalog rows: `fixtures/props/city/gardens.json`, `fixtures/props/city/courts.json`. Art: `packages/scene-assets/blender/courts.py`, sources `assets/source/courts/`.
- Paving look: the biome's `roads.paving` row with per-family finishes (`fixtures/biomes/summer.json`, `regionalBiome` in `packages/battle-renderer/src/terrain/biome.ts`); one slab grid per area (`areaBearings`, `terrain/surfaceField.ts`), joints filtered in `frame/terrainMaterial.ts`.
- Tests: `crates/mapgen/tests/street_props.rs` (`courts_are_structured_and_lawns_dressed_rather_than_left_open`, `yards_are_bounded_with_gates_and_car_parks_keep_their_aisles`, `lawns_are_crossed_by_paths_kept_clear`, `gardens_keep_to_the_back_of_their_own_lots`, `courts_and_gardens_keep_off_the_woods_that_certify_sight`, `a_city_with_its_courts_and_gardens_stays_within_the_part_limit`), `crates/mapgen/tests/parcels.rs` (`each_regional_family_asked_for_builds_every_map_type`, `a_region_the_presets_do_not_list_is_refused`), `crates/sim/tests/move_admission.rs`, the `t1-cover-by-parked-cars` movement scenario, `web/tests/sceneAssets/regionalAppearances.test.ts`.

## Dead ends

- **One paved plaza per district.** Paving each district ring as a single court, then filling it with groups, with wall-backed groups, and with car bays and trees, never read as lived in: a whole block floor is more open ground than any prop density fills ([05b with real art](assets/05b-art/)). Yards, car parks and lawns replaced it.
- **A tank-wide ring round every group and a tank's way beside every wall.** Safe for placement, but it emptied the courts and left centre districts unwalled. The ring is now a squad's width, with vehicle lanes and gates only where a vehicle is owed a way in.
- **Walling every yard.** Sealed lawns and pushed failed group moves from 38 to about 50 of 81 probe maps.
- **A "12 m from anything" fill metric.** Courts filled to 3.6–4.1 % by it still read empty; the shipped metrics measure open paving beyond 6 m and lawn beyond 8 m.
- **New paved kinds per material** (concrete, blacktop, gravel, setts) or one new `court` kind: both needed the terrain shader widened past four kinds.
- **Cover posts filtered by navigation-grid standability.** The stuck posts were standable; the slots between parked cars were too tight to walk into. The standoff rule fixed it and the grid check added nothing.
- **Gardens and courts inside the street-furniture pass.** They took the ground the sight certificate's copses needed and some maps were refused.
- **Cheaper short-leg replans** would cut most remaining failed group moves (route certification exhausting each mover's replan share), but change every battle's routes; they are outside this feature.

## Known findings outside this feature

Parked cars and some street trees and lamps stand on the renderer's drawn walk (the verge pass keeps bodies off the carriageway, not off the walk); shaded fences and walls read dark under the scene's light.

Presets `layout-presets-18` packed the courts tighter (a 4 m group ring, a 10 m grid, amenities weighted over trees): open centre and apartment lawn more than 5 m from any body fell from 22–25 % to 6–8 % on Market Town. More placement attempts (`street_props.attempts`) were tried and dropped: the knob is shared with the street furniture, which then lengthened some street drives past the sweep's 30 m detour. An unprimed critique of that town still found:
- From 160 m up, court pieces (gym bars, benches, ping-pong tables) are too small and thin to read, so lawns read as green with dots.
- A tree's planters stand under its crown, half hidden, the same way in every group.
- Some street trees stand in the parking lane between parked cars.
- Fence and railing runs read as separate panels with gaps.
- Some trunks are drawn pale cream and untapered, so they read as posts.
- A few parked cars poke out past a building's corner.
- The objective's dashed hold ring reads as stray white lines over the town from 400 m.

## Visual provenance

All captures are in-game shots from the ground rig's stations (`web/scenes/_groundStations.mjs` `openStations`/`shoot`, fog, HUD and effects off, models drawn), one Mixed Small map per region, file names `<region>-<framing>`. Folder names are the build steps that produced them.

- [before/](assets/before/): the user's complaint as it stood: every region's block interiors a bare lawn. The standard every later shot was held to.
- [03/](assets/03/), [05/](assets/05/), [05b/](assets/05b/), [05b-art/](assets/05b-art/): the plaza attempts (a whole district paved, then dressed with stand-ins and with real art). Judged still empty, they drove the switch to yards, car parks and lawns.
- [04/](assets/04/): paving once the map named its region; settled each region's finish and slab size (setts and gravel too fine at game distance).
- [05c/](assets/05c/), [05d/](assets/05d/): yards, car parks and lawns, then lawns crossed by paths and lanes with walls wherever a squad passes. 05c's sparse lawns and unwalled centre districts drove 05d's squad-width rings and vehicle lanes.
- [06/](assets/06/), [08a/](assets/08a/): garden dressing as stand-ins, then real art; distant fence shimmer and laundry sparkle found here led to boards as texture and cords drawn only up close.
- [08a-courts/](assets/08a-courts/), [08b/](assets/08b/), [08c/](assets/08c/), [08d/](assets/08d/), [05e/](assets/05e/): shared, China, New York and Paris court pieces, and the yard walls and railings; far railings reading as a dotted veil were accepted.
- [closeout/](assets/closeout/): the village balance against main (48 of 50 digests identical, same captures) and the benchmark frame cost before and after.
