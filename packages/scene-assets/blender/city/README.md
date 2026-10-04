# City buildings: kits and templates

A town's buildings are drawn as instances of a few shared **modules** (a window bay, a balcony, a roof, a wall shell), never as one mesh per building. The scripts here make the modules and say where each one goes on each building **template**. Large maps reuse a small kit across many buildings. Every building of every map is drawn this way, the hand-authored maps' few houses included.

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
- Materials, textures, UVs, tangents and vertex colour follow the conventions every static source follows ([scene-assets readme](../../README.md), "Textures"): texture limits belong to the shared validator, UVs use the source scale, and each alpha follows its named material meaning. A surface that takes a building's own tint (a wall colour) is tint-masked (`tint` in the material's extras, or the ORM texture's alpha).
- A mesh made for one template only (its wall shell, its roof) is a module like any other, named for its template.

### `templates.json`

[The source reader](../../src/templateSource.ts) owns the set schema and finding
codes. [The kit authoring helper](kit.py) writes it; [a hand-scripted set](homes.py)
and [a graph-derived set](china.py) are worked examples rather than a second schema.
A source receipt describes how the set was made; the bake does not trust that
receipt as proof of physical legality.

The descriptor is the simulation's [physical template](../../../../crates/contract/src/templates.rs).
Art is made to it: a wall that hides a unit physically must also hide it on screen.
Generated templates declare complete doors, floors and fighting bays; legacy
authored boxes carry only their admitted physical shape. Row transforms place
shared modules in that descriptor's frame, with positive scale and yaw; a tilt or
mirror belongs in a module variant. Every state has geometry at every drawn tier
so a building cannot vanish with distance.

A state follows the simulation's destruction rule, read through [collapse policy](collapse.py):
a fallen ruin fits the physical remains, while a gutted shell fits the standing
parts. Fit tolerances allow balconies, cornices and broken wall tops without
weakening the physical template. Release status records art acceptance; it does
not alter rendering or make an invalid source legal.

## From a set to a town

A new set is listed in `assets/catalog.json`: its kit as an appearance, and the set under `city_sets` with the catalogue it dresses. Then, with the `asset` CLI:

1. `catalogue` copies the `generated` sets' descriptors into the physical catalogue the map generator reads. A template belongs to exactly one set, and a set's descriptor and its catalogue row are the same template. The catalogue's hash moves, and what pins it moves in the same commit.
2. `bake`, then `check`. They refuse a set whose descriptor the simulation's contract refuses, whose art leaves its parts, whose rows name a module its kit lacks, or that leaves a template of its catalogue undressed. What the bake does with a set is in the [scene-assets readme](../../README.md), "City buildings". A template is retired the same way: it leaves its set, and `catalogue` drops its row.

The authored catalogue (`fixtures/building-templates.json`) runs the other way: its rows are written by hand, its hash is its maps' identity, and nothing derives it. Its set copies each row as its descriptor (`kit.dress`), so a new authored box needs only the set's script run again, then `bake` and `check`.

**The picture a set is judged by** is the line-up lab, `/lab/city-lineup`: every template on flat ground, drawn as the game draws a town, at any one tier, state and station (its route file lists the address parameters). `CITY_SET=<set> bun run --cwd web scene -- city-lineup` holds each template to its parts and fit on screen and writes, under `throwaway/evidence/city-lineup/`, a sheet per category at each tier and each template across each tier boundary (the scene file lists what else narrows a run).

The source-only opening coverage checks run without Blender: `python3 -m unittest discover -s packages/scene-assets/blender/city -p '*_test.py'`. Actual source exports also run the invariant before baking their geometry.

## Rules a set keeps

- **The same inputs write the same bytes.** Seeds are fixed, iteration is in a sorted order, and nothing reads the clock.
- **Sizes are whole bays and whole floors.** A facade's windows sit on the descriptor's bay lattice (3 m pitch) and its floors on `floor_heights_m`. A graph-made facade run is 3n + 2 or 3n metres long, with its corresponding corner piers ([source lattice](china.py)).
- **Every declared fighting bay has a visible opening.** Windows, doors and open fronts cover the physical eye and muzzle heights on each of the first three floor bands below that part's top. The scripted set reads those heights from the simulation fixture and refuses a blank bay before exporting. Opening rectangles on one wall do not overlap: a displaced existing pane is aligned to the fighting band, and any cladding patch is cut around the actual pane. Opening bounds come from the source geometry; they are authoring checks, never extra descriptor fields or a second garrison policy. Legacy authored boxes without resolved bays make no such declaration.
- **An exposed edge has a facade; an edge that is not exposed has none.** No windows on a party wall or an interior join.
- **Detail is budgeted per template**, in triangles drawn at each tier (`TEMPLATE_TIER_TRIANGLES`, `src/templateSource.ts`; the bake reports each template against it). The script prints what each template draws. A far building is one row: at the coarse tiers a script folds what is left of its modules into the template's own shell.
- **A destroyed building is the same building.** Its damage state is made on the same plan, in the same materials and tints, in the same frame, and by the same tier rule: shared wreckage (heaps of rubble, beams, burnt panels) is rows at the fine tiers, and the state's own shell is the whole of it at the coarse ones. It draws no more triangles than `intact` at any tier. Soot and breakage that must read from across the map are in the shell's own texture or vertex colour, never only in a fine tier's modules.
- **The game picks a tier by pixels to the metre** (`presentation.buildings.lod_px_per_m`): its thresholds belong to that fixture, independently of building height. A tower is at tier 1 or coarser in most frames, so its far tiers carry its character.
- **Nothing glows.** Emission is zero; interiors are unlit.
- **No real names.** Sign text is a generic word for a trade (tea, pharmacy, hotel) or comes from the project's own invented-name list; no brand, logo, place or landmark.
- **No street.** Sidewalks, street trees, lamps and props are not part of a building.

