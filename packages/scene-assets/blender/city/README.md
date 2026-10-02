# City buildings: kits and templates

A town's buildings are drawn as instances of a few shared **modules** (a window bay, a balcony, a roof, a wall shell), never as one mesh per building. The scripts here make the modules and say where each one goes on each building **template**. Maps reach 16,000 buildings; a kit is a few dozen modules. Every building of every map is drawn this way, the hand-authored maps' few houses included.

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
- Materials, textures, UVs, tangents and vertex colour follow the conventions every static source follows ([scene-assets readme](../../README.md), "Textures"): three texture channels at most 1024 px, UVs in metres, vertex-colour alpha is wear and nothing else, and a surface that is not opaque says so as its material's coverage. A surface that takes a building's own tint (a wall colour) is tint-masked (`tint` in the material's extras, or the ORM texture's alpha).
- A mesh made for one template only (its wall shell, its roof) is a module like any other, named for its template.

### `templates.json`

```json
{
  "set": "china_apartments",
  "kit": "city_kit_china_apartments",
  "fit": { "side_m": 1.6, "top_m": 4.5, "ruin_top_m": 0.6 },
  "source": {
    "script": "china.py",
    "blend": "vendor/procedural-buildings/CN_ApartmentBuilding.blend",
    "blender": "5.2.1"
  },
  "modules": ["window_a", "balcony_enclosed", "slab_35x11_4f_shell"],
  "templates": [
    {
      "status": "release",
      "recipe": { "Width": 35, "Depth": 11, "Floors": 4 },
      "descriptor": {
        "id": "china-apartment-slab-35x11-4f",
        "category": "urban_apartment",
        "regional_family": "china",
        "parts": [],
        "floor_heights_m": [],
        "entrances": [],
        "edges": [],
        "joins": []
      },
      "states": {
        "intact": [[2, 0, 0, 0, 0, 1, 1, 1, 15, 255, 255, 255]],
        "ruin": [[3, 0, 0, 0, 0, 1, 1, 1, 15, 255, 255, 255]]
      }
    }
  ]
}
```

- **`set`** and **`kit`** are the names `assets/catalog.json` lists the set under: its key in `city_sets`, and the kit's appearance (`unit: "kit"`). The catalog also says which physical catalogue the set's templates are rows of (`catalogue`): `generated`, what the map generator builds towns from, or `authored`, the boxes the hand-authored maps pin.
- **`modules`** lists each kit module the rows use, once, by id. Every one must be in the kit.
- **`descriptor`** is the physical template, exactly the contract's `BuildingTemplateDescriptor` (`crates/contract/src/templates.rs`): oriented boxes for parts, floor datums, entrances, facade edges with their bay lattice, supported joins. It is what the map generator places and the simulation builds, and **the art is made to it, never the other way round**: every wall stands on a face of a part, so what hides a unit in the simulation hides it on screen. A descriptor of the generator's catalogue must pass `require_complete`; one of the authored catalogue is a solid box with no floor, door or bay resolved, and must pass `validate`.
- **The template's frame** is the descriptor's: metres, Z up, the ground at z = 0, the origin at the footprint's centre, the first entrance on the street side.
- **`states`** holds the rows for each state a side can know a building in: `intact`, and the one state the simulation destroys it into. A building of 6 floors or fewer collapses, and has `ruin`; a taller one stands as a burnt shell, and has `gutted`. A template has exactly its own of the two. Which, and how tall a collapse's remains are, is the building prop type's rule, read from the catalog's resolved view (`fixtures/catalog.json`) by [`collapse.py`](collapse.py) here and through the unit catalog by the bake; nothing copies its numbers.
- **A row** is twelve numbers: `[module, x, y, z, yaw, sx, sy, sz, tiers, r, g, b]`.
  - `module` indexes `modules`.
  - The module's geometry is scaled per axis by `(sx, sy, sz)`, turned by `yaw` radians about +Z, then moved to `(x, y, z)`. That is all a row can say. A tilt, a mirror or any other transform is baked into a module variant of its own, so every scale is positive.
  - `tiers` is a bit per detail tier the row draws at (`1` is tier 0 … `8` is tier 3; `15` is all four, `0` is not a row). Small things drop out of the coarse tiers; a template's coarsest tier is little more than its shell. A state has a row at every tier: the game draws a block of a town at one tier and the whole map at the coarsest, so a state with none there would vanish with distance.
  - `r, g, b` (whole numbers 0 to 255, sRGB) tint the row's tint-masked surfaces; `255, 255, 255` leaves them as authored.
