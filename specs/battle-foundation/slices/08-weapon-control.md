# 08 — Independent weapons and engagement policy

**Status:** complete 2026-09-25. **Dependencies:** 03, 04, 05, 06, 07. **Milestone:** Village checkpoint.

## Contract and question

Do target selection and simultaneous aim/reload obey every interruption and policy rule?

User requirements owned or exercised: V07, V10, V12, V13, W01, W02, W03, W04, W05, W06, W07, W08, W09, W10, W11, W12, W13, W14, W15, W16, W17, W18, P11, U01. Read their canonical entries in [requirements](../requirements.md), then the applicable [implementation contracts](../contracts.md). Do not infer rules from a fixture label.

## API seam and ownership

`sim::weapons::advance(own_state, observation, order, dt) -> LaunchRequest + WeaponReadiness`; unit-owned Engagement/OrderQueue.

## Runnable artifact

/lab/weapons: infantry rifle+grenade and tank cannon+HMG, moving/Stop/visibility events, switch policies, attack identified/contact/ground and attack-move. Timers are raw diagnostic bars until slice 14. Cannon ammo variants share one mount.



## Verification and verdict

All contracts.md transition rows; 45-tick grace advances without hidden tracking; stationary resets; target lock through shot; explicit override; cost priority; general-purpose uncertain fire; invulnerable default-gun fallback; per-unit policy switch; attacker-specific return permission; automatic no-pursuit; explicit last-known pursuit; vehicle-only prefire withholding; rifle bodies remain hittable. Speculative exchanges may persist.

Run relevant native Rust tests and browser/TypeScript seam tests; run typecheck/lint and the registered scene without console/GPU errors. Keep prior accepted slice contracts green. Source test names in [research](../research.md) are reuse references, not evidence that these new tests already exist. Record measured results and any scope deviation in this slice and the README handoff.

Visual variable: **Action reason correctness**. Review crop/mask: **Selected-unit diagnostic action panel**. Explicitly out of scope: Final circular timer styling and damage balance.

1. Freeze fixture seed, tick, camera, viewport and DPR. The registered scene regenerates the full frame and tight 2×–4× crops with machine/config metadata in gitignored `throwaway/evidence/<fixture-id>/`; the verdict records metrics and dispositions.
2. Use [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) for candidate versus any previous/reference shot. State the target for this variable, report telemetry and a less-wrong verdict; do not reward mere baseline matching. With no predecessor, use the skill's single-image diagnostics and inspect their meaning.
3. **As the last visual acceptance check, run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) with an unprimed agent given only the actual candidate full frames, crops and neutral inspection task.** Inspect and record its actionable findings before accepting. Do not substitute implementer self-review.
4. Offer the relevant shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md). This is a **non-blocking** course-correction window of about five minutes. Continue independent work while it is open, using waits no longer than 60 seconds. If no answer, decide reversible matters on evidence, record the choice, close the opened Preview set and proceed. Silence does not authorize changing user rules.

A visual pass is not a substitute for behavioral tests. A failing contract or serious visual defect means fix or reslice, not quietly update a golden image.

## Additional handoff regression

Also test visibility loss → approximate contact → reidentification with automatic targeting enabled: grace must not be destroyed by fallback. Test loaded AP/HE swap and stock conservation, reload-kind interruption, and no AP speculative shot.

## Decision budget

Delegated: Table organization/internal target cache, never alternative policy behavior. Also permitted: internal naming and reversible diagnostic formatting. Every other consequential freedom is a spec gap: record and resolve it before expanding implementation. No compatibility shims, duplicate owners, historical renderer adapters or undocumented gameplay changes.

## Human feedback that changes the slice

Contradictory action reasons or repeated target churn triggers a transition-table revision, not ad hoc flags.

## Carried in from slice 06

Publish each side's visible projectile segments, clipped to what its units can observe (the ground visibility field plus line of sight). Hidden muzzle flashes, lights and trails must never disclose a true launch point beyond the firing contact that `Battle::record_fire` creates. Every weapon launch calls `record_fire`.

## Verdict — 2026-09-25

