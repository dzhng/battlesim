# 07: frame survey (lanes F-A, F-B, F-C)

## Contract

Every one of the 37 frames is measured against references. Every lineup note
is classified as **frame**, **art** or **dressing**, so slice 15 fixes all the
frames at once and the family slices fix only the art.

All frames follow one convention:

- The hull box reaches the hull and turret roof.
- Sights, antennas and roof guns are dressing, within the allowance: 0.3 m for
  bulky parts, 0.15 m for thin ones (`unitArt.ts:44-45`).
- The pivot sits at the centre of the turret ring.
- The muzzle sits at the real barrel length.

## Seam

Data only.

- **Output:** one `assets/frames/<family>.json` per family. It holds:
  - the published length, width and height, with their source;
  - the frame's `half_extents_m`, `eye_m`, and each mount's pivot and muzzle;
  - overlay stations;
  - a verdict for each number: keep, fix (with the new value), or art-only;
  - a classification for each of the user's notes.
- **References:** additions go to `assets/references/<family>/references.json`,
  following the Q4 ladder:
  1. free drawings are committed;
  2. copyrighted drawings are measured in `throwaway/`, and only the numbers are
     committed, with a source note;
  3. generated orthographic views are marked `generated`, scaled to published
     dimensions and checked against a photo of the same view
     (`packages/scene-assets/README.md:16`).

  No tracked library holds a drawing today; every entry is a photo.
- **Overlay tool (F-A, first step).** `drawing_overlay.py:42-43` drops every
  `wheel_*` mesh, which is all tracked running gear, and keeps `dressing_*`
  subtrees. Add a vehicle mode that keeps the wheels and the belt and skips
  `dressing_*`.
- **Visualization:** the coordinator assembles every family's overlay into
  `visualizations/frames.html`.

## Lanes

The lanes match the family lanes in `../parallel.md`:
- **F-A:** T-72 family, Armata, ZTQ-15, Challengers.
- **F-B:** Abrams and M10, Leopard, Leclerc and KF51, Gepard, Buk, M270 and MARS II.
- **F-C:** CV90 family, Bradley, Puma and Ajax, BMP and BRM, howitzers.

## Questions the survey must answer

1. **T-90M turret "too far forward".** Is that `pivot_m` (x = 0.0), or the
   turret drawn ahead of its pivot?
2. **T-80BVM turret "too small".** Measure it against the drawing.
3. **Howitzers: "turret too big or body too small".** The hull boxes are
   2.99–3.25 m tall and include the turret. Measure the hull and turret
   separately against published numbers for all four.
4. **Leopard 2A6 roof against about 2.6 m.**
   - The frame is 3.0 m tall: `europe.json` half extents `[3.85, 1.875, 1.5]`.
   - Does the PERI sight's top fit inside the 0.3 m dressing allowance over the
     new roof?
   - The 2A8 inherits the 2A7V frame.
5. **Puma.** Does the MELLS overhang its hull box?
6. **T-15.** What is the 30 mm barrel's real reach? Does it move `muzzle_m`
   (2.94 m)? Today the art draws 1.54 m of barrel (`t15.py:111`).
7. **M270 and MARS II.** Do the reference photos show the elevation hinge at the
   rear?
8. **Ajax.** Where are the roof machine gun's pivot and muzzle? Take them from
   the RWS art (`ajax.py:119-123`).
9. **The other notes.** Classify the M1E3, M10, Challenger, KF51, ZTQ-15, BMP,
   BRM and Buk notes.

**Fix thresholds:**
- a hull dimension off by more than max(0.10 m, 3 %);
- a pivot off by more than 0.15 m along x.

These are guesses: tune them, and record the final values here.

**Hard caps:** tracked half-width ≤ 2.1 m and half-length ≤ 4.8 m
(`crates/contract/src/limits.rs:21-24`). Both Challengers are already at 2.1.
A fix past a cap is out of scope and gets named.

## The variable and crop

- **The one variable:** how well the frame agrees with the references.
- **The crop:** the overlays in side, top and front views, against the
  slice 01 GLBs.
- **Out of scope:** any model edit.

## Verification

- `asset check` passes, which covers the reference library contract.
- Every note has a classification.
- The lanes run no `screenshot-critique`, because they judge measurements, not
  looks.
- `compare-screenshots` is used only where a photo checks a generated
  orthographic view.

## Delegated

- which source to trust when sources disagree (record why);
- the threshold tuning.

## Feedback that would change it

The user wants a frame left as it is, for example "keep the Leopard at 3.0".
That removes one row from slice 15.

## Results (2026-11-11, lanes F-A, F-B and F-C, reconciled in 07b)

- **Output.** There is one `assets/frames/<family>.json` per family, all in
  one shape (decisions.md C9). Slice 15 applies
  `assets/frames/fix-rows.json` as written:
  - 64 rows: 20 hull box, 41 shell origin, 2 mobility and 1 eye;
  - parents come before the units that extend them;
  - every hull row is inside the tracked caps.

  `visualizations/frames.html` shows each unit's outline, numbers and rows.
- **Final thresholds** (C10):
  - hull dimensions: max(0.10 m, 3 %);
  - pivots and muzzle points: 0.15 m on any axis;
  - a value in another convention is fixed whatever its size.
- **Conventions.**
  - The box top is max(roof, tallest bulky dressing - 0.3 m) (C4).
  - `muzzle_m` is measured from the pivot (C5).
- **Answers.**
  1. T-90M: the pivot is right; the turret art sits ahead of it.
  2. T-80BVM: the hull is 0.39 m long; the turret is too low, not too
     small.
  3. Howitzers: the M109A7, 2S19M2 and PLZ-05 boxes are too tall and the
     PzH 2000's too short. All four gun pivots are placeholders and move to
     their trunnions.
  4. Leopard 2A6: the roof is 2.64 m and the PERI 3.0 m, so the box is
     2.70 m. The 2A7V and 2A8 get 2.83 m.
  5. Puma: the MELLS fits; the PERI sets the box at 3.30 m.
  6. T-15: the muzzle moves from 2.94 to 3.4 m.
  7. M270 and MARS II: the hinge is at the rear.
  8. Ajax: the HMG pivot is at [0.15, -0.72, 2.78] and its muzzle
     [0.95, 0, 0.05].
  9. Every other note is classified in its family file.
- **Named changes beyond the lanes' lists:**
  - the Buk-M3 becomes tracked, with a 9.3 m hull (C6);
  - the Challenger 3 gets its own box and cannon;
  - the Bradley 25 mm and BMP-3 100 mm muzzles are fixed (C5).
- **Unresolved, kept at today's values:**
  - M109A7 width;
  - PLZ-05 hull length (its generated view failed the photo check, C7);
  - T-15 height;
  - the Bradley barrel's height.

  The Buk's pivot is a photo estimate (± 0.5 m).
