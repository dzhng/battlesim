# C52: preset layouts and connected roads

**Depends on:** C04; G0's versioned preset/coverage verdicts. **Kind:** required early slice.

## Question
Can each type/size/seed create its intended settlement and natural-cover composition with useful plains and connected approaches?

## Contract it unlocks
`generate_layout(&GenerationRequest, &PresetDefinitions) -> Result<MapPlan, Vec<Diagnostic>>` in Rust mapgen. Request fields are lossless seed, Open/Mixed/Metro, Small/Medium/Large and pinned generator/preset/physical-catalogue identity. Fixed extents and category rules come from the completed map; per-type counts/shares/weights/tolerances come from G0's versioned preset data, not player sliders or TypeScript constants.

Generate settlement plots, roads, forests, optional rivers/bridges and connected traversable plains. Ordinary settlement/street dimensions stay stable as larger maps add towns. Metro has one dominant central city and smaller surrounding districts; Open remains predominantly country with small residential towns. Road topology may use a richer connected network or a few main corridors. Sparse woodland varies coverage, preserving C72's one forest rule.

Balance top/bottom town area and forest coverage within G0's chosen metric/tolerance (forest area or count), without mirrored shapes. Reports may include both metrics; exact equality of both is not required. Keep useful approaches at 1,800 m weapon scale and infantry/vehicle access across town/plain transitions. Every map has flat base terrain except local river shaping. C65 owns curve densification; C03/C63 own classification/distance.

Use named random streams for sites, roads, forests/rivers, parcels and dressing so cosmetic changes do not relocate physical layout. Bound retries and return the failing feature/seed/config; never substitute a seed.

## API seam
`mapgen::layout` writes C04 MapPlan. Thin CLI/wasm adapters call the same library. The developer workbench exposes seed/type/size and plan layers; optional inspection handles are tooling, not a new player control or a second generator.

## What the human can run or see
A workbench showing full-extent Open/Mixed/Metro at every size: settlement bounds, roads, forest/river layers, usable plains and top/bottom metrics. Accepted `visualizations/map-character.html` is schematic inspiration, not a golden town count or ratio.

## Verification
- Fixed requests produce canonical byte-stable plans and IDs; changing size preserves metre scale while adding settlements.
- Validate exact dimensions, Metro's central hierarchy, Open height eligibility inputs, per-type composition and approximate coverage tolerance.
- Graph and physical movement connectivity agree across roads/bridges; infantry and vehicles reach the plain from towns.
- Impossible requests terminate within G0's bounded-work/generation budget.
- Visual variable: composition/access only. Compare full-map layers with the accepted schematic using compare-screenshots, then run unprimed screenshot-critique last. Use preview-shots for non-blocking review.

## Delegated to the implementer
Road/plot construction algorithms and shape variation within the ratified preset limits. New numeric defaults are spec gaps, not discretion.

## Must stay green
Fixed sizes, type character, useful plains, shared physical curves and deterministic failure.

## Feedback that would change this slice
Rejected composition updates versioned preset data and its evidence; it cannot silently shrink map dimensions.
