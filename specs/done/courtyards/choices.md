# Choices ledger: courtyards

Decisions made where the plan was silent, audited against the shipped code at closeout. Grouped by verdict, least confident first. **Review these first:** dense courts as infantry ground, lawns sparser than the brief, and paving at open-ground speed.

## Needs the user (each has a reversible provisional call in force)

#### Lawns stay sparser than the brief asked
**Choice:** Lawns get paths, a vehicle lane, planting beside paths and the shared and regional groups, but less than "stone or concrete blocks with actual stuff in them" implies: tighter rings or more groups pushed the move probe past its pre-courtyards baseline. About 10 % of lawn ground per region is still more than 8 m from anything (held under 2.5 % on maps short of the part limit). **Gap:** The plan set no density bar; the trade-off against vehicle placement appeared during the build. **Reach:** How full a town reads from the 120 m camera. **Verdict:** needs-user. Provisional call: the probe wins. To push fill further, raise the lawn group weights and lower `spacing_m` in each dense district's `props.courts`, and the `beside` weights and `beside_spacing_m` in `street_props.courts.lawn`, then re-check the move probe. **Confidence:** low.

#### Dense courts are infantry ground; a vehicle is owed a way in, not a way everywhere
**Choice:** Inside a dense block, vehicles get one way in from a street to each yard's front, each car park's aisle and a deep point of each sizeable lawn (8 m lanes, 8 m yard gates). Everywhere else, groups keep only a squad-abreast ring (6 m), and yard walls stand wherever a squad can pass between wall and building. So tanks reach a court but cannot roam it; squads reach everything they could before. **Gap:** The plan said each court keeps a way in for infantry and one for vehicles; the build first read it as "a tank can reach every point", which kept courts empty. **Reach:** Every town fight. **Verdict:** settled by the user (2026-10-07): a hull goes wherever it fits, and courts are not kept from armour. The layout stands as built; what kept tanks to the lanes was the route planner's 2 m cells, which wanted about 8 m of way. Since the planner judges tight cells against the bodies themselves, a tank roams any part of a court whose 6 m ring it fits. **Confidence:** high.

#### Paving moves vehicles at open-ground speed
**Choice:** `paving` (courts, yards, car parks, paths and industrial aprons, which were roads) has no road speed bonus. At road speed whole block interiors become shortcuts and columns cut diagonally through courts. **Gap:** The plan left paving's speed to the mechanics review. **Reach:** Every vehicle route through a town or industrial estate. **Verdict:** needs-user. Provisional call: open-ground speed. To reverse, set `surfaces.paving.speed_factor` to 1 in `fixtures/game.json`. **Confidence:** medium (a hard yard is physically as fast as a road).

## Sound, with a concern

#### Group moves into towns fail slightly more often than before courtyards
**Choice:** The courts ship with the move probe (`battle_sweep`, every type, size and region at seeds 1–3) at 41 of 81 maps with an unplaced destination (105 units, mostly tanks and supply trucks), against 38 before courtyards; the vehicle-placement, reachability and cover rules brought it back from 57, and near-identical court layouts moved it by 4–6 maps. Most remaining failures are route certification exhausting each mover's replanning share, which predates courtyards; cheaper short-leg replans would fix them but change every battle's routes. **Gap:** The plan had no placement bar. **Reach:** A group order into a dense town may leave a vehicle without a certified spot. **Verdict:** sound for this feature; the replan budget is a separate change. **Confidence:** medium.

