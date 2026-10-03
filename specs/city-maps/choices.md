# City-maps implementation choices

This is the consolidated decision ledger for the final city-maps implementation, audited against frozen root `437f631d` / equivalent main `3ccb1a36`. The explicit `partial_cmp` lint refactor adds no decision. Results, failed targets and release admission live in their owning Outcomes/evidence rather than this ledger.

The user selected 6/8/10 km playable sizes, the 50% median openness floor, main 1,800 × 400 m approaches, unchanged weapon ranges, bottom-three-floor fighting and accepted collapse/gutting rules. Sharp per-side knowledge, one truthful uncertain report per enemy with a common white outline/info label and three-second fade, China-first art, delayed planning, long-move road preference, business rules before numerical perfection, and deferral of hills/ridges, 20 km architecture and the later live map workbench are inherited direction. They are not agent choices below. The separate infantry-weapon-use implementation is a dependency, not part of this ledger.

The completed [scale choices](../done/city-maps-scale/choices.md) retain their own banked decisions: engine fingerprint, final exact-content/packed publication and reader lifetime, sampled infantry corridors, fog mutation/candidate ownership, proven omitted work and portable numerical families. Those final contracts supersede the parent's earlier transport and math prototypes; they are not relisted here.

A **template** is a reusable building design; its **parts** are the solid boxes the simulation places. A **kit** is a shared asset of building pieces, called **modules**; **rows** place pieces on a template. A **tier** is a detail level, from 0 finest to 3 coarsest. A **set** is one source script's kit and templates. A **catalogue** lists the physical templates a map may use. Opaque surfaces block the camera view, cutouts show authored holes, and blended surfaces provide transparency; these art modes do not decide physical collision or sensing.

Read the low-confidence checkpoint-preview and sampled-lane tradeoffs first, then the medium-confidence assault character, replay-slot migration, physical-bay source alignment and appearance approximations. Every entry states the gap, chosen rule, future reach and verdict without requiring the diff.

## Needs-user — recommended reversible calls

### Low confidence

#### Preview long journeys at physical checkpoints rather than simulate every empty metre

**When:** final movement-admission integration. **Confidence:** low.

**Choice and scenario:** The player orders a truck several kilometres away. Before displaying an accepted destination, a private rehearsal copies that side's known situation, physically drives the truck away from its start, carries it farther along a planned empty stretch, and physically drives its final approach. It also drives intermediate encounters with stationary vehicles and props the hull would touch. The actual truck never receives those jumps: it travels the complete route in the real battle. Simulating the entire journey in preview would supply stronger evidence about every steering turn, at substantially greater command cost.

**Gap and reach:** The original contract required demonstrated travel but did not select this evidence/cost tradeoff. Accepted long-route previews now establish route geometry plus driven checkpoints, rather than an independent reproduction of every empty stretch. The current rehearsal distance is rules data. A later change to steering or generated bottlenecks must still judge actual arrival; a marker is not a universal journey guarantee.

**Verdict:** needs-user for the evidence tradeoff. Recommended provisional call: retain the checkpoint policy while preserving actual-arrival obligations. It is reversible by restoring complete rehearsal if an accepted route exposes a skipped physical failure. Known skipped shoves were corrected, rather than accepted as part of the policy.

### Medium confidence

#### The default encounter is an assault on occupied ground, with reserves arriving from the other edge

**When:** C59, retained in the final assault recipe. **Choice:** Imagine blue driving from the bottom edge toward a town. Red's rifle squads already hold buildings near the objective, its AT team watches the road blue is actually approaching on, and its jeep and tank start on the top edge with ordinary scripted orders toward the town. Further overwatch rows can watch the clear field approaches. The alternative is a meeting engagement in which every red unit also starts at the edge and marches to its post; today's defender policy would not reliably organize that attack.

**Gap:** Top/bottom deployment and real initial garrisons were both required, without choosing how much defending force begins forward or which approach the first overwatch post watches. **Reach:** The recipe defines the player's tactical problem. Road-first overwatch does not promise protection of every 1,800 m field approach, and choosing the longest visible firing lane does not choose the best covered AT position.

**Verdict:** needs-user for the encounter's tactical character. Provisionally retain the occupied-town assault because it gives blue a defended objective with the existing opponent. Reverse it through recipe posts/preferences and an opponent that can conduct a meeting engagement; generation need not change. **Confidence:** medium. [Owner](../../crates/sim/src/encounter/mod.rs), [recipe](../../fixtures/encounters.json).

#### Passing another vehicle does not authorize pushing parked cover

**When:** kerbside movement integration. **Confidence:** medium.

**Choice and scenario:** A tank finds a friendly truck stopped ahead. Its traffic detour searches for an empty passage or another route; it does not shove parked cars out of the way. The same tank's ordinary journey can still push a lighter car when the physical rule permits. Giving traffic detours the ordinary shove capability could clear congestion sooner, but would rearrange roadside cover just because a colleague stopped there.

**Gap and reach:** The placement plan did not choose whether temporary traffic avoidance should change street cover. This constrains yielding behavior, not the tank's physical pushing strength.

**Verdict:** needs-user for the gameplay tradeoff. Provisionally retain the rule because a passing maneuver should preserve the surrounding fighting positions. Reversing it means restoring ordinary pushing permission to traffic replans and judging the resulting traffic/body behavior.

## Sound

### Low confidence

#### Try a small deterministic set of clear road lines before an expensive detour

**When:** kerbside integration. **Confidence:** low for exact sample/reach values; medium for the policy.

**Choice and scenario:** A vehicle's nominal road lane meets parked cars, but another line through the same road is clear. It tries its lane and six evenly spaced lateral alternatives. It first tries to merge gently over 32 m, then over 16 m and 8 m, holds the selected line through the row, and eventually returns. If these lines fail, the existing grid search looks for a detour. A continuous interval search could find a narrow usable line between samples, but would add another algorithm before an actual missed usable passage warrants it.

**Gap and reach:** Street placement did not prescribe a lane-search method. The sample count and merge distances are initial policy values, not a proof that every possible gap is discovered or numbers chosen by the user.

**Verdict:** sound within the demonstrated traversal scope: bounded simple candidates plus the existing fallback. Reconsider the sampling owner if a concrete legal gap is missed; deferred numeric tuning does not require perfecting these values now.

#### Current ground transitions reuse the carried-surface blend

**When:** C31. **Gap:** The town's asphalt must end where its countryside road continues, without a perfectly ruled material cut. **Choice:** Asphalt runs a short distance under gravel, and the gravel starts through drifting patches. The same blend also carries a track onto a road. A road ending across another road would ideally have its own transition treatment; today this general blend can read as a smudge. **Reach:** This constrains the ground material only, not road geometry or speed. **Verdict:** Sound as the retained initial mechanism, low confidence in its appearance — the unresolved visual read is disclosed and further aesthetic tuning is deferred; it is not recorded as a solved transition.

### Medium confidence

#### Source openings follow the existing authoritative physical fighting bays

**When:** C00/C40 source alignment. **Choice and scenario:** A soldier takes a legal bay in the first three fighting-floor bands. Its eye and muzzle must have a credible opening at that physical position, rather than a blank wall or glass beginning above its head. Keep the descriptor's existing uniform bay lattice and source dimensions authoritative. Tower panels, barn windows and industrial strip windows use their existing opening producers to cover those positions. Real doors and open fronts count at their actual dimensions. When an existing pane overlaps the required band, align that pane instead of stacking another. Decorative repair cladding is pierced around the actual pane, preserving the surrounding patch rather than offsetting competing surfaces.

**Gap and reach:** Some source recipes did not align their drawn openings with positions the existing physical template already admitted. The small shared exporter guard checks eye and muzzle heights below each part's roof and refuses overlapping opening rectangles; it adds no serialized eligibility field, new bay grid or garrison rule. Barn eaves can rise within the unchanged ridge/solid box to make upper openings fit. Selected scripted sets share this guard, graph apartments retain their graph-to-bay owner, and unresolved authored village boxes gain no invented grid.

**Verdict:** sound; the existing physical catalogue and garrison policy govern source appearance, and one producer owns opening geometry. Necessary facade/roof changes affect source appearance while the physical descriptors remain unchanged. **Confidence:** medium for their appearance consequences.

#### A fair deployment means comparable jeep drive times, within a cheap bounded adjustment

**When:** C59. **Choice:** If blue's quickest drive to the capture zone is longer than red's, move blue's column forward along its own road in fixed steps until the recipe's allowed difference is met. Time the recipe's pace vehicle, currently a jeep, on the simulation's actual route. Stop at the maximum advance and refuse if equality is still unattainable. Do not move towns, mirror terrain, or run a tactical balance search.

**Gap:** The plan required fair starts without choosing a measure. **Reach:** The 15 s allowance, 50 m steps and 1,500 m maximum advance are agent-selected recipe numbers. Equal pace-vehicle time does not imply equal infantry travel, tank access, defensive quality or battle outcome. Other missions can choose a different pace vehicle and allowance through the same recipe.

