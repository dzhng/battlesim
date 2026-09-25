# 12 — Reversible deployment progress

**Status:** complete 2026-09-25. **Dependencies:** 03, 04, 08. **Milestone:** Village checkpoint.

## Contract and question

Does setup reverse cleanly under move and cancellation without losing time or trapping a unit?

User requirements owned or exercised: W15, L01, L02. Read their canonical entries in [requirements](../requirements.md), then the applicable [implementation contracts](../contracts.md). Do not infer rules from a fixture label.

## API seam and ownership

`sim::deployment::advance(current, desired, duration) -> readiness + movement_gate`; unit-owned, used by later service and weapons.

## Runnable artifact

/lab/deployment: supply proxy with deploy/pack/Stop and queued movement; inspect one progress value and desired end state. State controls pose interpolation, not vice versa.



## Verification and verdict

Equal forward/back duration; 40%-deployed unit needs 40% duration to pack; 50%-packed reversal needs half duration to deploy; no translation until packed; no service before fully deployed; Stop clears move and returns desired deployed; repeated reversals do not reset or create stock/actions.

Run relevant native Rust tests and browser/TypeScript seam tests; run typecheck/lint and the registered scene without console/GPU errors. Keep prior accepted slice contracts green. Source test names in [research](../research.md) are reuse references, not evidence that these new tests already exist. Record measured results and any scope deviation in this slice and the README handoff.

Visual variable: **Deployment progress direction**. Review crop/mask: **Selected supply proxy and progress indicator**. Explicitly out of scope: Authored animation and actual supplies.

1. Freeze fixture seed, tick, camera, viewport and DPR. The registered scene regenerates the full frame and tight 2×–4× crops with machine/config metadata in gitignored `throwaway/evidence/<fixture-id>/`; the verdict records metrics and dispositions.
2. Use [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) for candidate versus any previous/reference shot. State the target for this variable, report telemetry and a less-wrong verdict; do not reward mere baseline matching. With no predecessor, use the skill's single-image diagnostics and inspect their meaning.
3. **As the last visual acceptance check, run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) with an unprimed agent given only the actual candidate full frames, crops and neutral inspection task.** Inspect and record its actionable findings before accepting. Do not substitute implementer self-review.
4. Offer the relevant shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md). This is a **non-blocking** course-correction window of about five minutes. Continue independent work while it is open, using waits no longer than 60 seconds. If no answer, decide reversible matters on evidence, record the choice, close the opened Preview set and proceed. Silence does not authorize changing user rules.

A visual pass is not a substitute for behavioral tests. A failing contract or serious visual defect means fix or reslice, not quietly update a golden image.

## Decision budget

Delegated: Primitive folded/unfolded pose; no new state flags duplicating progress. Also permitted: internal naming and reversible diagnostic formatting. Every other consequential freedom is a spec gap: record and resolve it before expanding implementation. No compatibility shims, duplicate owners, historical renderer adapters or undocumented gameplay changes.

## Human feedback that changes the slice

Confusing cancellation feedback requires a presentation fix unless the user explicitly changes reversal semantics.

## Verdict — 2026-09-25

`sim::deployment` owns setup. Each deploying unit (today only the supply truck) holds one progress value, counted in whole ticks, and one stationary posture. The target that progress heads to is packed while the unit has a movement goal; otherwise it is the stationary posture. The seam `deployment::advance(current, target, duration)` returns the new progress, `fully_deployed` and `may_translate`. `Battle::step` advances every unit's progress before movement. `movement::step_unit` holds a unit with some setup remaining in the new `packing` state, with no translation and no turning. `deployment::fully_deployed(unit)` is the readiness predicate for slice 13's service. The new `set_deployment` order (Deploy/Pack) travels the one command path. Progress and its target enter `Battle::digest`, with a presence tag, and are published as `deployProgress`/`deployTarget` on own units.

**Native tests** (`cargo test -p sim --test deployment`, 12). They cover:

- the seam: one duration each way, reversal from current progress, saturation, and the readiness and gate flags;
- deploying and packing each taking exactly 450 ticks;
- a 40%-deployed truck packing in exactly 40% of the duration, in place, reporting `packing`, then driving off;
- a 50%-packed reversal (Stop) redeploying in exactly half the duration;
- Stop clearing the move and the queue and returning the target to deployed, with setup resuming on the next tick;
- Stop after an explicit Pack setting up again;
- no service readiness on any tick before full deployment, and none once packing starts; units that never deploy are never ready;
- a queued move while deployed packing first, moving on the tick packing completes, and setting up on arrival;
- an explicit Pack holding a stopped truck packed, and Deploy cancelling a move so the truck sets up where it stopped;
- 24 uneven move/Stop reversals: progress moves exactly one tick per tick, never jumps or resets, no orders pile up and no actions appear;
- a deploy order leaving non-deploying units untouched;
- deployment state changing the digest, and replays reproducing identical digests.

