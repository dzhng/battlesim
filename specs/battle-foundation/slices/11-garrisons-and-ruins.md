# 11 — Buildings as abstract fighting positions

**Status:** complete 2026-09-25. **Dependencies:** 02, 04, 08, 09. **Milestone:** Village checkpoint.

## Contract and question

Can occupants gain probability-based protection without inconsistent projectile walls?

User requirements owned or exercised: P12, P13, M07, L08, L09, L10, U01. Read their canonical entries in [requirements](../requirements.md), then the applicable [implementation contracts](../contracts.md). Do not infer rules from a fixture label.

## API seam and ownership

`sim::garrison::{enter, exit, allocate_slots, collapse}` owns occupancy; world owns facade slots/shell and ruin geometry.

## Runnable artifact

/lab/garrison: two friendly squads share a building by capacity; entry/exit, outward firing, direct hits, wall misses, enemy behind building, collapse and escape. Use contracts.md target-independent perimeter hit-region representation.



## Verification and verdict

Whole-squad capacity atomicity; no duplicate occupant bodies; outgoing own-wall clearance; all-target collision semantics; cover applied once; shots beyond hit shell; structural-only damage; collapse survivor/corpse accounting and heavy suppression; no legal exit means casualty, not teleport; permanent impassable lower ruin blocks according to shape.

Run relevant native Rust tests and browser/TypeScript seam tests; run typecheck/lint and the registered scene without console/GPU errors. Keep prior accepted slice contracts green. Source test names in [research](../research.md) are reuse references, not evidence that these new tests already exist. Record measured results and any scope deviation in this slice and the README handoff.

Visual variable: **Garrison position and collapse readability**. Review crop/mask: **Building perimeter hit regions and lower ruin silhouette**. Explicitly out of scope: Window meshes, interiors and destruction animation.

1. Freeze fixture seed, tick, camera, viewport and DPR. The registered scene regenerates the full frame and tight 2×–4× crops with machine/config metadata in gitignored `throwaway/evidence/<fixture-id>/`; the verdict records metrics and dispositions.
2. Use [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) for candidate versus any previous/reference shot. State the target for this variable, report telemetry and a less-wrong verdict; do not reward mere baseline matching. With no predecessor, use the skill's single-image diagnostics and inspect their meaning.
3. **As the last visual acceptance check, run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) with an unprimed agent given only the actual candidate full frames, crops and neutral inspection task.** Inspect and record its actionable findings before accepting. Do not substitute implementer self-review.
4. Offer the relevant shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md). This is a **non-blocking** course-correction window of about five minutes. Continue independent work while it is open, using waits no longer than 60 seconds. If no answer, decide reversible matters on evidence, record the choice, close the opened Preview set and proceed. Silence does not authorize changing user rules.

A visual pass is not a substitute for behavioral tests. A failing contract or serious visual defect means fix or reslice, not quietly update a golden image.

## Additional handoff regression

Test outgoing shots toward all four facades and corners, target-facing slot capacity waits, observer rays from occupied slots, and another building blocking normally.

## Decision budget

Delegated: Slot distribution with safe capacity; placeholder collapse swap. Also permitted: internal naming and reversible diagnostic formatting. Every other consequential freedom is a spec gap: record and resolve it before expanding implementation. No compatibility shims, duplicate owners, historical renderer adapters or undocumented gameplay changes.

## Human feedback that changes the slice

If the proxy looks like unprotected exposed infantry or feels wall-transparent, reslice this representation before polished assets.

## Verdict — 2026-09-25

