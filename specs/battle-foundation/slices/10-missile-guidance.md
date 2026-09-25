# 10 — Supported AT ambush

**Status:** complete 2026-09-25. **Dependencies:** 05, 06, 07, 08, 09. **Milestone:** Village checkpoint.

## Contract and question

Can launcher visibility and prompt movement change an AT engagement without hidden homing?

User requirements owned or exercised: P05, P06. Read their canonical entries in [requirements](../requirements.md), then the applicable [implementation contracts](../contracts.md). Do not infer rules from a fixture label.

## API seam and ownership

`sim::flight::GuidanceSupport { launcher, target_observation, last_point }`; own sensor feed from sensing, support slots from weapon state.

## Runnable artifact

/lab/ambush: two tanks, AT team, hill escape, shared scout-only spotting, launch/Stop/move and last-point replay. Use ordinary and prepared crossfire variants of the same weapon data.

These are planned routes, not existing routes. Register this fixture with the one lab/scene registry and reuse production owners. No route-only copy of gameplay. The expected run entry is `bun run dev`; the deterministic verification entry is `bun run --cwd web scene -- <fixture-id>` after slice 01 establishes the task runner. Fixture IDs and supported scene invocation must be documented by that registry, not guessed from a filename.

## Verification and verdict

Own-ID launch and sustain; shared ID insufficient; movement/Stop/death/LOS releases support immediately; missile never reacquires; frozen point remains fixed despite hidden motion; stationary target at that point takes normal damage; same trajectory owner; bounded turn/lifetime; crew free despite missile still flying. Compare prompt/delayed retreat on fixed seeds.

Run relevant native Rust tests and browser/TypeScript seam tests; run typecheck/lint and the registered scene without console/GPU errors. Keep prior accepted slice contracts green. Source test names in [research](../research.md) are reuse references, not evidence that these new tests already exist. Record measured results and any scope deviation in this slice and the README handoff.

Visual variable: **Guidance and escape legibility**. Review crop/mask: **Missile/launcher-target corridor with observed point markers in lab only**. Explicitly out of scope: Radar/seeker AA, jet flight and cinematics.

1. Freeze fixture seed, tick, camera, viewport and DPR. The registered scene regenerates the full frame and tight 2×–4× crops with machine/config metadata in gitignored `throwaway/evidence/<fixture-id>/`; the verdict records metrics and dispositions.
2. Use [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) for candidate versus any previous/reference shot. State the target for this variable, report telemetry and a less-wrong verdict; do not reward mere baseline matching. With no predecessor, use the skill's single-image diagnostics and inspect their meaning.
3. **As the last visual acceptance check, run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) with an unprimed agent given only the actual candidate full frames, crops and neutral inspection task.** Inspect and record its actionable findings before accepting. Do not substitute implementer self-review.
4. Offer the relevant shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md). This is a **non-blocking** course-correction window of about five minutes. Continue independent work while it is open, using waits no longer than 60 seconds. If no answer, decide reversible matters on evidence, record the choice, close the opened Preview set and proceed. Silence does not authorize changing user rules.

A visual pass is not a substitute for behavioral tests. A failing contract or serious visual defect means fix or reslice, not quietly update a golden image.

## Decision budget

Delegated: Guidance steering numeric solver within configured turn limits. Also permitted: internal naming and reversible diagnostic formatting. Every other consequential freedom is a spec gap: record and resolve it before expanding implementation. No compatibility shims, duplicate owners, historical renderer adapters or undocumented gameplay changes.

## Human feedback that changes the slice

Retreat always fails/succeeds regardless of geometry indicates tuning or state error; retain N03 tradeoff.

## Verdict — 2026-09-25

**What was built:**
- A weapon row with `turn_deg_s` is guided. `flight::Guidance { point, turn_rad_s, supported }` steers a gravity-free, constant-speed round within its turn limit. Launch solving, `predicted_path` and flight share `LaunchProfile::gravity`.
- The launching mount holds `weapons::Support { projectile, target }`.
- `Battle::guide` runs each tick before flight. It renews the point from the launcher's own last sighting while the launcher stands still and lives. Otherwise it releases the missile: `Projectiles::release` fixes the point on the ground beneath, for good.
- Launching needs the launcher's own identification (reason `no_own_sight`). A second launch waits while one missile is guided (reason `guiding`).
- Own guided missiles are published as `guided`: position, point, supported.

**Native tests** (`cargo test -p sim --test guidance`, 12):
- own-sight launch and support;
- shared sight not enough (`no_own_sight`);
- with a quick-reload rule variant, a ready round waits as `guiding`, and the crew launches again after a Stop while the released first missile still flies;
- the launcher's death releases to a ground point (rule variant: a one-soldier, 1 hp launcher);
- moving releases at once and the crew moves on;
- Stop releases, and the point never moves;
- a released missile never reacquires a tank that drives back into view;
- the in-flight heading never turns faster than the limit;
- a stationary target at the last point takes normal damage;
- prompt vs late retreat on seeds 6–8;
- a lifetime bound;
- identical replays with guidance in flight.

A fresh code review found problems, all fixed:
- The "next round waits" and death-release tests were missing or never exercised.
- Tracks and own-sensor lists, now read a tick later by guidance, were missing from the knowledge digest.
- Guidance started at the unled point, while launch was checked against the solved intercept.
- Selection panicked on a firing area centred off the map; it now skips it.
- The shot's mount index was inferred from its spec index.

**Browser scene** (`bun run --cwd web scene -- ambush`, 9 checks):
- launch and guidance;
- release to a fixed ground point on sight loss;
- the point stays fixed;
- a prompt escape is not hit (red view: hp 100);
- a scout-only target reads "no own sight";
- a late escape is hit;
- moving releases and frees the crew;
- a crossfire launches twice and beats a prompt escape.

Every other scene stays green, and `bun run check` is green.

**Visual gate:** an unprimed critique of 4 frames and 2 crops.
- **Acted on:**
  - The released point was drawn as a ring on the tank, reading as still locked; it is now a ground cross.
  - Launchers and the scout were invisible; they now have ground rings and legend entries.
  - Nothing showed the outcome; strike marks now linger 3 s and the panel prints outcome lines.
  - Tracers were missing from the legend; they are now in it.
  - "The building" was ambiguous; the text now says "the building beside it", and teams are named west and south-east.
  - The framing was poor; the camera now looks east from behind the launcher, with close-ups of the cover.
  - The capture set was incomplete; it now includes late-escape, launcher-moved and crossfire in-flight and outcome frames.
- **Kept, with reason:**
  - The world-anchored readout icon is slice 14.
  - The shadow aliasing is renderer-wide.
- **Preview-shots:** not offered; the run is unattended.

**Deviations:**
- A missile released by movement is judged on actual movement, as W03 does for stationary weapons, not on the Move order itself.
- A guided missile flies pure pursuit on the observed position, while launch checks use the solved straight line; the two agree at launch and part only against a moving target.
- A slice 08 test changed: `attack_move_halts_for_what_only_a_stationary_weapon_reaches` now puts the tank inside the AT squad's own sight, because P05 forbids launching on the scout's identification.
- Release on garrison transitions: once slice 11 merged, `Battle::guide` also releases while the launcher is entering or leaving a building. There is no dedicated test, because moving to a building already releases, so only an exit from inside differs. Embark belongs to slice 17.
