# C14: template damage states

**Depends on:** C32; terminal geometry contract retained from Q4. **Kind:** slice.

## Question
Do reusable intact/ruin/gutted template states fit the same building geometry and accepted damage rules?

## Contract it unlocks
The C13 source exporter and C32 library builder evaluate patched damage inputs for each reusable recipe, producing ruin states for ≤6 floors and standing burnt/gutted states above 6. The physical ruin height uses the existing provisional Q4 ratio/clamp pending C50. A C50 ratio change must rerun this terminal bake/fit gate, publish a new appearance hash and rerun fit before C54/C51; tuning hp alone needs no rebake. Intact physical catalogue/map identity stays unchanged; resolved rule/config identity names the changed terminal bounds. Damage remains a simulation event; appearance resolution chooses the state the side has observed.

## API seam
Offline family/graph patch scripts → C13 state source placements → C32 packing/fit validator. C42/C43 own damage transitions; the template resolver never reads hidden live state or invents a terminal body.

## What the human can run or see
Intact/ruin/gutted contact sheets and sim-bounds overlays by physical floor class, including the new homes, towers and industrial templates.

## Verification
- Byte identity and terminal-state coverage for each released template.
- Ruin bounds fit accepted terminal geometry; gutted bounds retain standing height.
- The 9+ highrise layout category does not change the >6-floor gutting threshold.
- Compare terminal-state silhouettes/fit against Q4 geometry and intact template evidence with compare-screenshots; run unprimed screenshot-critique last; preview-shots non-blocking.

## Delegated to the implementer
Breach pattern/rubble distribution within physical bounds. Damage thresholds/timing and new states are not delegated.

## Must stay green
One sim damage owner and side-knowledge timing.

## Feedback that would change this slice
Rejected terminal fit changes its recipe; C50 alone tunes durability/ruin coefficients.

## Outcome

### The scripted sets: houses, farmsteads, towers, industry (2026-10-02)

Every template of `homes`, `farmsteads`, `towers` and `industry` (22 of the catalogue's 29) has the one damage state the simulation destroys it into, written by its set's script beside `intact`. The seven `china_apartments` templates are the other half of this slice.

**The rule the bake holds a state to** (`packages/scene-assets/src/templateSource.ts`, read from the building prop type through `Authority.collapse`, never copied):

- A template of six floors or fewer has `ruin` rows and no `gutted`; a taller one `gutted` and no `ruin`. Anything else is refused by name (`templates.state`: `template X (N floors) collapses, and has no "ruin" rows`).
- A `ruin` fits the remains: every part's plan, from its own base to the one ruin height, grown by the set's `fit.side_m` and by `fit.ruin_top_m` above (a new number in a set's `fit`, 0 when absent; 0.6 m in the three sets that have ruins). The finding is `templates.fit`, and names the ruin height.
- **The ruin height is the building's, not the part's.** The simulation gives every part of a collapsed building remains of one height: a quarter of the building's height (its tallest part's top), between 2 and 6 m (`crates/sim/src/battle.rs`, `Destroyed::Into`). A farm's 3 m shed beside its 8.4 m barn leaves 2.1 m of remains, as the barn does. The check and the scripts follow the simulation.
- A `gutted` state fits the standing parts, as `intact` does.
- The prototype set gives a stand-in its damage state as the same boxes, darker.

**The art.** A ruin is the building's own plan, materials and tints broken down to the remains: ragged stumps of its walls with their openings broken to the sills where the intact rows hung a window or a door, the heap inside them (a mound, low against the walls, banked out through their gaps), pieces of its own roof slipped over the heap, chimney stumps, charred beams and heaps of rubble strewn on it. A terrace's houses fall to different levels. A boarded barn burns to the foot between charred posts. A steel shed falls as torn cladding, leaning frame legs and its roof's own sheets, buckled and rusted, over its dado's stumps; a concrete warehouse as broken panels under slabs of its deck. A gutted tower is its shell built a second time in burnt facade recipes: every opening an empty dark hole, the soot of its fire in a tongue up the wall over it, bays blown out to the floor slabs, balconies scorched, broken or hanging, the roof and its huts burnt, the wall darker the higher it stands. Nothing glows.

**Shared.** `masonry.py`: `ragged_wall` (lifted from the village farmhouse, which writes the same bytes through it), `rubble_fill`, `scorched`, `weathered_roof`. `city/kit.py`: `ruin_block`, `ruin_sides`, `wreckage` and `litter`, `fold_far`, and the state checks. `city/collapse.py`: the simulation's rule. New modules: in each of `homes`, `farmsteads` and `industry`, `rubble_heap_s`, `_m`, `_l` and a beam, lying and fallen (charred timber, or steel in `industry`), and a `<template>_ruin` shell (a terrace has two houses' ruins and an end wall); in `towers`, a `<template>_gutted` shell, the `gutted_<kind>_<0..3>` panels (one set for every finish), three burnt balconies, a burnt entrance, and burnt huts, vents and plant.

