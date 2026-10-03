# How battle maps are generated

This report explains the business rules, their numbers, the measurements and the failure checks. It is intentionally more detailed than the code README: the user requested an inventory that can inform a later map workbench. The implementation and fixture files remain authoritative; this is a dated account, not another configuration source.

**Working report, 2026-10-02.** Integrated production is `main` at `73ff29dd`, generator `layout-12`, presets `layout-presets-11`. Continuous countryside coverage and explicit rendered surroundings are still being integrated. Sections below label those pending rules; neither their old version labels nor a successful narrow check establish final release admission. This report and its [complete parameter appendix](generation-parameters.md) must be refreshed when the final generation identity is frozen.

The current user direction is to finish the specific generation rules, get a playable game, then tune the numbers personally. The typical open-ground visibility floor is **50%**. Perfect parameter tuning, a live map workbench, hills/ridges and the 20 km architecture are outside current completion work. Riverbeds and banks remain part of this generator.

## What the player chooses and what is fixed

The player chooses one type and one size. A seed is drawn silently and included in the battle address; revisiting the same address on the same build reproduces its preparation. The seed is an unsigned 64-bit value serialized as decimal text, so JavaScript cannot round it. Type and size are part of each random stream's key: seed 1 on Open Small is not a resized copy of seed 1 on Metro Large.

| Size | Playable rectangle | Area |
| --- | --- | ---: |
| Small | 6,000 × 6,000 m | 36 km² |
| Medium | 8,000 × 8,000 m | 64 km² |
| Large | 10,000 × 10,000 m | 100 km² |

These dimensions are a user decision in the [shared generation contract](../../crates/contract/src/generation.rs). Buildings, streets and weapons keep their own real scale when the map grows.

| Type | Main settlement | Extra settlement mix | Buildings admitted | Initial woodland target | River chance |
| --- | --- | --- | --- | --- | ---: |
| Open | Small town | Villages and hamlets | All six categories except highrises; at most 6 floors | 4–22% | 50% |
| Mixed | Large town | Towns, villages and hamlets | All categories except highrises; at most 8 floors | 4–17% | 50% |
| Metro | City | Towns, villages and hamlets | All six categories | 2–8% | 40% |

The urban ceiling depends on type and size: Open 7% throughout; Mixed 26/20/16% for Small/Medium/Large; Metro 30% throughout. “Urban area” measures district ground polygons, not the sum of roof footprints. Initial forest targets describe the broad layout pass; later copses/tree lines furnish the remaining country. They should not be mistaken for a final count of every tree or final total woodland after furnishing.

Extra settlements are inclusive random count ranges, in addition to the main settlement:

| Type | Small | Medium | Large |
| --- | --- | --- | --- |
| Open | 2–3 villages; 5–6 hamlets | 5–7 villages; 9–11 hamlets | 9–11 villages; 15–17 hamlets |
| Mixed | 1–2 towns; 2–3 villages; 3–4 hamlets | 2–3 towns; 4–6 villages; 6–7 hamlets | 3–4 towns; 8–10 villages; 10–12 hamlets |
| Metro | 0–1 towns; 1–2 villages; 1–2 hamlets | 2–3 towns; 3–4 villages; 2–4 hamlets | 4–5 towns; 5–7 villages; 5–6 hamlets |

## The complete pipeline

The production entry point is [mapgen generation](../../crates/mapgen/src/lib.rs), shared by native tooling and Wasm preparation. Its order matters:

1. Validate the request, preset data, physical template catalogue and resolved game rules. **Pending coverage cutover:** preflight the proof resolution/placement forecast before expensive generation or unsafe allocations.
2. Draw a broad road skeleton and site the main settlement, possible river and other settlements, reserving long approaches.
3. Complete roads and physical bridge crossings.
4. Grow town blocks from those roads; select district uses and parks.
5. Grow broad forests around the built ground and reserved approaches.
6. Measure the finished layout and reject it if its actual geometry fails.
7. Lay local streets, cut parcels and place complete building templates.
8. Furnish the countryside with rural buildings, trees, copses, broken tree lines and loose physical cover.
9. Place street furniture after countryside approach corridors are settled.
10. **Pending coverage cutover:** temporarily lower the completed plan into shared physical map geometry, then certify it using actual trees and bodies; add legal forest features where an entire location cell has no certificate; reject any cell that cannot be covered. Final compilation below publishes the returned plan, including those additions.
11. Compile the plan to the common physical map and provenance record.
12. Resolve the assault recipe on that map using simulation queries, then start the battle and renderer.

A plan includes authoring facts such as districts, lots and measured approaches. The compiled map includes the physical geometry the simulation actually runs. Plan-only facts are passed to the encounter planner as sites; the renderer does not reinterpret the plan or invent gameplay geometry.

## Determinism and identity

Each independent consumer has a named random stream. The stream combines the lossless seed, type, size and consumer name. Stream names are hashed with FNV-1a and the seed is mixed into the shared integer RNG. Adding a draw for one consumer need not move the other consumers' placements. Uniform fractional draws are in `[0,1)`; integer count ranges include both endpoints. A weighted row is a relative weight, not automatically an absolute percentage.

Coordinates written by the layout are rounded to centimetres. Geometry uses the shared ground/curve shapes and `libm` where cross-target arithmetic needs a common implementation. Curved centrelines are densified with shared samples at no more than **2 m** spacing; measured lengths and surfaces use those rounded curves, rather than unrelated straight-line approximations.

A generation request pins the generator version, preset revision, seed, physical template catalogue hash, type and size. It also supplies execution limits. The compiled identity additionally pins canonical configuration/plan content and the relevant resolved physical rules. Forest settings, eye/target heights and the resolved unit/prop catalogue come from the same battle rules the simulation uses. There is no independent physical-settings file for generation.

Canonical identity ignores input JSON whitespace and object-key ordering, but meaningful authored sequence ordering remains significant because it affects implicit IDs. Execution allowance is admission policy, not map-content identity. Appearance identity is recorded separately. A version label is a caller-supplied pin, not cryptographic proof of a Git revision. Source receipts and Native/Wasm tests are the evidence that the named build actually produced the same content.

## Siting settlements and reserving approaches

The main settlement sits near the main road junction, rather than at an arbitrary free point. The junction may move **450 m in X and 80 m in Y** from the exact centre. Keeping its Y displacement smaller lets both top/bottom sides use it without mirrored geography.

Other settlement sites may stand on a road, near the main settlement or beside a river, with type-specific probabilities. Site candidates keep **80 m** from map edges. Settlement-to-settlement separation is **500/350/250 m** for Open/Mixed/Metro. A clustered candidate can be within **1,200 m** of its relevant cluster. The parameter appendix lists the full siting choices; they are proposal rules, not proof that a finished site is legal.

The main settlement must have a clear approach in **each half** of the map, at least **1,800 m deep and 400 m wide**. During siting, construction reserves a broader **650 m** front with **120 m** margin to leave room for later geometry. **16 bearing candidates** supply possible directions. Finished approaches are measured again; a reserved proposal is not sufficient.

An approach is useful open ground beside a settlement. It is not a promise that the road running through it is unobstructed in every future battle, or that every directional unit has a tree inside its sight lobe. Furnishing places obstruction beside the main firing lane, with **25 m** approach clearance. The coverage cutover may relinquish lesser approaches if necessary, but retains each actual surviving corridor's physical clearance and the mandatory main approaches. The ratio of retained minor corridors is diagnostic, not a target to optimize.

## Road network and transit checks

Country roads are **8 m** wide; dirt tracks are **4 m** wide. Layout roads bend through authored points roughly **300 m** apart, with swing up to **5%** of a run's length and a **150 m** cap. These are countryside proposal parameters; streets inside settlements are governed by frontage and junction rules.