- **`status`** is `release` for accepted art and `prototype` for a labelled stand-in. A stand-in never counts as coverage.
- **`fit`** is how far this set's art may reach past a part's faces: `side_m` for balconies, cornices and awnings, `top_m` for roof furniture above the part's top. The asset check holds every vertex of every state to it, at each tier its row draws at: inside some part grown by `side_m` on its four sides and `top_m` above. Nothing reaches below a part's base.
  - **A ruin is held to the remains, not the parts.** A collapse replaces every part by a box on the same plan, all of one height: a quarter of the building's height (its tallest part's top), never under 2 m nor over 6 m. A `ruin` state's rows fit those boxes, grown by `side_m` on the sides and by `ruin_top_m` above (the jagged tops of broken walls; 0 when a set does not say). The remains are what stops rounds and gives cover, so the art fills them and does not stand over them.
  - A `gutted` state fits the standing parts, as `intact` does.
- **`recipe`** and **`source`** record what made the template: the inputs, the script, the file and the Blender version. Nothing reads them.

## From a set to a town

A new set is listed in `assets/catalog.json`: its kit as an appearance, and the set under `city_sets` with the catalogue it dresses. Then, with the `asset` CLI:

1. `catalogue` copies the `generated` sets' descriptors into the physical catalogue the map generator reads. A template belongs to exactly one set, and a set's descriptor and its catalogue row are the same template. The catalogue's hash moves, and what pins it moves in the same commit.
2. `prototypes` regenerates the stand-in set for whatever templates still have no art.
3. `bake`, then `check`. They refuse a set whose descriptor the simulation's contract refuses, whose art leaves its parts, whose rows name a module its kit lacks, or that leaves a template of its catalogue undressed. What the bake does with a set is in the [scene-assets readme](../../README.md), "City buildings".

The authored catalogue (`fixtures/building-templates.json`) runs the other way: its rows are written by hand, its hash is its maps' identity, and nothing derives it. Its set copies each row as its descriptor (`kit.dress`), so a new authored box needs only the set's script run again, then `bake` and `check`.

**The picture a set is judged by** is the line-up lab, `/lab/city-lineup`: every template on flat ground, drawn as the game draws a town, at any one tier, state and station (its route file lists the address parameters). `CITY_SET=<set> bun run --cwd web scene -- city-lineup` holds each template to its parts and fit on screen and writes, under `throwaway/evidence/city-lineup/`, a sheet per category at each tier and each template across each tier boundary (the scene file lists what else narrows a run).

## Rules a set keeps

- **The same inputs write the same bytes.** Seeds are fixed, iteration is in a sorted order, and nothing reads the clock.
- **Sizes are whole bays and whole floors.** A facade's windows sit on the descriptor's bay lattice (3 m pitch) and its floors on `floor_heights_m`. A graph-made side is 3n + 2 metres long ([S2](../../../../specs/city-maps/spikes/S2.md)).
- **An exposed edge has a facade; an edge that is not exposed has none.** No windows on a party wall or an interior join.
- **Detail is budgeted per template**, in triangles drawn at each tier: 150,000 at tier 0, 50,000 at tier 1, 12,000 at tier 2 and 2,000 at tier 3. The script prints what each template draws. A far building is one row: at the coarse tiers a script folds what is left of its modules into the template's own shell.
- **A destroyed building is the same building.** Its damage state is made on the same plan, in the same materials and tints, in the same frame, and by the same tier rule: shared wreckage (heaps of rubble, beams, burnt panels) is rows at the fine tiers, and the state's own shell is the whole of it at the coarse ones. It draws no more triangles than `intact` at any tier. Soot and breakage that must read from across the map are in the shell's own texture or vertex colour, never only in a fine tier's modules.
- **The game picks a tier by pixels to the metre** (`presentation.buildings.lod_px_per_m`): at the battle's camera tier 0 reaches about 130 m, tier 1 about 320 m and tier 2 about a kilometre, whatever the building's height. A tower is at tier 1 or coarser in most frames, so its far tiers carry its character.
- **Nothing glows.** Emission is zero; interiors are unlit.
- **No real names.** Sign text is a generic word for a trade (tea, pharmacy, hotel) or comes from the project's own invented-name list; no brand, logo, place or landmark.
- **No street.** Sidewalks, street trees, lamps and props are not part of a building.

