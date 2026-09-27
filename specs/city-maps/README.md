# City maps: real city districts as battle maps (DRAFT)

> **Draft, 2026-09-27.** This is not a plan to implement. It is the input to a [write-spec](../../.agents/skills/write-spec/SKILL.md) interview with the user, and every section below can change. Research and links are in [`research.md`](research.md).

Fight on real city ground. Start with a piece of New York. The map is built from real building footprints and heights, street and sidewalk outlines and ground elevation. Our own scripts turn that into:
- the sim's bodies: every building is a body with a footprint, height, cover, hp and ruin;
- art: procedural buildings from our Blender kit, streets, sidewalks and street props.

It is played under the same rules as the village: Hollywood realism, physical fire and sharp fog of war.

**Done** means friends can play one city encounter on this Mac at no less than 30 FPS at the default camera. Buildings stop rounds and sight by their real heights, squads garrison them and fight from them, and they collapse to ruins under heavy fire. The map is recognisably the real place from above.

**Status:** draft, with no slices started. It starts **after battle-look closes**: slice 27, the review and close-spec.

## Next Agent Prompt

You are starting the `city-maps` spec in `/Users/david/dev/battlegame`. **Do not implement anything yet.**

1. Confirm battle-look is closed (`specs/done/battle-look/` exists). If not, stop and say so.
2. Read [`research.md`](research.md), the root README's "Rules from first principles" and "Hollywood realism", and [`AGENTS.md`](../../AGENTS.md).
3. Run the **write-spec interview** with the user, one question at a time, starting with the known unknowns below. Each has a recommended answer. Close by asking whether the plan must carry backward compatibility or migrations (the default is neither).
4. Then follow write-spec:
   - parallel drafts with distinct biases (at least: fewest slices, risk first, seam quality, and a perf lens);
   - synthesis;
   - a recursive fog audit;
   - refactor-clean over the plan.

   Then rewrite this README and create `slices/`, `choices.md` and `decisions.md`.
5. Slice C00 is a measurement spike. Nothing merges until its verdict exists.

## Goal

- A **map importer**: a deterministic offline tool that reads open city data and writes a `MapDefinition` fixture, plus a sources and attribution file. The same input bytes always give the same fixture bytes.
- **Buildings as compound bodies.** One building is a set of oriented boxes, from its footprint, with one integrity, one garrison and one ruin. Heights are real, so a 20-storey block hides what a 3-storey one doesn't.
- **Procedural city art, made by our own Blender scripts.** A facade and roof kit per archetype (NYC brick walk-up and brownstone, prewar loft, postwar slab, glass curtain wall), assembled onto each footprint by our code. Each building gets a ruin state and a far tier.
- **Streets as ground.** The road surface is roadbed polygons, with sidewalks, curbs and markings. Road speed comes from the surface, not from a polyline.
- **Street props as bodies:** parked cars and their wrecks, hydrants, lamp posts, bus shelters, newsstands, street trees, scaffolding and Jersey barriers. Each is a fixture row plus a Blender model.
- **Tall buildings in the rules:** fighting from upper floors (the eye height), and capacity by floor area. This is decided at the interview.
- **City scale holds up.** Sim tick, nav, fog (sim and renderer), publication bytes and frame cost stay within today's budgets for ~1,000–3,000 buildings on a 1.6 km map.

## Non-goals

- Photogrammetry or Google 3D Tiles: their terms forbid it (research.md).
- Any spiderbench asset or code (the licence table below).
- Interiors, interior mapping and room-by-room clearing.
- Civilians, moving traffic, day and night, weather. Night and rain belong to a later look spec.
- Underground: subway, basements, tunnels.
- Bridges across rivers, beyond today's bridge deck.
- Real landmark look-alikes (the Empire State, Chrysler and so on) and any real brand, logo, livery or plate. Generic art only.
- Multiple storeys as separate nav layers. Upper floors are a garrison rule, not walkable geometry.
- Cities other than New York until the NYC slice ladder is done (a firewall; the licence obligations differ).

