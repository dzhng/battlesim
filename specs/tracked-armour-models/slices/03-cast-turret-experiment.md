# 03: cast-turret experiment (lane E1)

## Contract

A recorded verdict, with evidence, on which of three candidates gives the
T-72B3's cast turret dome the best surface:

1. **current:** today's code;
2. **smoothed:** smoothed in code;
3. **skin:** a generated image-to-3D skin of the turret alone.

A skin is adopted only if it clearly wins (user decision Q1).

## Seam

- **Smoothed candidate:** a patch to `t72.dome_rings`, kept in `throwaway/`.
  The T-80 imports it (`t80.py:32`).
- **Skin candidate:**
  - Hunyuan3D 3.1 Pro on fal.ai through `tools/skin-generate.mjs`. `FAL_KEY` is
    never printed or committed.
  - At most two paid calls.
  - Fitted **by hand** in `throwaway/`. `aircraft_skin.py:110-124` orients a skin
    by its highest point (a fin tip), which is wrong for a turret, so no
    fitting machinery is built here.
- **Kept and committed:** every candidate, generated or not, under
  `assets/experiments/03/`, with the skin pinned the way
  `packages/scene-assets/blender/skins/*.json` pins jets.
- **Not changed:** no shipped script.

## The variable and crop

- **The one variable:** the dome's surface and silhouette.
- **The crop:**
  - the turret at the `battle_zoom` and `cinematic` shots, shaded and matcap;
  - beside the T-72B3 three-quarter-front reference (`assets/before/refmap.json`).
- **Out of scope:** the barrel, ERA blocks, the hull, skirts, paint and sag.

## Steps

1. **Fit probe, before judging looks.** Check the skin's bounds, its height,
   its seat on the turret ring, and its fit around the gun's mantlet.
   - Reject the skin if it needs unexplained offsets.
2. **Closeups** of all three candidates.
3. **`compare-screenshots`** of each candidate against the reference.
4. **Last check: an unprimed `screenshot-critique`** of the three candidates,
   unlabelled, in random order, with one round.
   - "Clearly wins" means the skin ranks first and scores at least 1 point
     higher on surface than the best code candidate, with no loss of
     silhouette.
5. **Checkpoint,** which never blocks: show the three domes with
   `preview-shots`.

## Output

The verdict and the scores go in `decisions.md`.
- If code wins (smoothed or current), slice 17 applies it to `dome_rings`.
- If the skin wins, slice 14 runs.

## Delegated

- the smoothing technique (subdivision level, or loft ring density);
- the fal request parameters.

## Feedback that would change it

The user prefers a candidate the critique ranked lower.

## Outcome (2026-11-11)

Done. The smoothed dome wins, and the skin ranks last, so slice 14 is not
needed. The verdict, scores and fit probe are in `decisions.md`, entries
E1-1 to E1-10; the evidence is in `assets/experiments/03/`.

The proxy refused Hunyuan, so the skin came from Tripo H3.1 through Bento's
fal proxy, from one view (E1-1). Slice 17 applies `smoothed_dome.py` to
`dome_rings` and steepens its lower band (E1-9).
