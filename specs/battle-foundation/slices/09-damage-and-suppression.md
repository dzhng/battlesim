# 09 — Consequences of physical fire

**Status:** complete 2026-09-25. **Dependencies:** 07, 08. **Milestone:** Village checkpoint.

## Contract and question

Do impacts and near misses produce the intended tactical consequences?

User requirements owned or exercised: V03, P09, P10, P13, P14, M06, M07, M08. Read their canonical entries in [requirements](../requirements.md), then the applicable [implementation contracts](../contracts.md). Do not infer rules from a fixture label.

## API seam and ownership

`sim::damage::resolve(events, bodies, armor, cover) -> casualties/suppression/destruction`; health and suppression are sim-owned.

## Runnable artifact

/lab/consequences: exposed/forest infantry, near-miss pinning, tank front/side/rear/roof, friendly collateral, accumulating corpses and blocking vehicle wrecks. Persistent remains use the normal world obstacle/render owners.



## Verification and verdict

Penetrating damage fixed after face check; failed penetration stops with zero HP loss; no overpenetration; near misses suppress without damage; suppression slows rather than retreats; lull recovery; blast per soldier and friendly teams; cover probability differs but conditional damage does not; late wreck invalidates known route; corpse stays without blocking navigation.

Run relevant native Rust tests and browser/TypeScript seam tests; run typecheck/lint and the registered scene without console/GPU errors. Keep prior accepted slice contracts green. Source test names in [research](../research.md) are reuse references, not evidence that these new tests already exist. Record measured results and any scope deviation in this slice and the README handoff.

Visual variable: **Impact, suppression and remains legibility**. Review crop/mask: **Exposed/covered pair plus wreck passage**. Explicitly out of scope: Garrison wall abstraction, smoke art and particle density.

1. Freeze fixture seed, tick, camera, viewport and DPR. The registered scene regenerates the full frame and tight 2×–4× crops with machine/config metadata in gitignored `throwaway/evidence/<fixture-id>/`; the verdict records metrics and dispositions.
2. Use [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) for candidate versus any previous/reference shot. State the target for this variable, report telemetry and a less-wrong verdict; do not reward mere baseline matching. With no predecessor, use the skill's single-image diagnostics and inspect their meaning.
3. **As the last visual acceptance check, run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) with an unprimed agent given only the actual candidate full frames, crops and neutral inspection task.** Inspect and record its actionable findings before accepting. Do not substitute implementer self-review.
4. Offer the relevant shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md). This is a **non-blocking** course-correction window of about five minutes. Continue independent work while it is open, using waits no longer than 60 seconds. If no answer, decide reversible matters on evidence, record the choice, close the opened Preview set and proceed. Silence does not authorize changing user rules.

A visual pass is not a substitute for behavioral tests. A failing contract or serious visual defect means fix or reslice, not quietly update a golden image.

## Decision budget

Delegated: Impact glyphs and sparse particles; probability sampling must be deterministic. Also permitted: internal naming and reversible diagnostic formatting. Every other consequential freedom is a spec gap: record and resolve it before expanding implementation. No compatibility shims, duplicate owners, historical renderer adapters or undocumented gameplay changes.

## Human feedback that changes the slice

If exposed and covered results differ only through damage reduction, or one lucky seed supports acceptance, reject.

## Verdict — 2026-09-25

`sim::damage` resolves each tick's flight events in order, inside `Battle::fly`:
- **Direct hits:** a soldier takes the weapon's damage. A vehicle takes it only if the round penetrates the struck face (P10); a failed penetration stops the round with zero loss.
- **Blast:** sampled per soldier. Hit probability is `(1 − (r/R)²) × cover exposure`; damage is `damage × (1 − r/R)`. It is occluded by solid props, skips the directly struck body, and spares no team.
- **Suppression:** from near misses and nearby impacts, the strongest per projectile per squad per tick. It is clamped to [0, 1], held through a 3 s lull, then decays.
- **Consequences (`Battle::consequences`):**
  - hostile damage or suppression grants return fire;
  - a destroyed vehicle becomes a permanent `wreck` prop, which each side learns by sight and reroutes around;
  - a death a side watched ends its track;
  - fallen soldiers stay as corpse records that block nothing.