## Spiderbench: what we reuse

**Verdict: nothing is reusable.** Its licence is "Source-Available, View-Only": no redistribution and no use "in any product, service, game or other distributed work" without written permission. On top of that, its stack (three.js, WebGL2) is incompatible with ours, its buildings aren't meshes (JS-generated, with shader-drawn facades), its city isn't real data (a hand-authored "Manhattan-style" island), and its Blender generator scripts (`tools/`) aren't published. We take **ideas** only, all of them public practice with older sources:
- lot subdivision plus per-district archetypes;
- facade detail in one shared material driven by per-building parameters (floor height, bay width, window style, material layer, tint), with distance-averaged windows;
- per-tile merged geometry with index sub-ranges per tile, one draw call per tile, and no BatchedMesh on ANGLE;
- a rooftop clutter kit placed in clusters by roof type;
- LOD budgets as a reference: cars at ~9.5k, ~1.5k and ~200 triangles, props at 40–3k.

| Asset family | Spiderbench source | Licence | Usable by us? | Flags |
|---|---|---|---|---|
| Buildings, facades, landmarks | JS generators (`buildings.js`, `facade.js`, `skyline.js`); no meshes | View-only, all rights reserved | **No**, ideas only | Landmark look-alikes (Empire State and others): trademark risk |
| Trees | JS generator (`trees.js`), leaf and bark textures | View-only | **No** | n/a |
| Cars, buses, taxis | `vehicles.glb`, 15 models × 3 LODs, from an unpublished Blender script | View-only | **No** | NYC taxi liveries and "plate_ny" atlas: trade dress and plates |
| Billboards and ads | `ts_ads.webp` and others, AI-generated ("Codex-generated") | View-only | **No** | Marvel IP (Daily Bugle, Oscorp); AI-art provenance |
| Street and roof props | `props.glb`, 20 meshes (dumpster, cone, barrier, subway entrance…) | View-only | **No** | n/a |
| Textures (walls, roofs, asphalt, sidewalk) | Baked by an unpublished script plus AI image generation | View-only | **No** | n/a |
| Map data (layout) | Hand-authored in `layout.js` | View-only | **No**, and it isn't real data anyway | n/a |
| Fonts | `public/assets/ui/fonts/` | SIL OFL 1.1 | Not needed; OFL isn't in our allow-list | n/a |

**What we build ourselves** (all `project-owned`, recorded in the reuse manifest with sha256 hashes, the way the village art is):
- the building kit: Blender scripts;
- textures: numpy recipes;
- street props and cars: generic makes, invented or no markings;
- the importer and the assembler.

**What we take from open data** (a new `fixtures/maps/<id>/SOURCES.json`, see below):
- NYC Building Footprints, Planimetrics and the 1 ft DEM: NYC Open Data, with no share-alike; credit the source, version and modifications;
- or USGS 3DEP: public domain.

## Pipeline

```
NYC Open Data (footprints + HEIGHT_ROOF, roadbed/sidewalk polygons)      pinned download, sha256 in SOURCES.json
USGS 3DEP / NYC DEM (ground)                                              ─┐
                    ▼                                                      │
tools/city-import   (offline, deterministic; EPSG:2263 → local metres, crop to map rect)
   ├─ footprint → simplify → decompose into ≤N oriented boxes (area error ≤ tolerance) → one building id
   ├─ HEIGHT_ROOF / floors → height_m, floors; FEATURE_CODE, year → archetype
   ├─ roadbed / sidewalk polygons → surface layer; DEM → height grid (Relief::Heightmap)
   └─ street-tree census / curb lines → props (trees, hydrants, parked cars…) by rules
                    ▼
fixtures/maps/<id>/map.json (MapDefinition)  +  SOURCES.json (licence, attribution, versions, hashes)
                    ▼                                     ▼
sim: bodies, nav, fog, garrison            renderer: building assembler (kit modules from our Blender
     (compound building = one id)                   scripts, baked as bundle v3) → per-building geometry
                                                    with LOD tiers, far tier, ruin; streets as a terrain layer
```

