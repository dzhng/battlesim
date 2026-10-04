# 05b Courts read as lived in, not as empty plazas

**Why:** after slice 05 the courts are paved but mostly empty (`../assets/05/`): amenity groups need open ground on every side, so none stands against a wall, the narrow courts of centre districts get almost nothing, and the mix leans to small groups. The user's complaint was exactly "they all look super empty".

**Unlocks:** a measured fill, so "empty" is a failing test rather than a taste call.

**Seam:** `street_props/courts.rs` and presets only (`street_props.groups`, each dense district's `props.courts`).
- A group may stand against a building wall or a court edge, keeping the vehicle ring on its open sides only. Reachability stays the slice 05 tests' claim; they must still pass unchanged.
- Fill the courts with what real ones hold, from existing kinds with art first: cars parked in bays along walls (`parked_car`), trees in planters (`street_tree`, `planter`), benches and bins beside the walls, then the slice 05 groups in the open middle.
- Weights rebalanced so large groups (a full basketball court, a pétanque square) appear where they fit.

**Verify (tests first):** a fill metric measured on a 2 m grid over every dense court of every type × size × region at seeds 1–3: the share of court ground farther than 12 m from any body, building or carriageway. Report the slice 05 value first, then hold the new code below a threshold chosen from the evidence (target well under a quarter) in a test. The slice 05 tests (inside the court, groups whole, family-gated, reachability for squads and the widest hull, doors, part limit, determinism) stay green unchanged. Sweep admission stays 81/81; part budget reported (courts before gardens).

**Run/see:** the slice 05 framings per region, plus the China town view; screenshot-critique; compare-screenshots against `../assets/05/`.

**Delegated:** bay depth and spacing, which walls take parking, the exact threshold within the target.