## What is here

- [`china.py`](china.py) exports the China apartment set from the vendored graph: which of the graph's instances are a building of ours, the materials, the tiers and the templates are its tables. A template is boxes that abut; only the outline of their union is built, one graph facade to each straight run, so a slab, a U and a closed court come from one rule and a join has nothing to hide ([S5](../../../../specs/city-maps/spikes/S5.md)).
- [`graph.py`](graph.py) reads a geometry-nodes building before it is realized: its instances with their transforms and tints, and the mesh it generated for the recipe. Every graph source starts here.
- [`detail.py`](detail.py) makes a kit mesh's coarser tiers by one rule, the smallest feature a tier keeps. It calls no Blender operator, so its output is the same bytes every run.
- [`ambientcg.py`](ambientcg.py) bakes a pinned ambientCG set (`../packs.py`) into a texture recipe at the size every texture in the game has.
- [`towers.py`](towers.py) models our own tower blocks: panel modules one bay wide and one floor high, placed by a row a bay; each template's shell carries the same grid as a texture from tier 1 out, and is the whole tower at tiers 2 and 3. Gutted, a tower is the same shell built in burnt facade recipes (empty openings, the soot of each one's fire up the wall over it, bays blown out to the floor slabs), and each bay shows the same quarter of the recipe at every tier. [`tower_sheets.py`](tower_sheets.py) photographs them, alone and among the other sets.
- [`kit.py`](kit.py) is the authoring helper of a hand-scripted set (modules, templates, edges, bays, rows, states, the two files), with [`homes.py`](homes.py), the houses, as its worked example and [`farmsteads.py`](farmsteads.py), the farms, as the one with several buildings to a template. `fold_far` keeps a template's fittings in its shell at the coarse tiers, as flat panels and boxes where the rows had them. It also holds what masonry ruins share: `ruin_block` (a fallen box: ragged stumps of its walls with their openings broken to the sills, the heap inside them, its roof slipped over the heap, its far tiers), `ruin_sides` (which walls stood and where they were pierced, read from the intact rows) and `wreckage` and `litter` (the shared heaps and beams, and the rows that strew them). The broken wall, the heap and the burnt materials themselves are `../masonry.py`'s (`ragged_wall`, `rubble_fill`, `scorched`), which the village farmhouse's ruin uses too.
- [`collapse.py`](collapse.py) reads the simulation's rule for a destroyed building: which damage state a template has, and how tall its remains are.
- [`village.py`](village.py) dresses the authored maps' catalogue: the courtyard farm of `../house.py` built on each box at its own size, one module standing and one fallen. Each of its three looks is built in full on the village house it belongs to; any other box borrows the nearest house's look a tier coarser, so the twelve farms fit a kit's byte budget.
- [`industry.py`](industry.py) models the industrial set through `kit.py`: five buildings, each a shell of its own and rows of shared bay-wide modules, folded into the shell at the two coarse tiers. A steel shed falls as torn cladding, leaning frame legs and buckled roof sheets over its dado's stumps, not as masonry. [`industry_sheets.py`](industry_sheets.py) frames buildings that size for `assemble.py`.
- [`facade_lab.py`](facade_lab.py) is the facade lab's kit: fence panels of the two cutout recipes, a pane of glass, and window and shop bays with their rooms. It has no templates and no map places it. [`facade_lab_render.py`](facade_lab_render.py) photographs it in Blender where the lab stands it, from the lab scene's own cameras: the picture its frames are compared with.
- [`assemble.py`](assemble.py) puts a set back together in Blender from its two files and renders it at the game's camera, in any state, with the simulation's boxes drawn over it (the parts, or a ruin's remains): the picture to judge a set by until the renderer draws kits. Its `damage` sheets set each template beside its destroyed state at 30, 80 and 250 m and at the two coarse tiers; `aftermath` is a street of them, standing and destroyed, mixed.

## Interiors

A room behind a window is not modelled per building. Every window draws one open **room box**, and the box's walls, floor and ceiling look one picture up in an **interior atlas**: the picture is projected onto the box from a pinhole far in front of the window, so the back wall shrinks with depth and the room shifts as the viewer moves. `interiors.py` makes the atlases; they are ours, rendered from scripted rooms, never photographs.

The contract a shader reads them by (the numbers are the constants at the top of `interiors.py`):

- **Two sheets** under `assets/source/city/interiors/`: `rooms.png` for apartments on any floor, `shops.png` for ground floors. Both are opaque sRGB colour.
- **Layout.** 2 columns by 5 rows of square 128 px cells, with no gutter. Cell `i` is at column `i mod 2`, row `floor(i / 2)`, counted from the image's top-left. A cell's picture is upright: the ceiling is at its top. A lookup clamps half a texel inside its cell. A cell is a power of two so that every mip down to one texel a cell holds one room only.
- **The box a cell assumes** is 3 m wide, 3 m tall and 4.5 m deep: one bay and one floor of the lattice. Its open face is the inside face of the window wall.
- **The camera a cell assumes** is a pinhole 16 m in front of the open face, on the box's axis, framing that face exactly. A point `x` metres across from the box's middle, `y` metres into the room and `z` metres above the floor is at `u = 0.5 + k x / 3`, `v = 0.5 + k (z - 1.5) / 3`, with `k = 16 / (16 + y)` and `v` running up from the cell's bottom edge. A box of another size divides by its own width, height and depth: its back wall is the cell's.
- **A cell is a finished picture, and nothing in it glows.** The only light in a room is the sky through its own window wall, baked in: the brightest pixel is far below a sunlit wall, and there is no lamp, screen or emissive surface. It is shown as it is, dimmed if a building wants, and never added as emission.
- **Any cell fits any window.** A room is composed round a window in the middle of its bay, reads the same mirrored left to right, and carries no lettering, so a building picks a cell and a mirroring by hashing the window's position.

The steeper the view, the more of a window is the cell's floor: the box's floor is the bottom ninth of the picture, and from the tactical camera it fills most of an opening. Tune how a room reads here, in the scene and its tone curve, not in the shader.

The sheets are not a set: they have no kit and no templates, and a template never names a cell. `assets/catalog.json` names the two pictures under `interiors`, and the bake gives every bundle with a room its sheet as an ordinary texture, the cells laid out again four to a row in a square.

What a kit author does (`parts.room`, `parts.room_box`; the facade lab's kit is the worked example):

- **A room's material names its sheet** and nothing else (`room(name, "rooms")`): no recipe, no wear. Its row's tint dims it if the material is tint-masked.
- **Its UVs are the box unfolded round its back wall.** The back wall is the unit square, and the floor, the ceiling and the two side walls hang off its edges, reaching one unit out at the open face. Those are straight lines in the box's own space, so the shader reads a fragment's place across the box and its depth into it straight off the UV and does the pinhole lookup itself, exactly, at every pixel. (The cell's `u`, `v` as the UVs would be wrong between vertices: the projection is not straight across a triangle, and a floor's diagonal was seven texels off.) `room_box` writes them; a box of any width, height or depth shows the whole of its cell.
- **One room box a window, in a module that stands where the window does.** The cell and its mirroring come from a hash of the module's position, so a row a window gives every window its own room, the same from every camera and at every tier. Rooms folded into one shell mesh would all be one room.
- **A box is narrower than its bay.** Two boxes that share a wall's plane fight for depth there; the lab's are 2.9 m in a 3 m bay.
- **Rooms at tiers 0 and 1 only.** From tier 2 out a window is a few pixels: the shell's dark pane is enough, and a room box a window does not fit a far tier's triangle budget.

## Surfaces that are not opaque

A material says so through the material helpers (`coverage=`; [scene-assets readme](../../README.md), "Coverage and rooms"), and the battle draws each kind its own way:

- **A cutout** (`coverage=("cutout", cutoff)` on a recipe with a coverage image, as `grille` and `perforated` are): a grille, a perforated sheet, a sign's cut letters. It is there or not texel by texel in colour, depth and shadow, so model it as one face (`parts.sheet`), not as bars: a hundred thin boxes alias where one face thins evenly with distance. Too far away to resolve, it is a veil as dense as the share of it that is there, and its shadow the same share of shade.
- **Glass** (`coverage=("blended", opacity)`): one face, never a thin box (two layers and their edges). It is drawn after everything opaque, darkens what is behind it by its opacity, casts no shadow and hides nothing from the depth the overlays and the fog read. Panes are not sorted: keep one glass's opacity and colour alike across a building, and two panes blend the same either way round.

[`facade_lab.py`](facade_lab.py) is the kit that shows all three with a room, and the lab route `/lab/facade` where they are judged.