**Verdict:** sound as the cheap geography/access sanity check the current direction permits. It is measured on the physical map rather than a line on a drawing. **Confidence:** medium for the proxy and numbers. [Owner](../../crates/sim/src/encounter/mod.rs), [all recipe parameters](generation-parameters.md#assault-encounter-recipe).

#### Current proposal numbers remain in their owners, grouped by the business rule they affect

**When:** C52/C53/C46/open-country passes and final parameter freeze. **Choice:** Changing a town's count range, a street block's depth, a copse's proposed size or the parking cadence changes how the same seed proposes geometry. Those proposals must still satisfy physical and finished-layout checks. The agent chose substantial defaults in six groups: settlement/type distribution; roads/transit and town growth; district/category mixes and parcel/frontage rules; river/bridge and broad-forest proposals; rural furnishing; street furniture and its search bounds. It also selected numeric geometry housekeeping constants in their existing source modules. The complete current 699 individual preset values and 76 source constants are recorded once in the linked parameter inventory.

**Gap:** The design named the desired business rules but left the initial ranges, weights, spacing, attempt counts and tolerances unspecified. **Reach:** These choices determine density, variety, refusal rate and cost, so later tuning must change the existing owners and identities. A proposal number is not permission to weaken physical truth, continuous sight coverage or the main approach requirement.

**Verdict:** sound as explicit initial defaults under the user's current tuning policy. Their aesthetic or balance optimum is not claimed. **Confidence:** medium. Read the [business-rule report](generation-report.md) and the corresponding [parameter groups](generation-parameters.md#presets-terrain), [towns](generation-parameters.md#presets-towns), [districts](generation-parameters.md#presets-districts), [classes/types](generation-parameters.md#presets-classes), [rural furnishing](generation-parameters.md#presets-open_country), [furniture](generation-parameters.md#presets-street_props) and [source constants](generation-parameters.md#source-level-numeric-geometry-constants).

#### The road transit model allows 215 seconds to retain network variety

**When:** C52 after the 183 s prototype. **Choice:** A generated Large road can bend or join another road outside the exact centre if the quickest journey along its rounded centreline, at the proposal road speeds plus the fixed delay allowance, is at most 215 s from each required edge to the central network. The earlier 183 s setting forced nearly every Large map into one centre crossroads. The final model trades some estimated travel time for varied roads.

**Gap:** “About three minutes” was a tuning target, without an exact maximum or a policy for varied Large roads. **Reach:** This is a generation estimate. An actual jeep still pays planning, road entry/exit, steering, collision and traffic, and infantry still walks. It cannot be reported as observed order-to-arrival or a guarantee from every point on every edge.

**Verdict:** sound under the accepted approximate transit direction; retain the explicit difference between the model and executed travel. **Confidence:** medium for 215 s and the 15 s allowance. [Model](generation-report.md#road-network-and-transit-checks), [current direction](scale-direction.md#vehicle-transit-target).

#### Town roads constrain the blocks, accepting a regular layout to avoid malformed streets

**When:** final C52/C53 town passes. **Choice:** When a town has two roads through it, build rows of districts from their actual frontages, choose each grid's heading from its boundary directions, and carry street lines toward legal junctions. Secondary roads run alongside or near square to the established structure; streets that cannot land legally end at their last served block. Near-centre parks are blocks the town grows around and then leaves open. The alternative would preserve more fan-shaped road variety while needing another subdivision method for every wedge.

**Gap:** The reference look required grown towns and sensible junctions without selecting a street/block algorithm. **Reach:** This removes many shallow forks, five-arm places, hooks and comb-like repeated bends, while retaining regular rectangles, some dead ends and hard district seams. Road connectivity is a hard check; a pleasing town silhouette is a separate visual judgment.

**Verdict:** sound as the current simple common model, with the loss of irregular variety stated. Numeric alignments, setbacks and bend rules remain the documented defaults rather than new acceptance claims. **Confidence:** medium. [Layout owner](../../crates/mapgen/README.md#layout-generation-layout), [street/parcel rules](generation-report.md#streets-parcels-and-buildings).

#### One category draw tries that category's legal templates before leaving a yard

**When:** C53. **Choice:** On a farm-zoned street, draw the category for the next lot once. Try its allowed templates from the selected regional family and floor range against the available frontage and setbacks. If none fits that location, leave it open rather than repeatedly drawing until a small house wins. A whole district with no legal parcel is refused. A terrace is one template with declared internal joins; separate placed buildings do not silently share walls.

**Gap:** Category weights did not say whether failed fit attempts redraw the category or how missing parcels affect the required district. **Reach:** Building sizes influence the realized mix, so a category's weight describes intended ground allocation, not an exact building-count percentage. The already-settled China-first release family restricts the allowed template pool; another shipping family needs admitted category coverage through the existing art/catalogue owner.

**Verdict:** sound; failed large-template fits do not systematically replace a district's intended use with easier small buildings. **Confidence:** medium for how well the final catalogue realizes the mix; high for whole-template physical ownership. [Owner](../../crates/mapgen/src/parcels), [report](generation-report.md#streets-parcels-and-buildings).

#### A possible river is one north-to-south constraint, placed before the surrounding settlements

**When:** C52 river integration. **Choice:** When the seed selects a river, draw one continuous north-to-south course after the main settlement and before the other towns and woods. Keep later settlement ground beside it. A road follows its bank or crosses once on a real bridge with dry landings; finished road measurement excludes unbridged water. The generator does not scatter separate streams or allow a town's later street grid to invent crossings. Bridge distribution itself is not forced to mirror across halves.

**Gap:** Rivers were required without selecting their count, orientation, generation order or network relationship. **Reach:** This places a comparable river length in the two deployment halves while permitting different local crossing opportunities. A river remains impassable water according to the shared physical owner; its course cannot be accepted merely because the plan picture draws a connecting road. More tributaries or other flow directions are future generation policies through the same contract.

**Verdict:** sound as a simple varied obstacle compatible with current road/fairness rules. **Confidence:** medium for north-to-south-only variety. [Owner](../../crates/mapgen/src/layout), [report](generation-report.md#rivers-banks-and-crossings).

#### Street furniture uses one legality check, with placement sizes owned by the map presets

**When:** C46 and kerbside integration. **Choice:** A parked car, lamp, industrial pallet or construction fence passes the same checks against real carriageways and vehicle lanes, doors, building faces, existing bodies, water, bridge access and retained firing corridors. Candidate slides/searches are bounded and stable. Preset rows supply the placed body's box and clearance; catalogue rows supply how that body behaves. Lamps/trees reserve space before car runs, and yard stock remains on its parcel. Random roadblocks and defender wreck barriers are not ordinary town decoration.

**Gap:** The plan named furniture without choosing its placed dimensions, ordering, lane margin or bounded placement policy. **Reach:** The final 0.9 m lane margin is a measured initial default, not a guarantee from a drawn kerb. A car's art has its own reference dimensions and is fitted to the physical placed box. Planning legality still needs executed widest-hull/door/traffic checks; the generator cannot promise all turning maneuvers from a grid-clearance test alone.

**Verdict:** sound; a new furniture kind is data through an existing general physical check. **Confidence:** medium for placement defaults, high for one legality owner. [Owner](../../crates/mapgen/src/street_props.rs), [all parameters](generation-parameters.md#presets-street_props).

#### Small rural features use count or length fairness, while broad geography uses area or river length

**When:** C52 and open-country integration. **Choice:** If one half has more copse plots or metres of tree line, favor placing the next feature in the lesser half, then compare the finished totals against a relative allowance and a small absolute allowance. For broad districts and woods compare area; for the river compare length. Do not mirror placement or score tactical outcomes.

**Gap:** The old common fairness formula used playable area, which would regard several extra tiny copses or homes as effectively zero. **Reach:** Sparse maps tolerate a few absolute differences without an impossible zero-difference requirement. Counts do not weigh each home's seats, each copse's protection, road cost or deployment safety. The integrated physical-coverage repair uses these same owners rather than creating a second balance system.

**Verdict:** sound as the bounded sanity policy, with final measured refusal rather than a claim of tactical equality. **Confidence:** medium for tolerances, high for meaningful units. [Formulas and defaults](generation-report.md#fairness-geometry-sanity-not-tactical-equality).

#### Sampled openness stays a distribution diagnostic alongside the continuous interruption requirement

**When:** open-country report and final user tuning amendment. **Choice:** For a standard infantry observer on eligible open-ground samples, test directions out to the true range or just inside the map edge. A direction is open only if the simulation's solid sight query passes and its foliage depth is zero. Sort each sample's open-direction percentage and report the median. Also show fully clear circles, below-half-open views and the planner's actual starts. Keep the inherited enclosure-rise tuning limit removed.

**Gap:** The selected 50% median floor did not specify the diagnostic's sampling calculation or whether every distribution statistic becomes a hard gate. **Reach:** A median above the floor permits some locally enclosed views. It does not mean half the map or half every screenshot is visible, and finite samples do not prove every playable position. Combined production fog still comes from all published eyes.

**Verdict:** sound; it keeps useful-open-ground information visible without reinstating numeric tuning the user deferred. **Confidence:** medium for the sample design, high for its stated scope. [Exact calculation](generation-report.md#what-the-visibility-percentage-means).

#### Lesser approach corridors may give way to mandatory physical sight coverage

**When:** open-country furnishing and continuous-coverage integration. **Choice:** First try to add a copse or tree line while keeping all retained settlement approaches clear. If an uncovered location cannot be certified, try again keeping the main settlement's required corridors. A successfully obstructed minor corridor is then removed from the approach list; the main 1,800 × 400 m corridors remain mandatory. If those required rules still cannot coexist, refuse the map.

**Gap:** The plan required interesting surroundings everywhere and clear approaches without choosing priority among optional minor-town approaches. **Reach:** The planner sees the approaches that actually survived furnishing, not the wider initial fans. An approach's disappearance is not silently hidden from encounter placement. No current weapon range is stretched to equal the clear corridor's depth.

**Verdict:** sound because the main approaches retain the user-selected contract and lesser opportunities are expendable. **Confidence:** medium for this priority policy. [Owner](../../crates/mapgen/src/open_country), [report](generation-report.md#siting-settlements-and-reserving-approaches).

#### Coverage uses a conservative sufficient certificate and can refuse a physically usable map

**When:** continuous actual-observer cutover. **Choice:** Split the playable area into clipped 100 m location rectangles. For each rectangle, require one real obstruction patch that proves some native ground-fog interruption for every supported circular ground observer at every point in that rectangle. A dense 3 × 3 fog-cell patch plus height/interior margins supplies the witness; distance, possible directions and an isolated in-map far target must remain valid across the whole rectangle. If no such witness can be constructed, refuse even if a more expensive exact search might discover that the map is acceptable.

**Gap:** A universal location rule cannot be established by centre samples, nearby building anchors, forest outlines or the rifle's range alone; the shorter-range jeep exposed that failure. The plan did not select a proof method. **Reach:** The method may add more copses than aesthetically necessary and conservatively refuse seeds. It supports the admitted flat generated land and local river shaping, not future relief or every heading of a narrow directional sight lobe. Those extensions require new proof assumptions, not a larger proximity knob.

**Verdict:** sound because a sufficient proof plus honest refusal preserves the requirement. **Confidence:** medium for construction density and refusal rate, high for rejecting unsupported admission. [Proof owner](../../crates/mapgen/src/open_country/coverage.rs), [scope](generation-report.md#global-interrupted-sight-continuous-physical-certificate).

#### Coverage has its own charged work bound, including raster and candidate growth

**When:** continuous-coverage integration. **Choice:** Before allocating the fine fog raster or sampling an edited forest, forecast the required cell, trunk and tree-line work. Charge structural geometry queries and raster reads to the same 512,000,000-operation envelope. Include tree-line thinning's worst-case quadratic work. Stop with an explicit refusal when the envelope cannot contain the requested proof rather than building enormous temporary lists or searching indefinitely.

**Gap:** The compiler's authored-part and point limits do not bound native fog resolution, generated trunks, spatial-index fanout or rejected placement trials. **Reach:** Edited spacing and width values can be valid positive numbers yet unsupported by this preparation bound. Charged operations are deterministic accounting units, not retired CPU instructions, seconds or a whole-tab memory guarantee. The chosen envelope must be revised through resource evidence if future supported inputs change.

**Verdict:** sound for finite progress and preallocation safety. **Confidence:** medium for the envelope's calibration; high for keeping its distinct scope. [Owner](../../crates/mapgen/src/open_country/coverage.rs).

#### Surrounding landscape is one visual margin on the map, with a separate physical rectangle

**When:** bounded surroundings integration. **Choice:** Selecting a 6 km battlefield produces 6 km of playable ground and a wider visual rectangle by expanding the common playable/rounded-height-field union with `render_margin_m`. The initial player margin is 1,500 m. Existing terrain backdrop, plots and scenery use that rectangle; authored developer arenas default to zero. A tank cannot deploy into this extra scenery, receive cover from it or navigate around its purely decorative trees.

**Gap:** The user required a larger landscape without selecting the metadata shape or a physical outer ring. **Reach:** Physical extents remain the height grid's actual rounded query domain rather than pretending every authored nominal size aligns. A margin change changes map/replay input identity and can change decorative fields inside the playable area. The generic horizon backdrop remains a separate display environment and still belongs in resource accounting.

**Verdict:** sound; one map-derived extent contract avoids a second terrain/grid owner. The initial margin and finite detailed-field rectangle are retained defaults, not an ideal landscape claim. **Confidence:** medium for the 1,500 m look; high for physical/visual separation. [Contract](../../crates/contract/src/map.rs), [report](generation-report.md#rendered-surroundings-common-extent-contract).

#### Saved encounters and generated recipes share one request field with explicit different meaning

**When:** C55/C58. **Choice:** For a generated source, `recipe_id=assault` asks the simulation planner to place the assault using its encounter seed. For `catalogue:market-town`, the same name fetches that folder's saved `encounters/assault.json`; the encounter seed is unused. It does not fetch the folder's offline `sites.json` and replan.

**Gap:** The preparation request had one encounter name for both acquisition paths. **Reach:** Editing a saved encounter changes what a newly started saved-map battle does, while a captured replay remains independent. Planning arbitrary recipes onto saved catalogue maps would require an explicit source contract that admits their sites, rather than giving the currently unused seed an accidental role.

**Verdict:** sound for the explicit current two-source contract, accepting the unused seed's limited meaning. **Confidence:** medium. [Request](../../crates/contract/src/preparation.rs), [source](../../web/src/maps/source.ts), [preparation](../../web/src/battle/prepare/prepare.ts).

#### Offline settlement sites stay beside saved maps without becoming authoritative map geometry

**When:** C59/C58 compact-map publication. **Choice:** Save `sites.json` with the generated folder so a tool can reproduce encounter planning without rerunning generation. These sites describe settlement centres, outlines, districts and surviving approaches; the compiled physical map itself carries none of them. The normal saved-map route runs its saved encounter and does not read sites. Runtime generation passes sites from that same generation outcome to the planner.

**Gap:** The planner needs town/objective facts, while the compiled map has no land-region representation and the simulation cannot consume a generator plan. **Reach:** Sites are validated for shape, but storing them beside a map is not a cryptographic proof that they describe that map. Current runtime generation obtains the pair together; offline replanning must retain their common receipts. Introducing shared map regions later should replace this authoring-only handoff if those regions provide all required facts.

**Verdict:** sound for the current limited use; do not advertise complete map-region ownership or arbitrary catalogue replanning. **Confidence:** medium. [Outcome and sites](../../crates/mapgen/src/lib.rs), [planner](../../crates/sim/src/encounter/mod.rs).

#### The saved-map resolver accepts both existing physical-library document shapes

**When:** C09/C58. **Choice:** A saved source can name the existing canonical `{ hash, templates }` library or a descriptor-list library. The resolver sends both through the same template catalogue owner, reconstructs the selected catalogue and checks its pinned hash. It does not independently infer physical facts from art. The alternative would migrate all library producers and authored pins merely to make their JSON wrappers identical.

**Gap:** Existing physical libraries had two legitimate wrappers, while the pass did not own both producers. **Reach:** This is a compatibility choice at one resolver boundary, not a second template interpretation. Consolidating producers later can remove the dual-wrapper reader through a named cutover; it must preserve catalogue selection identities or explicitly replace them.

**Verdict:** sound for the current existing producers, with a small format compatibility cost. **Confidence:** medium for retaining both wrappers long term. [Owner](../../crates/contract/src/maps.rs).

#### Replay persistence keeps one new browser slot and drops the old request-only cache

**When:** C55 captured-replay closeout. **Choice:** The new last-replay slot stores the complete captured prepared battle in IndexedDB, the browser's asynchronous transactional database. It replaces the old localStorage slot and has no migration or compatibility reader for browser-only request-form records. A player who kept a downloaded supported file can import it; the old request-only file itself is not a captured prepared replay.

**Gap:** The spec selected exact compiled replay inputs but did not require migration of the provisional request-only cache. **Reach:** A former browser-only saved replay disappears from the new viewer's slot. The replacement still stores just one last file, so another save/import overwrites it; it is not an archive or cross-build compatibility service.

**Verdict:** sound for removing the superseded format and retaining a single storage owner, with the loss of browser-only state disclosed. **Confidence:** medium for discarding that prior slot; high for the new storage contract. [Owner](../../apps/battle-lab/src/replayFile.tsx).

#### Preserve the narrow legacy fallback for buildings with no floor facts

**When:** floor-band seating and building lifecycle. **Confidence:** medium on source fit; high on the compatibility boundary.

**Choice and scenario:** An older authored box has physical walls but no authoritative floors or window bays. It keeps its old single ground band and approximately three-metre facade spacing. A new descriptor that does supply floors but leaves one facade's bays unresolved receives no seats on that facade. Guessing upper-floor windows would invent fighting positions the physical source does not establish; removing the legacy fallback immediately would remove existing authored fighting positions.

**Gap and reach:** The new seating contract did not say how incomplete older fixtures survive. Integrity and collapse classification share the one-band legacy fallback, but this does not claim that the artwork has one floor. Complete source facts should eventually remove the bridge; the fallback must not expand into a second window-layout authority.

**Verdict:** sound for preserved legacy inputs, with source-window fit left explicitly narrower than runtime compatibility.

#### Keep reasonable building and street-body values while finishing playability

**When:** physical city rules and final durability closeout. **Confidence:** medium for starting balance values; high for ownership.

**Choice and scenario:** A two-wing building's durability comes from the union of its ground footprints, multiplied by the bottom-three-floor band count and the catalog's coefficient. Overlapping wings therefore do not earn extra health merely because the author used two boxes. The immutable source footprint supplies that area even after a shell loses height. Street props similarly use ordinary weight, cover, integrity and destruction rows: a parked car can shelter soldiers and become a lower wreck, while an open scaffold obstructs movement without becoming a bulletproof wall.

**Gap and reach:** The physical rules left initial coefficients and per-body values unspecified. Keeping the current reasonable values gives the existing mechanics editor and later map workbench stable values to tune; city closeout does not become a search for ideal numbers. A closed site cabin is ordinary exterior cover, not a second garrison representation. An occupied cabin would need the building-aggregate contract.

**Verdict:** sound: data owns physical behavior and balance values remain initial choices subject to actual play defects. This is not a claim that all durability values are perfectly balanced.

#### Prefer a finishable bridge approach over the globally shortest theoretical route

**When:** movement-gap correction. **Confidence:** medium on route optimality; high on physical intent.

**Choice and scenario:** A tank's direct line crosses a river. Even a shortest-distance order asks the road graph for a bridge, then joins and leaves along its authored approaches. It avoids cutting diagonally onto a deck where its hull and turn cannot finish. The shortest policy compares distance and the fastest policy compares travel time among those physical candidates. A more global planner might find a shorter legal approach, but it must satisfy the same hull and formation constraints.

**Gap and reach:** The plan did not select connector discovery or sacrifice physical completion to exact old route winners. This may produce longer cross-river journeys; it does not redefine ordinary shortest moves or make water traversable.

**Verdict:** sound: physical completion is the general property. The retained route-quality tradeoff is not a universal error bound.

#### Use an estimated journey comparison to decide whether roads are useful

**When:** counted long-route planning. **Confidence:** medium.

**Choice and scenario:** A fast-move order looks for a road journey consisting of an off-road entrance, road travel and an off-road exit. It compares estimated travel time with the straight cross-country line using the mover's own speeds and the ground's slowing effect. It selects the road when that estimate wins; physical refinement then decides whether the journey actually fits. Searching the entire fine grid for a provably fastest route would do much more work before the player can move.

**Gap and reach:** The user selected useful road preference but not a definition of useful. This is a sensible-route policy rather than global optimality: walls and rivers may make the straight comparison optimistic. Nearby access starts with the current radius and has separate incremental fallback discovery for each endpoint; one failed entrance must not suppress a needed exit search.

**Verdict:** sound for bounded road-oriented routing. Preserve physical fallback and named limitations rather than treating the estimate as proof that the direct route exists.

#### Freeze automatic fast-move policy when each ordered leg activates

**When:** long-route planning and rehearsal correction. **Confidence:** medium for attack-move inclusion; high for the shared policy owner.

**Choice and scenario:** A queued move becomes active far from its goal. At that moment, endpoint distance beyond the user-selected long-leg threshold writes FASTEST onto that order, including attack-move, and it stays there through replans as the remaining distance shrinks. Pursuit and a building approach are not ordered legs and do not receive this conversion. A separate hidden road-preference flag would make the displayed policy disagree with behavior.

**Gap and reach:** The user chose the long-move road rule, but not policy storage or attack-move inclusion. Preview uses the same activation-distance classification. A short goal reached through a long detour still gets complete rehearsal. Before every private long-preview jump, actual displacement from the preceding rehearsal origin must earn the configured distance; replanning, shuffling or previous jumps cannot earn it.

**Verdict:** sound for a single explicit order policy; attack-move inclusion remains the material interpretation to acknowledge.

#### Build road connectivity from centreline strokes

**When:** long-route planning. **Confidence:** medium.

**Choice and scenario:** Two road strokes touch, so their bends and touching runs become graph junctions and arcs. Terrain that cannot support a run removes that arc. A paved polygon still grants its road speed but does not magically join the graph: a plaza joining two streets needs their centreline connectivity as well. Treating every paved polygon as a graph would need a connectivity contract the source does not provide.

**Gap and reach:** Polygon-road network semantics were unspecified. Generated roads use strokes, so current production has one explicit graph owner; future polygon-only roads must add proper connectivity rather than inherit an accidental join.

**Verdict:** sound within the stated map contract.

#### Use right-side lanes and local traffic escape

**When:** long-route traffic and movement-gap corrections. **Confidence:** medium.

**Choice and scenario:** Opposing vehicles travel on the right-hand side of a road instead of meeting on its centreline. They join and leave at a slant and merge into their lane gradually. On a narrow track a wide hull may extend off the paved surface, so terrain and body checks still decide whether it can pass. A stalled vehicle considers nearby vehicles together when planning around the knot; it can try a detour rather than let numeric priority force the same rear vehicle to wait forever.

**Gap and reach:** The first road journeys met head-on and stayed there; the plan had no lane or congestion-escape rule. This supplies local traffic behavior, not a general traffic scheduler. Narrow wooded roads and larger queues can still require yielding, and the local neighborhood is a chosen policy radius.

**Verdict:** sound: it fixes the general head-on and follower-reversal moments while retaining ordinary physical constraints.

#### Keep useful clearance corners until reached or passed alongside

**When:** kerbside movement correction. **Confidence:** medium.

**Choice and scenario:** A vehicle detouring around a stopped colleague has a lateral waypoint that supplies the needed clearance. It retains that nearby corner until it reaches it or passes alongside it, rather than dropping it early to begin a smoother turn straight toward the colleague. Ordinary turning-radius steering still controls the hull. Some empty-road turns may begin later or look less smooth.

**Gap and reach:** The old smoothing shortcut could discard a necessary clearance point. Removing that unsafe shortcut avoids a second turn-clearance oracle or a traffic-only exception.

**Verdict:** sound: retain the route's physical purpose before cosmetic turn smoothness.

#### Spend deterministic planning shares and distinguish search exhaustion from disconnection

**When:** counted route planning and dense-road refinement. **Confidence:** medium for allowances and accounting weights; high for progress semantics.

**Choice and scenario:** Several units receive routes at once. The planner shares the existing per-tick work allowance in stable unit order; unfinished jobs keep their cursors for the next tick. A dense exact-body query reads a fixed small number of slots per charged work unit, rather than hiding an entire bucket scan in one step. One indivisible overrun is charged back to the next tick. A job that reaches its overall search limit reports a distinct internal search-limit failure; it has not proved that no physical route exists.

**Gap and reach:** Fair bounded work was selected, but the allowance, step weights and finite-search exit were not. The player currently sees ROUTE BLOCKED for either finite exhaustion or demonstrated no route and can retain/retry the order when relevant knowledge changes. These accounting units make replay scheduling deterministic; they are not equivalent instruction costs on every workload.

**Verdict:** sound: bounded forward progress without new allowances, hidden work or pretending a tick budget is a geometry proof. Exact weights remain cost-policy values for later measurement.

#### Give every rehearsed mover a chance when another one remains blocked

**When:** group-movement admission. **Confidence:** medium for allocation; high for isolation.

**Choice and scenario:** A selected truck repeatedly replans around a stationary hull while an unobstructed rifle squad could reach its destination. Initial long routes use the live planner's search bound; subsequent searches receive one share per mover, with a share reserved for physical movement. When the truck spends its share, it stops in rehearsal and the squad can still prove arrival. Charging every replan to one group-wide pot would let the blocked truck refuse otherwise valid squad movement.

**Gap and reach:** Bounded partial placement existed, but repeated-search accounting was unspecified. The fair share can conservatively refuse a mover that a larger allowance might prove. Accounting is temporary preview evidence, not a new persistent planner or live scheduling rule.

**Verdict:** sound: one member's failure cannot consume every other member's proof.

#### Measure late sustained contact with the unchanged ten-unit participation proxy

**When:** final full-window city benchmark measurement. **Choice and scenario:** A city benchmark can begin with a large firefight and become quiet before the final minute. The scene checks rising own-unit shot counters in both opening and final minute and requires at least ten distinct firing own units in each, borrowing the endurance participation check. Rounds still in flight or an earlier burst cannot satisfy it. The report also keeps actual shot rate, so participation and fire volume remain different facts.

**Gap and reach:** Sustained heavy contact was selected, but ten unique firing units was an agent-selected proxy. It can reject an active fight with fewer participants and high fire volume. The measurement cannot silently replace this check with round count, lower it to match a current run or change staging merely to manufacture the count. A future product interpretation must name and justify its new participation/volume contract.

**Verdict:** sound as the retained conservative measurement policy, without claiming the proxy is an ideal definition of combat quality. **Confidence:** medium.

#### Resolve and fly a declared tour that the safe camera can actually follow

**When:** city benchmark camera policy and tour-v3 correction. **Confidence:** medium for initial framing; high for truthful trajectories.

**Choice and scenario:** The city tour is resolved after preparation, using the returned map's centre and rendered extent for its combat and overview phases. Its close orbit distances were increased where the original requested path entered tall buildings. The camera therefore follows the declared path instead of being lifted away from it by its legitimate safety correction. The report stores that exact applied tour and map/workload identity.

**Gap and reach:** Generated extent and safe orbit distances were unspecified. The initial framing is a representative choice, not ideal-shot approval. A changed tour or rendered margin is a different visual workload and must not be presented as a matching performance comparison. Player clearance and drift requirements are retained.

**Verdict:** sound: the measurement describes the trajectory actually drawn.

#### Reuse a private opaque contact slot across separate evidence episodes

**When:** final contact consolidation. **Confidence:** medium.

**Choice and scenario:** An unseen rifle squad fires, creating uncertain report slot 7. That evidence expires and its picture begins fading. A fresh shot from the same cause renews slot 7, replacing the fading picture rather than drawing a second circle alongside it. Each side's existing knowledge collection retains the inactive private handle; there is no second cause-to-report map and no simulation copy of the presentation fade timer.

**Gap and reach:** The user chose one report per enemy and the shared fade, but not identity storage. Report continuity can correlate separate evidence episodes with the same uncertain cause. The public report does not disclose the enemy's actual unit ID, unseen type, exact position or hidden movement. Previously identified type may be remembered truthfully; a never-identified source stays UNKNOWN.

**Verdict:** sound: one opaque owner satisfies the requested replacement behavior with less state. Its enduring correlation is the material implication the user now owns.

#### Order legal garrison and overwatch candidates without claiming tactical optimality

**When:** C59 objective-post placement. **Choice and scenario:** A rifle squad tries buildings near the objective in an encounter-seeded order, preferring the attack-facing side. It takes the first one with a physical seat for every soldier, enough distance from previous posts, legal standing ground outside a door and a route into the capture zone. Within the separately chosen road-first overwatch policy, bounded legal positions are compared by sampled view distance. A pretty long field view elsewhere cannot override the selected approach ordering.

**Gap and reach:** Whole squads and useful defense were required without candidate tie-breaking or a positional search algorithm. The seed chooses among otherwise legal garrison candidates; it does not reroll columns or physical rules. Seat capacity, door access and longest sampled sight do not establish tactical cover quality, and a legal overwatch post can remain exposed.

**Verdict:** sound as a bounded reproducible placement algorithm through existing simulation queries. **Confidence:** medium.

#### Admit one complete bounded publication instead of fragmenting observations

**When:** observation transport integration. **Confidence:** medium for the allowance; high for atomicity.

**Choice and scenario:** Reconnecting to a side can require a large exact fog and learned-ground snapshot. The publisher checks the entire record with overflow-safe size arithmetic before allocation and either emits one complete record within the current 64 MiB allowance or reports a visible error. It cannot omit distant learned cells or send an observation whose pieces may be shown separately. Fragmenting it would require another partial-record assembly protocol.

**Gap and reach:** Exact sparse data solved the original staging problem, but the plan did not choose atomic-record size or fragmentation. The allowance is a codec guard, not a frame-cost or peak-memory target, and high-entropy workload acceptance is separate. A rejected record leaves the previous baseline and cursors intact.

**Verdict:** sound: explicit bounded refusal preserves complete side knowledge with one protocol owner. Revisit fragmentation only if a required workload cannot fit.

#### Spend sparse-directory memory to avoid repeated terrain and foliage lookup work

**When:** sight-cost correction. **Confidence:** medium on memory tradeoff; high on sample semantics.

**Choice and scenario:** A sight ray repeatedly samples a wide mostly-flat battlefield. It addresses a direct directory of sparse terrain and foliage pages, where missing terrain means exactly flat ground and missing foliage means open ground. An entirely empty height field needs no directory. Keeping a tree/hash lookup at every sample would use less directory memory but repeat that work for every ray.

**Gap and reach:** The cost slice did not prescribe lookup representation. Existing values, interpolation, forest clearing and query ownership remain authoritative. Larger extents or finer cells should measure the memory trade rather than infer that this directory is universally preferable.

**Verdict:** sound: the representation changes cost while preserving physical samples.

#### Optional forest-body density is a candidate ceiling; shared defaults stay zero

**When:** forest-body integration. **Confidence:** medium for achieved density; high for refusal/activation semantics.

**Choice and scenario:** Optional logs and boulders are drawn from independently seeded jittered candidates after ordinary trunks, with logs considered first. A candidate that cannot fit is omitted; placement does not move trunks or hunt indefinitely for an exact count. Missing optional density stays zero. The shared default remains zero even with finished models: activating bodies changes authored forest routes and must be intentional.

**Gap and reach:** The plan did not choose whether density means a guaranteed final count. Sparse bounded startup and existing tree identity take priority. Dense or narrow woods may receive fewer bodies or none. Finished art alone does not authorize changing every map's collision world. Turning bodies on needs the active maps/scripts to handle the new physical cover. This does not add a rule turning felled trees into logs; their existing cleared-ground lifecycle remains.

**Verdict:** sound: stated candidate ceilings and conservative activation preserve physical honesty.

#### Concentrate a named full-world synthetic fight while preserving rear roles

**When:** distinct city-contact workload and `city-arena-2` / benchmark-preset-v4 role correction. **Choice and scenario:** The city benchmark prepares the complete Metro Large seed-4 world and places its 100-unit-per-side synthetic fight near the centre. Its own two-sided scripted commands control it, with recording starting after the first scheduled wave. Front displacement moves only the initial fighting line inward. Reserve units retain rear starts for later waves; logistics remain behind the line where supply goals expect them. Moving every living start inward would erase the scenario's rear jobs while leaving incompatible later orders.

**Gap and reach:** Measuring active local city combat needed a distinct workload from the village and from a player assault that travels from its own edge. Native/browser consumers use the same simulation-owned factory and preparation path. The new role-respecting staging is named separately from camera tour v3; a changed scenario is a changed measurement input. Ordinary endurance uses zero displacement. This synthetic local fight does not establish player infantry transit, tactical quality or every later corpse/wreck distribution.

**Verdict:** sound; one disclosed workload owner honors its declared roster roles. The role correction retains the combat rules, roster, random draws and wave cadence. **Confidence:** medium for representativeness, high for role preservation.

#### An open-front cart shed uses the accepted solid-box approximation

**When:** C16 farmstead. **Choice and scenario:** A cart shed is drawn as an open-front roof, while the simulation uses the same roughly 9 × 6 m solid building box as its physical approximation. A ray through the visually open front can therefore be blocked by the building body, just as a pitched roof's box contains solid air beside its slope. The release accepts this coarse shelter shape rather than adding a new opening-aware collision/sight representation.

**Gap and reach:** The source wanted a roofed cart shelter, while the common building contract describes solid box parts. More precise shelters would require an intentional shared physical-shape decision and corresponding appearance, rather than a renderer-only exception. The small shed's low physical height restricts its available fighting floors.

**Verdict:** sound as the accepted box approximation, with its sight/projectile consequence disclosed. It does not require closing the art or retuning the current map. **Confidence:** medium.

#### Current source family ships with generic project-made categories

**When:** Buildings-lane source coverage. **Choice and scenario:** The already-settled release family is China. Its vendored graph supplies apartment blocks, but not every house, farm, low attached row, tower or industrial building at the game's admitted physical sizes. Project scripts supply those missing categories through the same source-set contract and tag the complete family `china`. Their plaster, brick, tile and concrete remain generic project art; that tag does not certify a uniformly Chinese regional appearance.

**Gap and reach:** The chosen graph did not cover the required category/size matrix. Runtime stretching or substituting an unrelated family would hide that gap. New York and Paris remain later families through the same exporter; another shipping family needs real category coverage and preset selection rather than an art fallback.

**Verdict:** sound within the already-settled one-family direction, with generic regional appearance disclosed. **Confidence:** medium.

#### Pitched roofs raise the solid part to the ridge

**When:** C16/C17. **Gap:** The original house boxes had no roof shape. **Choice:** A one-floor pitched house uses a part about 5.2 m tall, a two-floor house about 8.3 m and a three-floor house about 12 m; the roof ridge is the box top, rather than allowed art above an eaves-height box. A round just above the eaves can therefore hit the box even beside the sloping roof. **Reach:** These are physical catalogue changes, affecting sight and projectile stops as well as art. **Verdict:** Sound as an explicit box approximation, medium confidence — it prevents a visible roof from failing to hide a unit, while accepting the solid air beside the pitch. More accurate roof physics is a different shape contract.

#### Glass trades reflection cues for readable dark windows

**When:** C25. **Gap:** Ordinary glossy glass reflected the bright horizon and looked like boarded windows at grazing views. **Choice:** One scene shading function lights a pane, with a bounded reflection and a partly eye-facing shading normal; opacity remains its coverage at every angle. The pane darkens the room but offers little gradient or glint to prove it is glass. **Reach:** Material recipes can add dust or waviness; a stronger generic mirror would obscure interiors again. **Verdict:** Sound as a bounded current composition, medium confidence — it prevents pale plates without asserting that shopfronts now read convincingly as glass.

#### Interior pictures use one daylight exposure compromise

**When:** C15/C26. **Gap:** “Unlit at scene exposure” did not define how a rendered room picture should respond to the outdoor light. **Choice:** A room's picture is multiplied by the brightness of a white matte surface in sun shadow, then fogged; it gets no direct sun, local lamp, cast-light highlight or emission. The source sheets are toned for that treatment. A sunny facade and a shaded facade still receive the same room picture brightness. **Reach:** At a steep tactical view a window mainly shows the picture's floor; far windows can remain dark panes rather than readable furniture. **Verdict:** Sound, medium confidence — it keeps daylight rooms from glowing and follows scene exposure with one shading owner; the close/far visual compromise remains.

#### Tree crown detail is limited rather than expanded to the closest reference

**When:** C73. **Gap:** The required tree appearance and frame budget did not select branch/clump counts. **Choice:** Broadleaf trees use about 24 clumps and two branch generations; tiers 0–2 keep the same crown structure with simpler meshes, and tier 3 is its lobed volume. Spruce uses a spire-shaped coarse outline, pine a flat-pad crown, and snag its bare skeleton. The unused denser crown would expose more branches and sky. **Reach:** Species must preserve silhouette across detail tiers and fit the shared scenery budgets; no distinct shadow hull invents another crown. **Verdict:** Sound, medium confidence in the visual trade — it preserves one shape down the tiers with bounded cost, while accepting lumped crowns as an appearance limit.

#### Forest dressing receives shadows but does not cast them

**When:** C79. **Gap:** Ferns, branches and small rocks needed a draw policy. **Choice:** A fern is double-sided opaque geometry in the depth and colour passes, fogged at its own centre and shaded by trees; it is absent from every sun-shadow cascade. It can therefore look like a sticker where its own contact shadow would help. **Reach:** Every nonphysical floor piece shares this lower-cost policy; adding shadows needs a measured decision through the existing shadow owner. **Verdict:** Sound as the initial presentation trade, medium confidence — it avoids multiplying a dense decorative population across cascades without claiming its grounding is ideal.

#### Street furniture uses project models and some opaque screens

**When:** C45. **Gap:** The expected standalone export of eight vendored street items never arrived, and the scenery path needed finished assets. **Choice:** The project scripts make all street items, including lamp, bench, bollard, bins, hydrant, utility box, planter and scooter. Bus-shelter glass stays glossy opaque geometry; Heras mesh becomes progressively fewer wires and a bare frame. A unit can be hidden from the camera by a pane the simulation shoots and sees through. **Reach:** Replacing these appearances is a source/bake change through the same scenery kinds; no new physical row is required. **Verdict:** Sound as the retained initial scenery implementation, medium confidence — there is actual generic art for each body, with the opaque-screen mismatch openly unresolved. This is not an acceptance of shelter transparency.

#### Lone tree crowns have no simulated foliage volume

**When:** C45. **Gap:** Street-tree bodies are trunks outside forests, while forest foliage is the only existing canopy authority. **Choice:** A lone trunk draws a crown from the existing species, but receives no forest-style sight attenuation under that crown. The camera can hide a unit under leaves while the simulation sees it and the x-ray identifies it. **Reach:** Adding concealment along street trees requires a simulation foliage decision; enlarging the visual crown alone cannot confer it. **Verdict:** Sound as the disclosed boundary of the current trunk mechanic, medium confidence — it avoids silently inventing a new sensing rule to support decorative foliage.

#### Crop and grass variation keep the painted ground as their colour owner

**When:** C80–C85. **Gap:** Multiple grass species needed variation without a visible colour boundary when blades disappear at range. **Choice:** Each clump starts from its plot's ground colour; species add relative root/tip/head colour and dryness. Drifts choose patches of species, and individual/tussock brightness stays bounded. Dryness and grain differences fade with the clump. Crops follow the same painted rows and keep their tramlines bare. **Reach:** A dry tuft in meadow is still related to meadow colour, rather than carrying an unrelated straw palette; hay means standing dry grass and stubble the cut field. **Verdict:** Sound, medium confidence in the initial mix — close blades and distant paint share a colour authority, while exact crop alignment, patch scales, row strength and palettes remain later tuning.

#### Banks and water use material cues tied to river distance

**When:** C69–C71. **Gap:** A physically shaped bank still needed a readable surface. **Choice:** Distance from the river paints a narrow wet-silt band, then bare earth with an inward-wandering edge and increasing grass. Its lightness is lifted against the adjoining plot rather than one global grass colour. Water adds short light streaks along constant distances from the bank, with normals still supplying ripples. **Reach:** Both sides of a bend share the same rule; it does not make inside point bars, outside cut banks or flowing streak motion. **Verdict:** Sound as initial river appearance, medium confidence — all cues follow the physical channel with bounded lookups; flat bands and lane-like streaks remain visual limits.

#### Bank shading reduces the physical slope to keep the far bank visible

**When:** C69–C71. **Gap:** A true steep normal under a low sun made one bank look like an unexplained dark stripe. **Choice:** The material derives a smoothly blended bank normal from the river cross-section, scales its relief and caps the slope; the bed is shaded flat. The physical height field retains its real bed and bank. **Reach:** Shading can soften relief without changing movement or camera height, but cannot be used as proof that the bank visibly reads as a slope. **Verdict:** Sound, medium confidence in initial relief — the visual normal has one owner and avoids triangle steps without modifying the physical surface.

#### Yards are whole plots claimed by nearby buildings

**When:** C31/yard pickup. **Gap:** A saved map exports buildings and roads but no settlement outline for ground painting. **Choice:** A plot near a building becomes yard, a farther nearby plot becomes meadow, and the remainder uses the field mix. A road crossing the line from plot centre to building centre prevents a yard claim; the broader meadow remains radial. Polygon roads use their native triangle edges for this crossing test. **Reach:** A whole plot can extend across a road even when its centre's claim is valid; this is not a clipped yard polygon or frontage plan. **Verdict:** Sound for the existing plot contract, medium confidence — every map supplies the same inputs, with no new settlement schema and an explicit coarse boundary.

#### Yard-road ownership uses a static scan rather than another index

**When:** C31 pickup. **Gap:** Crossing tests needed access to road geometry. **Choice:** During plot construction, likely buildings scan nearby road records, skipping distant edges by bounds; no work repeats each frame. An additional index could speed this stage but would retain another preparation structure. **Reach:** Complete preparation must still satisfy startup requirements; a future correction should use the existing surface geometry owner. **Verdict:** Sound, medium confidence — the simpler initial implementation is proportionate until this actual preparation work is shown material; this is an initialization choice, not per-frame work.

#### Countryside roads look urban where yards flank both sides

**When:** C31. **Gap:** The physical map calls its through-road `country_road` even inside a settlement. **Choice:** The renderer splits its stretches and draws asphalt only where yard plots lie near both sides, carrying short gaps across the town. One yard beside a crop does not urbanize the road. Units keep the original road kind and speed throughout. **Reach:** This is a presentation classification, with thresholds in biome data; explicit street sections from the map could replace it later. **Verdict:** Sound for the current yard owner, medium confidence — it derives the town read from one existing ground classification without changing physical roads.

#### Asphalt, shoulders and field palettes retain current restrained values

**When:** C66–C68/C84/C85. **Gap:** The design did not select exact material palettes and fade sizes. **Choice:** Asphalt is neutral grey; gravel and tracks vary toward warmer equal-lightness colours. The shoulder changes hue and thins grass instead of painting a separate sand outline. Only dirt tracks have wheel ruts and a continuous centre strip, which disappear when too narrow to read. Fine field texture varies around its mean, while broad dry patches shift hue one way. **Reach:** Future values remain biome data; the current road/field appearance is not a reason to change physical speed or terrain. **Verdict:** Sound, medium confidence — the mechanisms preserve far means and avoid new dark patches, while exact palette, rut, shoulder, line and crop numbers are deliberately left for later tuning.

#### Ground proof thresholds distinguish paint from physical classification

**When:** C62/C70/C31. **Gap:** “Not a dark road/bank” and “read the ground class” lacked a measurement policy. **Choice:** Road and bank guards inspect rendered bands against the adjoining field, allowing either lighter ground or enough hue separation; banks are checked individually so a sunny bank cannot mask a dark far bank. The class-mask view instead records the simulation's paving, water and plot/forest class at the terrain shader's own lookup. **Reach:** The mask's distances are exact only within the drawing lookup's reach; beyond that it identifies the correct side, not an exact metric distance. Initial contrast/hue bounds are guards, not perfect aesthetic judgments. **Verdict:** Sound, medium confidence — physical classification and finished appearance answer different questions without inventing a second surface model.

#### The camera may wait at a tall building, then cut past it

**When:** C57. **Gap:** The requested camera pose can be on the other side of a tower with no nearby safe path. **Choice:** The eye lifts over roofs within its lift reach, slides where possible, and otherwise waits during recovery before cutting to the clear goal. It always looks at the requested target. A continuous wide detour around the tower would swing far away from the player's framing. **Reach:** The drawn pitch/distance may leave wheel limits while safety holds the eye; a cut is an explicit state, not relaxed collision. **Verdict:** Sound, medium confidence in the initial feel — safe framing is guaranteed without a second camera pathfinder, with the visible wall wait disclosed.

#### Camera prediction and release use current tuning data

**When:** C57. **Gap:** Avoiding a wall only after reaching it stopped the camera abruptly, and a grazing goal oscillated. **Choice:** Motions with a rate predict 0.75 s ahead for buildings, while wheel steps and drags have no prediction. Terrain is not predicted. A goal needs 0.75 m more clearance than the drawn eye, and the same margin controls release; smoothing and bounded search samples remain fixture data. **Reach:** A straight prediction can mispredict an eased swoop if lengthened; the values are initial tuning, not a generally optimal trajectory. **Verdict:** Sound, medium confidence in those values — prediction starts recovery early and release hysteresis prevents repeated safety toggles through one policy.

#### Camera diagnostics separate exact raw framing from safe geometry recovery

**When:** C57/C27. **Choice and scenario:** A diagnostic needs to frame a wall exactly while another trajectory demonstrates the playable camera safely avoiding it. The existing `__lab.setCamera` hook draws its supplied pose without normal limits or clearance; the existing `placeCamera` path applies safety and is used by play and benchmarks. In the camera fixture, a destruction switch asks the existing library/rule owner for legitimate collapsed-courtyard remains, then supplies the same side-known geometry to drawing and camera obstacles. It does not invent a low ruin for a tall tower that should remain gutted.

**Gap and reach:** Exact evidence framing and safe recovery answer different questions; the fixture also needs known destruction without running a battle. Raw pose captures cannot establish that a playable camera can occupy that pose. The camera fixture isolates recovery from known geometry, while the separate shelling lab exercises real damage/publication. The raw hook retains its established name for its existing untyped scene callers, so future authors must choose the intended path consciously.

**Verdict:** sound; distinct diagnostic contracts reuse the production geometry and safety owners rather than another renderer or a fake physical state. **Confidence:** medium for the diagnostic split and raw hook's naming.

#### Overview haze follows orbit distance beyond the original range

**When:** Generated-map lab. **Gap:** Haze tuned for a 2 km orbit washed out wider maps. **Choice:** Beyond that orbit distance, haze distances are scaled by the camera's expanded distance, and overview distance/pitch follow map extents. A distant map remains visible without changing its physical size. **Reach:** Overview atmosphere differs from a literal fixed-density atmosphere; near village framing keeps its original treatment. **Verdict:** Sound, medium confidence in the look — it extends the existing scene parameters rather than introducing a separate overview renderer.

#### Surface lookup ends its hierarchy at a record budget

**When:** C63. **Gap:** Edge-on distant pixels can cover hundreds of metres and require nearly every map primitive. **Choice:** A hierarchy of grid levels serves increasing pixel widths; it stops growing when average nearby lists exceed a level's budget. A still wider pixel uses the last level and cannot blend in distant forest floors/verges beyond that level's reach. Finest cells and total cell count are bounded too. **Reach:** The index retains local distances but deliberately limits very wide-pixel filtering on dense maps; this is not arbitrary-distance exactness. **Verdict:** Sound as a bounded presentation contract, medium confidence in its overview trade — finite work is named instead of allowing unbounded fragment searches. This deliberately bounded filtering is separate from exact local membership.

#### Per-template triangle limits are reports, not bake refusals

**When:** C13/C22. **Gap:** The template budgets began as estimates to be ratified by the drawing design. **Choice:** The bake reports 150,000/50,000/12,000/2,000 triangles for tiers 0–3 without rejecting an excess; source helpers separately enforce damage no costlier than intact, except the disclosed village exception. **Reach:** A new template cannot assume the reported number is a hard validation gate; complete workload/frame goals remain binding. **Verdict:** Sound as a clearly named advisory budget, medium confidence — rigid guesses are not substituted for actual frame admission.

#### Cutout depth alone chooses the covered multisamples

**When:** C24. **Gap:** A fine grille must thin at distance without the prepass and colour disagreeing. **Choice:** Ordinary filtered mips become a covered-sample count in the four-sample depth prepass. The colour pass shades only depth-equal samples, without making a second discard decision. Resolved edges round the count; unresolved averages dither. The caster dithers the same filtered coverage. **Reach:** Very sparse authored coverage can disappear below a sample, and some unresolved sheets can band or mottle; no coverage-preserved texture variant depends on a material cutoff. **Verdict:** Sound, high confidence in silhouette ownership and medium in distant appearance — one stage chooses the visible holes by construction.

#### Smoke starts when the viewing side learns destruction

**When:** C27. **Gap:** Soot alone did not distinguish some burnt towers at range. **Choice:** Known gutted parts create dark roof smoke then a thin tail; known ruins create pale dust then thin smoke. They start on discovery using the existing wreck-source machinery and share its budget. A late observer therefore sees smoke begin when it learns, rather than receiving hidden destruction time. **Reach:** This affects every map's destroyed building through the existing effect timer; smoke is a cue alongside the damaged silhouette. **Verdict:** Sound, high confidence in knowledge ownership and medium in initial durations — the visual cue never reveals an unseen event.

#### Forest stands and unusual species are biome data

**When:** C75/C76. **Gap:** A large forest needed coherent species groups instead of independent random trees. **Choice:** Seeded spatial cells choose broadleaf/conifer family, then weighted species, with some mixing. Birch and snag are weighted exceptions; snag's interior-distance data excludes it near edges, in strips and past the map. Forest-floor moss/humus use rotated smooth noise; their colour strengths are biome data. **Reach:** Nearby separate woods can belong to one stand; changing weights/tints/stand scales does not create new sensing rules. **Verdict:** Sound, high confidence in ownership and medium in initial values — one placement rule expresses variety without named species logic scattered through rendering.

#### Forest bodies fit their physical boxes and use the tree art helpers

**When:** C78. **Gap:** Logs/boulders needed real models and the floor dressing needed matching bark/rocks. **Choice:** One floor script imports tree bark/export helpers; logs and rocks use vertex colour, with a shared seeded rounded-rock generator. Models fit the simulation's placed boxes, including the 0.7 m log thickness, and small explicit below-ground fit tolerances seat them on slopes. **Reach:** Enlarging a log's art to look more blocking would misrepresent cover; changing its physical thickness is mechanics work. **Verdict:** Sound, high confidence in fit/ownership and medium in initial surface detail — actual bodies determine what art must occupy.

#### Decorative height and mixing are owned by scenery and biome data

**When:** C79. **Gap:** Floor plants must not appear to hide a soldier whom the simulation sees. **Choice:** Dressing has no body, a validated 0.9 m maximum height and tier triangle caps; biome scale cannot exceed one. Density, kind weights, drifts, fade size and inset are biome data. Small rocks are built below their intended limit, though a distinct rock-only height validator is absent. **Reach:** A new tall decorative bush cannot become concealment by appearance alone. **Verdict:** Sound, high confidence in the height boundary and medium in initial density — decorative population rules are explicit without entering the simulation.

#### Tree-line ground is a tapered verge, not burnt forest floor

**When:** C86. **Gap:** Brown strip floors looked like scorch marks with blunt ends. **Choice:** A strip uses the neighbouring plots' verge colour and grass, tapering ground and hedge rows over each exported end. Crops remain in the physical strip's square corner outside that painted taper. **Reach:** Grass can grow on simulation forest ground because its presence is presentation-only; a bend's first chord owns its own taper. **Verdict:** Sound, high confidence in the shared verge mechanism and medium in end treatment — one existing field-margin palette avoids a new strip-floor material.

#### Mixed stroke/polygon distances retain the existing maximum rule

**When:** C03/C63. **Gap:** A paved polygon touching a stroked road does not automatically expose a correct per-kind internal material boundary. **Choice:** Their union still uses the maximum of existing distances; loading yards draw with the sidewalk row while carriageways use stroke edges. The mechanism supports today's uses but cannot produce a feathered boundary between arbitrary adjacent coloured polygon kinds. **Reach:** Another polygon material arrangement needs an explicit boundary decision, not an assumption that this index solved union shading. **Verdict:** Sound within the retained surface contract, medium confidence — unchanged geometry ownership is disclosed with its limitation rather than labelled a solved general join.

#### Road markings are functions of strokes and fade in physical width

**When:** C30. **Gap:** Generic markings needed placement without another map list. **Choice:** Stroke distance lays whole centre dashes, suppressing a dash near a crossing; another carriageway running on across determines crossing bars, not a side-road end. Paint has a dull worn material and fades as its metre width becomes subpixel, unlike screen-sized animated tactical marks. **Reach:** Junction arms share generic crossings; polygons have no centreline markings and the current dashes can still compete with a white tactical outline. **Verdict:** Sound, high confidence in geometry ownership and medium in initial readability — one surface function determines markings without new physical data.

#### Fields use tract headings and long native road/river guides

**When:** Generated fields/C86. **Gap:** Repeated orientation jitter at recursive cuts produced a fan across a large map. **Choice:** The land keeps one heading above tract scale, turns once per tract, then uses a nearby long road/river/tree-line guide instead of treating every short bend segment as an independent field boundary. **Reach:** A straight cut across a curved guide still yields wedges; following the actual curve requires map-owned field polygons. **Verdict:** Sound, high confidence in bounded orientation and medium in remaining shapes — one tract heading prevents a random walk without inventing another terrain schema.

#### Fog candidates remain exact ray geometry and change with geometry

**When:** C20. **Gap:** Every eye could not test every building, and no completed S4 technique verdict existed. **Choice:** Existing footprint grids provide nearby boxes, then conservative angular sectors narrow each ray before the same exact intersection. Candidate lists travel inside the existing eye-rebuild table, with a separate structure/eye table for whole-building visibility. Equivalent reordered boxes do not invalidate eyes; added/removed/moved geometry does. **Reach:** Sharp horizon/whole-roof rules stay unchanged, including roofs reaching inward from just outside an eye radius. The exact ray/intersection owner remains responsible for the answer. **Verdict:** Sound, high confidence in geometry semantics — conservative candidates reduce work without replacing exact intersections.

### High confidence

#### The compiler lowers a plan once into the shared physical map and refuses unowned fields

**When:** C04 and its ground integration. **Choice:** A road/forest/river/building proposal reaches the compiler as shared contract geometry. The compiler validates and materializes the map the simulation runs; the renderer draws that map and resolves matching appearance, without looking back at district intentions or choosing physical dimensions. The native/Wasm outcome is either `{ status: "ok", result }` with the complete map, identity and report, or `{ status: "error", diagnostics }`. If an author asks for an unsupported plan field, such as a new region feature without a shared owner, it returns the error form instead of discarding the field and returning a thinner success. The CLI distinguishes admission refusal from usage/filesystem failure and writes no accepted map on refusal.

**Gap:** The plan did not select a second runtime model, parser or fallback policy. **Reach:** Generated and authored maps share lowering; the production generator imports neither simulation nor art. Verification examples can load the compiled result into the simulation without introducing a runtime dependency cycle. Current compiler support includes the admitted surfaces/forests/rivers/bridges, not a completed land-region schema.

**Verdict:** sound; the battle and renderer inherit one physical interpretation and unsupported intent stays visible. **Confidence:** high. [Owner](../../crates/mapgen/src/lib.rs).

#### Content identity, acquisition provenance and resource allowance remain different facts

**When:** C04/C09/C60 and final physical-input integration. **Choice:** Saving a generated map pins its resolved physical content, seed, generator/preset/catalogue identities and canonical configuration. Moving identical bytes to another catalogue folder does not change the map's physical identity. A source receipt records either actual repository path/revision/byte hash or supplied label/byte hash; it does not invent a Git revision for a temporary input. Execution limits are admission policy, so raising only an allowance does not claim the map changed. Art has its separate identity.

**Gap:** The shared schema did not choose which of storage addresses, source history, geometry, input rules and resource policy belong in each identity. **Reach:** Meaningful authored row order remains significant because it affects implicit IDs; whitespace/key order does not. Version labels pin expectations but do not themselves prove source revision. Full selected catalogue/forest/eye-target physical inputs are pinned even if an edit happens not to move the generated map.

**Verdict:** sound; each record claims the fact its owner can establish. **Confidence:** high. [Identity](../../crates/contract/src/identity.rs), [map acquisition](../../crates/contract/src/maps.rs), [physical generation inputs](../../crates/contract/src/generation_physics.rs).

#### Saved buildings store their template and frame, and the map hash covers the materialized result

**When:** C58 compact saved maps. **Choice:** Instead of repeating every turned wall, facade bay and entrance for each of thousands of houses, a saved building keeps its template ID, placement frame, body kind and authored owner/part IDs. The resolver materializes exactly that template, validates it and hashes the resulting full map. Category and regional family come from the template, not repeated per-building declarations. A wrong library address cannot succeed if its hash or placement does not match.

**Gap:** Compact saving did not choose what the map hash covers or which facts genuinely belong to each placement. **Reach:** Saved and resolved maps share a generic map shape rather than duplicate every ground field. A physical template edit changes the affected resolved map identity. Catalogue loading must still admit repeated parts/bays before cloning; the compact file's size alone says nothing about resident battle cost.

**Verdict:** sound; storage compression preserves the actual physical contract rather than saving a request for later regeneration. **Confidence:** high. [Saved map contract](../../crates/contract/src/maps.rs), [resolver](../../web/src/maps/resolve.ts).

#### Admission counts emitted work before materialization and stays narrower than whole-world capacity

**When:** C04/C09/C58. **Choice:** A caller states maximum authored/materialized parts, cumulative facade-bay positions and ground points. Count repeated template output before cloning geometry or allocating all bay positions, and count rounded curves as well as authored polygon vertices. The catalogue policy matches what current runtime generation may save. Reject invalid finite/positive map headers, illegal IDs, shapes, placements or river/bridge geometry even if their submitted hash matches.

**Gap:** A valid descriptor or matching content hash does not make repetition cheap or physics valid. **Reach:** The usual 60,000 parts, 600,000 bay positions and 200,000 ground points do not bound every runtime trunk, dense terrain/navigation grid, JavaScript parse, renderer upload, Wasm high-water capacity or tab footprint. Coverage has its separate envelope; complete startup/resources remain separately measured.

**Verdict:** sound; it prevents oversized materialization without presenting narrow limits as complete capacity admission. **Confidence:** high. [Compile limits](../../crates/contract/src/generation.rs), [catalogue admission](../../crates/contract/src/maps.rs), [report](generation-report.md#compilation-limits-and-refusals).

#### Decimal seeds, portable geometry and separate random streams preserve the exact request

**When:** C52/C59 and Native/Wasm boundaries. **Choice:** A full u64 map or encounter seed crosses JavaScript as canonical decimal text. Type, size and purpose join the seed in the random-stream key, so changing forest draws does not accidentally shift town placement and one seed is not a scaled copy across nine preset cells. Written positions use centimetres; bearings and state-bearing cross-target geometry use the shared portable math owners. The existing battle's numeric seed stays separately bounded by JavaScript's exact-integer limit.

**Gap:** The boundary and determinism requirements did not select stream naming, quantization or how to preserve seeds beyond Number's safe range. **Reach:** Same-build reproduction names all three inputs rather than conflating map and battle randomness. Meaningful changes receive new version/configuration identities. Numerical tolerances admit physical placement at large world coordinates; they do not authorize stretching art or weakening legal joins.

**Verdict:** sound; each target receives the same requested integer and uses the same geometry decisions. **Confidence:** high. [Seed and identity](../../crates/contract/src/identity.rs), [request](../../crates/contract/src/preparation.rs), [report](generation-report.md#determinism-and-identity).

#### Generation uses the battle's physical forest and observer rules rather than a lookalike copy

**When:** final shared physical-generation inputs. **Choice:** If a forest outline contains a nominal tree location but the actual sampler rejects its trunk beside a road, that tree contributes nothing to coverage. The certifier uses the same seed/jitter/exclusion sequence and trunk/foliage geometry as the simulation, taken from the explicitly supplied resolved battle rules. Circular observers come from that same catalogue. A rules edit therefore can change generation configuration identity even while stored map bytes happen to remain identical.

**Gap:** The original generator could consult a forest outline or a separate presumed observer range without proving what the battle stands and sees. **Reach:** Every CLI/Wasm generation entry point must receive the actual resolved rules; there is no implicit second physical-settings file. Complete selected sections are pinned conservatively, rather than trying to hash only fields a particular seed appeared to use. A future forest/observer edit must be admitted by the proof rather than merely passing a positive-number check.

**Verdict:** sound; physical placement and sensing have one owner. **Confidence:** high. [Shared inputs](../../crates/contract/src/generation_physics.rs), [forest sampler](../../crates/contract/src/forest.rs), [coverage](../../crates/mapgen/src/open_country/coverage.rs).

#### Rural single trees, copses and broken tree lines are shapes of the existing forest owner

**When:** open-country furnishing. **Choice:** To place a rural lone tree, create a small forest plot that the shared sampler admits one trunk on; a copse is a forest polygon and a broken tree line is a sequence of narrow forest strokes. They receive the same actual trunk/foliage placement, forest-ground concealment and sensing as larger woods. A decorative crown is not substituted for physical foliage. Small low-cover bodies use their existing prop catalogue effects and do not automatically earn a tall sight obstruction.

**Gap:** The catalogue could already place a trunk body, but a trunk outside forest ground did not have the forest canopy/concealment the rural sight rule needed. **Reach:** A soldier under that small forest plot receives the forest-ground concealment rule even when the plot holds only one tree. This is different from an ordinary lone street-tree prop, whose physical/crown policy belongs to its existing owner. Rendered field boundaries remain decorative and do not relocate physical tree lines.

**Verdict:** sound; all rural woody features reuse one physical rule rather than creating copse/tree-line/lone-tree sensing exceptions. **Confidence:** high. [Owner](../../crates/mapgen/src/open_country), [forest contract](../../crates/contract/src/forest.rs).

#### One joint-closing pass serves the layout roads and the later parcel streets

**When:** C52/C53 road-end integration. **Choice:** Round only actual bends through the shared curve owner, close square road ends where another carriageway should cover them, merge compatible straight-through runs, and retain a crossing when a third road joins there. Run the same `joints::close` after broad layout and after local streets. Rivers retain their own round-ended geometry. If a proposed tidy-up would lose a real joint, retain the original affected roads rather than manufacture a disconnected prettier picture.

**Gap:** Separate layout/street cleanup could leave flat heels, duplicate runs or different junction rules at the two construction stages. **Reach:** Physical surface membership, graph connectivity and finished pictures consume the same rounded strokes. Generated output receives a new identity when cleanup changes ground; previously saved authored maps are not silently rewritten by loading. Residual unusual ends remain a visual judgment, not proof the road graph is disconnected.

**Verdict:** sound; one final geometry rule spans both generation steps and prioritizes actual network continuity. **Confidence:** high. [Owner](../../crates/mapgen/src/joints.rs), [shared strokes](../../crates/contract/src/ground.rs).

#### Candidate retries stay inside the named seed, and rejected coverage trials leave no geometry behind

**When:** C52 through continuous coverage. **Choice:** If a proposed settlement, river, parcel or coverage feature fails, try a bounded number of candidate places within that request. A coverage proposal stages its forest additions, exclusions, counts and raster effects together; commit only when it earns the whole-cell witness, otherwise restore them together. If required geometry or proof cannot fit, return the named seed/type/size and diagnostic. Never swap in another seed, shrink required towns, mark an uncovered cell covered or substitute a successful gallery image.

**Gap:** The plan required bounded search without choosing the failure transaction and what a failed seed means. **Reach:** Optional furnishing proposals can be omitted when their rule allows it; required buildings, connected/dry roads, main approaches and global coverage cannot. A refused requested seed remains a visible refusal, rather than a successful replacement image or a reason to loosen admission.

**Verdict:** sound; failure preserves both deterministic identity and the promised quality. **Confidence:** high. [Generator](../../crates/mapgen/src/lib.rs), [coverage trial owner](../../crates/mapgen/src/open_country/coverage.rs).

#### The planner checks physical placement and planned reach without claiming arrival or combat

**When:** C59 and final movement integration. **Choice:** A tank start must have room for its tested footprint and separation from other bodies; a squad start needs enough reachable standing places. A garrison requires the simulation's actual usable seats and a legal door approach. Required unit types must have planned routes into the objective zone, while the chosen pace unit supplies route timing. The encounter seed orders otherwise legal garrison buildings; the planner does not invent cover, capacity or navigation rules.

**Gap:** “Legal and reachable” did not select a footprint sampling and objective-route criterion. **Reach:** A completed plan does not show a widest hull turning through a parked street, individual soldiers completing kilometres of movement, or sustained weapon use. Those contracts need executed play. Every changed relevant geometry invalidates its old access/arrival proof, even if the same route name is retained.

**Verdict:** sound for preparation's limited claim. **Confidence:** high for shared physical queries and proof separation; the footprint sample/default details remain recipe/algorithm defaults. [Legality](../../crates/sim/src/encounter/legality.rs), [planner](../../crates/sim/src/encounter/mod.rs).

#### One map resolver serves catalogue files through thin browser, Node and native adapters

**When:** C09/C60/C61. **Choice:** `market-town` is a catalogue ID, never a path. Check that grammar before IO, fetch its separate map/source documents and the physical library named by its source record, and give their text to the shared Rust resolver. Browser/Node adapters select files; they do not independently expand saved templates or validate world physics. Reuse a fetched library across maps and preserve the resolver's localized refusal. Listing metadata can be read without the large map and is checked against resolved content during catalogue validation.

**Gap:** The catalogue requirement did not select address syntax, adapter division, fetched-document layout or listing validation. **Reach:** A wrong label/size/feature summary cannot drift silently from a saved map. Maps are fetched on demand rather than embedded in scripts; generated and synthetic tool worlds need no fictitious saved folder. Generic saved-map inspection reuses the geometry lab. Village/lab factories retain their existing scenario composition and rules through this acquisition owner.

**Verdict:** sound; acquisition and listings each retain one owner. **Confidence:** high. [Adapters/resolver](../../web/src/maps), [catalogue listing](../../web/src/maps/catalogue.ts).

#### The page pins a generated request from the current documents and hands their text to the worker

**When:** C55 request/preparation boundary. **Choice and scenario:** The player selects type and size. Before starting preparation, the page asks the existing generator boundary for its version and template-catalogue hash and reads the current preset revision. It creates a pinned generation request with those identities, the lossless seed and explicit limits. Rules, presets, templates and recipes travel as document text beside that request; the worker checks the request and prepares from exactly those documents. The request does not duplicate all the resolved rules.

**Gap and reach:** The spec selected version/catalogue pins without choosing which caller fills them or whether rules are embedded in the request. The page does a bounded metadata parse before worker startup, while generation/placement stays off the page thread. Labs can keep their own resolved rules through the same document contract. The prepared replay captures the resulting complete scenario, so these initial request documents are provenance for that capture rather than a recipe to regenerate it during playback.

**Verdict:** sound; one existing generator/check boundary supplies the pins and the simulation still admits the resulting request. **Confidence:** high. [Request builder](../../web/src/maps/source.ts), [preparation protocol](../../web/src/battle/prepare/protocol.ts).

#### Preserve Rust's exact physical text when composing the scenario

**When:** C09/C55/C58. **Choice:** The JavaScript adapter may parse a map for counts and labels, but the scenario receives the resolver/generator's original map text. It also receives the planner's original encounter fields and the resolved rules. Reprinting a parsed object would lose negative zero and can alter precise physical numeric interpretation. The extraction relies on the shared Rust outcome's declared field order and delimiters and fails explicitly if that layout changes.

**Gap:** The JSON boundary did not select how to preserve authoritative physical number bytes while exposing useful metadata in JavaScript. **Reach:** The outcome's serialization layout is an actual interface: changing it requires changing the existing text extractor and its consumer checks together. Exact primitive/raw-token parsing belongs to the physical contract rather than a global scenario-parser replacement or independent JavaScript geometry decoder.

**Verdict:** sound for the current explicit text interface. **Confidence:** high. [Source extraction](../../web/src/maps/source.ts), [saved resolver](../../web/src/maps/resolve.ts), [preparation composition](../../web/src/battle/prepare/prepare.ts).

#### Preparation's worker keeps its world and becomes the battle authority; the page builds a plain query world

**When:** C33 and its simplification. **Choice:** Deploy starts one preparation worker. It resolves/generates the map, builds the simulation world, lets the planner borrow it and then hands that same retained world into the battle on the same worker channel. The page independently builds a `WorldView` from the scenario's map/rules with the simulation's code for picking, height, surfaces, learned foliage and camera clearance. It has no navigation or mutable battle state. The custom exported query index and its lossless transfer are removed.

**Gap:** Reusing the planner's world did not select worker lifetime or page-query representation. The original no-page-world design added a second query representation without measured startup benefit. **Reach:** Normal player preparation performs two world builds, worker and page, instead of claiming one for the whole browser. Public static queries remain one implementation. Side-known clearing/destruction still comes from observations; the static page world cannot reveal hidden battle changes. The developer stress factory retains its own temporary placement world and uses its actual synthetic scenario map.

**Verdict:** sound; startup margin buys a simpler query architecture while retaining the expensive planner/battle reuse. **Confidence:** high. [Preparation](../../web/src/battle/prepare/prepare.ts), [worker handoff](../../web/src/battle/prepare/worker.ts), [measured simplification](startup-lane.md#startup-measurement).

#### Navigation away or cancellation owns all pending preparation lifetime

**When:** C55/C33. **Choice:** A battle address names one request and opens its own page. Leaving or cancelling closes its worker. The client also marks it cancelled, so an answer already in transit cannot create a stale battle; the abandoned promise stays silent. Refusal and initialization errors show their existing HUD/loading exits without waiting for successful meshes or world exports. Restart creates a fresh preparation session rather than repurposing an authority that owns another battle.

**Gap:** The plan required responsive cancellation/replacement without selecting routing or the exact pending-promise behavior. **Reach:** The URL supplies same-build share identity, and returning to the menu preserves the current type/size/seed choice. There is no new client-side router, preparation cache or delayed stale-answer queue. A worker failure releases its allocations and remains a failure, not an empty success.

**Verdict:** sound; lifetime and stale-answer rules have one session owner. **Confidence:** high. [Client](../../web/src/battle/prepare/client.ts), [worker](../../web/src/battle/prepare/worker.ts).

#### Prepared replays capture the complete scenario and report instead of regenerating the request

**When:** C55 closeout at `ae1a9cac`. **Choice:** Saving a battle writes `{ battle: { scenario, report }, replay }`. The scenario contains the exact compiled map, placed encounter and resolved rules. The report carries the request, identities, counts, seeds and camera anchor already produced by preparation. Playback creates `PreparedWorld.from_scenario` in the existing worker and gives the page's `WorldView` the same captured scenario. It does not read today's presets, recipes, map documents or physical template library to reconstruct the fight.

**Gap:** The spec required captured inputs but did not select the file envelope or how to recover viewer labels and camera state. The earlier request-only departure did not meet that requirement. **Reach:** This supersedes request-only regeneration and needs no historical generator or second replay metadata schema. Appearance still comes from current compatible assets; pixel-exact historical art and cross-build replay remain outside scope. Saving/restoring the captured scenario does not depend on regenerating current fixtures.

**Verdict:** sound; a current fixture edit cannot silently change the replay's physical battle. **Confidence:** high. [Replay file](../../apps/battle-lab/src/replayFile.tsx), [restore path](../../web/src/battle/prepare/prepare.ts).

#### The last captured replay uses one transactional browser record, and imports wait for persistence

**When:** C55 IndexedDB closeout. **Choice:** A compiled largest-map replay is roughly 47–48 MB, so store the last saved/imported JSON text in one IndexedDB record rather than localStorage's much smaller quota. An import parses the envelope and waits for a completed write transaction before opening its viewer or navigating. If persistence fails, show the error and keep the current page; do not launch an older stored battle. A download can still succeed when local persistence is blocked.

**Gap:** Captured compiled inputs require much larger persistence than the provisional request form, without a specified storage API or archive. **Reach:** Adds one native browser database/store, no package dependency, archive or compression format. Transaction completion is the point at which navigation can trust the file exists. Runtime identity remains the simulation's responsibility after envelope parsing.

**Verdict:** sound; complete compiled inputs fit the intended storage path, and write failure has an explicit user-visible outcome. **Confidence:** high for transactional persistence. [Owner](../../apps/battle-lab/src/replayFile.tsx).

#### A pending asynchronous replay read is a distinct state, and cannot overwrite an import

**When:** C55 asynchronous-read closeout at `3504012c`. **Choice:** Before IndexedDB answers, the file is unread, not absent. Both viewers show the existing loading screen; the menu temporarily disables replay navigation. A confirmed empty slot may then show its ordinary missing-file state. If the player imports a file while the read is pending, the hook's import counter prevents the older read result from replacing that newer file. The village viewer constructs its scenario only after a confirmed village-format file.

**Gap:** Synchronous-storage readers previously conflated “nothing saved” with “the read has not completed.” **Reach:** No transient wrong viewer, speculative village battle, guessed progress percentage or second loading component is created. Read failure resolves through the existing absence behavior; writes/imports still report their own failures.

**Verdict:** sound; asynchronous readiness is part of the viewer boundary. **Confidence:** high. [Owner](../../apps/battle-lab/src/replayFile.tsx).

#### The later workbench will consume the current rule owners; this release creates no editor architecture

**When:** final business-rule report and tuning freeze. **Choice:** Document the grouping and semantics needed for a later map editor: proposed densities/weights, physical truth, numeric geometry safeguards, retry limits and diagnostic outputs. A future control should regenerate through the current native/Wasm generator and expose the map plus its measurements. No workbench UI, persistence, migration, hot reload or duplicate generator is implemented by this release.

**Gap:** The user wants to tune every generation business-rule number personally later but has not selected that editor's interaction or persistence design. **Reach:** The report is a dated inventory, not a second configuration source. Later controls cannot turn off a physical or mandatory quality contract merely because a displayed numeric value looks like another slider. Editing presets requires refreshed generation identities and source receipts through their current owners.

**Verdict:** sound; records enough to hand off the tuning surface without taking unrequested future architecture decisions. **Confidence:** high. [Inventory](generation-report.md#reading-and-later-tuning), [complete parameters](generation-parameters.md).

#### Keep source-building identity separate from the current live damage owner

**When:** compound buildings and destruction chains.

**Choice and scenario:** A house and attached wing share one integrity owner, so hitting either wears down the same building. When they become weaker remains, every live replacement part shares a fresh damage owner, while the immutable authored building ID still selects the original physical/art source. The exhausted old body has no live HP. Reusing its damage key would make the new remains already destroyed; treating each wing independently would create separate buildings out of one authored structure.

**Gap and reach:** The aggregate contract did not specify replacement-owner lifetime or source ancestry. Existing replacement links and retained source association provide those facts without a second geometry table. Live observations use current part extents; remembered knowledge can retain historical identities without pretending they are live. Composite motion remains unsupported.

**Verdict:** sound: original identity, present geometry and current integrity have distinct necessary roles and each has one owner. **Confidence:** high.

#### Apply one blast contribution and one terminal transition to the entire building

**When:** compound damage lifecycle.

**Choice and scenario:** An explosion reaches both a wing and the main house. The shared building takes the strongest part contribution once, rather than the sum. At its terminal transition, every current part changes together. A side seeing any part learns the replacement aggregate together; a completely hidden side retains what it previously knew until reveal. Dividing a house into extra source boxes therefore does not make the same blast stronger or allow half a building to collapse independently.

**Gap and reach:** The one-structure plan did not choose blast aggregation or partial-visibility publication. Direct hits and explosions share the integrity owner and preserve sharp side knowledge.

**Verdict:** sound: damage and learning follow the authored physical aggregate. **Confidence:** high.

#### Put scaled integrity and destruction policies in the body catalog and validate the whole chain

**When:** building lifecycle and placement-context validation.

**Choice and scenario:** A catalog body says whether HP is fixed or building-scaled, and whether its terminal remains are a lower ruin or a gutted shell. Permission to garrison is independent. When an ordinary prop or vehicle wreck is placed, the shared validator follows every declared destruction state: if a later state needs building geometry or seats, the ordinary placement is refused now. A placed aggregate's replacement inherits that geometry owner. Checking only the initial row would admit a crate whose later state suddenly needs a nonexistent building.

**Gap and reach:** The schema allowed capabilities whose required geometry was absent, including movable aggregate parts that could tear a compound apart. One context checker replaces scattered guards; aggregates and their replacement states remain immovable until motion can transform all their parts together. A new building type is data, not another kind-name branch.

**Verdict:** sound: admit only capabilities the complete physical owner can perform. **Confidence:** high.

#### One occupied physical seat plan supplies both sensing eyes and firing muzzles

**When:** floor-band seating and facade sensing.

**Choice and scenario:** A squad enters a stepped building. Seats come from exposed physical bays in the bottom three floor bands, distributed across building-frame directions and spread through each group's bays. Each directional eye uses the highest occupied living seat, with stable ties; weapons use their actual living carriers' muzzles and the existing position-change delay. Averaging two separated east-facing spans could put the eye inside a wing, and choosing a hypothetical vacant window could permit a shot no participant can fire.

**Gap and reach:** The plan did not choose allocation, eye selection or carrier ownership. A seat retains its exact physical edge. Whole-squad capacity is checked again when entry completes because supply can restore soldiers during entry. A damaged holdable shell clips source bays by its remembered height for commands, then checks live physical support on entry; an unseen collapse cannot alter the side's admission.

**Verdict:** sound: one actually occupied position supplies the physical shot and observation. **Confidence:** high.

#### Keep dead occupants attached to their floor until support disappears, using side knowledge for their picture

**When:** building corpse-support correction.

**Choice and scenario:** A soldier dies on the third fighting floor, then the last living squad member dies and releases the garrison. The corpse retains its building support owner. When the floor later disappears, it settles at its existing XY position onto the ground/deck; the terminal gutted exterior also removes fighting floors. A hidden collapse updates a side's own casualties immediately, but a remembered enemy corpse stays at its last-seen pose until sight returns.

**Gap and reach:** Keeping support only in the living squad's hold stranded earlier deaths in the air. Existing unit/knowledge owners retain this conditional pose, without new public corpse columns. Presentation refreshes dying, resting and fading anchors without restarting the death or resurrecting a body whose fade ended. Identity, facing and survival do not reroll.

**Verdict:** sound: support is a physical fact and the enemy picture remains learned evidence. **Confidence:** high.

#### Ground visibility and learned ground share one side/stream identity

**When:** fog and learned-ground transport.

**Choice and scenario:** A diagnostic switches from blue to red while a blue frame is already in flight. The client invalidates the displayed stream, returns that blue buffer as credit, and still counts its completed tick toward an outstanding advance. It exposes a red frame only with red fog and red learned ground from the same epoch, the number naming the publication stream. Independent fog/ground streams would need another coordination state machine; waiting to show every stale blue frame would delay the requested view change.

**Gap and reach:** The plan required exact side isolation but did not choose handling of completed obsolete frames. Visibility keeps its own revision within the common side/epoch identity. Transport bookkeeping does not enter battle outcomes.

**Verdict:** sound: authority continues in order while obsolete pictures do not mix side knowledge. **Confidence:** high.

#### Publication failure ends the client and settles every waiting operation

**When:** transport admission and recovery.

**Choice and scenario:** A frame's fog baseline is valid but its ground baseline is wrong. The decoder rejects the entire frame before either cursor advances. If the worker or application fails, outstanding commands, advances and replay requests all reject, and held buffers return exactly once. A later status message cannot restart that failed client; the user starts a fresh client to recover. Automatic resynchronization would need to preserve partly applied state and resolve ambiguous pending operations.

**Gap and reach:** Earlier failure handling settled startup while leaving later requests stranded. This establishes explicit terminal failure and immutable paired baselines rather than a hidden retry manager.

**Verdict:** sound: a refused observation cannot expose mixed knowledge or leave the producer waiting forever for credit. **Confidence:** high.

#### Preserve exact public IDs with paired integer limbs

**When:** aggregate/publication reader cutover.

**Choice and scenario:** A physical prop ID grows beyond float32's exact-integer range. The stream sends its low and high 16-bit limbs and readers reconstruct the exact value. Building, live-owner, replacement, source and garrison references use the same established representation; building rows name parts in the existing physical export rather than duplicate their geometry. A single rounded float could silently select the wrong building.

**Gap and reach:** Aggregate identity expanded the public seam. Every consumer changes together through the published layout; unrelated animation and physical columns keep their existing roles.

**Verdict:** sound: identity must remain exact across the simulation/browser boundary. **Confidence:** high.

#### Share immutable navigation geography and update only side-known changes

**When:** incremental navigation-grid integration.

**Choice and scenario:** Both sides begin with the same authored map and share one immutable terrain/body grid base. A side learning a moved wreck overlays its belief, removes the body's old stamp and updates only affected cells, clearance tiles and region metadata. A hidden enemy-side change cannot alter that overlay. Rebuilding a complete large grid for every learned tree or wreck would repeat startup-scale work during play.

**Gap and reach:** The first counted planner built two whole grids, then rebuilt them after knowledge changes. The existing known-body owner supplies mutations; body update and route-candidate invalidation do not introduce a second physical oracle. A route is rechecked when learned changes can intersect it, not when unrelated distant knowledge changes.

**Verdict:** sound: immutable facts are shared, beliefs remain per side, and derived indexes follow their existing revision owner. **Confidence:** high.

#### Road corridors use public terrain; local refinement uses only the side's known bodies

**When:** counted road planning and kerbside refinement.

**Choice and scenario:** A road graph proposes a corridor through a street. Local checks refine it for the mover's width using actual footprints that the side knows, including remembered moved/removed bodies. A coarse cell touching a parked car can still admit an exact clear segment; destinations and broad cell searches remain conservative. A hidden wreck is discovered physically, then triggers normal learning and replanning. If a known obstruction closes an arc, another graph journey is tried; inability to join a road falls back to ordinary cross-country planning.

**Gap and reach:** The plan did not choose this public-geography/known-body split or coarse false-blockage correction. Terrain and map edges still constrain full width. Navigation's width proof is not a certificate for every rectangular hull orientation; actual movement remains the collision and turning authority.

**Verdict:** sound: useful corridors do not grant a new ability to pass bodies or see hidden changes. **Confidence:** high.

#### Terrain can prove permanent disconnection; removable bodies cannot

**When:** movement-gap correction.

**Choice and scenario:** A truck's endpoints lie on opposite components of unbridged river terrain. Component labels can reject the crossing without searching a whole map half. A wreck blocking a bridge cannot supply that permanent conclusion: it may move, break or be unknown to one side. Equal terrain labels only permit a physical search; they do not promise a route around current bodies.

**Gap and reach:** The bounded planner needed cheap impossible-crossing proofs. Labels come from terrain before body stamping, while long probes and validation retain cursors across work steps.

**Verdict:** sound: a no-route proof follows an enduring physical barrier rather than temporary knowledge. **Confidence:** high.

#### Keep an unfinished plan deterministic and commit it against current knowledge

**When:** counted planning.

**Choice and scenario:** A squad waits for a route and takes ordinary stationary cover. Its observation says PLANNING and shows the requested goal. A newly learned obstacle does not automatically restart unrelated search work, but the finished candidate must fit the side's current picture before it commits; otherwise the job starts again. A traffic detour retains the prior usable route if no replacement is found. Search scratch storage returns to a pool on completion/cancellation rather than freeing a large map in one tick.

**Gap and reach:** Delay was authorized but pending, cancel, revalidation and retained-memory behavior needed concrete choices. All authoritative unfinished work enters replay/digest state. A blocked verdict arriving after the original order flash triggers the existing mark reveal again so the player can see the result.

**Verdict:** sound: one deterministic scheduler, current-knowledge admission and explicit lifetime. **Confidence:** high.

#### Rehearse intermediate physical shoves rather than assume pushability means passage

**When:** movement-preview correction.

**Choice and scenario:** A truck can push the first light crate, but the crate is trapped against another one and the game does not chain-shove. Route search may allow the lighter body; preview must physically drive that encounter to discover the obstruction. A conservative hull sweep and segment clip locate contact even when a long diagonal prop's centre lies far away. Props outside the swept hull do not require driving an unrelated encounter.

**Gap and reach:** Checkpoint preview initially treated props as a planning concern. The sweep chooses where to rehearse; it does not invent another shove solver or prove every wheeled turn.

**Verdict:** sound: movement remains the authority for the effect a proposed shove actually has. **Confidence:** high.

#### Omit expensive sight work only when existing predicates already settle the answer

**When:** sight/fog cost and final bounded solid-before-foliage correction.

**Choice and scenario:** A wall already blocks an observer's ray to a target, so the sensing query returns false before integrating leaves behind that wall. The same physical predicates and results remain; only their order changes. Fog can also skip a ray after proving that every cell it could add is already visible. It cannot infer that from a few sampled cells or change the eye order, cadence or shape.

**Gap and reach:** Broad optimization is deferred, but the current cost pass exposed work that cannot affect an answer. These are necessary-work corrections through existing query owners, with no new rule numbers, persistent cache or state. They do not alone establish browser frame admission.

**Verdict:** sound: omit work only after a general proof that its output cannot matter. **Confidence:** high.

#### Keep logs, boulders and tree lines inside existing physical owners

**When:** forest-body and tree-line integration.

**Choice and scenario:** A diagonal log candidate fits only if its whole bounding circle stays inside forest/map bounds and clear of roads, water, trunks and other bodies. A bounded local navigation check preserves existing connected components and open boundaries for the actual catalog movers; a failed candidate rolls back. A tree line uses the same forest stroke, trunks, canopy and foliage as a wood. It can conceal far identification while remaining see-through nearby, rather than becoming an always-opaque hedge.

**Gap and reach:** New country bodies and narrow vegetation did not need another blocker, sight or generation authority. Conservative placement can reduce optional density; it avoids introducing an enclosed pocket merely because a few sampled routes still work. Drawn crowns and shrubs must honor the same physical envelope.

**Verdict:** sound: geometry and physical properties remain the common rule. **Confidence:** high.

#### Measure every grass tier and runtime multiplier against the existing height cap

**When:** effective-height validation.

**Choice and scenario:** A grass model fits at its near detail level, but a farther tier or the product of two runtime scale maxima could exceed the field-height cap. Validation measures actual vertices at every level and combines both multipliers; the affected biome scales are reduced to fit. Measuring only the first tier or each scale independently would admit a taller effective plant.

**Gap and reach:** The cap was selected; how to validate effective geometry was not. The same constants serve appearance/culling bounds, without creating a new gameplay height rule. Physical cap acceptance does not choose silhouette or shadow/fog appearance.

**Verdict:** sound: validate the geometry that can actually be drawn. **Confidence:** high.

#### End old contact intent before fresh evidence can renew the same picture slot

**When:** contact lifecycle consolidation.

**Choice and scenario:** Report 7 expires on the same tick that the hidden cause fires again. At tick start, expiration ends orders aimed at that expired area and clears acquired aim. The existing attacker-pruning rule then asks whether another current track still grants return-fire permission. Later fresh evidence may renew report 7 and earn new aim, but cannot resurrect the old command merely because the picture's identifier matches.

```text
expire old evidence
end intent tied to that expired evidence
retain return-fire permission only through another valid track
observe this tick's new evidence
allow fresh intent under the ordinary rules
```

**Gap and reach:** One-slot visual identity did not choose retirement's tick phase. Presentation continuity is separate from combat permission, including independent identification grace.

**Verdict:** sound: a reused handle cannot prolong stale authority. **Confidence:** high.

#### One battle session supplies the contact picture to every real consumer

**When:** contact presentation and diagnostic consolidation.

**Choice and scenario:** The simulation removes a live uncertain report. The session's presentation collection retains its last evidence for the user-required three-second fade, and both the circle and its existing info panel consume the same opacity. Renewed evidence overwrites the same held slot; a replay clock moving backwards clears that visual memory. Commands and picking still use only live observation. Weapons and fog diagnostics share that presented collection; the contacts lab uses the normal BattleView instead of its own legend/list/session.

**Gap and reach:** The user specified uniform labels, borders and fading but did not choose their shared owner. This is derived animation from published evidence and battle time, not extra knowledge in the simulation. There is no raw-observation fallback, diagnostic timer or duplicate label component. Static material specimens remain geometry demonstrations.

**Verdict:** sound: one consumer boundary prevents the circle and label from disagreeing about life. **Confidence:** high.

#### Keep contact-outline contrast independent of decorative ink and reuse the text halo

**When:** contact-consumer closeout.

**Choice and scenario:** A report over bright grass needs a white rim even when its decorative hatch is faint. Rim opacity follows contact life directly; hatch strength no longer dims it. The contact title inherits the HUD's existing black text halo, so its lettering remains readable against that landscape. A new label renderer or separate contrast timer would create another presentation policy for the same report.

**Gap and reach:** The user's requested common white border/info label did not select the contrast plumbing. This makes the existing components honor that request, without turning source type into a new visual distinction or changing unknown evidence into a named unit.

**Verdict:** sound: common life controls the complete callout, while decoration cannot accidentally weaken its required outline. **Confidence:** high.

#### Remove fallen own actors from authored formation scripts; keep public commands strict

**When:** final stress-command diagnosis.

**Choice and scenario:** A scheduled formation order lists ten own units, one of which has died before its application tick. The authored script removes that known fallen actor and sends the nine survivors through ordinary command preparation. Unknown IDs and foreign-side IDs remain and trigger strict refusal; a group whose own actors all died becomes an empty no-op. The player's or network client's exact submitted selection still receives ordinary destroyed-actor validation.

**Gap and reach:** Fixed authored groups otherwise let one casualty refuse every later survivor and reserve in the group. This is a general scripted-battle policy, including ordinary endurance and replay, not a city-only exception. It intentionally changes affected scripted outcomes/digests without new actor lists, order owners, schemas or rule numbers.

**Verdict:** sound: setup scripts address their surviving formation while the public interface continues to validate exactly what the caller submitted. **Confidence:** high.

#### Count a supplied unit's return only after an admitted and accepted order

**When:** final comparison-controller return correction.

**Choice and scenario:** A rifle squad is fully replenished, but its preferred offset near the objective is physically refused. The comparison controller asks the existing preview about that point, then the objective centre. It sends an attack-move only to an admitted point and clears the resting entry/counts a rejoin only when ordinary command acceptance succeeds. If neither point works, it resets the existing resting timestamp and waits the existing ten seconds before checking again. Merely emitting the preferred order would claim a return that never happened.

```text
after full service and the existing wait:
  try preferred return point, then objective, through movement admission
  submit the first admitted destination
  if accepted: clear rest state and count the return
  otherwise: keep rest state and restart the existing wait
```

**Gap and reach:** The authored comparison policy assumed its preferred point remained legal. This fixes controller intent and reporting through existing battle authority; it changes no player command, formation, weapon, mobility or generator rule. A new acknowledgement/retry framework would duplicate the owner.

**Verdict:** sound: the return count records an actual accepted action, and failed admission has a bounded retry cadence. **Confidence:** high.

#### Use the stress scenario's actual geography at preparation

**When:** generated stress/preparation integration.

**Choice and scenario:** The late stress factory adds wrecks to the resolved generated map before producing its scenario. Preparation constructs the retained world from that scenario, not the earlier untouched generator map, and forwards its camera start and living counts. Otherwise, picking/drawing and the authority could disagree about the wrecks present at startup. Normal encounter recipes plan against their retained prepared world.

**Gap and reach:** The synthetic factory existed before worker reuse and still has its temporary placement world. Its cost is not included in the normal menu's one-authority-construction claim; synthetic and player preparation remain named separately.

**Verdict:** sound: consumers build the geography the selected scenario actually contains. **Confidence:** high.

#### Preserve the selected clear approach and existing weapon ranges as separate contracts

**When:** final playability reconciliation.

**Choice and scenario:** A town has the user-selected 1,800 m deep by 400 m wide clear approach, but current production weapons have a 900 m maximum range. The assault must move within those existing weapons' reach to fight. An old explanatory assumption that weapons would engage at 1,800 m is corrected in the plan rather than extending weapon range, changing cover/elevation or adding transport to manufacture that engagement.

**Gap and reach:** The map geometry requirement and its historical rationale diverged. The user preserved current ranges and deferred business-rule-number tuning. Map workbench tuning remains separate from the mechanics owner's weapon rules.

**Verdict:** sound: respect both selected contracts and correct the unsupported explanation. **Confidence:** high.

#### The legacy village preserves its established ruin through a narrow triangle-ratio exception

**When:** C14/C37 legacy village source integration. **Choice and scenario:** The older detailed farmhouse ruin can use more triangles than its intact village house at some detail tiers. The village source explicitly sets `damage_budget=False` to preserve the established C37 appearance, while new generated sets retain the ordinary damage-no-costlier-than-intact helper rule. This does not let the ruin exceed its physical remains or skip damage-state, fit or deterministic bake admission.

**Gap and reach:** The new helper's relative triangle guard was not the older village's appearance contract. Rebuilding that ruin solely to satisfy the new ratio would change established art without showing a release resource defect. Future new sets do not inherit the opt-out automatically; they either satisfy their own relative guard or need a justified source policy.

**Verdict:** sound as a narrow legacy appearance-preservation exception with all substantive fit/state/bake contracts retained. **Confidence:** high.

#### Source sets own physical shape and appearance together

**When:** C10–C13/C32. **Gap:** Keeping manually authored physical boxes beside a new source export would give the building two authors. **Choice:** Each script writes `kit.glb` and `templates.json`; the latter contains both the physical descriptor and the per-state module rows. `asset catalogue` derives the generator's catalogue from those descriptors, and bake/check compare them. A module row translates a piece, rotates it around its vertical axis and scales it; a tilt or mirror is a distinct module. **Reach:** A building's shape changes in its source set and deliberately moves the physical catalogue/map identity; art-only edits do not. **Verdict:** Sound, high confidence — one producer keeps simulation and art in agreement without stretching a graph at runtime.

#### Legal sizes follow the bay lattice rather than arbitrary old boxes

**When:** S2/C11/C16–C19. **Gap:** The graph's legal lengths were unknown and art stretching was forbidden. **Choice:** Graph sides use whole 3 m bays plus corner piers; scripted industry uses whole bays, and floor levels come from the descriptor. The long apartment side and its entrance face the street; dock-height warehouse openings are not pedestrian entrances. Tower/works compounds fit supported equal-height joins rather than inventing a podium the contract cannot join. **Reach:** Physical dimensions, entrances and catalogue identity change with those source facts. **Verdict:** Sound, high confidence — a chosen reusable building is placed as built, rather than distorted to any vacant rectangle.

#### Compound buildings are dressed from the union outline

**When:** C11/S5. **Gap:** Independently exported wings leave windows and facades inside a join. **Choice:** A U block's abutting part boxes remain physical parts, but the exporter traces their outer union and puts a facade on each exposed straight run. The inner wall between wings is never created. **Reach:** Compound footprints must use legal facade-run lengths; there is no later join-hiding or interior-corner repair layer. **Verdict:** Sound, high confidence — one exterior outline produces the correct exposed building without managing invisible faces.

#### Authored maps and generated maps share art but retain separate physical catalogues

**When:** C32/C37. **Gap:** The village's pinned simple boxes and the generator's complete building descriptors differ in purpose. **Choice:** One template-art library covers both catalogues; every set names which it dresses. Generated descriptors derive from sources, while authored maps retain their hand-written boxes and borrowed appearance at their own dimensions. Merging the catalogues would move unrelated generated identity. **Reach:** Template ids must be unique across both; completing village floors/doors would be a separate physical change. **Verdict:** Sound, high confidence — one drawing path serves both without rewriting existing battle geometry.

#### Kits reuse ordinary static bundles and the one loader

**When:** C13/C32. **Gap:** Shared building modules needed storage and installation. **Choice:** A kit is an ordinary static appearance whose states are modules; the existing bundle codec, validator and loader carry its meshes/materials. One packed runtime library carries exact float placement rows rather than per-template files or centimetre quantization. **Reach:** A new building piece uses existing resource lifetimes, while close-fitting rows retain their exact placement. **Verdict:** Sound, high confidence — reuse avoids a second art codec and quantization seams for a small row file.

#### Art identity is separate from physical map identity

**When:** C32. **Gap:** Materials, detail and status must change without invalidating a physically unchanged map. **Choice:** The library's art hash covers its module/material/row/tier/status content and binds each kit's original bundle hash; a separate covers list names the physical catalogues. A roof texture update moves art identity but no map or replay. **Reach:** Physical shape still deliberately moves the catalogue and saved-map identity. **Verdict:** Sound, high confidence — each identity names the facts its consumers actually depend on.

#### Buildings never silently fall back to boxes, another state or another family

**When:** C32/C37/C27. **Gap:** Incremental construction had temporary massing and missing-state fallbacks. **Choice:** Missing template rows, damage rows, modules or selected kits are named refusals. Every shipped building category has real art; the massing owner and prototype template set are removed. A tiny stand-in kit remains only for ordinary props whose kind has no model. **Reach:** A stale library stops the map rather than hiding or substituting a building. **Verdict:** Sound, high confidence — an incomplete appearance cannot masquerade as the selected physical template.

#### Physical fit is judged through the simulation's own descriptor code

**When:** C13/C32/C14. **Gap:** The art checker needed legal descriptors, catalogue hashes and ruin bounds. **Choice:** Bake/check call the contract through WebAssembly, rather than reimplementing legality. Every drawn vertex at every active tier must lie inside some physical part grown by the source set's side/top allowances, and none below its base. Ruins use the actual remains height plus their explicit jagged-top allowance. **Reach:** Balconies need parcel spacing equal to both neighbours' side allowances; slopes/foundations cannot be excused by an implicit below-ground fit. **Verdict:** Sound, high confidence — tolerated art overhang is explicit, and physical legality keeps one owner.

#### Damage art follows the physical terminal state and the intact source

**When:** C14. **Gap:** The vendored graphs had no damage inputs, and dramatic remains could exceed the body left by collapse. **Choice:** The exporter derives damage from intact walls/materials/openings: low ruins for the building rule's collapsible floor range, full-height gutted shells above it. Door gaps are read from intact rows, and timber/steel use their own broken forms. Coarse tiers fold the same damage into their shell. **Reach:** Changing collapse height/state belongs to mechanics; art cannot leave a tall skeleton above a low physical ruin. **Verdict:** Sound, high confidence — the destroyed building remains the same building, held to the authority's actual remnants.

#### One shared static-chunk owner handles buildings, trees, shrubs and corpses

**When:** C22. **Gap:** Building culling could duplicate scenery's bookkeeping or make model drawing depend on a scenery layer. **Choice:** The neutral frame owner buckets instances into chunks and computes visible/shadow ranges; each population supplies its bounds and detail rule. Building modules are anchored to their whole building, so no wing changes tier separately. **Reach:** New static populations reuse spatial bookkeeping without sharing meshes/materials or presentation policy. **Verdict:** Sound, high confidence — one spatial owner avoids parallel culling implementations.

#### Buildings keep whole-map coarse rows and a bounded nearby pool

**When:** C22. **Gap:** Expanding every template's detailed rows across a city would scale retained records with its finest art. **Choice:** Tier 3 is available for the whole map. Chunks near the view expand tiers 0–2 into a fixed pool, nearest first within a per-frame row budget; unavailable chunks draw coarse until ready. One draw range groups each module/tier instead of one draw for each chunk's copy. **Reach:** Camera cuts may reveal coarse art briefly; captures must wait for pending residency, and pool pressure degrades detail rather than hiding buildings. **Verdict:** Sound, high confidence — memory and expansion work are bounded with an explicit complete fallback to the same template's own coarse art.

#### Building tier depends on pixel density per metre of wall

**When:** C22/C23. **Gap:** Projected building height keeps a tall slab's small trim detailed far longer than a house's. **Choice:** Every chunk chooses one tier from its distance and the configured pixels-per-metre thresholds, independent of each building's height. A tall tower usually draws a coarse facade sooner than its screen height alone would suggest. **Reach:** Source sets must preserve character in coarse shells; the chunk boundary is a visible step and there is no current hysteresis. **Verdict:** Sound, high confidence — the selected detail follows the physical size of windows/trim, rather than rewarding height with disproportionate geometry.

#### Each template's own coarse shell replaces the proposed merged-tile builder

**When:** C23. **Gap:** The far-tier slice assumed massing boxes and proposed merged tiles. **Choice:** Source scripts already fold each template into its coarse shell, preserving its colours, openings and characteristic shape. Those shells draw through the same model path; no map-wide baked tile geometry is retained. **Reach:** A new template supplies its own coarse representation and does not require rebuilding a second town asset. **Verdict:** Sound, high confidence — reuse spends existing margin on simpler ownership; the coarsest representation remains each template's own art.

#### Buildings share model pipelines and update observed destruction in place

**When:** C22/C27. **Gap:** A building row is static geometry but its known damage state can change. **Choice:** Building module draws use the models layer's buffers/materials/pipelines. A knowledge update hides/shows changed intact records in place and rebuilds the small damaged population, rather than re-expanding every intact building. Consecutive draws use the raw GPU pass after shared state is bound. **Reach:** Units behind buildings use the existing depth/x-ray treatment, and a still camera adds no repeated expansion. **Verdict:** Sound, high confidence — buildings are ordinary model records under one draw owner, with work proportional to actual state change.

#### Near building shadows use their rows one tier coarser

**When:** C22. **Gap:** A simple box or shell caster puts recessed windows into incorrect shade. **Choice:** Near shadows use the template's row geometry one tier coarser; distant/offscreen populations cast coarse only where their swept shadow bounds can reach the view. A tree beyond the view whose shadow lands outside it is omitted. **Reach:** Detail must remain coherent across adjacent tiers; sun-caster work follows relevant view bounds instead of the whole map. **Verdict:** Sound, high confidence — shadows remain the same asset's geometry with bounded scene relevance.

#### Material format 4 explicitly identifies opaque, cutout, blended and room surfaces

**When:** C21. **Gap:** A bundle previously had no coverage or room contract. **Choice:** Every material names its coverage mode, with a cutoff for cutouts; an optional room-sheet name identifies interiors. Coverage is base alpha times the normal texture's alpha. Opaque alpha is ignored; blended surfaces cannot use wear, and rooms have no recipe/wear or independent light. Missing or malformed mode/sheet is rejected. **Reach:** Old format-3 material headers cannot quietly become opaque format 4; validator and decoder have distinct responsibilities. **Verdict:** Sound, high confidence — explicit metadata carries the drawing contract instead of inferring it from pictures.

#### Blender helpers write explicit coverage metadata after export

**When:** C21. **Gap:** Blender's exporter otherwise infers alpha mode from node-graph patterns. **Choice:** Shared material helpers record known modes/cutoffs and the existing texture attachment step writes their glTF values; room sheet names travel as extras. China export also writes stable tangents itself where the exporter is not byte-repeatable. **Reach:** Source determinism depends on explicit export values rather than exporter heuristics; it is not a claim that every old prop script is deterministic. **Verdict:** Sound, high confidence — known source facts are written directly with one helper owner.

#### One mesh contains separate index ranges for surface kinds

**When:** C24. **Gap:** Mixed materials in one mesh need different depth/colour policies. **Choice:** Installation groups triangle indices into opaque, cutout, room and blended ranges; those ranges draw through their respective pipelines while sharing mesh buffers. An all-opaque mesh keeps its normal range. **Reach:** Each module can mix kinds without a material branch penalizing every ordinary model or duplicate meshes. Unit impostors and x-rays currently draw only their opaque ranges; future unit cutouts require an explicit extension. **Verdict:** Sound, high confidence — geometry storage and pipeline selection remain separate responsibilities.

#### Glass is the final model range in the existing world pass

**When:** C25. **Gap:** Blending needed an ordering, depth, fog and shadow policy. **Choice:** Glass draws after opaque world/water, reads depth, writes colour only, casts no shadow and leaves the fog mask underneath unchanged. Panes remain unsorted in static packing order; this assumes similarly coloured panes rather than arbitrary coloured transparent layers. **Reach:** Effects drawn later are not attenuated behind glass, and overlays/x-ray see through it. Coloured layered glass needs a new ordering decision. **Verdict:** Sound, high confidence within ordinary window glass — it reuses the world pass and does not let a seen pane reveal hidden ground.

#### Interior atlases are project-rendered room cells carried as ordinary textures

**When:** C15/C26. **Gap:** Rooms needed repeatable imagery and delivery without side-loaded files. **Choice:** A fixed-seed CPU path tracer renders ten apartment cells and ten shop cells, 128 px each. Source sheets use two columns by five rows; bake rearranges them into square texture layers and generates mips that keep cells separate. Each kit carries the sheets its room materials use, with GPU texture deduplication. **Reach:** More variety is another sheet/source recipe; the room picture does not import photographs or upstream lamp styling. **Verdict:** Sound, high confidence — one asset pipeline delivers deterministic committed source imagery through the existing texture contract.

#### Room projection is computed from an unfolded box at each fragment

**When:** C26. **Gap:** Interpolating already-projected texture coordinates distorted the floor across its triangles. **Choice:** A room box's UVs carry its linear position on unfolded faces; the fragment applies the pinhole projection there. A stable hash of the module's flat position chooses the cell and mirroring, so a camera move never changes the room. **Reach:** One row per window can show a distinct room; folding all rooms into one module would give them one hash. Boxes can have different dimensions without another material parameter. **Verdict:** Sound, high confidence — the nonlinear projection belongs where actual fragment position is known.

#### Near windows cut the wall; crowded corners use a blind

**When:** C26/scripted sets. **Gap:** A room behind an uncut wall is invisible, and two adjacent room boxes can overlap at a corner. **Choice:** Source helpers cut window openings in near shell tiers and fit rooms in street/back/end order. A room may move or shorten within minimum sizes; if it cannot fit, that window closes with a blind rather than an intersecting tiny room. Rooms normally draw at tiers 0 and 1; tower rooms are inside tier-0 panels, with coarse facade paint replacing them thereafter. **Reach:** Some windows are intentionally blind, and coarse tiers preserve the window tone rather than full rooms. **Verdict:** Sound, high confidence — sources refuse overlapping room volumes and avoid geometry that fights for depth.

#### Destruction is drawn from each side's existing published knowledge

**When:** C27. **Gap:** The renderer needed to distinguish low ruin from full-height gutted state without hidden simulation access. **Choice:** When a side learns a replaced part, the published prop type is compared with the building rule's gutted type; other replacements mean ruin. One known part changes the whole template's art because the simulation destroys/learns the aggregate together. Unknown destruction leaves the intact art. **Reach:** Camera obstacles and drawing consume the same known parts, with no new publication or guessed height test. **Verdict:** Sound, high confidence — one authority decides both state and when a side may see it.

#### One facade specimen kit and two production-path building labs replace bespoke sheets

**When:** C24/C22/C27. **Gap:** Asset appearance needed repeatable inspection without changing the physical catalogue to add test buildings. **Choice:** The facade lab places a kit with no template; lineup and block labs draw real template art through the game's own model path, forcing tiers with threshold data. The ruin lab's scenario blast emitter uses ordinary damage/publication. Per-set Blender sheet scripts are removed; the assembly helper is only a source preview. **Reach:** Diagnostics inspect actual contracts without a parallel renderer or a fake physics state. **Verdict:** Sound, high confidence — the remaining tools each have one clear question and share production owners.

#### Kits are fetched on demand and installed atomically

**When:** C32/requested-kit consolidation. **Gap:** Every page fetched every large kit regardless of its map. **Choice:** Catalog load takes non-kit appearances and the small template library; a map asks the library which kits its own templates need, and the loader fetches missing names once. It checks everything and installs a complete generation, retaining the old generation on failure. Fetched kits persist until catalogue replacement; models install only the current map's selection. **Reach:** Non-kit downloads remain eager, kit residency has no eviction policy, and named fetch failures still do not reach the loading cover. **Verdict:** Sound, high confidence — one loader and one map-to-kit owner avoid a second catalogue and partial art installation.

#### Compression has its own address and bounded verification

**When:** C32 lossless transport. **Gap:** Shared kit/library wire bytes needed reduction without changing decoded art. **Choice:** Gzip payloads have encoded hashes/lengths distinct from original art hashes/lengths. One owner verifies encoded length/hash, inflates into the declared bound, cancels overflow, then verifies original length/hash before decoding. Platform streams provide gzip; no raw fallback or parallel raw runtime copy remains for kits/library. Unit/scenery/clip readers retain their identified raw contract. **Reach:** Another compressor may produce another URL for identical art; physical identity and decoded geometry remain unchanged. **Verdict:** Sound, high confidence — immutable encoded addresses and exact art identities describe different bytes honestly.

#### The complete selected wire download is admitted before kit requests

**When:** C32 lossless transport. **Gap:** Caching or loading the library first could bypass an aggregate download allowance. **Choice:** Browser and native tool count one library plus each unique selected kit hash before kit payload requests or a cached return, and check the library alone before reading it. Requested city objects require their gzip records; non-kit-only producers may omit the table. A catalogue replacement retries all originally requested names rather than only the old missing subset. **Reach:** The 50 MiB shared wire gate, 50 MiB raw per-kit gate, decoded resident memory and GPU goals are separate; smaller download proves none of the latter. **Verdict:** Sound, high confidence — the limiter runs before the work it limits and a retry fulfils the same complete selection.

#### Trees share one validated size and canopy envelope

**When:** C73–C76. **Gap:** Species could change apparent coverage simply by being larger than the physics. **Choice:** The scenery table owns tier triangle caps and one tree-size band (11 m top, 0.40 m bole radius at breast height within tolerance); every tier fits the physical canopy height/radius. Placement scales width no larger than its art and no longer crushes edge crowns to fit the forest floor. The canopy may overhang a field exactly as the simulation's foliage does. **Reach:** Species vary in shape rather than arbitrary body size, and visual fit remains bounded at every tier. **Verdict:** Sound, high confidence — art validation and bounded placement compose without copying the physical canopy rule into a new renderer authority.

#### Canopy closure is a broad appearance guard with an accepted initial value

**When:** C76. **Gap:** “Mostly closed, floor still seen” lacked a concrete guard. **Choice:** The models-on/off class-mask pair bounds visible floor share between 0.12 and 0.40 at two canopy stations; the current accepted value is a given. Girth and conifer art reach tune within the validated canopy envelope. **Reach:** The guard avoids both a sealed lid and an orchard but does not measure torso visibility or certify every forest arrangement. **Verdict:** Sound, high confidence — it records the selected floor-share interpretation and keeps the accepted value as a given rather than a new taste question.

#### Nonphysical forest dressing is seeded by bounded spatial cells

**When:** C79. **Gap:** Expanding thousands of plants per hectare over a whole map would retain too many records. **Choice:** Each 64 m cell reproduces its own dressing from its coordinates. A 128-cell pool lays nearest visible cells first within a budget, evicts least-recently wanted cells, and leaves far excess undrawn. Individual pieces shrink at the foot according to pixel height in the vertex stage; no per-piece CPU work repeats. **Reach:** Detail can arrive after a cut, cleared ground refreshes existing cells, and the same coordinate produces the same plants independent of forest naming. **Verdict:** Sound, high confidence — retained decorative instances and generation work are bounded by the view.

#### Tree lines draw the physical concealment that bare trunks failed to show

**When:** C86. **Gap:** The simulation's wooded strip obscured distant squads while its picture showed gaps between bare boles. **Choice:** Strip forests carry hedgerow shrubs below crowns; ordinary woods retain waist-high dressing. Shrubs fit their full reach on strip ground, within nearby physical foliage and away from water/paving/bodies, shrinking or omitted when too narrow. They form their own static-chunk population, share forest shadow/fog/clearing, and have a cost toggle. **Reach:** This adds no new body or sensing rule; narrow strips may have no shrubs. **Verdict:** Sound, high confidence — the visual obstruction follows existing physical foliage instead of inventing extra concealment.

#### Street appearance binds to placed boxes, with repeatable modular items

**When:** C45. **Gap:** Street body rows have no fixed size; each placement supplies it. **Choice:** The art catalogue declares the model's authored footprint and the renderer fits each placed box per axis. Barrier, Heras and scaffold rows repeat modules along a long box instead of stretching one. A car wreck has its own appearance fitted to the car plan and physical 0.7 m remains height. **Reach:** A mismatched placed car can still squash; body presets and source footprints must be chosen coherently. **Verdict:** Sound, high confidence — authored size and placed size have distinct owners, and terminal art follows the actual remains.

#### Ground uses one exact local index of native primitives

**When:** C63. **Gap:** Every fragment/grass clump could not scan every road, forest and river at city scale. **Choice:** An index buckets native rects, strokes, triangles and boundaries; a pixel selects the grid level appropriate to its footprint. Rects join the common records table to stay within existing storage bindings. Distance functions remain the existing native-primitive functions; beyond local reach only inside/outside is promised. **Reach:** Grass, colour, masks and water use one geometry lookup instead of a resampled distance bake. **Verdict:** Sound, high confidence for local geometry ownership — exactness within declared reach is distinct from the bounded wide-pixel filtering choice above.

#### Paving exports carry physical area kind, and drawing separates rule from paint

**When:** C66/C28/authored cutover. **Gap:** The old exported kind collapsed all carriageways to one speed-surface name. **Choice:** The existing kind column now identifies road/country road/track/sidewalk against the shared area-kind list. The terrain computes physical paving separately from painted strokes, polygon looks and nonphysical sidewalk bands; masks read physical paving, colour/grass read paint. Rural authored maps now name country roads explicitly and the all-road guessing fallback is deleted. **Reach:** A town-only map correctly draws asphalt without content-based inference. **Verdict:** Sound, high confidence — exported facts grow in meaning without a second column or duplicate physical rule.

#### Sidewalks and curbs are ground appearance, not extra collision geometry

**When:** C28/C29. **Gap:** Generated streets have carriageway strokes and loading-yard polygons but no physical sidewalk band. **Choice:** Biome rows paint walks outside strokes, draw yards with the sidewalk row, and shade kerbstones with a normal tilt rather than adding raised mesh. Another carriageway suppresses curb/paint at the crossing. **Reach:** Units move on the ground beside a road as before; this can look flush at low views and does not provide cover or a physical step. **Verdict:** Sound, high confidence — visual ground details remain in the existing surface owner and cannot create unobserved obstacles.

#### Road shoulder wear is shared by colour and grass

**When:** C67/C68. **Gap:** Separate density/colour rules could put intact grass over worn ground or worn paint under dense blades. **Choice:** One wear calculation supplies both colour and clump thinning. Rut darkening compensates elsewhere in the road so the mean is unchanged, and the centre strip/ruts fade to exactly the plain surface. The existing paving lookup also returns lane position/direction along its winning stroke. **Reach:** Further shoulder/rut tuning retains aligned boundaries with no second centreline scan. **Verdict:** Sound, high confidence — ground and blades cannot disagree about the same wear event.

#### Ground masks and toggles observe the actual draw owner

**When:** C62/C67/C81/C85/C86. **Gap:** Visual/cost comparisons needed the same surface class and a feature-off frame. **Choice:** The terrain itself writes the ground-class view; mixed/non-ground pixels are black and grass/water are omitted there. Feature toggles remove trees, dressing, hedge, road wear or field texture through their current owners; field texture removal repacks plot data rather than adding a permanent shader flag. Generated stations follow returned map anchors and drawn surfaces; fixed authored stations retain their coordinates. **Reach:** A mask is diagnostic classification, and feature-off cost is an intervention on actual production paths; the rig's extra page terrain build remains preparation overhead. **Verdict:** Sound, high confidence — verification does not invent a CPU-painted surface or a second feature implementation.

#### Grass preset shape, wind and field placement have separate owners

**When:** C80–C83. **Gap:** “Clumping” and wind were ambiguous between one model and a field of models. **Choice:** Radius controls one clump, biome drift/thinning controls where clumps gather, and vertex alpha carries the preset's wind response through the ordinary asset path. Headed blades keep a vertex pair at their head in every tier. Rough/prairie and named crop kinds replace an undifferentiated generic crop. **Reach:** A preset's wind cannot exceed the biome wind, and a new crop adds ordinary data rather than a new placement mechanism. **Verdict:** Sound, high confidence — model construction and population layout remain distinct, reusable contracts.

#### Camera safety uses the same side-known building geometry as drawing

**When:** C57/C27. **Gap:** Hidden destruction must not silently open a path through a building still drawn intact. **Choice:** Ground and each side-known building part block the eye; known remains replace standing boxes, while unseen destruction leaves standing obstacles. Trees, fences, walls, wrecks and units do not enter this camera collision policy. Push-out chooses the nearest current side or roof, and a camera cut uses the target side. **Reach:** A corner has the conservative clearance of a grown box; safety moves the eye while preserving the player's looked-at point. **Verdict:** Sound, high confidence — camera and picture share knowledge and geometry instead of observing hidden simulation state.
