# 15 — Replayable combined-arms encounter

**Status:** complete 2026-09-25. **Dependencies:** 04, 06, 09, 10, 11, 13, 14. **Milestone:** Village checkpoint.

## Contract and question

Does the combined encounter reward scouting, suppression, flanking and rotation?

User requirements owned or exercised: N01, N04, N05, N06. Read their canonical entries in [requirements](../requirements.md), then the applicable [implementation contracts](../contracts.md). Do not infer rules from a fixture label.

## API seam and ownership

`ScenarioDefinition` + `OpponentPolicy(ObservationFrame) -> commands` compose existing owners; no new combat logic in scenario/UI.

## Runnable artifact

/battle/village and /replay/village with authored setup, defensive bot, reset/pause/seed/replay controls and fixture-specific hold condition. Use encounter.md scripts and acceptance targets; all tactical play uses real commands and same observation boundary.



## Verification and verdict

All ten-seed tactical comparisons and service roundtrip from encounter.md; same-build replay digest; bot hidden-state metamorphic test; reset resource cleanup; full UI command path; no scenario damage cheats. Archive result tables and full image set, including failures.

Run relevant native Rust tests and browser/TypeScript seam tests; run typecheck/lint and the registered scene without console/GPU errors. Keep prior accepted slice contracts green. Source test names in [research](../research.md) are reuse references, not evidence that these new tests already exist. Record measured results and any scope deviation in this slice and the README handoff.

Visual variable: **Composed tactical readability**. Review crop/mask: **Whole village frame plus scout/ambush/supply crops**. Explicitly out of scope: Full match economy, aircraft and finished art.

1. Freeze fixture seed, tick, camera, viewport and DPR. The registered scene regenerates the full frame and tight 2×–4× crops with machine/config metadata in gitignored `throwaway/evidence/<fixture-id>/`; the verdict records metrics and dispositions.
2. Use [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) for candidate versus any previous/reference shot. State the target for this variable, report telemetry and a less-wrong verdict; do not reward mere baseline matching. With no predecessor, use the skill's single-image diagnostics and inspect their meaning.
3. **As the last visual acceptance check, run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) with an unprimed agent given only the actual candidate full frames, crops and neutral inspection task.** Inspect and record its actionable findings before accepting. Do not substitute implementer self-review.
4. Offer the relevant shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md). This is a **non-blocking** course-correction window of about five minutes. Continue independent work while it is open, using waits no longer than 60 seconds. If no answer, decide reversible matters on evidence, record the choice, close the opened Preview set and proceed. Silence does not authorize changing user rules.

A visual pass is not a substitute for behavioral tests. A failing contract or serious visual defect means fix or reslice, not quietly update a golden image.

## Additional handoff regression

Ordinary disables Red spawn index 4; crossfire enables it. Verify the own-optics ambush trigger and that replay does not rerun the defender.

## Decision budget

Delegated: Authored patrol/defense micro within own observation; map offsets/tuning with recorded paired evidence. Also permitted: internal naming and reversible diagnostic formatting. Every other consequential freedom is a spec gap: record and resolve it before expanding implementation. No compatibility shims, duplicate owners, historical renderer adapters or undocumented gameplay changes.

## Human feedback that changes the slice

If village is not replayably interesting, revise scenario/config or split the failing mechanic; do not declare full-game completion.

## Verdict — 2026-09-25

**Result: accepted as the village checkpoint, with one encounter target unmet and split out** (see below and decisions.md "15 — Village tuning log").

**What was built:**
- `sim::village` builds the authored scenario from `game.json` for either variant. Ordinary drops red spawn 4; crossfire keeps it.
  - `Defender` is the red policy. It reads only red's `ObservationFrame` and issues ordinary commands through `Battle::accept`: garrisons on the first tick, one own-optics AT attack, and a single retreat per unit.
  - `Referee` judges the hold from authoritative state and publishes `ObservationFrame.encounter` (seconds held, verdict).
  - `trial` runs a script against the defender.
