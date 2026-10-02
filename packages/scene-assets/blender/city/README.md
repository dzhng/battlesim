# City buildings: kits and templates

A town's buildings are drawn as instances of a few shared **modules** (a window bay, a balcony, a roof, a wall shell), never as one mesh per building. The scripts here make the modules and say where each one goes on each building **template**. Maps reach 16,000 buildings; a kit is a few dozen modules.

Blender runs here and nowhere else: these scripts are an explicit source step, run by hand through `asset blender`, and their output is committed (Git LFS) under `assets/source/city/`. The asset bake, the asset check, the map generator and the game never start Blender.

## What a source set is

One script run writes one **set**, a folder `assets/source/city/<set>/` holding two files:

- **`kit.glb`**: the set's modules.
- **`templates.json`**: the set's templates: each one's physical shape and, per state, the rows that place modules on it.

A set is one source's work: the China graph's apartment blocks, our own houses, our own towers. Several sets make up one regional family; a template says which family it belongs to.

### `kit.glb`

- A **module** is a root-level empty named by the module's id (`[a-z0-9_]+`). The meshes under it are its geometry, with the usual `_LOD0`..`_LOD3` name suffix for the four detail tiers, finest first; a mesh with no suffix is in every tier. Every module has geometry in all four tiers, never more triangles in a coarser one.
- Every root-level object is a module's empty. A mesh at the root is refused.
- Geometry is in the module's own frame: metres, Z up, the origin wherever its rows expect it (a window's sill centre on the wall plane, a shell's footprint centre on the ground). The frame is the empty's, so a script may lay its modules out side by side in the file; where an empty stands is not content.
- Materials, textures, UVs, tangents and vertex colour follow the conventions every static source follows ([scene-assets readme](../../README.md), "Textures"): three texture channels at most 1024 px, UVs in metres, vertex-colour alpha is wear and nothing else. A surface that takes a building's own tint (a wall colour) is tint-masked (`tint` in the material's extras, or the ORM texture's alpha).
- A mesh made for one template only (its wall shell, its roof) is a module like any other, named for its template.

### `templates.json`

```json
{
  "set": "china_apartments",
  "kit": "city_kit_china_apartments",
  "fit": { "side_m": 1.6, "top_m": 4.5 },
  "source": { "script": "china.py", "blend": "vendor/procedural-buildings/CN_ApartmentBuilding.blend", "blender": "5.2.1" },
  "modules": ["window_a", "balcony_enclosed", "slab_35x11_4f_shell"],
  "templates": [
    {
      "status": "release",
      "recipe": { "Width": 35, "Depth": 11, "Floors": 4 },
      "descriptor": { "id": "china-apartment-slab-35x11-4f", "category": "urban_apartment", "regional_family": "china", "parts": [], "floor_heights_m": [], "entrances": [], "edges": [], "joins": [] },
      "states": {
        "intact": [[2, 0, 0, 0, 0, 1, 1, 1, 15, 255, 255, 255]]
      }
    }
  ]
}
```

- **`set`** and **`kit`** are the names `assets/catalog.json` lists the set under: its key in `city_sets`, and the kit's appearance (`unit: "kit"`).
- **`modules`** lists each kit module the rows use, once, by id. Every one must be in the kit.
- **`descriptor`** is the physical template, exactly the contract's `BuildingTemplateDescriptor` (`crates/contract/src/templates.rs`): oriented boxes for parts, floor datums, entrances, facade edges with their bay lattice, supported joins. It is what the map generator places and the simulation builds, and **the art is made to it, never the other way round**: every wall stands on a face of a part, so what hides a unit in the simulation hides it on screen. A descriptor must pass `require_complete`.
- **The template's frame** is the descriptor's: metres, Z up, the ground at z = 0, the origin at the footprint's centre, the first entrance on the street side.
- **`states`** holds the rows for each state a side can know a building in. `intact` is required; `ruin` (a collapsed building, 6 floors or fewer) and `gutted` (a burnt shell that still stands, taller) arrive with the damage pass.
- **A row** is twelve numbers: `[module, x, y, z, yaw, sx, sy, sz, tiers, r, g, b]`.
  - `module` indexes `modules`.
  - The module's geometry is scaled per axis by `(sx, sy, sz)`, turned by `yaw` radians about +Z, then moved to `(x, y, z)`. That is all a row can say. A tilt, a mirror or any other transform is baked into a module variant of its own, so every scale is positive.
  - `tiers` is a bit per detail tier the row draws at (`1` is tier 0 … `8` is tier 3; `15` is all four, `0` is not a row). Small things drop out of the coarse tiers; a template's coarsest tier is little more than its shell.
  - `r, g, b` (whole numbers 0 to 255, sRGB) tint the row's tint-masked surfaces; `255, 255, 255` leaves them as authored.