Each map connects the bottom and top edges through usable roads to the central network. A cross-map east/west road is optional: **50%** chance. When that cross-road is absent, one side road has **60%** chance. The middle exit window spans **30%** of the top/bottom edges; the side exit window spans **70%**. An edge road can join a prior road within **1,500 m** by road of the central junction, with **60%** proposal chance when timing allows it. Separate edge-road junctions keep **700 m** apart. These probabilities produce variation; connectivity is still a hard finished-geometry check.

Roads pass straight through settlement ground and turn outside it. Main streets reach through **90%** of their site's intended axis; gate padding is **30 m**. Proposed turns are limited to **50°**. A site within the through-road reach can be incorporated into the run; other sites get connected links, bounded at **3,500 m**, with up to **4 corridors** considered. The exact road fields and geometric housekeeping tolerances are in the appendix.

The transit model computes the quickest path on the actual rounded road graph. Country-road speed is **110 km/h = 30.56 m/s**; dirt-track time includes its **0.75** speed factor. Add **15 seconds** for planning, turns and slower stretches, then require top-edge-to-centre and bottom-edge-to-centre estimates to be no more than **215 seconds**. The central network junction must be within **500 m** of the true centre. A top-to-bottom road journey must exist. An east-to-west journey is reported when present, not required on every map.

The straight road budget is `(215 − 15) × 110 / 3.6 ≈ 6,111 m`; tracks reduce that useful distance. This is a generation estimate, not proof of actual vehicle arrival. Executed movement must include footprint clearance, access to the road, acceleration, steering, traffic and planning. Infantry retains foot movement. An accepted long-order road preference is an existing movement rule, not another speed multiplier granted by generation.

## Town shape, districts and parks

A settlement is grown from its roads. Those roads divide the site into pieces, then into blocks; growth selects blocks outward from connected frontage. The outline is the edge of the resulting blocks, including enclosed open ground. This avoids round towns cut into sectors about one point.

Growth variation can lengthen/shorten the effective centre distance by **60%**, across patches **3 blocks** wide, with at least **3 patches**. Blocks split at a random **40–60%** share and can skew up to **8°**. A corner must leave at least **50°**. A block needs road frontage; a district with no viable frontage for its allowed template refuses rather than disappearing.

Avenues are **10 m** wide. Aligned opposite cuts within **30 m** can form a crossroads; junction clearance is **100 m** where applicable. Cities have **2–4** secondary roads; large towns have **1–2**, with their own attachment windows, **60–120°** turn ranges and **300/250 m** spacing. These roads create town arms and fields between them instead of scaling one central blob.

Town classes have separate proposal area, aspect, built share, block size, ribbon bias and neighbourhood size. Hamlet/village/small-town/town/large-town site areas are respectively **5–11/18–46/60–110/95–210/420–620 hectares**. A city site's proposal area is **16–24%** of playable map area. Final built districts occupy a class-specific fraction; the city built-share range is **66–80%** of its proposed site. Neither proposal area nor built share is the same as a type's final urban-area ceiling.

Parks are chosen near the centre from blocks that have built neighbours on every side, within the **8 nearest candidate blocks**, with a **0.3 ha** minimum. Small towns can have **0–1** park, towns **1**, large towns **1–2**, and cities **2–3**. Town/large-town/city outer blocks have a **6%** open-block chance. An open block inside a town is legitimate empty ground; it still must satisfy the global interesting-surroundings requirement.

District kinds are farm, village, garden suburb, small centre, centre, apartments, core and industrial. A class's zone table determines which districts occur by proximity to its centre and road frontage. District tables then set building-category mixture, street grid, lot setbacks and furniture. Weighted category mixes describe allocated ground, not an exact ratio of building counts: a large farmstead and a small house consume different amounts of space.

## Streets, parcels and buildings

The parcel pass chooses one regional building family for the entire map. The current selected family list is **`china`**. Buildings come from validated physical templates and are placed whole with translation and rotation. A parcel fits the template; the runtime does not stretch a building to fill the parcel or run Blender.