**Tiers.** Wreckage and burnt panels are rows at tiers 0 and 1; at tiers 2 and 3 a destroyed template is its damage shell and nothing else (a terrace: one row a house and one for its end). A damage state draws no more triangles than `intact` at any tier; `kit.py` refuses a set that does. Triangles drawn, tiers 0 to 3:

| Template | Intact | Ruin or gutted |
|---|---|---|
| `china-home-10x8-1f` | 2,620 / 1,132 / 190 / 100 | 2,586 / 980 / 166 / 24 |
| `china-home-12x9-2f` | 5,832 / 2,276 / 382 / 204 | 3,052 / 1,052 / 206 / 28 |
| `china-home-8x11-1f` | 3,140 / 1,252 / 188 / 98 | 2,678 / 968 / 168 / 26 |
| `china-home-9x9-2f` | 4,688 / 1,720 / 262 / 162 | 2,768 / 1,038 / 158 / 26 |
| `china-home-ell-2f` | 5,806 / 2,242 / 368 / 201 | 4,462 / 1,704 / 260 / 44 |
| `china-townhouse-2f` | 3,284 / 1,344 / 214 / 124 | 2,532 / 988 / 154 / 24 |
| `china-terrace-3x2f` | 7,156 / 2,708 / 504 / 262 | 6,120 / 2,316 / 462 / 62 |
| `china-terrace-5x3f` | 16,316 / 6,768 / 1,166 / 512 | 9,744 / 3,500 / 722 / 102 |
| `china-shops-4x3f` | 17,796 / 6,620 / 1,184 / 486 | 9,472 / 3,376 / 648 / 90 |
| `china-corner-shop-3f` | 8,800 / 3,320 / 540 / 280 | 3,944 / 1,452 / 184 / 30 |
| `china-farmstead-long` | 8,920 / 3,370 / 460 / 220 | 8,186 / 2,918 / 418 / 76 |
| `china-farmstead-small` | 7,884 / 3,094 / 380 / 206 | 5,558 / 2,268 / 286 / 58 |
| `china-farmstead-yard` | 13,488 / 5,104 / 802 / 344 | 9,756 / 3,938 / 464 / 94 |
| `china-shed-15x24` | 5,166 / 2,256 / 438 / 192 | 3,192 / 1,348 / 298 / 16 |
| `china-warehouse-48x24` | 9,558 / 4,304 / 1,000 / 600 | 7,712 / 2,984 / 266 / 28 |
| `china-warehouse-72x33` | 9,044 / 4,184 / 1,134 / 702 | 8,568 / 3,406 / 652 / 48 |
| `china-works-54x36` | 15,884 / 5,946 / 1,354 / 484 | 12,082 / 3,984 / 666 / 52 |
| `china-depot-90x39` | 20,580 / 8,636 / 1,920 / 800 | 13,010 / 4,460 / 452 / 66 |
| `china-tower-slab-10f` | 27,306 / 7,866 / 1,234 / 436 | 20,840 / 5,798 / 1,182 / 406 |
| `china-tower-12f` | 24,238 / 1,842 / 768 / 332 | 16,800 / 1,810 / 762 / 318 |
| `china-tower-16f` | 37,432 / 12,094 / 970 / 486 | 27,968 / 8,578 / 938 / 450 |
| `china-tower-20f` | 41,854 / 2,526 / 906 / 308 | 28,048 / 2,418 / 882 / 284 |

**Proved.** Each script run twice writes the same bytes. `asset validate` finds nothing on any of the four kits. No physical descriptor moved, so the catalogue and its hash stand. With the strict rule, `asset bake` refuses the library with seven findings and no others, each a `china_apartments` template with no damage state: all 22 templates here pass the state and fit rules. For the line-up the library was baked locally with stand-in states for those seven (their intact rows, squashed to the remains for a ruin), which is not committed: the line-up's own check (every template at every tier, in each state, inside its parts grown by the fit) passed for all four sets. The scene-assets tests cover the rule: the state a template's floors call for, a ruin held to the remains at three building heights, the one height for every part, the top allowance, and the authority reading the fixture's row.

