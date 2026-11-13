# Slice 06: jet guide experiment, verdict

**Verdict: code wins on the surface, for both airframes.** A guide smoothed in
code reads cleaner than its generated skin under slice 05's paint. The
coordinator adds "jets off the generator" to the README queue (user decision
Q1b).

## What was compared

| File label | F-35A | Su-35S |
|---|---|---|
| A | smoothed guide (code) | shipped skin |
| B | shipped skin | smoothed guide (code) |
| raw | the guide as committed | the guide as committed |

`KEY.txt` says the same. The labels were swapped between aircraft so the
blind critique couldn't learn a position.

- **Shipped:** `f_35a.glb` and `eastern_sukhoi_flanker_family_su_35s.glb`,
  re-exported with slice 05's jet paint at its 0.35 trial (J06-4).
- **Code candidate:** the committed guide (`guides/f35.py`,
  `guides/flanker.py`), built through `smoothing.py`, which was a throwaway
  wrapper in the lane worktree. It works on the near tier's painted bodies
  (fuselage, belly, intakes, nacelles, booms and the gun bulge):
  1. joins them;
  2. voxel-remeshes them at 25 mm into one closed shell, so their junctions
     blend;
  3. smooths that shell (factor 0.6, 12 iterations);
  4. decimates it to 20,000 triangles. That's the skin's near-tier budget,
     26,000 (`aircraft_skin.TRIS`), less what the lofted flying surfaces
     keep.

  Wings, strakes, stabilators and fins stay lofted aerofoils, because voxels
  would eat their thin trailing edges. Markings are laid onto the new shell
  afterwards.
- **Near-tier triangles:** F-35A code 38,204 against skin 50,165; Su-35S code
  66,332 against skin 80,349.
- **Crop:** fq, side and top closeups (`closeups.py`), shaded and matcap, at
  2x (`*-crop.png`). The full raw/A/B sheets are `*-fq-raw-A-B.png`.

## Unprimed critique (one round, blind)

| | Silhouette | Surface | Fittings | Materials | Cohesion | Mean |
|---|---|---|---|---|---|---|
| F-35A code | 6 | 6 | 5 | 5 | 6 | 5.6 |
| F-35A skin | 6 | 4 | 4 | 5 | 4 | 4.6 |
| Su-35S code | 6 | 6 | 4 | 6 | 6 | 5.6 |
| Su-35S skin | 6 | 3 | 5 | 5 | 4 | 4.6 |

- **F-35A, code better.** The skin's wing roots have a "sawtooth, crinkled
  blend", with ripples on the wing and tail tops and slivers at the canopy and
  intake. The code's only surface fault is faceting along the upper fuselage.
- **Su-35S, code better "by a wide margin".** The skin's spine and leading-edge
  extensions are "crumpled fabric", and its fins wobble. The code has
  faceting bands and a faint dent.
- **What the code still needs** (fittings, not surface):
  - a canopy pod with a hard step onto the spine;
  - box intakes;
  - the Su-35S's wing-tip pods standing clear of the tips;
  - the F-35A's sensor window as a slab on the skin.
- **Neither model reaches the make-3d-models bar** (no score below 6, mean
  at least 7). The verdict is relative: which surface the follow-up should
  build on.

## Telemetry (`compare-screenshots`, skin → code)

`compare-telemetry.txt` holds the metrics. In the top matcap, the code's edge
energy is 0.84× the skin's (Su-35S) and 0.90× (F-35A). Fewer edges there
means fewer ripples in the skin, which agrees with the critique. The
silhouettes match closely: side distances are 0.015–0.047.

## Limits

- The closeups are Blender renders, not the battle camera.
- One critique round, as the slice asked.
- The smoothing is a first try. Faceting along the bodies is its visible
  cost, and it is fixable: a finer voxel, more smoothing, or a denser
  decimation.