- **Suppression effects:** it slows infantry movement (`max_move_penalty`) and scales reload progress (`max_reload_cycle_penalty`) without resetting it. This is the transition-table row slice 08 deferred.
- **Cover:** forest ground cover at the aimed point widens incoming spread, and never softens a hit.

**Native tests:**
- `cargo test -p sim --test damage` (14):
  - fixed damage per penetrating hit, and zero for a failed penetration;
  - face selection at each face and at corners;
  - a wreck that reroutes the side that saw it. The watched death completes the attack with no last-seen area, a side with no eyes there learns of no wreck, and orders to the destroyed unit are refused;
  - near misses suppressing without damage, clamped at 1;
  - suppression slowing without retreat, and recovering after the lull;
  - blast per soldier across both teams, over 12 seeds;
  - a wall shielding soldiers inside the blast radius, over 6 seeds;
  - the same HE and HMG fire at a squad in the open and 45 m into a forest over 16 paired seeds: the forest loses less;
  - corpses staying put and blocking nothing, with a tank routed straight over one;
  - return fire granted by untargeted damage;
  - forest spread from the first metre;
  - identical replays.
- Library test `damage::tests` (paired trial over 20 000 rolls): cover scales fragment-hit probability to the configured multiplier (±0.03), and every covered hit does exactly the exposed hit's damage.
- No overpenetration is pinned by `flight_collision`.

A fresh code review found problems, all fixed:
- A soldier killed earlier in a tick still took later hits. Soldier liveness is now `hp > 0`; the separate flag is gone.
- Zero-strength near misses reset the lull.
- The penalty factors could go negative.
- The firing unit's own impacts suppressed it.
- The digest was missing health, suppression and corpses, and knowledge sets lacked length prefixes.
- Enemy corpses were published grouped by their hidden unit; they are now sorted by place.

**Browser scene** (`bun run --cwd web scene -- consequences`, 6 checks):
- the wreck is where the tank stood;
- the tank is no longer identified;
- the truck's route avoids the wreck;
- HE kills in the open and blue sees the fallen;
- the friendly squad nearby is suppressed;
- the fallen stay.

Every other scene stays green; the sensors and contacts labs now hold fire. `bun run check` is green.

**Visual gate:** single-image diagnostics ran on 5 frames and 4 crops. An unprimed critique followed.
- **Acted on:**
  - The wreck and its detour weren't framed, so each shot now has its own camera, and there is a frame of the tank alive before the kill.
  - Fallen soldiers were illegible, so they are now lighter, tinted by side, and lie on a pale ground mark.
  - The meters read backwards, so they now use low/high/optimum.
  - Nothing showed where rounds landed, so there are scorch rings for 2 s, published through a new `impact` flag on visible segments.
  - Tracers were ambiguous, so they are now coloured by side and in the legend.
  - The halo was faint, so it now has a stronger fill and a crisp rim.
  - The crops missed their subjects, so there is now one per squad.
  - Firing durations were unequal, so each squad now gets the same 20 s; casualties go to `casualties.txt`, and the Rust paired trial owns the rate claim.
- **Kept, with reason:**
  - The selection highlight replacing team colour is the shared convention since slice 04.
  - Units drawn over the forest canopy is the renderer-wide forest representation.
- **Preview-shots:** not offered; the run is unattended.

**Changed contracts:**
- A unit's rounds no longer strike its own soldiers. The slice 07 flight test was repinned (see choices.md).
- Turrets now start on the hull's bearing. This was a slice 08 bug: turrets started pointing east.