`sim::garrison` owns occupancy; the unit owns its garrison (`Unit::garrison`: building, phase and timer, the entry point, the building's slots and one seat per soldier). The world owns the facade slots (`Prop::facade_slots`, `Slot::faces`) and the ruin geometry. `garrison::Structures` holds building health and which ruin replaced which building.

- **Entering and leaving:** `garrison::validate` refuses a squad at the order (`not_infantry`, `not_a_building`, `capacity_full`) using only the side's own units. `garrison::advance` runs each tick after commands. It starts the stationary entry timer within `entry_distance_m` of the walls, seats whole squads when the timer ends (or holds them in `waiting_for_room`), and runs the exit timer. A leaving squad is placed as a formation on free ground outside, nearest to where it is going.
- **Slots:** an occupant's capsule, eyes and muzzle stand at a perimeter slot 0.45 m outside a facade. Capacity is reserved evenly around the facades. `garrison::allocate_slots` moves soldiers to free slots facing each locked weapon's observed aim point, one tick after the lock. A weapon with nobody facing waits and reports `no_facing_slot`. Outgoing rounds fire only from slots whose facade the line leaves by more than `slot_facing_min_deg`, so they never meet their own walls. Every round queries the same colliders.
- **Sensing:** a garrison sees only from its occupied slots (`sensing::eyes`), for identification and for the fog sweep. It is seen only at its slots, and at the fixture's building concealment.
- **Cover (`sim::damage`):** an identified target seen garrisoned widens incoming spread by the building multiplier. Blast fragments reach its soldiers at the building's fragment probability. The stronger of forest and building wins, applied once. A blast ignores the occupants' own building as an obstacle. Impacts on a building report structural damage, and only weapons with `structural_damage` wear it down.
- **Collapse:** `garrison::collapse` replaces the building with a 2 m ruin of the same footprint through `world.add_prop`. Each occupant survives with `survival_probability_on_collapse`. A survivor walks out to the nearest legal, reachable, free ground within `exit_search_radius_m`, or dies where it stood. Survivors carry at least `suppression.collapse_level` and reform over the next seconds.
- **Knowledge:** every side plans routes around ruins, since a ruin's footprint equals the authored building. A side learns the ruin as a known prop when any of its footprint is seen. The published known prop carries `replaces`, the building it stands in for.
- **Protocol and publication:** `Order::Garrison`/`ExitBuilding`, three `OrderError`s, two `ActionReason`s (`no_facing_slot`, `changing_position`), the own-unit fields `garrisonBuilding`/`garrisonPhase`/`garrisonProgress` with the layout's `garrisonPhases`, and the known-prop field `replaces`. All go through `sim::publication` and `web/src/battle/sim/observation.ts`. `useUnitControl` garrisons on a right-click on a building (Shift queues) and exposes `exitBuilding`.
- **Digest:** garrison state, building health and ruins enter `Battle::digest` with presence and length tags. So do order kinds and soldier offsets.

**Native tests** (`cargo test -p sim --test garrison`, 16). They cover:

- whole-squad capacity atomicity: 8 + 8 + 4 refused whole; the third squad refused while two are on their way in and while they are inside; two same-tick orders leaving one squad waiting for room, never split; the scouts fitting once a squad leaves; tanks and non-buildings refused;
- entry after arriving, stationary for exactly `enter_exit_s`, with weapons reporting `changing_position`;
- no duplicate occupant bodies: 16 soldiers at 16 distinct slots, 0.45 m outside the walls, four per facade;
- outgoing own-wall clearance toward all four facades and all four corners, with no round meeting its own walls;
- target-facing slot capacity waits: in a full building only the squad's two north slots fire north and the grenadier reports `no_facing_slot`; once the other squad leaves, four rifles and the grenadier fire;
- all-target collision semantics: rounds aimed at an occupant hit its capsule, rounds just wide of it hit the wall, and rounds aimed at a soldier beyond the building hit the shell;
- enemy fire into a full building: direct hits on occupants, machine-gun misses stopped by the wall without wearing it, and HE on the wall taking exactly its structural damage (no round passing through the building);
- cover applied once: the strongest of forest and building for spread and fragments, never the product;
- collapse accounting: a lower ruin on the footprint, the building gone for good, every occupant alive a tick earlier now a survivor or a corpse, survivors outside the ruin on traversable ground within the search radius of their slots and at least `collapse_level` suppressed;
- no legal exit means a casualty, not a teleport: with knee-high walls filling the ground round the building, every occupant dies where it stood;
- the permanent lower ruin: a low line meets it, a line above it passes, and a squad sent across walks round it;
- observer rays from occupied slots only: an AT squad on three facades does not see a squad beyond the empty fourth, and the enemy sees the occupants only at their slots;
- another building still blocking a garrison normally (`blocked_trajectory`, no rounds);
- leaving: the whole timer; Stop keeps the squad inside; a move order leaves first on the side it is heading to, with every soldier clear of the walls;
- a Shift-queued garrison after a move (U01);
- garrison state changing the digest every tick, and a replay reproducing every digest.

Every earlier native suite stays green (140 Rust tests in all). The vitest suite gained a decode test for the garrison phase and progress, the `garrisonPhases` names, occupants on the perimeter and a ruin's `replaces` (42 tests). The fog sweep now marks the ground an observer stands on. It had been left dark under a lone squad.

**Browser scene** (`bun run --cwd web scene -- garrison`, 14 checks):

- a red squad behind the building unseen from outside it;
- two squads entering after a stationary timer;
- every occupant at its own perimeter slot, 0.45 m outside the walls;
- the scouts refused whole with `capacity_full`;
- the occupants on the far facade seeing the squad behind the building;
- a squad leaving after its timer to free ground outside;
- the scouts fitting once it has left;
- outgoing rounds never striking their own walls (256 blue segments, none on the building);
- red's misses stopping in the walls while its fire hurts occupants;
- the collapse into a lower ruin on the footprint (`replaces` 0, 2 m tall);
- survivors outside the ruin at 100% suppression;
- every occupant a survivor or a fallen soldier (10 inside before);
- a squad sent across the ruin routing round it;
- every capture framing the building clear of the panel.

Every scene stays green (`bun run --cwd web scene`: 109 checks across all fixtures). `bun run check` is green.

**Visual gate:** the scene (seed 11, 1280×800, DPR 1) captures five full frames: outside, firefight, firefight from the far side, collapse and ruin detour. It also captures 2× crops of the perimeter slots from both sides and of the ruin, in `throwaway/evidence/garrison/`. With no predecessor, the compare-screenshots single-image diagnostics ran on all eight images:

- Full frames: colour entropy 1.6–2.6 bits, dominant colour share 0.55–0.71, edge density 0.09–0.16, luminance contrast 85–115. The sparse flags are honest: the lab is a flat field with one building.
- Slot crops: entropy 1.2–1.3 bits, dominant share 0.70–0.71, edge density 0.05. These trip the "sparse" suspicion because the building's roof fills most of the crop.
- Ruin crop: entropy 3.0 bits, dominant share 0.52, edge density 0.10.

I inspected every frame and crop myself. Occupants stand on pads at the slots on the two facades facing each camera. The red tracer ends on the wall with impact rings, and blue tracers leave outward. The ruin is visibly lower and broken. The route runs round the ruin.

**Unprimed critique:** one fresh agent reviewed the first full set of frames and crops.

- **Acted on:**
  - The ruin read as a checkerboard floor. It is now a low dark slab strewn with jittered, turned heaps of brick and concrete tones, none above the 2 m collider.
  - The dark bands it read as stale shadows are the fog of war (ground blue cannot see; a 2 m ruin still hides the ground behind it from 1.6 m eyes). The legend now names it.
  - The dark square under a lone squad was a fog bug: an observer's own cell was never marked seen. Fixed in `visibility::sweep`.
  - Grey discs with blue bars were blue's fallen, which the legend did not explain. It now has "blue fallen", "selected squad's route" and "ground blue cannot see".
  - Only two facades were visible from one camera. The scene adds a far-side capture and crop.
  - The wall hid half of each occupant pad. The pad now fits inside the 0.45 m standoff.
  - The pinned state of a squad inside was not drawn, because its halo would cover the building. Occupant pads now edge toward orange as the squad is pinned.
  - "Health 100%" with two soldiers left was misleading. The bar is now strength against the full squad.
  - A destroyed squad vanished from the list. It now reads "eliminated".
  - "Leave building" stayed enabled with nobody inside. It is now enabled only when a selected squad is in a building.
  - The outside frame was too wide to read. It is framed closer.
- **Kept, with reason:**
  - The thick tracer bars and the browser-default meter colours are the shared lab conventions from slices 07 and 09.
  - Impact rings at the base of a struck wall are drawn at ground level, as in slice 09.
  - Routes and rounds run off the frame edges because they are long.
  - The panel's size is the shared lab layout.
  - "rifle #0. rifle #1" is a misreading: the log text is joined with a comma.

**Deviations:**

- **Preview-shots:** not offered; the run is unattended.
- **Lab story:** red's tank shells the garrison from 250 m on its squad's spotting, rather than being provoked by blue. An infantry attack order on a tank selects no compatible mount, so blue's grenadier would have killed the spotter instead.
- **Fixture values added:** `garrison.entry_distance_m` (4), `garrison.slot_standoff_m` (0.45) and `garrison.slot_facing_min_deg` (6). The existing `sensors.building_range_multiplier` (0.2) is now used for garrison concealment. This is recorded in choices.md as open to review.
- **New code:** `Rules.buildings` and `Rules.garrison`; the building fields on `CoverRules`; `SuppressionRules.collapse_level`; `WorldGeometry::segment_clear_except`; `buildStandingStructures` and `buildGarrisonOverlay` in the renderer; and the lab map `fixtures/garrison-lab.json`.
- **Harness:** the garrison scene clears React's development performance measures as it fast-forwards. Thousands of ticks otherwise exhaust them ("Data cannot be cloned, out of memory"). This is noted for slice 16.
- The implementation choices are recorded in choices.md (entries marked "When: slice 11").