Local paved streets are generally **7 m** wide, with **2 m** verges; dirt-track streets use the track width. Global stepping parameters are **45 m** for street candidates and **4 m** for lot fitting. A dead-end extension can run on up to **60 m**, with a **35 m** tail minimum and **60 m** junction clearance in the parcel rules. District-specific depths, lengths, bending and omitted cross-street chances then change the street character.

For example, farm streets use **150 m depth × 220 m length** blocks and **30%** cross-street skip; garden suburbs use **84 × 140 m** and **15%**; apartments use **130 × 230 m** and **20%**. These are district layout parameters rather than template dimensions.

Front/side/rear setbacks depend on the district. Garden suburbs use **6/6/10 m**; small centres use **2/1.5/6 m**; industry uses **30/12/10 m**, plus a **30 m** paved apron. Aprons are paved parcel polygons, not automatically carriageways. A prop on an industrial apron is not a road obstruction unless it overlaps a real carriageway or required access.

A category with no legal family/template/floor fit cannot quietly be substituted by an unrelated category. Street surfaces are closed at their joints before buildings are fitted. Streets may end at their last served lot, but must connect to the network. Every district must receive a legal parcel; final roads/streets must not cross water off a deck. Dense IDs and building ownership are checked during compilation.

## Rivers, banks and crossings

A possible river is drawn from top to bottom with primary wavelength **900–1,800 m**, amplitude **10–18%** of wavelength, an overtone gain of **0.15** and frequency ratio **1.6–2**. Authored points are **16 m** apart; a **8°** point-turn control limits width/depth variation. Nominal water width is **16–38 m**, with **15%** width swing.

Bank grade is **0.25**, freeboard **1.2 m**. These derive a minimum dry-bank clearance of `freeboard / grade = 4.8 m` before grid interpolation margins. A river remains **500 m** from side edges, **40 m** from settlements, **10 m** from forests, **22 m** from nearby road proposals and **150 m** from the main junctions under the relevant placement tests.

A bridge uses the catalogued `bridge_deck`, **14 m** width, **9 m** landings, **0.1 m** deck elevation, **0.8 m** thickness and **12 m** road approach. Proposed spans are at most **90 m**, with at most **45°** skew. Reusing an existing crossing is considered within **600 m** for country roads and **1,200 m** for tracks; detour worth is governed by **800 m**. These are proposal/admission settings, not permission to drive through water beside a bridge.

The common river validator checks water width against terrain-cell resolution and whether the sampled terrain can carry the grade. A crossing must have dry, reachable ends and a continuous physical deck. Finished network measurements omit unbridged water runs; any such generated run is a refusal. Native world tests separately verify land-to-ramp-to-deck traversability. Hills/ridges are deferred, so current generated height variation is riverbed/bank shaping, not a general elevation generator.

## Broad forests and their physical trees

Broad woods use large-radius proposals of **450–950 m**, small-radius proposals of **110–340 m**, minimum **60 m**, aspect **0.4–1**, outline noise **0.35** and **36 outline points**. They keep **25 m** from settlements and **40 m** from other broad woods. The broad pass has at most **80 woods**, with a **0.5 percentage-point** forest-share tolerance. Infill has **50%** chance and cover settings **0.5–1.6**. Forest area is measured from authored polygon ground, distinct from visual crown coverage or runtime body count.

The physical rule is shared with the simulation: a **9 m** trunk lattice with jitter up to **30%** of spacing, trunk radius **0.35 m**, trunk height **10 m**, canopy radius **6.5 m**, canopy height **12 m**, and **2 m** clearance from roads/water/bodies. A candidate trunk must lie inside its forest and on physical ground after exactly those exclusions. The shared seed/jitter/rejection sequence ensures the coverage certifier and battle stand the same trees.

Foliage attenuation is **0.011 per metre**, scaled by the body’s concealment. Forest concealment uses the rule’s infantry/vehicle values **0.25/0.7**. The current forest-floor log and boulder rates are **zero per hectare**. Visual dressing is an appearance concern and cannot create extra simulation cover.