- `sim::village::scripts` holds the four comparison scripts: blue player stand-ins using only blue's observation and public map/objective. `cargo run -p sim --release --example village_report [max_s] [script]` prints the ten-seed table.
- The scenario and opponent enter `Battle`. The defender runs after each observation, and a replay applies both sides' accepted commands with the defender and input off. The scenario digest now covers the opponent and encounter rules.
- `weapons::facade`: a ground attack on a building aims at its wall, from the real muzzle.
- Routes:
  - `/battle/village` has the production readouts and command bar, a variant select, seed, pause/resume, reset, a replay download, and the hold zone drawn on the ground.
  - `/replay/village` opens the last save or a picked file, with input off. A replay for another scenario or config shows the simulation's mismatch error.
- Fixes found on the way:
  - An exception while building a battle in the worker is now reported (`authority.ts`).
  - Our own GPU device `destroy()` is no longer logged as a fault.
  - `onViewportReady` handles a failed start.
  - The ground ring has one owner (`mesh.groundRing`).

**Tests:**
- `crates/sim/tests/village.rs`, 7 tests:
  - variant spawns and AT policy;
  - three garrisons, accepted once;
  - the AT attacks only after its own sighting and held fire before it;
  - a hidden blue change leaves red's frames and orders identical (metamorphic);
  - replay matches every tick digest, issues no commands of its own, and another variant refuses it;
  - the referee captures, contests and defeats;
  - a scripted trial repeats from its seed with no refused orders.
- `web/tests/observation.test.ts` decodes the encounter status.
- Scenes: `village` (9 checks) and `village-replay` (4 checks: digest parity against a played battle, status, input off, mismatch refused).

**Tactical comparison** (seeds 1,2,3,5,8,13,21,34,55,89; 900 s):

| script | captured | blue cost lost | tank losses | a tank survived | rejoined |
|---|---|---|---|---|---|
| unsupported-road-push | 0/10 | 4762 | 14 | 6/10 | 0 |
| scout-suppress-flank | 8/10 | 1040 | 0 | 10/10 | 5 |
| ordinary ambush, retreat 0.75 s | 0/10 | 1205 | 0 | 10/10 | 0 |
| ordinary ambush, retreat 3 s | 0/10 | 1582 | 3 | 10/10 | 0 |
| prepared crossfire, 0.75 s | 0/10 | 1205 | 0 | 10/10 | 0 |

- Supported captures ≥7/10 at fewer cost points than the unsupported push: **met**.
- Prompt retreat loses fewer tanks than delayed, with a survivor in ≥7/10: **met**.
- Crossfire produces ≥1 tank loss: **not met**. The tanks turn back on the red tank's first long-range hit, about 400 m short of either AT team. Split to slice 10's ambush lab, which shows a crossfire beating a prompt escape.
- Service roundtrip: depletion, repair, refill and replacement are in slice 13's supply tests and scene. Re-entry into combat is the "rejoined" column: 5 served units returned to the fight.

**Visual gate:**
- The unprimed critique, acted on:
  - The objective was not marked; the hold zone is now a dashed ring, solid while held.
  - The Pause button contradicted a paused status; it now follows the authority.
  - Blue's start sat under the panel edge; the opening camera was reframed.
  - The replay said "seed from file"; it now shows the file's seed.
- Kept, with reason:
  - The lab panel covers part of the scene (lab convention, as in slice 14).
  - There is no replay timeline or scrubber; the spec asks for export and import only.
  - Tracers, contact areas and wrecks keep their slice 06/09 meanings.
  - Units are small at the overview distance.
- Preview-shots: not offered; the run is unattended.

**Code review, acted on:**
- The scenario digest now pins the opponent and encounter.
- The supported script no longer stalls while armour stays identified, and never sends an order with no units.
- `trial` counts refused orders, and the report asserts none.
- The AT makes one attack.
- A bad replay variant and a failed scenario build show errors.
- Views are keyed per loaded file and never pair a replay with the other variant's scenario.
- `facade` uses the real muzzle.
- Script and route constants now come from the fixture.
- The download URL is revoked late.
- Recorded rather than changed: the public hold status (choices.md, needs user).