`sim::weapons` owns the mounts. Each mount has one lock, one aim and one reload. It chooses targets only from its side's knowledge, and fires only through `flight::solve_launch_past` then `flight::launch_along`. Every launch is recorded with `Battle::record_fire`. `Battle` flies rounds against soldier capsules and vehicle hulls, adds the projectile store to its digest, and publishes each side's rounds: its own whole, and the enemy's clipped to its visible ground. The attack orders (`attack` on identified/contact/ground, `attack_move`, `set_engagement`) travel the one command path.

**Native tests** (`cargo test -p sim --test weapons`, 24). They cover:

- selection order and cost priority;
- uncertain fire from general-purpose weapons only, and never AP at an area;
- the default-gun fallback at invulnerable targets;
- the lock held through its shot, and a loaded weapon dropping a target it can no longer reach;
- the explicit override;
- stationary reset on movement;
- aim/reload overlap;
- the AP/HE swap conserving stock, and the reload restarting on a kind change;
- the 45-tick grace, with automatic targeting on and a last-seen area available;
- grace expiry, with reacquisition from zero under a new handle;
- an explicit attack on the area during the grace, whose abandoned aim is not restored;
- the per-unit policy switch, and return fire at the attacker only;
- attack orders switching to fire at will;
- no automatic pursuit, explicit pursuit to firing range, and pursuit of the last report (never the hidden unit);
- attack-move halting and resuming, halting for a target only a stationary weapon reaches, and never for one it cannot hurt;
- vehicle-only prefire withholding, and friendly infantry staying hittable;
- enemy tracers lying over seen ground at both ends.

A fresh code review found problems, all now fixed and pinned by tests:

- Valid-but-unengageable locks were never replaced.
- Pursuit and halting read display reasons. Both now come from `weapons::Reach`, which `advance` returns per unit.
- Stationary weapons never halted an attack-move.
- An attack-move halted for invulnerable targets.
- An ended order's lock reset its aim.
- Reasons went stale.
- Tracers reached into unseen ground.
- The digest was missing lock targets, reasons, reach and the planned goal, and did not tag optional values.
- A ground target used the client's z value.

**Browser scene** (`bun run --cwd web scene -- weapons`, 7 checks):

- AP lock on the identified tank;
- the 29-tick grace keeping the same handle;
- visible flight segments;
- rifles preferring a firing area over an invulnerable tank;
- attack on ground;
- the per-unit policy switch;
- attack-move with its goal.

Every other scene stays green. `bun run check` is green.

**Visual gate:** single-image diagnostics ran on 4 frames and 3 panel crops. An unprimed critique followed.

- **Acted on:**
  - The reload bar read backwards (empty when loaded), so a loaded weapon now shows it full.
  - Rounding made an unfinished timer read complete, so values are now rounded down to two decimals.
  - All reasons shared one yellow, so reasons are now coloured by family: acting, waiting on a timer, or prevented.
  - The header's "idle" was ambiguous, so it is now "movement: idle".
  - "moving" showed while holding to engage, so there is a new published `halted` state.
  - Target names changed while out of sight, so a target is now always "enemy N · kind" or "enemy N · out of sight".
  - The command log had no empty state, so it now shows one.
  - The red tank sat on the frame edge, so the layout is compressed and the camera reframed.
- **Kept, with reason:**
  - Selection highlight and contact-ring colours are shared conventions from slices 04 and 06.
  - Tracers are not attributed to a weapon: the per-weapon readout is slice 14.
  - Raw weapon identifiers stay until slice 14's readouts.
  - Shadow aliasing is renderer-wide.
- **Preview-shots:** not offered; the run is unattended.

**Deviations:**

- `MoveState::Halted` is new.
- The mount records in `game.json`, and the `moving_scatter_multiplier` and `friendly_prefire_margin_m` physics keys, are new (see choices.md).
- Lab fixture: `fixtures/weapons-lab.json`.
- **Deferred to slice 09 (both delivered there):**
  - the suppression transition row (reload scaled by suppression);
  - return-fire permission from area fire that lands on a unit. Today only a shot aimed at the unit grants it.
- **Enemy tracer clipping:** it uses the side's ground-visibility field, refreshed every 6 ticks, and each drawn stretch ends at its last seen sample. It does not add a per-round line-of-sight cast.
