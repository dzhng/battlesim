# Distant impostors (placeholder)

**Status:** placeholder. No new static-prop impostor representation, asset,
renderer path, or threshold has been approved. Existing unit/corpse card tiers
and their `impostor_px` data are separate production behavior and are not this
experiment's contract.
**Updated:** 2026-10-22.

## Next Agent Prompt

You are picking up a bounded investigation into 2D impostors for objects that
are much farther away than the current three-view stress captures. Start by
reading the closed [distant-detail record](../done/distant-detail/README.md),
the [performance rationale](../done/performance/README.md), and
[`renderer`](../../.agents/skills/renderer/SKILL.md). Reproduce the existing
wide baseline before changing code. Then use
[`write-spec`](../../.agents/skills/write-spec/SKILL.md) to turn the open
questions below into the smallest independently reviewable experiment. Do not
revive the rejected card plumbing or add a production switch before a matched
visual and GPU result qualifies it.

Before ending a pass, update this section with the current evidence, the next
pickup point, and any decision that moved from unknown to settled.

## Question

Can a simple 2D representation reduce GPU work only at a genuinely far zoom,
while preserving the object's presence, orientation cues, ground contact, and
shadow behavior at the transition? The previous card pilot is evidence about
one implementation and one workload, not a universal rejection of impostors.

## Known constraints

- The existing parked-car card pilot regressed the retained wide stress case:
  GPU time rose from about 32.14 ms to 40.01 ms and measured FPS fell from
  22.8 to 20.4. Its visual benefit was not reliable.
- The production renderer currently uses the ordinary far-car mesh and the
  shared Rust presentation selection path. The pilot's asset, shader, and
  client plumbing was removed.
- The investigation must keep simulation outcomes, side knowledge, caster
  eligibility, and browser/native presentation contracts unchanged.
- Tiny-prop shadow omission is a separate shipped optimization. An impostor
  must not make a visible object or a legitimate offscreen shadow contributor
  disappear.
- The target is a farther zoom where the object is genuinely subpixel or near
  subpixel. The transition distance, representation, and hysteresis are open
  decisions; do not copy the old card threshold without evidence.
- The experiment must state its client scope before implementation. A browser
  result does not qualify native automatically; if both clients are in scope,
  each must pass the same representation, visual, and timing gates.

## Open questions to resolve before implementation

1. Which object classes, if any, are visually stable enough for an impostor?
2. At what projected size does the mesh stop paying for readable information,
   and what minimum size still needs a silhouette, heading cue, or shadow?
3. Is a camera-facing card, a small view-dependent atlas, or another simple
   representation the least expensive shape that preserves those cues?
4. How are depth, alpha coverage, ground contact, and shadow casting handled at
   the transition, including oblique and low-sun views?
5. Does the representation help in the far view after upload, bind, and
   fragment costs are included, rather than only reducing mesh vertices?

## Required evidence for a future spec

Use the real Metro XL 100-versus-100 scene and a frozen simulation state. The
baseline set is the existing close, tactical, and wide views at 65 m, 260 m,
and 2,400 m, with the same Low-quality 0.75 render scale, 1280×720 viewport,
tick 90, seed, map target, heading, pitch, and digest recorded in the durable
[baseline receipt](baseline.json). The original raw measurements remain under
`throwaway/car-shadow-20261021/results-final/measurements.json`; use the same
Apple Metal adapter or record the deliberate difference. The receipt's target,
digest, FOV, aspect, near plane, and per-view camera values are part of the
comparison contract, not suggestions.
Add at least one farther in-map view where the same local population and shadow
casters remain in the workload; record its exact camera distance and framing.
Never manufacture a win with an empty or out-of-map view.

Compare ordinary mesh baseline against the candidate impostor with the shipped
tiny-prop shadow policy held constant. Do not use an undefined “combined” arm:
if a second variable is introduced, give it its own named arm and question.
Run interleaved, GPU-locked repeats for each client in scope. Report GPU
duration by pass, CPU/frame duration, submitted mesh/instance/caster work, and
memory in separate categories (GPU buffers, textures, heap/RSS); installed
capacity is not submitted work and FPS is not a reciprocal GPU measurement.

Capture a deterministic zoom-in and zoom-out path through the transition with
the same heading and pitch. Record the enter and exit thresholds and any
hysteresis; if there is no hysteresis, prove that repeated crossings do not
flicker. Keep the object crop and its ground/shadow contact in every shot.

The candidate must also preserve the caster contract: run a dedicated
offscreen-caster fixture whose shadow lands on visible ground, compare caster
identity and cascade masks with the mesh baseline, and compare the receiver
shadow. A local object crop or aggregate triangle count is not enough to prove
that an offscreen contributor survived.

For each client in scope, compare simulation digest, replay output, and the
side-filtered publication before and after. A renderer-only candidate must
produce equality; any deliberate difference becomes a named decision before
implementation.

Every visual candidate must use
[`compare-screenshots`](../../.agents/skills/compare-screenshots/SKILL.md)
against the ordinary mesh and receive a fresh unprimed
[`screenshot-critique`](../../.agents/skills/screenshot-critique/SKILL.md).
Review the object crop and its ground/shadow contact separately from the whole
frame. Open the resulting shots with
[`preview-shots`](../../.agents/skills/preview-shots/SKILL.md) for a short,
non-blocking human review window.

The recommended acceptance floor for a future spec is at least three
interleaved A/B pairs under the GPU lock, with the candidate winning at least
two pairs and the aggregate median GPU duration at least 10% lower at the
farther view. No existing close, tactical, or wide view may regress by more
than 5% median GPU duration, and no single view may regress by more than 10%
in any pair. CPU/frame median must stay within 5% of baseline; GPU buffers and
textures within 5%; heap/RSS within 10%. Report all categories even when they
are diagnostic. There must be no object, orientation, transition,
ground-contact, caster-set, or shadow defect in the crop and offscreen-caster
reviews. The future spec may tighten these floors but must choose them before
implementation. If the candidate misses any floor, record the result here and
leave the ordinary mesh in place.