## Furnishing the countryside

The countryside includes occasional homes/groups, short broken tree lines, copses, lone trees and low physical objects. Densities are proposal minima/rates, not the final total: mandatory coverage can add more where otherwise bare ground remains.

| Feature | Current proposal numbers | Important constraints |
| --- | --- | --- |
| Rural home groups | 0.5 groups per road/track kilometre; 260 m apart; 35% chance of a 60–150 m lane | One farmstead weight 45; 2–4-home group weight 55; group category mix and template fit still apply |
| Homes' yards | 8 m front, 7 m side, 8 m rear; 3 m door clearance | 14 m group spread; 70% tree chance with 12–16 m tree plot; 70% loose-body chance |
| Tree lines | 0.1/km²; 60–130 m long; 12 m wide | Broken into 45–90 m stretches with 12 m gaps; 60% roadside chance, 8 m road gap, 300 m alignment reach |
| Copses | 0.1/km²; 300–1,200 m² | Aspect 0.5–1; 12-point outlines |
| Lone tree/clump plots | 1.2/km²; 9 m plot | Physical forest placement decides actual accepted trunks |
| Low cover clusters | 1.2/km²; 7 m spread; 1.2 m gap | Weighted boulders, stacked logs, wrecked cars and pallets; their physical catalogue controls effects |

Country features keep **12 m** from map edges, **60 m** from settlements, **40 m** from forests, **3 m** from roads, **12 m** from water, **6 m** from yards and **25 m** from kept approaches, subject to each rule's geometry. Attempt counts are **40** for rural groups/tree lines/copses and **30** for lone trees/field cover. A missing optional decorative proposal can be omitted; a missing required sight certificate cannot.

Low cover does not automatically count as a tall sight obstruction. A 0.7 m wreck or small log may help infantry in combat without shortening a vehicle eye’s circular ground view. Coverage must earn its result from actual height, foliage and body queries rather than feature names.

## Global interrupted sight: pending physical certificate

The shipped proximity method could mark a cell as covered from a nearby building anchor or wood outline without proving every intervening point. A real 450 m jeep counterexample exposed that gap even when a 600 m rifle sample saw a tree. The pending cutover replaces that admission rule; an old `reach_m=480` knob is removed, not retained as a second rule.

Construction uses **100 m location cells**, clipped at the map edges, and the actual **8 m native fog cells**. A valid witness contains a fully occupied **3 × 3 fog-cell patch** plus physical height/interior margins. Accepted body/trunk geometry constructs the patch. A rasterized tree outline with no accepted trunks earns nothing.

For every circular ground-observer type, the proof derives native ray count, angular step, far sampled radius, foliage shortening and clearance to an isolated far fog cell. The shortest current circular ground observer is the **450 m jeep**; infantry uses **600 m**. The shared whole-cell witness reach is derived from all these inputs (about **416 m** in current rules), not a tunable substitute for the true observer range.

Every corner of a clipped location rectangle must lie within the derived reach. The predicate also bounds possible normalized directions over the entire rectangle and requires the reserved far target to remain inside the map for every observer. This is why a feature just beyond an edge cannot “prove” an otherwise clear in-map circle. A conservative nearby index supplies possible witnesses; the same whole-rectangle predicate decides lookup, proposed placement and final acceptance.

A cell without a witness tries legal feature centres in stable near-first order, at spacing `min(cell/2, fog_cell)`—currently **8 m**—and tries copse then tree line. Geometry is staged transactionally: rejected trials roll back the forest additions, exclusions, tallies and raster effects. Coverage tries preserving all approaches first, then the main approaches if needed. It refuses the map if no physical feature certifies a cell. It does not pick another seed or silently label the cell covered.

The proof has a **512,000,000 charged structural-operation** envelope. That is an explicit preparation bound, not CPU instructions or milliseconds. It counts raster/query work; pathological resolution/trunk spacing/tree-line width is forecast before allocating enormous lists. A tree-line forecast also bounds the worst-case quadratic thinning work. Exhaustion is a named refusal, not evidence of absence of a route or a smaller-quality map.

