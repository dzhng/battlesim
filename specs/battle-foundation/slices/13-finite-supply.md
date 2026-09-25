# 13 — Recovery with finite stock

**Status:** complete 2026-09-25. **Dependencies:** 08, 09, 11, 12. **Milestone:** Village checkpoint.

## Contract and question

Can surviving forces recover while supply stock and action rules stay honest?

User requirements owned or exercised: L02, L03, L04, L05. Read their canonical entries in [requirements](../requirements.md), then the applicable [implementation contracts](../contracts.md). Do not infer rules from a fixture label.

## API seam and ownership

`sim::supply::service(deployed_source, eligible_recipients, stock) -> StockDebit/Recovery events`; deployment is consumed, never reimplemented.

## Runnable artifact

/lab/supply: damaged tank, depleted AT squad, casualty rifle squad, empty truck and incoming fire. Deploy, service, pack and relocate. Show stock and clear eligibility reason.

These are planned routes, not existing routes. Register this fixture with the one lab/scene registry and reuse production owners. No route-only copy of gameplay. The expected run entry is `bun run dev`; the deterministic verification entry is `bun run --cwd web scene -- <fixture-id>` after slice 01 establishes the task runner. Fixture IDs and supported scene invocation must be documented by that registry, not guessed from a filename.

## Verification and verdict

Stationary/non-firing recipient gate; incoming fire does not interrupt; shared stock atomic/no negative; round-robin service; reserve/loaded accounting; new soldier IDs preserve old corpses; eliminated squad not resurrected; no stock regeneration/self-refill; moving source cannot service; full deployment required.

Run relevant native Rust tests and browser/TypeScript seam tests; run typecheck/lint and the registered scene without console/GPU errors. Keep prior accepted slice contracts green. Source test names in [research](../research.md) are reuse references, not evidence that these new tests already exist. Record measured results and any scope deviation in this slice and the README handoff.

Visual variable: **Service eligibility and stock feedback**. Review crop/mask: **Supply radius with selected recipient and resource panel**. Explicitly out of scope: Supply animations, fuel and rear-depot refilling.

1. Freeze fixture seed, tick, camera, viewport and DPR. The registered scene regenerates the full frame and tight 2×–4× crops with machine/config metadata in gitignored `throwaway/evidence/<fixture-id>/`; the verdict records metrics and dispositions.
2. Use [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) for candidate versus any previous/reference shot. State the target for this variable, report telemetry and a less-wrong verdict; do not reward mere baseline matching. With no predecessor, use the skill's single-image diagnostics and inspect their meaning.
3. **As the last visual acceptance check, run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) with an unprimed agent given only the actual candidate full frames, crops and neutral inspection task.** Inspect and record its actionable findings before accepting. Do not substitute implementer self-review.
4. Offer the relevant shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md). This is a **non-blocking** course-correction window of about five minutes. Continue independent work while it is open, using waits no longer than 60 seconds. If no answer, decide reversible matters on evidence, record the choice, close the opened Preview set and proceed. Silence does not authorize changing user rules.

A visual pass is not a substitute for behavioral tests. A failing contract or serious visual defect means fix or reslice, not quietly update a golden image.

## Decision budget

Delegated: Service UI wording and internal iteration storage; priority/rates follow config. Also permitted: internal naming and reversible diagnostic formatting. Every other consequential freedom is a spec gap: record and resolve it before expanding implementation. No compatibility shims, duplicate owners, historical renderer adapters or undocumented gameplay changes.

## Human feedback that changes the slice

If service requires repeated manual micro or silently stalls, improve reason feedback/eligibility presentation.

## Verdict — 2026-09-25

**What was built:**
- `sim::supply::service` runs each tick after fire. A supply truck serves units that are fully deployed (`deployment::fully_deployed`), standing and in reach. It pays from `Unit.stock`: finite rounds first (at `round_costs`), then vehicle health, then replacement soldiers (new ids; the fallen stay).
- A recipient must be:
  - alive and own-side;
  - not a supply truck;
  - within `radius_m`;
  - not moving and without a movement order, so turning on the spot counts as moving;
  - not fighting: not launching, aiming, reloading or traversing on a target, and not guiding a missile.
- Incoming fire does not matter.
- Recipients are served in ascending unit order, each by the first set-up truck in reach that can pay.
- Every own unit publishes `stock` and a `service` reason:
  - out of range;
  - source not deployed;
  - moving;
  - firing;
  - serving;
  - no stock;
  - full;
  - garrisoned.
- `UnitSetup.condition` and `stock` author worn starts.

**Native tests** (`cargo test -p sim --test supply`, 16):
- no service before full deployment;
- rounds refill to capacity at price and rate;
- repair, with no self-repair;
- trucks never serviced, even by each other;
- replacements with fresh ids, the fallen kept;
- no resurrection;
- ammunition before soldiers, and an emptied launcher reloads;
- moving and firing recipients wait;
- a unit in a fight is not served between shots;
- incoming fire (shells from 970 m on a squad that cannot answer) does not stop service;
- whole payment in unit order, with no regeneration;
- an empty truck never blocks a stocked one;
- a truck that is packing or driving serves nobody;
- identical replays.

A fresh code review found problems, all fixed:
- "Firing" meant launching this very tick, so fighting units were served between shots.
- An empty truck blocked a stocked one.
- A unit turning on the spot under a move order counted as stationary.
- Trucks could repair each other.
- An unpriced finite round would have been free; prices are now required at load.
- Authored health was not clamped.
- Soldier formation spots were missing from the digest.

**Web tests:** vitest round-trips stock and service.

**Browser scene** (`bun run --cwd web scene -- supply`, 6 checks):
- not served while setting up;
- stock pays for restored rounds;
- the squad back to 8, the fallen kept;
- scouts beside the empty truck wait with "no stock";
- a moving recipient waits;
- a relocating truck serves nobody and its stock stays.

The lab shows no incoming fire: a squad under fire it cannot answer needs more room than the lab map. The Rust test owns that case.

Every other scene stays green, and `bun run check` is green.

**Visual gate:** an unprimed critique of 3 frames and 3 crops.
- **Acted on:**
  - The deployment ring's green and orange collided with served and waiting; recipient rings now use a light cyan (full = served, broken = waiting), and reach rings are white.
  - The empty truck drew a reach ring; it now has none.
  - Units by a truck not yet set up had no mark; they now show as waiting.
  - The panel lacked maximums and setup seconds; it now shows both.
  - There was no waiting frame; a moving-recipient frame is now captured.
  - The camera was reframed.
- **Kept, with reason:**
  - The AT team's tiny proxy is the shared unit scale.
  - Command-log numbering is shared across labs.
- **Preview-shots:** not offered; the run is unattended.