Every earlier native suite stays green. The vitest suite gained a decode test (progress, target, `packing`, and the `postures` names from the published layout) and an interpolation test (progress blends between ticks). The observation tests now pack a frame before reading its pointer, because packing can move the buffer.

**Browser scene** (`bun run --cwd web scene -- deployment`, 13 checks):

- a stopped truck deploys from spawn toward deployed;
- 40% deployed: still packing and unmoved one tick short of 40% of the duration, fully packed at exactly 40%;
- movement starts only once packed;
- Stop clears the move and the queue, and the target returns to deployed;
- one tick before full deployment the panel says service is not ready; at full deployment, after the whole duration, it says ready;
- a queued move while deployed packs first, in place (captured at 75% deployed, then checked at 50%);
- a 50%-packed reversal redeploys in half the duration;
- packing takes the whole duration, and Pack holds the truck packed;
- eight uneven reversals leave progress at exactly the net ticks spent each way, with no orders left;
- every capture keeps the truck and its whole progress ring clear of the panel and the frame edges;
- every command acknowledged without error.

Every other scene stays green (`bun run --cwd web scene`: 90 checks across all fixtures). `bun run check` is green.

**Visual gate:** the scene captures five states (40% deploying, fully deployed, 75% packing, packed, after reversals). Before each capture it centres the camera on the truck with the same offset, so every state is framed identically. Each state gets a full 1280×800 frame, a 2× crop sized to hold the truck and its whole ring, and a 2× crop of the readout, in `throwaway/evidence/deployment/`. With no predecessor, the compare-screenshots single-image diagnostics ran on all 15 images:

- Full frames: colour entropy 1.7–1.9 bits, dominant colour share 0.59–0.64, edge density 0.07–0.10, luminance contrast 75–89. These trip the "sparse" suspicion honestly: the lab is a flat field with one road, two crates and one truck.
- Truck crops: entropy 2.0–2.6 bits, dominant colour share 0.33–0.42, edge density 0.07–0.10, contrast 155–182.

I inspected every frame and crop myself:

- The ring's arc length and colour matched the published progress and target in each state: a green 40% arc with a clockwise arrowhead; a full arc with the solid disc; an amber 75% arc with its arrowhead pointing back; a thin empty track.
- The legs and mast visibly extend at full deployment and hide completely when packed.
- The panel's bar and label use the ring's colours.

The camera was reframed twice. The first frames had the moved truck partly off the right edge; the framing check now projects the whole ring.

**Unprimed critique:** the orchestrator ran an unprimed screenshot critique on the first evidence set. The second commit responds to it:

- **Acted on:**
  - Direction was shown by colour alone (green versus amber, both filling clockwise), which is weak under red-green colour blindness. A white arrowhead now sits at the arc's moving end and points the way progress runs. The panel label carries matching ↻/↺ arrows.
  - It was unclear what the packing bar measured, and a 50% capture could not show it. The bar now reads "deployed 11.2/15.0 s", so the fill always means how deployed the unit is. Packing is captured at 75%.
  - The truck crops cut off the ring. Crops are now sized from the projected ring, at 2×.
  - The pink move path covered the arc's end. The ring now draws at 0.7 m, above route ribbons at 0.3 m.
  - Fully deployed looked like a nearly full deploying ring. It now adds a solid dark-green disc, and the label reads "✓ fully deployed".
  - The empty packed ring read as a heavy selection ring. The track is now a thin light-grey line.
  - The 40% capture was framed differently from the others. Every capture now centres on the truck with one fixed offset.
  - The lab's crates moved into view; they had ended up behind the panel.
- **Kept, with reason:**
  - The placeholder model poses: finished art is out of scope.
  - Truncation of the command log: it is a shared lab convention.
  - "movement: packing": `MoveState` is the movement field by design.

**Deviations:**

- **Preview-shots:** not offered; the run is unattended.
- **New published names:** the `packing` movement state, the own-unit fields `deployProgress`/`deployTarget`, and the layout's `postures` list.
- **New code:** `Rules.service` (`ServiceRules`, reading `deploy_and_pack_s`), `Battle::unit(id)` for tests and later owners, and the lab map `fixtures/deployment-lab.json`. No `village.json` value was added.
- **Not done:** no weapon needs deployment yet, so the "deploying" action reason and deployment-gated fire are not implemented.
- The implementation choices are recorded in choices.md (entries marked "When: slice 12").