## Slice ladder (draft; the interview reslices it)

Every slice inherits battle-look's standing gates:
- replay and digest parity;
- presentation reads only the observation;
- one owner per concept, and hard cutovers;
- frame cost in a `frame-cost.md`;
- for visual slices: compare-screenshots against a reference crop, an unprimed screenshot-critique last, and a non-blocking preview-shots checkpoint.

Sim slices verify through slice 30's scenario runner (GIFs the agent reviews).

| # | Slice | API seam | Verifiable by |
|---|---|---|---|
| **C00** | **Spike: city at scale, in boxes.** Hand-convert one NYC crop to today's schema, with each building as one box. Measure what breaks. Throwaway: it never merges. | None; the output is a verdict in `spikes/C00.md` with numbers | Per-tick sim time, nav build, publication bytes, the renderer fog's occluder loop cost, benchmark frame cost at 500 / 1,500 / 3,000 buildings; scenario-runner GIFs of a squad crossing a block |
| C01 | **Map importer** | `city-import <source-dir> <rect> → map.json + SOURCES.json`; a pure function from pinned source bytes to fixture bytes | Golden fixture test (byte-stable); footprint→box decomposition within area and edge tolerance; a top-down PNG of footprints over boxes |
| C02 | **Contract: compound buildings, heightmap ground, surface polygons** | `PropDefinition` gains a `building` id (parts share integrity, garrison and ruin); `Relief::Heightmap`; `MapDefinition.surfaces` (roadbed and sidewalk polygons, replacing road polylines on city maps) | Native tests: a hit on one part damages the building; collapse swaps every part; nav reads the surfaces; digest and replay parity |
| C03 | **Sim at city scale** | Occlusion grid resolution (`fog_cell_m` per map); publication deltas for known props; bucket-grid sizing | A `city_report` example (like `village_report`): tick p50/p99 within Q12's soft target; publication bytes; scenario GIFs of sight down a street and blocked across a block |
| C04 | **Renderer fog at city scale** | The per-eye occluder set is culled by a spatial grid (or building tops are rasterised into the terrain march), so cost stops scaling with every known occluder | Paired `FOG_COST=1` run at C00's worst count; oracle agreement vectors unchanged; the fog critique question |
| C05 | **Building kit** (asset lane, parallel with C02–C04) | `packages/scene-assets/blender/city_kit.py`: facade bays, corners, ground-floor storefronts, cornices, roofs and rooftop clutter per archetype, each with `_LOD0..3`, plus a rubble and ruin kit; texture recipes (brick, limestone, curtain glass, tar roof) | `asset check`; workbench sheets per module; screenshot-critique |
| C06 | **Building assembler** | `buildingGeometry(footprintParts, height, floors, archetype, seed) → tiers + far tier + ruin`: deterministic, and cached per map (at bake time or at load, per the interview) | Byte-stable geometry test; a lab route `/lab/city-block` showing a real block at battle, ground and strategic cameras; benchmark row; compare-screenshots against an aerial reference crop (reference images we are allowed to keep) |
| C07 | **Streets as ground** | The terrain's surface layer reads `surfaces`: asphalt, sidewalk, curb, crossing markings, with scars on top | Lab frames; the "never reads as fog or shadow" critique question |
| C08 | **Street props as bodies** | New `PropKind` rows (`car`, `car_wreck`, `hydrant`, `lamp_post`, `bus_shelter`, `street_tree` via forest or trunk, `jersey_barrier`, `scaffold`); importer placement rules; Blender models | Scenario runner: a squad takes cover behind parked cars and a tank shoves a car aside; `asset check`; sheets |
| C09 | **Tall buildings in the rules** (tweak-mechanics first) | Garrison slots by floor band (eye height = the floor's height), capacity from floor area × floors (capped), collapse rules by height class | Native tests: a squad on floor 5 sees over a 3-storey building; scenario GIFs; a paired village report showing the village is unchanged |
| C10 | **Playable city encounter** | `fixtures/<city-encounter>.json` reusing the village's rules; a menu entry; attribution on screen and in the credits | 30 FPS benchmark at the default camera; whole-battle critique; friends play it |

Graph: C00 → C01 → C02 → {C03, C04, C07, C08} → C09 → C10, with C05 → C06 in parallel from C00. C06 needs C02's building parts.

## Known unknowns: the interview questions, each with a recommended answer

1. **Real map or "NYC-like" procedural?** *Recommend* real footprints and heights from NYC Open Data, with procedural facades. Realism comes cheaply from the data, and no licence risk comes with it.
2. **Which piece, and how big?** *Recommend* the same 1.6 × 1.6 km as the village, which keeps the nav and fog costs known. Start with a mixed low and mid-rise district, where streets and heights make the tactics interesting and garrisons stay meaningful: for example Greenpoint / Long Island City, or the Lower East Side. Midtown's supertalls come second.
3. **Fighting from upper floors?** *Recommend* yes, as a garrison rule. Floor bands give eye and muzzle height, capacity comes from floor area, and there are no walkable storeys. A squad high up sees over lower roofs, which is Hollywood-true and cheap.
4. **What happens to tall buildings under fire?** *Recommend* height classes. Low-rise buildings (up to ~6 floors) collapse to ruins as today. Mid and high-rise ones lose floors or facade, get a burnt look and lose cover tier, but never fully fall to ordinary fire. That's what a war film shows.
5. **Other cities, and the ODbL?** *Recommend* NYC first on NYC Open Data, which has no share-alike. Other cities later use OSM or Overture: the derived `fixtures/maps/<city>/` is published under ODbL in its own folder, with "© OpenStreetMap contributors" in the credits. Accept that obligation explicitly before C11+.
6. **Map data provenance record.** *Recommend* a `fixtures/maps/<id>/SOURCES.json` with the dataset URL, version and date, licence, attribution text, the modifications made and the sha256 of the pinned download, checked by a test, like the reuse manifest does for art. The allow-list for data is public domain, NYC Open Data terms and, if Q5 accepts it, ODbL.
7. **Where building geometry is made.** *Recommend* that Blender makes the **kit modules** (bundle v3, validated as usual) and a deterministic TS assembler composes each building at map bake. Per-building Blender exports would mean thousands of bundles. C00 or C06 measures this.
8. **Fog cell size.** *Recommend* letting the map choose `fog_cell_m`. Try 4 m on city maps and measure it in C00. At 8 m, cells straddle 18 m side streets.
9. **Street furniture density and which kinds are bodies.** *Recommend* bodies only for what gives cover or blocks: cars, barriers, shelters, trees, scaffolds. Lamp posts and hydrants are thin and don't occlude, so they stop rounds but aren't cover.
10. **Spiderbench.** *Recommend* not asking the author for permission. There's little we'd take even if we could, because the meshes are three.js-era vertex-coloured work and the generators aren't published.
11. **Landmarks and brands.** *Recommend* none: generic buildings on real footprints, with invented or no signage.

## Firewalls (out of scope)

- Spiderbench assets and code: view-only licence.
- Google 3D Tiles and any photogrammetry: their terms forbid it.
- Interiors, walkable upper floors, underground.
- Civilians and traffic simulation.
- Night, weather and seasons: a later look spec on the same data.
- Cities other than NYC until C10 closes, and any ODbL data until Q5 is accepted.
- A performance budget beyond the 30 FPS floor.
- Changing the village map, apart from shared schema cutovers (C02). The village fixture migrates in the same slice.
