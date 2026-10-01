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

## Outcome

`mapgen::layout::generate_layout` is implemented, with its CLI commands, Wasm exports and the inspection picture ([crate guide](../../../crates/mapgen/README.md)). The presets are data in [`fixtures/map-presets.json`](../../../fixtures/map-presets.json), revision `layout-presets-1`. The decisions the spec left open are in the [choices ledger](../choices.md#c52-layout-generator).

**What a plan holds.** Settlements with their districts, country roads and dirt tracks, forests, and the measured open approaches. No rivers (C69 is not admitted by the compiler yet), no in-town streets and no buildings (C53).

**Measured over 300 seeds for each type and size (2,700 plans), all accepted:**

| | Settlements | Built ground | Main settlement's share of it | Forest | Country road km | Track km | Slowest edge to centre |
|---|---|---|---|---|---|---|---|
| Open Small | 8–10 | 2.0–4.7% | 24–56% | 4–22% | 12–28 | 2–22 | 114–175 s |
| Open Medium | 15–19 | 1.9–4.4% | 14–38% | 4–22% | 20–41 | 5–47 | 147–183 s |
| Open Large | 25–29 | 2.0–4.0% | 10–25% | 4–22% | 28–66 | 10–79 | 180–183 s |
| Mixed Small | 7–10 | 9.6–19.3% | 48–78% | 4–17% | 12–32 | 0–16 | 114–175 s |
| Mixed Medium | 13–17 | 7.4–13.4% | 38–64% | 4–17% | 20–47 | 3–32 | 147–183 s |
| Mixed Large | 22–27 | 6.5–10.3% | 28–50% | 4–17% | 30–68 | 7–59 | 180–183 s |
| Metro Small | 3–7 | 11.3–22.0% | 69–97% | 2–8% | 12–27 | 0–9 | 114–176 s |
| Metro Medium | 8–12 | 14.1–24.4% | 64–86% | 2–8% | 19–44 | 0–17 | 147–183 s |
| Metro Large | 15–19 | 15.3–24.3% | 64–82% | 2–8% | 31–63 | 1–25 | 179–183 s |

Every accepted plan has all settlements road-connected, top/bottom built ground and forest inside S7's tolerances, an 1,800 m by 400 m open approach to the main settlement in each half, and each edge within 183 s of the centre by road at 110 km/h (15 s of that is the allowance for planning and turns). A plan takes 1–6 ms to generate natively (median by cell). The slowest single plan took 37 ms in a quiet run and 430 ms when the machine was busy with other work. A plan has 800–6,700 ground points. Native and Wasm give the same bytes for seven recorded requests (`fixtures/parity/map-layout/`).

**Metro's urban share (M18).** The city's envelope is 16–24% of the playable area at every size; built ground on Small Metro comes to 11–22%. Small Metro then met the approach rule in 300 of 300 seeds. Holding the city at one share: 24% and below passed every seed, 28% passed 98%, 32% 95%, 36% 88%, 40% 59%, 45% under 1%. Above 24% the first thing to fail is room for the satellite towns; the approach itself starts failing at 40%.

**What the user's references changed.** Two Broken Arrow screenshots of a Mixed map, given during the slice, shaped the towns: a district is one use (a garden suburb, an apartment block, an industrial compound) rather than a blend; a larger settlement leaves fields and woods between its districts instead of filling a disc; settlements stand on a main road that runs through them, with industry where the road enters; and every district has a stable id, an area and an anchor point so the encounter planner can put objectives on it.

**Still wrong or unfinished.**

- **Large always has four roads meeting at the centre**, against M21. Within 183 s a road from the middle of an edge can reach the centre only by going nearly straight to it (5.13 km of road against 5 km in a line), so a fork more than about 130 m from the centre is out of time. Small and Medium have the slack and use it: an edge road joins an earlier one up to 1.5 km from the centre, and only 22–30% of Small plans and 38–41% of Medium plans are a four-way crossroads at the centre. Large is one in 100%. What varies there is the roads' skew (exits up to a kilometre off the midpoint), the settlements they run through, and the rest of the network (0–41 loops, 4–8 exits). Raising `transit.max_s` buys forks on Large: about 600 m out at 200 s.
- **Districts are rings of radial sectors**, which reads as a dartboard on the picture. They are zoning for C53 to cut into blocks, not street patterns.
- **Roads that join a settlement end at its far edge** as a short main street, which reads as a stub until C53 adds streets.
- **Movement is not proved.** Connectivity and journey times are measured on the road graph of the plan. No generated map has been loaded into a battle here, so the slice's "infantry and vehicles reach the plain from towns" check is still open, as is the comparison against `visualizations/map-character.html`.
- **No workbench.** The picture is `mapgen inspect`; a browser workbench with seed, type and size controls was not built.
