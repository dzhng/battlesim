# Research (draft, 2026-09-27)

The findings behind [`README.md`](README.md). The external sources are linked; statements about our own code name the file.

## Spiderbench (github.com/xikhar/spiderbench)

**What it is.** A browser web-swinging game ("Spiderbench — a browser web-swinging game written by Claude"), made by one person steering Claude Code, as a benchmark of AI-written game code. The [announcing tweet](https://x.com/xikhar/status/2104001664793600012) says: "Third iteration with Opus 5.5 medium … It turned blender, image-gen, and three.js into this beauty, which runs on your browser." x.com returned HTTP 402 to a direct fetch, so the tweet text came from the fxtwitter mirror API.

Read at commit `e0940d3` (a single squashed commit, "Update", 2026-09-28 +0530), shallow-cloned to the session scratchpad. It has no Git LFS. The checkout is 268 MB and has 216 files. The largest files are `suit_normal.png` (19 MB), `people.bin` (10.4 MB), `spiderman.glb` (10.3 MB) and `city/vehicles.glb` (8.4 MB), with 1–7.7 MB city textures.

**Stack.**
- three.js 0.186 on WebGL2, with `postprocessing` and `n8ao`, built by Vite;
- plain JavaScript, with no TypeScript and no tests;
- cascaded shadows, SSGI, AO, SSR, TAA, motion blur and bloom.

None of that is portable to our TypeGPU/WebGPU renderer (the renderer skill forbids three.js).

**How it builds the city.** The whole city is procedural, generated in the browser at load with a fixed seed. It uses no real-world data:
- `src/world/layout.js` hand-authors a "Manhattan-style" island: shoreline control points, avenue x positions and an 80 m street pitch, with named wide and narrow streets, Broadway diagonals and a park. There is no OSM, no NYC Open Data, no photogrammetry and no Google 3D tiles; grep finds no geodata reader.
- `buildings.js` (240 kB) subdivides each block into lots and picks an NYC archetype per lot (tower, glass curtain wall, deco, postwar slab, loft, walk-up, brownstone), using district weights and hashed per-lot random streams. It emits axis-aligned boxes, n-gon prisms and setbacks.
- The same `solid*()` call writes the render triangles and the matching collision primitive (`collision.js`), so what you see is what you collide with.
- The facade detail is shader work, not geometry (`facade.js`). Per-vertex attributes carry floor height, bay width, window size, style, material layer and tint. The fragment shader draws windows, parallax reveals, **interior mapping** (fake rooms from an interiors atlas), storefronts, belt courses and rain streaks, with distance anti-aliasing to the average colour. One material draws a whole 256 m tile in one call.
- Landmarks are hand-coded look-alikes: Empire-State-like, Chrysler-like, 432-Park-like, One-WTC-like, Grand Central, Times Square (`skyline.js`, `landmarks.js`, `timessq.js`, `grandcentral.js`).
- Rooftop clutter (`rooftops.js`) is a vertex-coloured kit merged per tile, placed in clusters by roof type: HVAC, water tanks, bulkheads, dishes.
- Street furniture (`props.js`) is procedural low-poly JS geometry, drawn through distance-culled instancing pools with dithered LOD cross-fades (`pool.js`).
- Trees (`trees.js`, `treetrunk.js`) are procedural branching trunks with alpha-tested leaf-card clumps, in two LODs.
- Performance tricks: 256 m tiles merged into 512 m super-tiles, each tile drawing its own index sub-range of shared buffers (`tilebatch.js`). THREE.BatchedMesh was tried and dropped: it stalled Chrome/ANGLE. Facade details fade out at 320–450 m, and a typed-array geometry store keeps a ~2 M-vertex build under the tab's heap limit.

**Its meshes (GLB, exported by Blender 5.2's glTF exporter, no embedded images or materials: vertex colour plus shared atlases).** I parsed these with a script:

| File | Contents | Triangles |
|---|---|---|
| `city/vehicles.glb` (8.4 MB) | 15 cars: sedan ×2, SUV ×2, hatch, crossover, pickup, van, truck, bus, tour bus, and four taxi liveries (`taxi`, `taxi_gr`, `taxi_hy`, `taxi_mv`). Each has LOD0/`_l1`/`_l2`. | LOD0 ~9–10k, l1 ~1.3–2.8k, l2 ~50–230 |
| `city/props.glb` (0.8 MB) | 20 props: antenna, barrier, bike, bike kiosk, billboard, cone, dish, dock, drum, dumpster, fence, garden, HVAC, payphone, roll-off, shed, sign pole, subway entrance and pit, vents | 40–3,100 |
| `spiderman.glb`, `thug.glb` | characters | n/a |

Textures run 512²–4096², and some are strips: `walls_col.jpg` is 1024×16384, a stack of 16 wall layers. They are a mix of PNG, JPG and WebP. They were made by `tools/blender/city_textures.py` and "AI-generated" image tools (`tools/imagegen/...`). **The `tools/` directory isn't published**: `src` references `tools/blender/city_props.py`, `city_vehicles.py`, `city_textures.py`, `city_npc.py` and `imagegen/signage/pack_ads.py`, but none of them is in the repo. So the Blender generators we'd most want aren't available even to read.

**Ads and brands.** The billboard art is "Codex-generated photographic ads" (`ts_ads_meta.js`), packed into a 4096² atlas. The README says the advertised brands are invented, "apart from in-universe Marvel names such as the Daily Bugle and Oscorp" (both appear in `src`). The taxis carry NYC-style liveries and a "plate_ny" atlas cell.

## Spiderbench licence

`LICENSE` is a **"Source-Available, View-Only License"**, © 2026 Shikhar, all rights reserved. It permits viewing, and running locally for personal non-commercial evaluation. **Without written permission, it does not permit:**
- any commercial use;
- redistribution, or publishing any part of it, modified or not;
- using it "in any product, service, game or other distributed work".

The only exceptions are the fonts in `public/assets/ui/fonts/`, under SIL OFL 1.1, and npm dependencies under their own licences. On top of that, the README disclaims Marvel/Sony/Disney IP.

Consequence: **no mesh, texture, shader, audio, layout data or code from spiderbench may enter this repo**, under any reuse-manifest mode (`copy`, `adapted` or `technique`). The OFL fonts are outside our allow-list (`packages/scene-assets/src/schema.ts` `ALLOWED_LICENCES = CC0-1.0, MIT, project-owned`), and we don't need them. Ideas aren't copyrightable, and the techniques it uses are all published practice with older public sources:
- interior mapping: van Dongen, 2008;
- lot subdivision and archetypes: Parish & Müller, "Procedural Modeling of Cities", SIGGRAPH 2001;
- tile batching.

We can implement those from the public literature. We should not work from a line-by-line reading of its files.

## Real-world city data

| Source | What | Licence and obligations |
|---|---|---|
| [NYC Building Footprints](https://data.cityofnewyork.us/City-Government/Building-Footprints/5zhs-2jue) ([metadata](https://github.com/CityOfNewYork/nyc-geo-metadata/blob/main/Metadata/Metadata_BuildingFootprints.md)) | Every building polygon, with `HEIGHT_ROOF` (above ground), `GROUND_ELEVATION`, `CONSTRUCTION_YEAR`, `FEATURE_CODE`, `BIN` and `BASE_BBL`. EPSG:2263, in US feet. Photogrammetric features are ±2 ft. Updated daily. | NYC Open Data. The FAQ says there are "no restrictions on the use of Open Data", but use is under NYC.gov's [terms of use](https://www.nyc.gov/main/terms-of-use), and the city "may require" users who republish or build it into an application to "identify the source, version, and modifications" ([Technical Standards Manual](https://cityofnewyork.github.io/opendatatsm/publicpolicies.html)). There is no share-alike. Plan to credit the source, dataset version and date, and say it was modified. |
| [NYC Planimetric: Roadbed](https://data.cityofnewyork.us/Transportation/NYC-Planimetric-Database-Roadbed/i36f-5ih7/about_data), [Sidewalk](https://data.cityofnewyork.us/City-Government/NYC-Planimetric-Database-Sidewalk/vfx9-tbb6) ([capture rules](https://github.com/CityOfNewYork/nyc-planimetrics/blob/main/Capture_Rules.md)) | Roadbed, sidewalk, median, park and hydro polygons | NYC Open Data, as above |
| [NYC 1 ft DEM](https://data.cityofnewyork.us/City-Government/1-foot-Digital-Elevation-Model-DEM-/dpc8-z3jc) (2010 LiDAR); [2017 topobathymetric LiDAR](https://data.cityofnewyork.us/City-Government/Topobathymetric-LiDAR-Data-2017-/7sc8-jtbz) | A bare-earth, hydro-flattened ground surface | NYC Open Data, as above |
| [USGS 3DEP 1 m DEM](https://www.sciencebase.gov/catalog/item/543e6b86e4b0fd76af69cf4c) | Bare-earth elevation for all of the US | US public domain; a citation is requested, not required |
| [OpenStreetMap](https://www.openstreetmap.org/copyright) via [Overpass](https://wiki.openstreetmap.org/wiki/Overpass_API) or Geofabrik extracts | Footprints with `height`, `building:levels`, `min_height`, `roof:shape` and `building:part` ([Simple 3D Buildings](https://wiki.openstreetmap.org/wiki/Simple_3D_Buildings)); roads; landuse; trees | **ODbL 1.0.** Attribution: "© OpenStreetMap contributors", linked to /copyright. **Share-alike:** a derived database (our map fixture JSON is one) may be distributed only under ODbL. The game, as a [Produced Work](https://osmfoundation.org/wiki/Licence/Community_Guidelines/Produced_Work_-_Guideline), can carry any licence, but if it is published, the derived database "has to be published as well" (ODbL §4.6), or offered. Heights are often missing, so the fallback is levels × a floor height. Overpass's public instance asks for fewer than 10k queries and under 1 GB a day, and heavy use should go to extracts. |
| [Overture Maps](https://docs.overturemaps.org/attribution/) buildings and transportation | OSM, conflated with Microsoft, Esri and others | ODbL, with OSM attribution, plus CC BY 4.0 components. The same obligations as OSM. |
| [Google Photorealistic 3D Tiles](https://developers.google.com/maps/documentation/tile/policies) | Photogrammetry meshes | **Not usable.** Offline use and caching are prohibited, as are objects "extracted, traced, or otherwise derived … from Photorealistic 3D Tiles". Visualisation only, with Google attribution. |

The recommendation for NYC is NYC Open Data plus the NYC DEM or USGS 3DEP. Both are free of share-alike, and the NYC data has measured roof heights for every building, which OSM often lacks. For other cities, OSM or Overture brings the ODbL obligations above, so keep that derived data in its own `fixtures/maps/<city>/` folder under ODbL, apart from the MIT/CC0 art.

**Architecture and trademarks.** In US copyright law, 17 U.S.C. §120(a) lets anyone make pictorial representations of a building that is visible from a public place. But some landmark owners assert trademark rights in their building's image; the Empire State Building is the well-known case. So landmark look-alikes are a legal-review question, not a technical one. A generic extrusion of a real footprint is low risk.

## Our stack: what a city stresses

These are facts from the code, read 2026-09-27.

- **Map schema** (`crates/contract/src/map.rs`). The map is a fixed-size rectangle (the village is 1600×1600 m) with a 4 m height grid built from additive `Relief` (ridge, mesa), plus water rects, road polylines with a width, bridges, forest rects and props. **Each prop is one oriented box** (`center`, `yaw`, `half_extents`, `base_z`). `PropKind` is a closed Rust enum of 14 kinds, so a new body kind is an enum row plus a fixture row.
- **Body table** (`fixtures/village.json` `props`). Each row sets `blocks.{infantry,vehicle}`, `stops_rounds`, `occludes`, `weight_class`, `cover_tier`, `conceals`, `hp`, `armor` and `destroyed` (`removed`, `cleared`, or `{into: {kind, height_m}}`). A building is 400 hp and collapses into a 2 m ruin. Today's map has 61 props.
- **Garrison** (`crates/sim/src/garrison.rs`). A building is an abstract fighting position with a flat `capacity_soldiers: 16`. Soldiers stand at **perimeter slots at ground level** just outside the facade, and a collapse leaves a ruin. Upper floors don't exist.
- **Sim fog** (`crates/sim/src/visibility.rs`). Radial sweeps over an **8 m `OcclusionGrid` of prop tops** (`sensors.fog_cell_m`). The cost scales with the grid, not with the prop count, and heights are honoured, so tall buildings occlude properly. At 8 m, though, a cell straddling an 18 m side street's building line can block the street's own sight line. The cell size is a question for a city.
- **Renderer fog** (`packages/battle-renderer/src/frame/fogVisibility.ts`). One horizon map per eye: a terrain march, then a merge that **loops every known occluder for every azimuth bin** (`for i < P.occluderCount`). That is O(eyes × azimuth × occluders). It is fine at 61 props but must change for thousands of city boxes: a per-eye bucketed cull, or building tops rasterised into the terrain march.
- **Navigation** (`crates/sim/src/navigation.rs`). A 2 m grid with 0.5 m infantry sub-cells and a clearance field per push class. Its cost is by area, so a 1.6 km city costs the same as the village, and it grows with the square of the map side.
- **Props** have a uniform XY bucket grid for raycasts (`world/props.rs`), and every prop's integrity enters the digest. Known props are per side and publish through the observation, so publication bytes scale with known props.
- **Renderer buildings** (`models/propAppearance.ts`). Each building is one of three house GLBs (`house_a..c`, each with a `_ruin`), chosen by box size and fitted to it. Buildings are `MAP_ONLY`. Walls and fences are `MODULAR`, repeating a module along the box. There is no per-building procedural geometry.
- **Asset pipeline** (`packages/scene-assets/README.md`, the renderer skill's `references/procedural-assets.md`). Headless Blender scripts produce GLBs with `_LOD0..3` tiers. The bake produces a content-addressed bundle v3 with 256 px albedo, normal and ORM textures, 1024 px at most. The validator checks provenance against the reuse manifest's allow-list.
- **Performance.** The village frame has headroom: about 8.3 ms p50, 2–4 ms GPU. Models are "not measurable" at battle scale thanks to projected-pixel tiers and impostor cards. Tree impostors were rejected for shimmering under 4× MSAA.

## Other prior art worth reading (ideas only)

- Parish & Müller, [Procedural Modeling of Cities](https://cgl.ethz.ch/Downloads/Publications/Papers/2001/p_Par01.pdf) (SIGGRAPH 2001): lot subdivision and facade grammars.
- van Dongen, [Interior Mapping](https://www.proun-game.com/Oogst3D/CODING/InteriorMapping/InteriorMapping.pdf) (CGI 2008). It is probably wasted at RTS range; an emissive window atlas is enough.
- [OSM2World](https://osm2world.org/) and [Blender-OSM / blosm](https://github.com/vvoovv/blosm): OSM to 3D, with footprint-to-roof shapes. Both are GPL, so read their docs for ideas and never their code.
