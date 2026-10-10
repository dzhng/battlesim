# 18 — Closing scene (D14) and closeout

**Status:** planned. **Depends on:** every slice above. **Owns:** D14, D40, L12 (fix only if shown).

## Contract

D14: you buy an Apache in a skirmish. It flies in, pops over a village and kills a tank. An IFV shoots it down, and the wreck flattens trees.

## API seam

- A deterministic encounter `closing` on the `air` map with `test_attack_heli`. The red tank and IFV are bought through purchase commands (D40).
- A recorded skirmish replay at a pinned `/battle?...` seed, with the real Apache.

## What you can run or see

The scene `closing`, and the recorded replay.

## Verification

Run once, now:
- the full `check`;
- `verify`;
- the balance report (record the shift; don't retune).

Fix L12 only if a shot shows it.

**Visual variable:** the whole battle beat
**Crop or mask:** full frames at fly-in, the tank kill, the shoot-down and the wreck at rest
**Out of scope (later slices):** none: this is integration

1. Judge the candidate against its target with [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md): telemetry plus a less-wrong verdict. With no target, use its single-image diagnostics.
2. Human checkpoint (**non-blocking**): open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and leave about 5 minutes for a reply, carrying on with independent work meanwhile. If the user is silent, decide on the evidence, record the decision and why in this slice and in [choices](../choices.md), close the opened shots and go on.
3. Last check: run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) on the full frames and crops. Give the unprimed agent only the shots and a neutral task. Record its actionable findings before you accept the slice.

## Delegated to the implementer

The seed choice.

Any other choice is a spec gap. Record it in [choices](../choices.md) and settle it before you widen the slice.

## Must stay green

Existing replay digests and `menu_reel` stay unchanged unless this slice names a digest change. Every test this slice touches must pass.

## Feedback that would change this slice

The user's verdict on the D14 replay.