This certificate proves a physical interruption under its supported generated height/rule assumptions. It does not prove attractive composition, optimal feature density, most directions open, every future edited forest rule supported, or uninterrupted sight blocked for every heading of a narrow directional lobe. Native visibility regressions and pictures provide separate checks. General relief is refused by this proof until its assumptions are expanded in a later terrain implementation.

## What the visibility percentage means

The [shared sight-report calculation](../../crates/mapgen/examples/common/sight.rs) measures a standard infantry observer on a **300 m square grid**, beginning halfway into the first cell. Eligible samples are traversable open ground outside settlements, off forest ground and without a nearby body. Woods, towns and water are not counted as ordinary open-field positions.

At a sample, an eye at ground plus **1.6 m** tests a target at ground plus **1 m**, to the infantry's **600 m** range. Directions are evenly spaced; ray count is `max(64, ceil(2π × maximum_range / fog_cell_m))`, giving **472** for 600 m and 8 m. At a map edge, the endpoint stops just inside the edge. A direction is open only when the simulation’s `sight_clear` passes and `foliage_depth` is exactly zero.

The percentage at that point is `open_directions / tested_directions`. Sort those percentages, then take the middle result: **the reported typical value is a median, not an arithmetic average**. The user-selected gate is median ≥ **0.50**. It does not guarantee ≥50% at every position or mean that half the image/map area is visible. A lightly tree-interrupted direction fails the full-range metric even if almost all of its length remains visible.

The report also counts samples with a completely unbroken circle, and reports the fraction with less than half their directions fully open. Starting-column positions are checked separately, and actual rendered fog combines every published eye. Consequently:

| Evidence | What it can establish | What it cannot establish |
| --- | --- | --- |
| Sampled infantry percentages | Typical openness and useful counterexamples | Continuous/all-observer coverage |
| Whole-cell physical certificate | Supported circular ground observers have some interrupted sight at every admitted location | Pleasing composition or a chosen openness distribution |
| Starting-column queries | The planner’s actual unit positions on that map | Other positions or the combined screen-space fog boundary |
| Production fog screenshot | The actual published union at the frozen pose/state | All locations, all seeds or hidden simulation state |

The prior `min(80%, bare_median − 7 percentage points)` target is superseded. The separate `enclosed_fraction ≤ bare_fraction + 6 percentage points` tuning gate is removed under the user's playability-first direction; its measurement remains visible. Current focused pending-candidate medians are approximately **59.75% Open Small seed 1, 54.24% Mixed Small seed 1, 60.17% Metro Small seed 1 and 57.84% the recorded Mixed Small seed 55012999855851041**. Open Small seed 1’s below-half-open fraction is about **21.86%**, versus **3.83%** bare. These are narrow working-candidate measurements, not release-wide acceptance, and must stay attached to their exact geometry/source receipt.

## Fairness: geometry sanity, not tactical equality

The map splits at its Y midpoint. Layout fairness compares built district area, broad forest polygon area and river length between halves:

`abs(top − bottom) ≤ max(relative_tolerance × (top + bottom), absolute_tolerance × whole)`

For town/forest, `whole` is the entire playable area. For river length it is map depth. Town tolerances are **0.10 relative / 0.004 whole area**, forest **0.20 / 0.005**, river **0.20 / 0.05 map depth**. These allowances tolerate small absolute differences on sparse maps while detecting obviously one-sided geography.

Country features have a separate per-kind test:

`abs(top − bottom) ≤ max(0.25 × (top + bottom), per_kind_absolute_allowance)`

Absolute allowances are **3 homes, 250 m tree line, 2 copses, 3 lone-tree plots and 6 low-cover bodies**. Placement favors the lesser half; the final measurement still checks the actual result. The physical-coverage cutover uses that same placement owner for a bounded fairness repair, without inventing a tactical balancing system.

