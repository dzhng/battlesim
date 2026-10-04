# Choices ledger: New York and Paris building families

Decisions made where the plan was silent, audited against the shipped code. Grouped by verdict, least confident first. **Review these first:** the shared industrial massing and the New York towers without setbacks. The family drawn by seed with no menu choice was reviewed and reversed: the player may now pick a region.

## Needs the user (each has a reversible provisional call in force)

#### Some New York and Paris industrial lots share China's shapes and differ only in dress

**When:** the towers and industry. **Choice:** A New York shed, a Paris shed and a China shed are the same 15 × 24 m steel-framed box with the same roof layout, in different cladding, colours and doors. The same is true of the sawtooth works in all three families, and of the twin-span warehouse in China and Paris. Picture flying over an industrial district: a New York map's works and a Paris map's works have identical silhouettes and only read as different up close. The other five lots got a regional builder: New York's loft (a brick building with a water tank), warehouse and depot (stepped brick fronts), and the Paris atelier and barrel-vaulted depot. The unbuilt alternative is a distinct massing for every industrial lot of every family. **Gap:** The plan named the regional character but not whether industrial massing must differ. **Reach:** A future art pass can swap a lot's builder per family without touching the generator; the footprints must stay as they are, because they are what lets the generator fill lots. **Verdict:** needs-user. Provisional call: keep the shared massing, since industrial sheds look much alike worldwide. To reverse it, give the family's design in `industry.py` its own builder for that lot. **Confidence:** low.

#### New York towers carry their character in cornice, crown and water tanks, not setbacks

**When:** the towers and industry. **Choice:** A Manhattan tower usually steps back as it rises (a "setback"). The physical template contract gives a tower one box part, and two stacked parts would need a join rule it doesn't have. So New York towers are straight shafts with a heavy cornice, a limestone crown and rooftop water tanks. The plan's coverage table had promised setbacks. The unbuilt alternative is setbacks drawn as art inside one tall box, but that would draw air where the physics has wall: a soldier behind the visible setback would be blocked by a body the camera shows as empty. **Gap:** The plan promised setbacks without checking that the physical contract could hold them. **Reach:** Real setbacks need a stacked-part join in the template contract (`crates/contract/src/templates.rs`). That is a physics decision, not art. **Verdict:** needs-user. Provisional call: straight shafts. To reverse it, add vertical part stacking to the contract, then give the towers stepped parts. **Confidence:** low.

## Sound

#### Paris apartment blocks count the mansard as a floor and stay smaller than China's tallest

**When:** the graph apartments. **Choice:** A Haussmann block's top storey is in its zinc mansard, with dormer windows. Each dormer is a real window over a fighting bay, so the mansard counts in `floor_heights_m`, and a "6-floor" Paris block is five stone storeys plus the mansard. To keep the finest detail tier under its 150,000-triangle budget, the tall Paris blocks are 47 × 14 m and 35 × 14 m, against China's 59 × 14 m and 53 × 14 m. When a Paris block burns, the mansard is gone and the shell reads as a flat slab with holes. **Gap:** The budget and the graph's density forced a size the plan did not fix. **Reach:** Garrison bays include the mansard. The smaller tall blocks fill a little less ground per lot, but the per-family generation sweep fills every type and size. **Verdict:** sound. **Confidence:** medium.

#### New York and Paris apartments are single rectangles; only China has U and court blocks

**When:** the graph apartments. **Choice:** China's graph can wrap a facade round a U or a courtyard. The New York graph mitres its cornice only at outside corners, and the French graph builds one box. So every New York and Paris apartment template is one rectangular part, and a district gets its variety from sizes and floor counts instead. **Gap:** The plan made compounds optional. **Reach:** A later pass could add compounds from the scripted builders rather than the graphs. **Verdict:** sound. **Confidence:** medium.

#### New York apartment lengths follow the shop rhythm: 38, 47 and 56 m

**When:** the graph apartments. **Choice:** The NYC graph puts one 9 m shop under every three 3 m bays, so a street front must be a whole number of shops. The New York slabs are 38, 47 and 56 m long, where China's are 35, 47, 59 and 53 m. Back walls of a ground floor without shops take the first floor's facade, because the graph leaves that floor empty. **Gap:** The graph's legal lengths differ from China's. **Reach:** These are physical sizes, so they decide which lots a New York block fits. **Verdict:** sound. **Confidence:** medium.

#### New York water tanks, blade signs and porches stand within widened art fits, never past the physics