#### Squads walk round fences and walls; they don't climb them
**Choice:** Garden fences and yard walls and railings block movers; a squad goes through a gate or round. Hedges and washing lines block nothing and only hide. **Gap:** Silent. **Reach:** Suburb and dense-court fights channel through gates. **Verdict:** sound. **Confidence:** medium (fences a squad can't climb may channel fights more than a viewer expects).

#### Courts and gardens are dressed last, and courts take the remaining parts first
**Choice:** Courts and gardens are placed after the open-country cover pass, kept off the woods, because placed earlier they took the ground the sight certificate's copses needed and some maps were refused. At the authored-part limit courts take parts first (at most `courts.share` of what is left) and gardens get the rest, lots dressed in a drawn order so bare gardens are spread out. **Gap:** Silent on order and on who yields at the limit. **Reach:** On the largest maps some gardens and apartment yards stay bare. **Verdict:** sound. **Confidence:** medium.

#### Bodies on a map rise about sevenfold
**Choice:** Gardens, courts and boundaries take Mixed Small seed 1 from about 4.4k bodies to 30.4k (map-wide, building parts excluded). In the city-contact benchmark the GPU frame rose from 12.9 to 14.0 ms mean (+9 MiB of buffers), and the scene, CPU-bound at about 20 FPS before and after, spends more CPU per frame (median 20.3 to 24.4 ms, p95 47.6 to 55.9 ms). **Gap:** The plan had no body budget beyond the part limit. **Reach:** Frame cost in towns. **Verdict:** sound for the GPU; the CPU cost is a concern for a later performance pass. **Confidence:** medium.

#### Regional looks travel gzipped; shared scenery stays an ungated eager load
**Choice:** An appearance tagged with a region is fetched only for that region's maps, gzipped and counted in the map's 50 MiB download (China 28.2, New York 24.4, Paris 27.5 MiB after all art). Shared scenery (about 267 MiB raw before this feature, mostly soldiers) still loads raw on every map and is not counted. The court, garden and boundary pieces, signature ones included, are untagged, so every map fetches all three regions' signature art (each piece at most 1 MiB raw); only the regional benches and bins are tagged. Where a region has none of its own looks for a kind but other regions do, it draws the tinted stand-in. **Gap:** The plan said "load only the map's region" without saying how the gate counts. **Reach:** `MAP_DOWNLOAD_MAX_BYTES`, `fetchedOnRequest`. **Verdict:** sound. **Confidence:** medium (shared scenery remains ungated).

#### Art bakes textures at 128 px and drops thin geometry at distance
**Choice:** Garden, court and boundary pieces bake textures at half the street set's size to fit 1 MiB; fence boards are texture on solid panels, washing-line cords and railing bars are geometry only up close, because they shimmered at distance. **Gap:** The cap left texture size to the art. **Reach:** How pieces read at 12 m and beyond; far railings read as a dotted veil. **Verdict:** sound. **Confidence:** medium.

#### Paving grid fades to plain colour with distance
**Choice:** Each paved area has one slab grid along the direction most of its outline shares, a plain border where it meets the walk, and joints that fade between 24 and 2.5 pixels per slab; from low cameras the grid seems to stop 40–60 m out. **Gap:** Silent on paving at distance. **Reach:** The terrain material. **Verdict:** sound; the fade distance is taste. **Confidence:** medium.

#### The ground scene's open-ground sample floor was lowered
**Choice:** The `town-250` check (open ground between a town's buildings) needed more than 50k sample pixels; courts now pave most of that ground, so the floor is 10k (18k measured). Its claims (yard share above 0.8, no crops) are unchanged. **Gap:** A check written before courts existed. **Reach:** `web/scenes/_town.mjs`. **Verdict:** sound. **Confidence:** medium.

## Sound

#### A vehicle's destination is the nearest clear point it can reach
**Choice:** A vehicle ordered somewhere parks where its whole hull stands clear of every known body, including ones it could shove, and that it can reach on its side's known grid; a click into a walled lawn, across water or up a cliff moves to the nearest such point within 48 m, else the order is refused. Squads are placed as before. **Gap:** Denser courts left vehicles wedged against parked cars or sent into sealed lawns. **Reach:** Every vehicle move and garrison gathering; group moves into towns fail no more often than before courtyards. **Verdict:** sound. **Confidence:** high.

#### A cover post keeps its standoff from every body
**Choice:** Take-cover offers a post only where a soldier's centre stands half a metre (0.2 m beyond his body) from every body and friendly hull, not just the one he hides behind, so men don't file into the gaps between parked cars. Checking the navigation grid instead was tried and added nothing. **Gap:** A regression the courts exposed. **Reach:** Every take-cover order. **Verdict:** sound. **Confidence:** high.

#### Courts are yards, car parks, paths and lawns
**Choice:** Each built parcel of a dense district is its own paved yard bounded by one regional kind per district (China masonry wall, New York iron railing or chain-link, Paris railing on a low plinth) with door gates and one vehicle gate; car parks run along streets; the rest is lawn crossed by paths. A shared lot edge carries one boundary. This replaced paving each district as one plaza, which no density of props filled. **Gap:** The plan paved the district ring. **Reach:** `parcels/courts.rs` (`CourtPlan.kind`), presets `street_props.courts`. **Verdict:** sound. **Confidence:** high.

#### Amenity and boundary physics come from what they are made of
**Choice:** Concrete ping-pong tables, masonry garages and walls, and steel dumpsters stop rounds; sheds, kiosks and bike sheds hide and give light cover; chain-link, railings, gym and playground frames and hoops block movers but not sight or rounds; a swing blocks only vehicles; laundry and hedges only hide; court floors and pétanque pitches block nothing. A tank flattens walls and railings; a truck flattens New York's iron railing and shoves chain-link and garden fences aside, but stops at China's wall and Paris's plinth railing, whose plinth also gives light cover. **Gap:** The plan named the pieces, not their physics. **Reach:** Every town fight; the village balance report against main kept 48 of 50 digests and the same captures. **Verdict:** sound. **Confidence:** high.

#### A region's look of a shared piece is a second source
**Choice:** Shared pieces (bench, bins) get China, New York and Paris looks as second sources with the shared box, tagged with the region; where a region has its own look, only its looks are candidates. Signature pieces (ping-pong table, garages, kiosk) are their own kinds because their bodies differ. **Gap:** Which pieces read regional was delegated. **Reach:** `PropAppearances`, `assets/catalog.json`. **Verdict:** sound. **Confidence:** high.

#### Regional paving is a finish and a slab scale
**Choice:** China grey cast concrete in 4 m bays, New York pale 1.5 m flags, Paris pale limestone 1.2 m flags; setts and gravel are too fine to read at game distance. **Gap:** Palettes and slab sizes were delegated. **Reach:** The biome's paving row. **Verdict:** sound. **Confidence:** high.

#### Trees may stand on paving
**Choice:** Forest trunks keep off carriageways only; keeping them off all paving refused maps on ground-sight coverage. **Gap:** Silent. **Reach:** The sight certificate. **Verdict:** sound. **Confidence:** high.

#### Garden dressing
**Choice:** Suburb and village lots get up to one each of shed, washing line and table in the rear setback, and most get a boundary of one kind (hedge to fence 3:1 in villages, 1:1 in suburbs), the same in every region. **Gap:** The mix was delegated. **Reach:** Presets `props.gardens`. **Verdict:** sound. **Confidence:** high.

Trivial discretion: kind and module names, catalog file split (`gardens.json`, `courts.json`), group recipes, slab joint widths, scene framings, the shared `Stream` draw helpers.