Comparable counts do not imply mirrored geography, equal building quality, equal route cost, equally safe deployment or equal combat outcomes. Fairness work is limited to cheap obvious corrections. The assault recipe's separate **15 s jeep-route deployment difference** is an encounter rule, not proof of tactical equality and not this map-area formula.

## Street furniture: one physical legality rule

The town furniture pass uses catalogue bodies and preset sizes: parked cars, lamps, street trees, shelters, benches, bins, planters, bollards, hydrants, utility boxes, scooters and industrial/construction stock. The appendix includes every body’s half extents and clearance; half extents must be doubled to obtain full size.

Every candidate must remain on the map, off actual carriageways and the widest vehicle's lane, clear of building faces and door-to-street paths, apart from accepted bodies, off water/bridge approaches and out of retained firing lanes. General clearances are **0.9 m lane margin, 0.3 m kerb gap, 0.9 m wall gap, 3 m door clearance and 8 m corner clearance**. A placement can slide at most **6 m**, in **0.25 m** steps; yards have **6 attempts**.

Cars park in runs of **2–5**, with **0.5 m** bumper gaps and **3 m** gaps between runs. Two-sided parking requires a **10 m** carriageway. Lamps/tree rows are placed before car runs, so cars stop at their space. Street trees are allowed on relevant avenues at **24–26 m** spacing; a trunk body is physical, while crown rendering remains an appearance fit.

Construction sites have at most **3 per settlement**, need at least **14 × 14 m** parcel ground, leave a **4 m** gate and **1 m** fence inset, and hold a cabin, fence, one skip and **2–5** pallet stacks. Yard stock stays on its owning parcel and out of a door's way. Roadblocks and wreck barriers are defender preparation concerns, not random town furniture.

Planning alone cannot prove widest-hull passage. Existing kerbside/traffic/door tests and executed movement separately validate the interaction between placed bodies and navigation/collision.

## Compilation, limits and refusals

The compiler admits only geometry with a shared physical owner. It validates finite coordinates, legal shapes, bounds, building template references and placements, dense authored IDs, river/bridge soundness and complexity before publishing the map. Unsupported requested fields are refused rather than discarded.

Typical current request limits are **60,000 authored/materialized parts, 600,000 facade-bay positions and 200,000 ground points**. Ground points include polygon vertices and rounded curve samples; runtime forest trees are not simply another authored building-part count. These limits do not alone bound all Wasm memory, fine grids, resident renderer data or startup time. The coverage proof has its separate structural envelope.

| Failure | Meaning |
| --- | --- |
| Invalid request | Malformed input or mismatched version/revision pins |
| Invalid catalogue/missing template | Supplied geometry identity or selected descriptor is unusable |
| Invalid physical rules | Generation cannot use the battle's actual forest/observer settings safely |
| Invalid presets | A range/probability/geometry row is unsound or names an absent kind |
| Generation failed | Bounded siting/placement/certification cannot satisfy a named rule for this seed/type/size |
| Invalid bounds/placement/IDs | Compiled physical geometry violates its shared contract |
| Invalid river | Height sampling, water or bridge landings cannot carry the proposed crossing |
| Complexity limit | The request’s explicit output allowance would be exceeded |
| Unsupported plan field | A requested feature has no admitted common physical representation |

The same seed stays the same request. Searches can retry candidate sites within that seed, but never replace a refused map with another seed. Retry ceilings are **60 main-centre attempts, 200 site attempts, 60 forest attempts, 150 river attempts; repairs 4 settlements/5 woods/4 blocks; 6 infill rounds and 3 fit rounds**. The parcel pass also has a bounded retry with avenue trimming disabled when trimming removes required frontage. A legal optional furniture omission is distinct from failure of required buildings, road reach or global coverage.

## Encounter preparation and actual play

Generation produces geography; the [assault recipe](../../fixtures/encounters.json) decides who fights there. The planner uses the simulation's world/navigation/placement queries. It chooses the main settlement with a required open approach, a **150 m** objective radius and **30 s** hold objective, with **900 s** maximum assessment.