**When:** the art passes. **Choice:** A set's "fit" is how far its art may reach past the physical box: a cornice past the wall, a tank above the roof. A New York tower's 4.7 m water tank stands on the roof deck and reaches about 4.05 m above the physical box's top, so its set allows 4.1 m of art above the top (`water_tank.fit_over`, derived from the tank's height). The apartments allow 8 m above (tanks on stands) and 1.9 m to the side (blade signs). The tank does not stop rounds or sight: a soldier behind a rooftop tank is not hidden by it. Porches are the reverse: the ground-floor wall is set 2.4–2.6 m back inside the box, so the porch roof stays inside the physical box and still hides what it draws over. **Gap:** The plan did not size the overhangs for regional features. **Reach:** Rooftop furniture stays decorative. Making it cover would be a physics change. **Verdict:** sound. Art never claims more cover than physics gives. **Confidence:** medium.

#### The silo is its own tall part

**When:** the homes and farms. **Choice:** The American farm's silo is a 4 × 4 m part 13.3 m tall, with hatches at its fighting bays, so it hides and stops what its picture shows. Because it is the tallest part, the farm's ruin height rises to 3.3 m. **Gap:** A silo is taller than any China farm part. **Reach:** Only the New York yard farm. **Verdict:** sound. **Confidence:** medium.

#### New York and Paris reuse China's shopfronts, signs and chimneys where they fit

**When:** the homes and farms. **Choice:** The new families' homes use the existing shop window, shop door, sign board and chimney modules, in new colours, rather than new regional versions. Paris rows add a wider four-pot chimney on each party wall. **Gap:** The plan did not say which fittings must be regional. **Reach:** A future art pass can give a family its own shopfront module. **Verdict:** sound. Shopfronts are generic at the game camera, and the regional read comes from walls, roofs and windows. **Confidence:** medium.

#### Material calls: brownstone is smooth render, Paris slate is matte, one limestone for all Paris blocks

**When:** the art passes. **Choice:**
- Brownstone uses the smooth render recipe tinted brown, because the coursed-stone recipe read as brick.
- Paris slate uses a rougher twin of the slate recipe (`roof_slate_matte`), because the dark glossy slate reflected the sky and one slope read as flat grey.
- Every Paris apartment block shares one untinted limestone, while New York brick takes a colour per template.
- New York trim uses the same `ashlar` recipe as Paris, tinted.
- `facing_brick_pale` remains a tintable neutral brick beside China's red `brick`.

**Gap:** Material choices were left to the art passes. **Reach:** Recipes are data in `textures.py`. **Verdict:** sound. **Confidence:** medium.

#### The New York loft is two storeys and the Paris depot's vault crowns within 12 m

**When:** integrating the three art passes. **Choice:** The catalogue test holds industry to one or two floors and to storeys no taller than 12 m. The first New York loft had three storeys, and the Paris depot's barrel vault made its one storey 12.5 m tall. Rather than widen the band, the loft became two storeys and the vault's rise dropped from 4.6 m to 4.1 m. **Gap:** The art pass didn't know the band. **Reach:** The band stays as it was for every family. **Verdict:** sound: the requirement holds and the art changed. **Confidence:** medium.

#### Far-tier visual limits are kept where the fix would move China

**When:** the art passes. **Choice:** Some far-tier "pops" (a detail changing as the camera crosses a tier boundary) come from shared machinery that China uses too:
- shutters fold to colour bands;
- ruins lose debris at tier 2;
- burnt roof holes move between tiers;
- curtains and air conditioners leave at tier 1.

Fixing them would change China's committed art, so they are disclosed and left. Some limits are specific to Paris: corner balconies wrap round onto the side wall (the graph's own geometry), and a gutted block reads as a slab. **Gap:** China's bytes are frozen by the plan. **Reach:** A later shared art pass can fix them for all three families at once and rebake. **Verdict:** sound, as disclosed limits. **Confidence:** medium.

#### Each family's whole art is held to the 50 MiB per-map download

**When:** integrating the three art passes. **Choice:** The download test used to add every generated template's kit together, which only made sense while there was one family. A map draws one family, so the test now holds each family's full kit set to 50 MiB. Measured after the final bake: China 27.9 MiB, New York 24.0 MiB, Paris 27.2 MiB. Every kit's integrity is still decoded and checked. **Gap:** The old test assumed one family. **Reach:** A fourth family is measured on its own art. **Verdict:** sound: the limit is unchanged and applied to what one map can fetch. **Confidence:** high.

#### The two saved maps that read the whole generated library select their China templates by id

**When:** integrating the three art passes. **Choice:** Market Town and the camera lab pinned the generated library's old hash. Adding templates would have made them unresolvable. Their `SOURCES.json` now lists the 29 China template ids, which hash to exactly the catalogue they pinned. So the released battlefield is unchanged, and no regeneration or review was needed. **Gap:** City-maps' precedent was to regenerate saved maps. **Reach:** These maps no longer move when templates are added; the camera-lab test compiles against the same selection. **Verdict:** sound. **Confidence:** high.

#### Presets move to layout-presets-13 and the parity records' requests are repinned

