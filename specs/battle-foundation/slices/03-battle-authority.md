# 03 — Commands, observation transport and replay

**Status:** complete 2026-09-25. **Dependencies:** 01, 02. **Milestone:** Village checkpoint.

## Contract and question

Can one authority process commands and publish coherent player state independently of drawing?

User requirements owned or exercised: S01. Read their canonical entries in [requirements](../requirements.md), then the applicable [implementation contracts](../contracts.md). Do not infer rules from a fixture label.

## API seam and ownership

`Battle::accept/step/observe`, WASM filtered exports, `web/src/battle/sim` worker/client and `Replay` from architecture.md.

## Runnable artifact

/lab/authority: own-unit movement stub driven by real tick commands, ordered ack log, pause/step/reset, replay comparison and consumer-credit control. Enemy fixture data remains absent until sensing produces permitted observations. Use the actual contract early; no all-truth presentation placeholder.



## Verification and verdict

Same-build same-seed command replay tick digests; direct versus worker parity; no ticking behind loading cover; ordered burst/ack; invalid side/target rejection; withheld credit pauses instead of queuing/dropping observations; no use-after-transfer; scripted advance resolves on consumption; cancellation/disposal/failure terminate work.

Run relevant native Rust tests and browser/TypeScript seam tests; run typecheck/lint and the registered scene without console/GPU errors. Keep prior accepted slice contracts green. Source test names in [research](../research.md) are reuse references, not evidence that these new tests already exist. Record measured results and any scope deviation in this slice and the README handoff.

Visual variable: **Authority status honesty**. Review crop/mask: **Tick/ack/status panel and one moving proxy**. Explicitly out of scope: Final HUD design, combat and fog.

1. Freeze fixture seed, tick, camera, viewport and DPR. The registered scene regenerates the full frame and tight 2×–4× crops with machine/config metadata in gitignored `throwaway/evidence/<fixture-id>/`; the verdict records metrics and dispositions.
2. Use [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) for candidate versus any previous/reference shot. State the target for this variable, report telemetry and a less-wrong verdict; do not reward mere baseline matching. With no predecessor, use the skill's single-image diagnostics and inspect their meaning.
3. **As the last visual acceptance check, run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) with an unprimed agent given only the actual candidate full frames, crops and neutral inspection task.** Inspect and record its actionable findings before accepting. Do not substitute implementer self-review.
4. Offer the relevant shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md). This is a **non-blocking** course-correction window of about five minutes. Continue independent work while it is open, using waits no longer than 60 seconds. If no answer, decide reversible matters on evidence, record the choice, close the opened Preview set and proceed. Silence does not authorize changing user rules.

A visual pass is not a substitute for behavioral tests. A failing contract or serious visual defect means fix or reslice, not quietly update a golden image.

## Additional handoff regression

Replay includes both sides; disable controller/input generation while replaying and prove no double execution of bot commands.

## Decision budget

Delegated: Internal packing layout with Rust-exported descriptor; scheduler helper names. Also permitted: internal naming and reversible diagnostic formatting. Every other consequential freedom is a spec gap: record and resolve it before expanding implementation. No compatibility shims, duplicate owners, historical renderer adapters or undocumented gameplay changes.

## Human feedback that changes the slice

If render stalls alter simulation outcomes or UI reads hidden truth, fix the boundary before continuing.

## Verdict — 2026-09-25

Accepted. What shipped:

- **Rust authority.** `sim::battle::Battle` provides `new/accept/step/observe/digest/replay/from_replay`. Commands carry `CommandEnvelope { side, seq, order, queued }` and produce `CommandAck { seq, applied_tick, error }`.
  - A command whose sequence number is next for its side is recorded for replay, even when its content is invalid (unknown or foreign unit, out of bounds). Both sides are recorded.
  - A replay refuses live input and rejects a mismatched scenario or config digest.
  - Movement is a straight-line stub at base speed, standing on the walkable surface; slice 04 replaces it with routing.
  - At slice 03, `ObservationFrame` carried `tick` and complete `own` units only. Each later slice adds its own fields; there are no placeholders.
- **WASM.** The `Battle` class packs each side's frame into a reusable Rust buffer. `observation_layout()` publishes the field order and unit tags.
- **Browser.** `web/src/battle/sim` has `authority.ts`, a host-agnostic state machine with a fixed 30 Hz schedule, a two-buffer credit pool and a catch-up cap of 4 ticks with explicit "running slow". It also has `worker.ts`, `client.ts` (worker and in-thread "direct" transports), `module.ts`, `observation.ts` (layout-driven decode) and `protocol.ts`.
- **Static geometry.** The main thread builds the public static map with the same Rust `WorldView` from the same map JSON, for rendering and ground picking. The authority stays in the worker.

Tests:

- **6 native:** ordered next-tick acks; rejection without effect; queue, replace and stop; own-only observation; same-seed replay digests over 240 ticks with both sides and a rejected command; replay refuses changed scenario or config.
- **8 TypeScript state-machine tests on the real WASM:** no tick before start; withheld credit stalls with consecutive ticks and nothing dropped; ordered acks with verdicts; wall-clock debt shed; exact scripted advance; dispose frees and stops; a failed load reports an error; a replay reproduces every digest and refuses input. Transfers really detach buffers, so a use-after-transfer would throw.
- **14 browser checks** (`/lab/authority`): real-time 30 Hz ticking once the frame is up; right-click move acknowledged for the next tick; the tank drives to the clicked point; an unknown unit is rejected with a reason; the log shows unit, target, apply tick and verdict, newest first; the panel names the selection and its order; withheld credit stops ticks and says "waiting-consumer"; releasing resumes; pause holds and step advances exactly one; a hidden tab suspends; **an in-thread replay of the worker's accepted commands matches every worker tick digest** (this is also the direct-versus-worker parity proof); reset rebuilds from the seed.

Visual gate (authority status honesty): the first critique found the log ambiguous. Its numbering ran against the sequence numbers, and it didn't name the unit, target or apply tick. It also found that rejections weren't distinguished, the credit toggle was jargon, nothing named the selected unit, and the framing was too wide. All were fixed. The final critique found remaining wording problems: a rejected line still said "applies at tick", verdicts wrapped at the line end, and the log used a different unit name from the selection. Fixed: each entry now leads with its verdict ("✓ accepted, applied at tick N" / "✕ rejected (reason)"), units are named `tank #0` everywhere, and disabled buttons read as disabled. Destination markers belong to slice 04. Shadows are out of scope. The scene checks pin the wording.

Deferred, and recorded in the README handoff: presentation interpolation between completed ticks, owned by 04's movement readability.