Blue currently deploys a jeep, recon, two tanks, three rifle squads, AT and supply in a column. Red deploys three garrison rifle squads, return-fire-only AT overwatch, a jeep and a tank. Edge inset is **150 m**, column spacing **30 m**, search step **50 m**, maximum advance **1,500 m**, with jeep pace and **15 s** route-difference allowance. Garrison reach/apart/door standoff are **150/60/4 m**; overwatch standoff/apart/sight are **5/25/1,000 m**. Full recipe numbers are in the appendix.

A prepared encounter proves legal placement and planned reach. It does not prove attacking infantry has walked kilometres to the fight, the widest tank has actually traversed the street, or a battle can sustain an actual **1,800 m** engagement. Those are executed movement/combat checks in C50/C51/C54. Geometry changes invalidate the relevant saved-map access and arrival evidence.

## Rendered surroundings: pending common extent seam

Playable/physical bounds stay fixed. Existing backdrop terrain and visual tree/plot placement already supply environmental drawing outside map geometry; the pending seam will describe player-map rendering explicitly instead of inferring gameplay bounds from mesh vertices.

The initial candidate player margin is **1,500 m**, reusing the existing backdrop-tree reach rather than seeking an optimal number. It is visual-only: no new deployment space, navigation obstacles, cover or forest sensing. A single map margin derives explicit playable, physical and rendered rectangles. Existing developer arenas retain their existing display environment without gaining new map surroundings. Flat display background is distinguished from physical terrain.

This seam must be judged in a real production view and included in loading/resource/frame accounting. It uses existing terrain/scenery owners; it does not require another terrain/tree generator, a physical outer ring or 20 km allocations. Its exact schema/selected margin remains pending until the pass is integrated.

## Which checks run when

Generation-time hard checks measure its output: connected settlements/streets, dry/bridged road crossings, urban/forest layout bounds, approximate top/bottom geography, mandatory main approaches, transit estimates, physical placement and compiler limits. The coverage cutover adds continuous whole-cell admission and explicit exhaustion refusal. Finite test samples are counterexample searches and never substitute for that admission proof.

The [mapgen tests](../../crates/mapgen/tests) exercise deterministic generation, invalid edited input, cross-target identity, river crossings, parcels/frontage, street joints, physical cover, retained approaches and sight. Report examples own the parameter/geometry sweeps and simulation-backed sight/battle measurements. Final C54 requires **nine type × size cells, at least ten seeds each**, with a gallery and retained refusals; one success in each cell is not the whole gate.

The browser separately verifies responsive preparation/cancellation, current Native/Wasm publication/replay identity, readable frozen pictures, correct public fog, camera movement, reset/resource lifetime and usable actual contact/combat. The current release criteria include **under 30 seconds from Deploy to playable** (M31) and **≥30 FPS**, with the final complete-world startup/benchmark/tour using the same identified geometry. Perfect performance tuning is deferred; loading failure, crash, unusable frame rate and stalled gameplay remain defects.

Each pass gets the narrow check it can affect. Run full `check` and `verify` once at whole-spec closeout. Balanced map counts and passing unit tests do not prove a pleasant or playable battlefield; final play review remains a separate requirement.

## Reading and later tuning

The [complete parameter appendix](generation-parameters.md) contains every current preset row, the generation-relevant forest/eye settings, the complete assault recipe and the source-level numeric constants with their owning comments. Ranges, units, probabilities, category weights, placement attempts, geometry safeguards and diagnostics are deliberately visible.

The later map workbench can group these by business rule and show a live regenerated map plus measurements. It should consume the existing preset/physical rule owners rather than create a second generator. That editor is a later spec: no UI, persistence model, hot-reload mechanism or parameter migration is introduced here. For now the report differentiates fixed user requirements, tunable proposal data, physical truth, numerical geometry tolerances and diagnostic measurements so future controls do not accidentally weaken the game’s contracts.