**When:** integrating the three art passes. **Choice:** Adding families changes the presets' content, so their revision label moves, and the parity records' request JSON names the new catalogue hash and revision. Their native halves were re-recorded with `BLESS_PARITY=1`. No record changed its exit code or status; only outcome hashes moved. The deliberately stale `layout-presets-0` record stays stale. **Gap:** Plan said "identities move", without the mechanics. **Reach:** Every generated map's identity changed, so replays from before this change won't load, as the build policy already warns. **Verdict:** sound. **Confidence:** high.

#### One generalized graph exporter, three per-graph tables

**When:** the graph apartments. **Choice:** The China apartment script became `graphset.py`, which every graph-derived set shares (outline, rooms, tiers, damage, files), plus `filters.py` and `tangents.py`. `china.py`, `nyc.py` and `paris.py` now hold only their graph's tables. China's output stayed byte-identical through the move. **Gap:** The plan asked for one owner, not its shape. **Reach:** Another vendored graph is a new table, not a copy. **Verdict:** sound. **Confidence:** high.

#### China's blank back bays are a named exception; new sets refuse blank bays

**When:** the graph apartments. **Choice:** A garrison soldier needs a real opening at eye and muzzle height in every declared fighting bay. China's graph leaves a few back ground-floor bays without one, which was already the case before this work. Refusing them would move China's bytes. So the exporter prints China's blank bays and refuses them for every other graph set. **Gap:** The new check exposed an old China gap. **Reach:** A future China rebuild can fix the descriptor and drop the exception. **Verdict:** sound, as a disclosed exception. **Confidence:** high.

#### Scripts take `<script> [family] [out]`, refusing unknown families

**When:** the homes, farms, towers and industry passes and the closing code review. **Choice:** Every scripted kind (homes, farmsteads, towers, industry) reads its arguments through `kit.family_set` and `kit.design`:
- no argument builds China into its committed folder, as before;
- `towers.py paris` builds `towers_paris`;
- a mistyped family stops with an error before anything is written.

The old China-only form `homes.py <out>` now needs `china` first. Before review, homes and farmsteads silently built China into whatever folder was named. **Gap:** The plan didn't define the command. **Reach:** Source receipts record `"towers.py paris"`, while China's still say `"towers.py"`. **Verdict:** sound. **Confidence:** high.

#### A set builds only the fittings its templates place

**When:** the closing code review. **Choice:** A family that has no water tank shouldn't export one. Each script now offers its fittings (`kit.offer`), and a fitting is built the first time a row places it (`kit.need`). The "module no template places" check therefore stays strict for every family. An interim `optional` flag had quietly disabled that check for China's towers and industry. **Gap:** Shared fitting tables across families. **Reach:** A new fitting is offered once and appears only where it is used. **Verdict:** sound. **Confidence:** high.

#### Generation gate: each family alone, every type, as a test; every size by sweep

**When:** integrating the three art passes. **Choice:** `each_regional_family_asked_for_builds_every_map_type` asks for each family in turn and generates Open, Mixed and Metro Small through the full pipeline (towns and countryside). Size adds settlements but no new district kinds, so sizes are covered by the release sweep: 3 families × 9 cells × 4 seeds, all 108 maps generated. The catalogue test separately holds each family to three or more variants of every category, at its scale. **Gap:** The plan named the gate but not its cost split. **Reach:** A family missing a category fails in about 4 s. **Verdict:** sound. **Confidence:** high.

#### Sign text stays generic trade words

**When:** the graph apartments. **Choice:** The NYC graph's sign "TASTE VIETNAM" names a country, so it is swapped for "BAKERY". "HOTEL", "PIZZA", "BOOKS" and "COFFEE SHOP" stay, because the city readme allows generic trades. **Gap:** The graph's word list was unchecked. **Reach:** Another graph's sign list needs the same check. **Verdict:** sound. **Confidence:** high.

Trivial discretion: template ids (`nyc-…`, `paris-…`), set names (`<kind>_<family>`; China keeps its committed names), texture seed blocks (51xx–53xx for clapboard, ashlar and meulière; 61xx and 63xx for the pale facing brick and washed-gravel panels), and colour choices per template.

## Settled by the user

#### A map's family is a seeded draw unless the player picks one

**When:** planning. **Choice:** When a player presses Play, the map's seed already decides its family through its own named random stream (`parcels::family`), one in three each for China, New York and Paris. The menu still offers only type and size, and the share address `/battle?type=&size=&seed=` still names exactly one battlefield. The unbuilt alternative is a "region" control on the menu, which would add a third address parameter and a new request field. **Gap:** City-maps called for "preset selection" without saying whether the player picks. **Reach:** A menu choice needs a request field. Left out of the request when unset, it changes no earlier address or request hash; only a chosen region makes a new identity. **Verdict:** reversed by the user after closeout: the menu offers each region and random, and an unlisted region is refused at `$.region`, never redrawn. **Confidence:** high.