**The village set** (C37, merged while this pass ran) already had a `ruin` for every box, the farmhouse's. It is held to the remains like any other: its set now says `ruin_top_m` 0.6 (one line of its `templates.json`), and its kit is the same bytes through the lifted wall. Its ruin outdraws its farm at two tiers, so its script opts out of the helper's budget (`damage_budget=False`); the art was not touched.

**The intact art changed, on purpose.** Descriptors did not. The modules whose bytes moved are each set's shells (and, in `homes` and `farmsteads`, the chimneys; in `towers`, the window, door and ground panels' far cards), for these reasons:

- *Roofs.* The `roof_tile` recipe is small tiles in one colour with nothing wider than a tile, slate and stone roofs use a new neutral `roof_slate`, and a roof's stains are soft fields metres across in its own vertex paint: no tartan from the game's camera.
- *The coarse tiers keep the picture* (the line-up's list in [C23](C23-far-tier.md)). Houses and farms: walls cut into foot, body and head so the wall keeps its colour at every tier; fittings folded into the shell at both coarse tiers, a window as its pane in a pale surround between its shutters, a door in its own paint, a chimney as a block the colour of its cap; a far house is now one row. Towers: the facade recipes draw curtains in the same bays the tier 0 rows hang them in; a balcony column's recipe carries its fronts' own colour; a folded hut keeps its lid and door; vents and plant fold in at tier 2; the roof is a stained field inside a band the parapet shades, with mended patches, plant and more vents. Industry: roofs keep their rust and mends at tier 3 on a coarser grid, with their flashings, rims and furniture; windows stay at tier 3; a far dock bay stays dark.
- *One structural thing a tower.* The 12-floor tower has a terracotta top floor under a roof slab that stands out; the 16-floor one has ribbon glazing between its balcony columns; the 20-floor one is as it was.
- *Asymmetry.* Three houses have a dormer off to one side.

**Judged by looking.** Blender sheets from `city/assemble.py` (intact beside destroyed at 30, 80 and 250 m and at the two coarse tiers, and a street after the battle a set), and the game's own line-up at every tier, state and tier boundary. One unprimed critique of both read every state correctly (a total collapse; a tower burnt out and standing, "unmistakably the same building"), told the two states apart at a glance at 80 and 250 m in the houses' and farms' streets, and found these, which were acted on: stumps that read as battlements (their tops now run on from one to the next), the heap as a level tray (a mound with a talus banked out through and round the walls), ruins that changed tone from tier to tier (the heap's rim band, and the far tiers' sides, band and roof pieces where the near ones have them), chimney stumps that read as black holes far off, roof pieces as clean cards (torn to trapezoids) and a moire on tiled roofs (softer courses), timber that fell as masonry (boards between charred posts), a stray chimney at a terrace's end at tier 3, stumps brown whatever the wall's colour in the game (soot now leaves the lower half its colour), a gutted tower's white roof furniture and black roof rectangles, and its facade darkening from tier 0 to 1.

**Not done, and known.**
- A ruin is as tall as the rule says and no taller: a three-storey shop and a bungalow leave piles of nearly one height, and rubble reaches at most `fit.side_m` past the walls, so a ruin is a filled plan, not a spread heap. Both are the simulation's box.
- At the coarsest tier a large industrial ruin is a low dark slab with its roof's patches: it is told from hardstanding by its rim and colour, not by form. A gutted tower is told from an intact one in shade by its black openings, blown bays and ash, which thin out with distance.
- Shop fascias, the warehouse's red band and the works' sawtooth do not survive into their ruins.
- The critique was not rerun after its fixes; the final sheets were looked at by their author.
- The committed runtime is not rebaked: the library cannot be sealed until the apartments have their states.

Decisions: [choices](../choices.md).

## Outcome: the China apartment set

All seven templates of `china_apartments` carry their damage state: `ruin` on the 4, 5 and 6 floor slabs and the U and court blocks, `gutted` on the 8 floor slab and the 7 floor point block. The physical descriptors are unchanged.

**The graph has no damage inputs** (its 53 inputs are size, facade and street options), so the states are made in the exporter from the intact export (`city/damage.py`, `city/china.py`), not by evaluating a patched graph:

- **Ruin.** Each run's wall is cut into 1.5 m columns broken off at a ragged height no higher than the remains box (a quarter of the building's height, 3.3 to 4.8 m here), with its inner face and a broken top; half its sign boards and window surrounds hang on, scorched. Each part is piled with rubble to a height of its own (half to 0.85 of the box), a smooth mound running out up to 1.1 m past the walls in tongues; broken blocks, many small and a few large, lie over it, the floors lie in it as tilted slabs, and charred fittings of the kit (air conditioners, window cages, a balcony, a shutter, a window frame) are thrown down on it as rows. Nothing stands above the remains box.
- **Gutted.** The same walls, smoked: the paint keeps a tenth of its colour and the bands, sills and plinth are blackened. Every opening is an empty dark hole, with a black soot fan above most of them that rises to the next opening, still half black there, so a column of windows is one streak; about 3.5% of the bays above the shops are blown out to the floor slab. Balconies stay as charred frames without glass, one window in four keeps a charred frame, half the cages stay (half of those hanging), awnings, laundry, lanterns and sign text are gone. The roof's clay is smoked to a brown grey; where the fire came through there is a hole, the top storey's black floor and the inside of its walls showing through it, and round the hole the tiles are off a slab grey with ash. What stood on the roof there is gone. Nothing glows.
- **Tiers** follow the intact rule: fittings are rows at tiers 0 and 1, and the state's own shell holds everything at tiers 2 and 3, so a far ruin is one row. No damage state draws more than its intact state at any tier.

