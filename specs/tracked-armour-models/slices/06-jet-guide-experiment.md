# 06: jet guide experiment (lane J, after 05)

## Contract

A recorded verdict on whether smoothing the F-35 and Su-35 skin *guides* in code
ties with or beats their generated skins. It is judged under slice 05's paint,
because glossier paint shows lumps.

This slice ships nothing. If code ties or wins, the README queue gains the
"jets off the generator" spec (user decision Q1b).

## Seam

- **Guides:** `packages/scene-assets/blender/guides/f35.py` and
  `guides/flanker.py`.
- **Candidates:** smoothed guide renders are built in `throwaway/`.
- **Committed:** renders and the verdict sheet, under `assets/experiments/06/`.
- **Unchanged:** the shipped skins and pins.

## The variable and crop

- **The one variable:** airframe surface smoothness.
- **The crop:** fq, side and top closeups of each guide candidate beside the
  shipped skinned model, shaded and matcap.
- **Out of scope:** paint, markings and proportions.

## Steps

1. Smooth each guide in code.
2. **Closeups.**
3. **`compare-screenshots`** against the shipped skin.
4. **Last check: an unprimed `screenshot-critique`**, unlabelled, with one round.
5. Record the verdict in `decisions.md`. If code ties or wins, add the follow-up
   to the README queue (coordinator).

## Delegated

The smoothing technique (subdivision, or loft density).

## Feedback that would change it

The user wants the jets moved off the generator right away. That would be a
separate spec, not this one.

## Result (2026-11-11, lane J)

**Code wins on the surface for both airframes.** In one blind, unprimed
critique round, the code's surface scored 6 against the skins' 4 (F-35A) and
3 (Su-35S), and the code's mean was 5.6 against 4.6 on both. Neither reaches
the make-3d-models bar yet. The coordinator adds "jets off the generator" to
the README queue.

The sheet, scores, telemetry and the throwaway smoothing script are in
[`assets/experiments/06/`](../assets/experiments/06/README.md). The decisions
are J06-1 to J06-3 in [`decisions.md`](../decisions.md).