## What is here

[This directory](./) holds source exporters and their shared geometry helpers.
[Kit authoring](kit.py) owns scripted module placement and far-shell folding;
[graph extraction](graph.py) reads geometry-node instances before realization, and
[the graph-set exporter](graphset.py) is what every graph-derived set shares (outline,
rooms, tiers, damage, the descriptor, the files): [China](china.py), [New York](nyc.py)
and [Paris](paris.py) hold only their graph's tables and how to read it.
Both paths must write the same admitted set contract. [Damage construction](damage.py)
and [collapse policy](collapse.py) keep destroyed art tied to the original plan
and physical remains. Material acquisition uses the shared [pinned pack reader](../packs.py).

[Assembly review](assemble.py) provides a quick source-side view with physical
boxes. Its approximate materials do not certify glass, cutouts or interiors;
the rendered game labs do. [Facade review](facade_lab.py), its [source-side render](facade_lab_render.py)
and [opening checks](facade_test.py) isolate those authoring questions. Source
scripts and their usage comments own exporter arguments and supported outputs.

## Interiors

A room behind a window is not modelled per building. Every window draws one open **room box**, and the box's walls, floor and ceiling look one picture up in an **interior atlas**: the picture is projected onto the box from a pinhole far in front of the window, so the back wall shrinks with depth and the room shifts as the viewer moves. `interiors.py` makes the atlases; they are ours, rendered from scripted rooms, never photographs.

[The atlas generator](interiors.py) owns dimensions, cell layout and source-camera
projection. [The atlas baker](../../src/interior.ts) repacks those pictures for
runtime use, while [the material shader](../../../battle-renderer/src/models/surfaceFragments.ts)
reads the corresponding projection. Those owners define the numerical contract;
copying it into a second table would let the source and renderer disagree.

A cell is a finished, non-emissive picture. Its only light is the baked sky through
its own window. Rooms must read as matte surfaces in sun shadow, never as glowing
windows. Any cell fits any window: its centered composition tolerates mirroring
and contains no lettering, so a position hash may select and mirror it consistently.

The steeper the view, the more of a window is the cell's floor: the box's floor is the bottom ninth of the picture, and from the tactical camera it fills most of an opening. Tune how a room reads here, in the scene and its tone curve, not in the shader.

The sheets are not a set: they have no kit and no templates, and a template never names a cell. `assets/catalog.json` names the two pictures under `interiors`, and the bake gives every bundle with a room its sheet as an ordinary texture, the cells laid out again four to a row in a square.

[Room geometry helpers](../parts.py) write an unfolded box, not already projected
UVs. The shader projects each fragment: perspective is not linear across a
triangle, so preprojected vertex UVs create visible diagonal errors. A separate
module at each window preserves per-window identity; folding all rooms into one
shell would make them share one selection.

[Kit room planning](kit.py) cuts openings at the near tiers and fits room boxes
without shared depth surfaces. A room that cannot fit closes its blind instead of
intersecting its neighbor. Coarse shells retain dark panes when windows are too
small for room geometry to earn its triangle cost. Keep interior tone and
composition in the source pictures; shader brightness is not an authoring control.

## Surfaces that are not opaque

A material says so through the material helpers (`coverage=`; [scene-assets readme](../../README.md), "Coverage and rooms"), and the battle draws each kind its own way:

- **A cutout** (`coverage=("cutout", cutoff)` on a recipe with a coverage image, as `grille` and `perforated` are): a grille, a perforated sheet, a sign's cut letters. It is there or not texel by texel in colour, depth and shadow, so model it as one face (`parts.sheet`), not as bars: a hundred thin boxes alias where one face thins evenly with distance. Too far away to resolve, it is a veil as dense as the share of it that is there, and its shadow the same share of shade.
- **Glass** (`coverage=("blended", opacity)`): one face, never a thin box (two layers and their edges). It is drawn after everything opaque, darkens what is behind it by its opacity, casts no shadow and hides nothing from the depth the overlays and the fog read. Panes are not sorted: keep one glass's opacity and colour alike across a building, and two panes blend the same either way round.

[`facade_lab.py`](facade_lab.py) is the kit that shows all three with a room, and the lab route `/lab/facade` where they are judged.
