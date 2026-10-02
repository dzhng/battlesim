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

## Outcome (the China apartment set)

All seven templates of `china_apartments` carry their damage state: `ruin` on the 4, 5 and 6 floor slabs and the U and court blocks, `gutted` on the 8 floor slab and the 7 floor point block. The physical descriptors are unchanged.

**The graph has no damage inputs** (its 53 inputs are size, facade and street options), so the states are made in the exporter from the intact export (`city/damage.py`, `city/china.py`), not by evaluating a patched graph:

- **Ruin.** Each run's wall is cut into 1.5 m columns broken off at a ragged height no higher than the remains box (a quarter of the building's height, 3.3 to 4.8 m here), with its inner face and a broken top; half its sign boards and window surrounds hang on, scorched. Each part is piled with rubble to a height of its own (half to 0.85 of the box), a smooth mound running out up to 1.1 m past the walls in tongues; broken blocks, many small and a few large, lie over it, the floors lie in it as tilted slabs, and charred fittings of the kit (air conditioners, window cages, a balcony, a shutter, a window frame) are thrown down on it as rows. Nothing stands above the remains box.
- **Gutted.** The same walls, smoked: the paint keeps a tenth of its colour and the bands, sills and plinth are blackened. Every opening is an empty dark hole, with a black soot fan above most of them that rises to the next opening and no further; about 3.5% of the bays above the shops are blown out to the floor slab. Balconies stay as charred frames without glass, one window in four keeps a charred frame, half the cages stay (half of those hanging), awnings, laundry, lanterns and sign text are gone. The roof's clay is smoked to a brown grey; where the fire came through, the tiles are off a slab pale with ash, with holes burnt through it. Nothing glows.
- **Tiers** follow the intact rule: fittings are rows at tiers 0 and 1, and the state's own shell holds everything at tiers 2 and 3, so a far ruin is one row. No damage state draws more than its intact state at any tier.

| template | state | rows | tier 0 | tier 1 | tier 2 | tier 3 |
|---|---|---|---|---|---|---|
| slab-35x11-4f | ruin | 20 | 8,664 | 2,879 | 840 | 143 |
| slab-47x11-5f | ruin | 27 | 10,110 | 4,132 | 1,266 | 190 |
| slab-59x14-6f | ruin | 42 | 15,969 | 6,310 | 1,785 | 281 |
| block-u-5f | ruin | 50 | 18,963 | 6,716 | 2,052 | 336 |
| block-court-6f | ruin | 67 | 24,874 | 9,175 | 2,406 | 435 |
| slab-53x14-8f | gutted | 269 | 61,055 | 22,773 | 5,656 | 1,330 |
| point-20x20-7f | gutted | 129 | 29,593 | 10,731 | 2,658 | 710 |

The exporter holds a ruin to the parts' plan grown by `fit.side_m`, the ground, and the ruin height with no allowance above it; a gutted block to the intact rule. Two runs write the same bytes. Pictures: the game's own line-up lab (`scene -- city-lineup`, every template at every tier in each state, held inside its parts) and a Blender reassembly of the two set files, intact beside damaged at 30, 80 and 250 m, with an unprimed critique ([choices](../choices.md#c11c12c13-the-china-apartment-kit)).