- **`status`** is `release` for accepted art and `prototype` for a labelled stand-in. A stand-in never counts as coverage.
- **`fit`** is how far this set's art may reach past a part's faces: `side_m` for balconies, cornices and awnings, `top_m` for roof furniture above the part's top. The asset check holds every vertex of every state to it, at each tier its row draws at: inside some part grown by `side_m` on its four sides and `top_m` above. Nothing reaches below a part's base.
- **`recipe`** and **`source`** record what made the template: the inputs, the script, the file and the Blender version. Nothing reads them.

## From a set to a town

A new set is listed in `assets/catalog.json`: its kit as an appearance, and the set under `city_sets`. Then, with the `asset` CLI:

1. `catalogue` copies the sets' descriptors into the physical catalogue the map generator reads. A template belongs to exactly one set, and a set's descriptor and its catalogue row are the same template. The catalogue's hash moves, and what pins it moves in the same commit.
2. `prototypes` regenerates the stand-in set for whatever templates still have no art.
3. `bake`, then `check`. They refuse a set whose descriptor the simulation's contract refuses, whose art leaves its parts, whose rows name a module its kit lacks, or that leaves a catalogue template undressed. What the bake does with a set is in the [scene-assets readme](../../README.md), "City buildings".

## Rules a set keeps

- **The same inputs write the same bytes.** Seeds are fixed, iteration is in a sorted order, and nothing reads the clock.
- **Sizes are whole bays and whole floors.** A facade's windows sit on the descriptor's bay lattice (3 m pitch) and its floors on `floor_heights_m`. A graph-made side is 3n + 2 metres long ([S2](../../../../specs/city-maps/spikes/S2.md)).
- **An exposed edge has a facade; an edge that is not exposed has none.** No windows on a party wall or an interior join.
- **Detail is budgeted per template**, in triangles drawn at each tier: 150,000 at tier 0, 50,000 at tier 1, 12,000 at tier 2 and 2,000 at tier 3. The script prints what each template draws. A far building is one row: at the coarse tiers a script folds what is left of its modules into the template's own shell.
- **Nothing glows.** Emission is zero; interiors are unlit.
- **No real names.** Sign text is a generic word for a trade (tea, pharmacy, hotel) or comes from the project's own invented-name list; no brand, logo, place or landmark.
- **No street.** Sidewalks, street trees, lamps and props are not part of a building.

## What is here

- [`china.py`](china.py) exports the China apartment set from the vendored graph: which of the graph's instances are a building of ours, the materials, the tiers and the five recipes are its tables.
- [`graph.py`](graph.py) reads a geometry-nodes building before it is realized: its instances with their transforms and tints, and the mesh it generated for the recipe. Every graph source starts here.
- [`detail.py`](detail.py) makes a kit mesh's coarser tiers by one rule, the smallest feature a tier keeps. It calls no Blender operator, so its output is the same bytes every run.
- [`ambientcg.py`](ambientcg.py) bakes a pinned ambientCG set (`../packs.py`) into a texture recipe at the size every texture in the game has.
- [`assemble.py`](assemble.py) puts a set back together in Blender from its two files and renders it at the game's camera with the part boxes drawn over it: the picture to judge a set by until the renderer draws kits.