| template | state | rows | tier 0 | tier 1 | tier 2 | tier 3 |
|---|---|---|---|---|---|---|
| slab-35x11-4f | ruin | 20 | 8,696 | 2,918 | 926 | 143 |
| slab-47x11-5f | ruin | 27 | 10,125 | 4,192 | 1,415 | 190 |
| slab-59x14-6f | ruin | 42 | 15,977 | 6,368 | 2,013 | 281 |
| block-u-5f | ruin | 50 | 19,017 | 6,831 | 2,287 | 335 |
| block-court-6f | ruin | 67 | 25,002 | 9,338 | 2,925 | 434 |
| slab-53x14-8f | gutted | 271 | 56,879 | 23,268 | 5,304 | 1,266 |
| point-20x20-7f | gutted | 128 | 29,690 | 11,496 | 2,716 | 710 |

The ruin height and the six-floor threshold are the simulation's, read through `city/collapse.py`. The exporter holds a ruin to the parts' plan grown by `fit.side_m`, the ground, and the ruin height; the set names no `ruin_top_m`, so nothing stands above the remains, and the bake's rule passes it with no findings. A gutted block is held to the intact rule. Two runs write the same bytes. Pictures: the game's own line-up lab (`scene -- city-lineup`, every template at every tier in each state, held inside its parts) and a Blender reassembly of the two set files, intact beside damaged at 30, 80 and 250 m, with an unprimed critique ([choices](../choices.md)).

Two unprimed critiques of the game's line-up were run, the second after fixing the first.

| finding | disposition |
|---|---|
| the gutted roof's dark patches read as a tree's or a cloud's shadow (twice: soft, then hard-edged) | fixed: holes with the storey below showing, a rim of bare ashen slab, the whole roof smoked |
| the gutted walls read as a dim building in shade; soot hung under the sills; openings showed the wall's colour | fixed: a fan drawn across the opening above painted it; fans now stop under it, and the wall loses its paint colour. The lining behind every opening is four times darker |
| the rubble read as a faceted camouflage tarp, tile in a chequer of diagonals | fixed: smooth, off its grid, one material to a cell |
| the ruin changed picture at 319 m (dark thick walls, bars and debris gone) | fixed: the coarse stumps take the fine ones' colour; tier 2 keeps the sign boards, the thrown-down fittings and a few large blocks |
| ground showed through the tier 3 heap | fixed: a cell at the heap's edge could turn over |
| intact tier 3: balcony fronts floating off the wall, seen from behind as stripes | fixed: laid on the wall |
| saturated sign boards on a burnt or collapsed block | fixed: scorched, half of them down |
| a ruin is a low, level plateau for a five-storey block; the yard of the U and the court is clean | kept: it is the remains box the simulation leaves (a quarter of the height), and the yard is outside the parts |
| stumps step like battlements; blocks look sprinkled; fallen floors are clean rectangles | open: the model pass. Slanted breaks need `damage.break_off` to cut on a tilted plane |
| sign text and lanterns go at tier 1, roof clutter at tier 3 | kept: the tier review accepted both at those distances |

