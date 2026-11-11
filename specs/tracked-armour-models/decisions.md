# Decisions: tracked armour models

This ledger records every call behind the plan, with who made it and why.
Lanes append to the end and never rewrite an entry. Record a reversal as a new
entry that points back at the old one.

A builder who finds a call the plan didn't make records it here before
continuing.

Who decided, in order of authority:
- **User:** a given. Don't re-open it without the user.
- **Agent for the user:** decided on the user's behalf during the planning walk
  of 2026-11-11, under the delegation the user opted into. Its basis is the
  confirmed preferences below. The user may still correct it.
- **Synthesis:** decided while merging the three plan drafts on 2026-11-11.
  Where it applies, the genuine alternative is named.
- **Territory:** settled by the code.

## The user's confirmed preferences (the basis for agent answers)

1. The wider scope wins.
2. Limits that aren't gameplay bend to quality. Gameplay contracts hold, except
   the frame fixes named here.
3. Practical fallbacks beat purity.
4. Cheap experiments are welcome when they might remove machinery.
5. Ask every question up front, then finish without approval stops.
6. Numbers are guesses: tune them against the evidence.
7. Fix easy issues on sight.
8. Run expensive checks once, at the end.
9. Opus subagents do the 3D work.
10. Push green milestones.
11. Get the proportions right, aiming for cinematic realism, not every detail.

## User decisions (2026-11-11)

- **Q1 Route.** A procedural pass, plus a three-way test of the T-72B3 cast
  turret dome: as it is now, smoothed in code, and as a generated skin of the
  turret alone. A skin is adopted only if it clearly wins (slices 03 and 14).
- **Q1b Jet skins.** Smooth the F-35 and Su-35 guides in code and compare them
  with their generated skins. If code ties or wins, moving every jet off the
  generator becomes its own follow-up spec (slice 06). The user asked for this
  experiment; the agent filled in its details.
- **J1 Jet paint sheen.** Jets get the glossy, metallic-looking sheen of real
  jet paint in the sun. The reference is the user's screenshot,
  `assets/references/jet-sheen-reference.png` (slice 05). The user asked for
  it; the agent filled in its details: helicopters stay satin, metalness
  stays 0 unless gloss alone fails, and it ships with the jet lane.
- **Q2 Scope.** All tracked armour: 37 appearances. Wheeled siblings in shared
  scripts are re-exported and checked only.
- **Q3 Budget.** The catalog-load tripwire (`schema.ts:756-773`, 640 MiB, about
  635 used at planning time) is raised freely to the measured load plus room.
  - There is no startup requirement. Startup depends on the player's
    connection, so it isn't treated as hard.
  - Wreck trimming is out of scope.
  - The code agrees: the tripwire is documented as "a loose tripwire … not a
    target".
- **Q4 Measurements.** Use the first option available:
  1. Free drawings go into the reference library.
  2. Copyrighted drawings are measured in `throwaway/`, and only the numbers
     are committed, with a source note.
  3. Generated orthographic views are marked `generated`, scaled to published
     dimensions and checked against photos. A generated photo is an acceptable
     backup.
- **Q9 Wrong frames.** Frames found wrong are fixed as a named change to the
  digests, with one balance sample at the end.
  - The user expected small hit-rate effects, and the check confirmed it.
  - Leopard 3.0 → 2.6 m changes the vertical hit rate from 75 to 71 % at 300 m,
    58 to 52 % at 500 m, and 38 to 34 % at 850 m.
  - The basis: aim at 75 % of hull height, with spread cut off at 3σ and
    3.43 mrad for every tank gun.
- **Q9b Ajax.** Add a roof machine gun mount `on: autocannon`, the same
  mechanism the Leopard uses (`crates/contract/src/weapons.rs:286-289`). It is a
  named balance change.
- **Proportions.** Correct proportions matter, but not every small detail:
  "we're aiming for movie realism, cinematic realism".
- **Taste:**
  - **T1:** clean factory paint.
  - **T2:**
    - crisp, hard plates;
    - a narrow bevel that catches the light;
    - panel lines and seams painted in.
  - **T3:** silhouette first; small fittings only where they read.
  - **T4:** track sag between the return rollers.
  - **T5:** hatches closed, no crew.
  - **T6:** one clean scheme per nation.
  - **Context:** the closest view is the zoomed battle camera and cinematics.
    There is no single target game, and colour grading is out of scope.
- **Lineup notes.** The user's per-model defect list is the done list for each
  family. Each family slice carries it word for word.
- **No backward compatibility or migrations.** Saved preference: "recompute
  over compat layers".

## Agent decisions for the user (2026-11-11, planning walk)

- **Q5 Order.** Shared code comes before families, so families build on
  finished parts, and families in shared-code clusters go first. Opus lanes
  run in worktrees, with one commit per family and milestone pushes.
  Synthesis refined this into the graph in the README.
- **Q6 Bar.** The make-3d-models critique bar, with scores recorded, plus a
  lineup cohesion sheet at the end. There are no approval stops; anything still
  below the bar is named to the user.
- **Q6b Done.** A family is done when it meets the bar AND every one of the
  user's lineup notes for it visibly reads as fixed beside its reference. The
  user shrugged when asked; their notes are the most direct definition of done.
- **Q7 Experiments.** Run them first, in parallel, with one critique round each.
- **Q8 Reuse.** This spec covers the tracked group, and the README queues the
  later passes. The skill gains only lasting lessons.
- **Q8b Edge wear.** `chip=1.0` on every tracked model. "Clean factory paint
  (as now)" describes the look 33 of the 37 had, and the edge highlights help
  the bevels catch the light. Today `abrams.py:502` uses the default 0.35.
- **Q8c M10.** Exaggerate its nose and its turret's length-to-height ratio,
  within the frame, so it doesn't read as a small Abrams.
- **D1 Seams.** Experiment (slice 04): shallow geometry grooves darkened by the
  existing vertex occlusion bake, against a per-model baked detail map. Adopt
  the bake only if it clearly wins. Textures tile per scheme, so a per-model
  map is new machinery.
- **D2 Side panels.** One shared skirt and fender helper whose panels never jut
  past the hull outline (slice 09). It answers the "front side panels stick
  out" notes on 11 vehicles, which come from boxes each family draws itself.
- **D3 M270 / MARS II.** Draw the turntable and elevation hinge at the rear of
  the launcher box. The launcher stays stowed on its fixed mount.
- **D4 T-15.** Keep the 30 mm cannon, drawn at its real barrel length and shroud
  thickness. If that moves `muzzle_m`, it joins slice 15.

## Synthesis decisions (2026-11-11, merging the three drafts)

Three drafts, each with a different lens: fewest slices (Opus), risk first
(Codex GPT-6.1 Sol) and seam quality (Opus). The brief they worked from is
`assets/drafting-brief.md`.

- **S1 Fix the lint before the baseline (slice 00).** The overlap check never
  compares two parts on a real vehicle: `_part_group` resolves every mesh to
  the root (`vehicle_lint.py:243-259`, `:400-401`). Exports also fail on any new
  issue (`vehicle_export.py:56-67`). So the lint must tell the truth before the
  baseline is recorded, or every later diff carries old issues.
  - Grouping rule: keep each hull child as its own group, which is the code's
    intent.
  - Exception: if the T-72B3 alone shows more than 30 overlap issues and most
    are plate-on-plate by design, group at the articulated nodes instead
    (`hull`, `turret`, `wheel_*`, `track_*`) and record that here.
- **S2 Split `vehicle_parts.py` into eight modules by concept, with direct
  imports and no facade (slice 02).**
  - Alternative (fewest-slices draft): move only the running gear, and queue
    the rest.
  - Why the full split: the pass adds helpers in four concepts (skirts, guns,
    roof, seams). The file is 51,243 B, frozen by the ratchet, and would stay
    frozen after a partial move. A facade would give every name two owners.
  - The codemod is mechanical and proven byte-identical.
- **S3 Coordinator bakes at every push milestone.** This replaces the walk's
  "bake once". `bun run check` reads the committed runtime against the frames
  (`web/tests/sceneAssets/unitFit.test.ts:91-110`), and `asset check` compares a
  fresh bake against `assets/runtime`.
  - The walk's real hazard was several lanes baking or committing runtime.
    Lanes still never do. Only the coordinator bakes, on the merged tree, at
    each push.
  - Alternative: hold every push until slice 31. Rejected, because it breaks the
    push-milestones preference.
- **S4 Lanes follow import clusters,** not the walk's family order.
  - This keeps each lane's files disjoint, and LFS receipts can't merge.
  - The CV90120 goes with the CV90 lane (`cv90120.py:29`).
  - The Challengers go to lane A, to balance load.
  - Air defence and rockets go to lane B.
  - Within each lane, the walk's order holds.
- **S5 Jet gloss keys on the aircraft being a jet, not on the paint scheme.**
  Schemes are shared between jets and helicopters:
  - `us_compass_grey`, `russian_air_grey`, `chinese_air_grey` and
    `nato_air_grey` all have both;
  - the Su-25 jet wears `russian_helicopter_camo`.

  `aircraft_parts.py` is 37,191 B, over the ratchet, and must not grow.
- **S6 The gun helper (`vehicle_guns.gun_tube`) is proven on the four broken
  copies in slice 08.** Every other tracked gun adopts it in its family slice.
  - The see-through barrel is in four copies, not one: `t72.gun_2a46` (also
    T-80 and T-90), `type99.main_gun` (`type99.py:175-190`), Challenger 2
    (`challenger.py:220-225`) and ZTQ-15 (`type15.py:177-181`).
  - The Abrams and Leopard close the gap with evacuator caps.
  - The allowlist pattern hiding it is `fume_extractor` (line 61). `*_sleeve`
    never matches `thermal_sleeve_0`, because `fnmatch` must match the whole
    name.
- **S7 Side-panel helper: built and proven in slice 09, adopted in family
  slices.**
  - The proof is replacing `cv90.follow_tiles` (`cv90.py:92-116`, the helper's
    prototype) and the Bradley front fender (`bradley.py:73`).
  - Seven tank families also have their own `skirt_top`/`skirt_foot` and migrate
    in their own slices.
  - Alternative: migrate all 11 noted vehicles in the coordinator slice.
    Rejected, because it puts lane files in a coordinator slice and mixes
    variables.
- **S8 A winning bake becomes a follow-up spec.** If slice 04's bake clearly
  wins, this pass still ships grooves, and the user is told so. The bake needs
  a second unique UV, bundle support and a shader change
  (`models_material.wgsl:35-43`). Basis: practical fallbacks and limited scope.
- **S9 Frame amendments after slice 15.** The survey (07) covers all 37 before
  the chain, so late finds should be rare. A family lane that finds one files a
  shared-part request. The coordinator runs the same chain for it at the next
  push milestone, and it is named here.
  - Alternative (fewest-slices draft): fix it in the art within the frame. That
    was rejected, because it contradicts Q9.
- **S10 Three critique rounds per model,** then name what is still below the bar
  and move on.
- **S11 Python tests go into `asset check`, not `bun run check`.** That is all
  five: `catalog_frames_test`, `vehicle_lint_logic_test` and
  `vehicle_export_test` run under plain `python3`; `vehicle_lint_test` and
  `wreckage_test` run under the pinned Blender. The brief said four and missed
  `vehicle_export_test.py`.
- **S12 The frame overlay gets a vehicle mode (slice 07).**
  `drawing_overlay.py:42-43` drops every `wheel_*` mesh, which is all tracked
  running gear, and keeps `dressing_*`. No tracked reference library holds a
  drawing today, so the Q4 ladder runs for every family.
- **S13 The BRM-3K builds on `bmp.py`'s BMP-3 hull,** the way the T-80 builds on
  the T-72 (slice 28). `brm.py` describes and redraws the same hull. One owner
  also fixes the BRM's nose by construction.
- **S14 Concept splits before growth.** Buk moves into its own `roster/buk.py`,
  out of `air_defence.py` (26,205 B), so it also gets its own receipt.
  `abrams.py` (29,056 B) is split by concept before it grows.
- **S15 Planning scratch moves out of `scratch/`.** That folder is untracked but
  not ignored (`.gitignore:6` ignores only `throwaway/`). The lineup record now
  lives in `assets/before/`. Everything else moved to
  `throwaway/tracked-lineup/` when the spec was written.

## Lane F-C, frame survey (slice 07, 2026-11-11)

The CV90 family, Bradley, Puma, Ajax, BMP and BRM, and the SP howitzers. The
rows and evidence are in `assets/frames/{cv90,bradley,puma,ajax,bmp,brm,sp_howitzers}.json`.

- **FC1 Which source wins.** Manufacturer, official or Jane's figures beat
  secondary sites. A free drawing beats a photo. A square-on photo is
  measured only where a drawing or a published figure cross-checks it.
  - Why: the M3A3 side photo read the Bradley's turret roof at 2.68-2.78 m,
    and Jane's gives 2.565 m. A low, close camera reads the turret's side
    armour edge, not the roof, so a photo-only roof is an upper bound.
  - The BMP-2 square-on photo (CC0) turned out to be stretched 1.17x
    horizontally on upload: its road wheels are wider than tall. Corrected,
    it agrees with the 1981 drawing within 0.07 m.
- **FC2 A box's height is the turret roof.** Published heights for six of
  these units are to a sight, a hatch or an MG mount (Bradley 2.98, Puma 3.6,
  CV90120 2.65, BMP-2M 2.45, M109 3.25, 2S19 2.99), so they come down.
- **FC3 Sight heads that break the dressing allowance are named, not
  solved.** Once the box drops to the roof, four sights stand 0.45-0.6 m
  above it and about 0.4 m across: the Puma's PERI, the Ajax's ORION, the
  CV90120's panoramic sight and the M2A4's CIV. That is over the 0.3 m bulky
  allowance and wider than a thin part (`unitArt.ts:44-45`). The BMP-2M's
  sight housings sit 0.01 m past it.
  - This is the Leopard PERI question (slice 07 Q4) again, and `unitArt.ts`
    is the coordinator's.
  - The options for slice 15: allow sight heads above the bulky allowance by
    rule, draw the sight heads under 0.15 m across, or set each box at the
    sight top minus 0.3 m. The lane recommends the first. A sight head is
    not cover, and a box at sight height is what made these frames wrong.
- **FC4 CV9040C height 2.50 m.**
  - The free Mick Bell plan measures the base Strf 9040's roof at 2.37 m.
  - The C kit raises the published overall height from 2.5 to 2.71 m, with
    roof armour and roof smoke dischargers.
  - 2.50 m sits between those, and keeps the model's roof (2.54 m) inside
    tolerance. The alternative is 2.37 m, if the C kit adds nothing to the
    roof.
- **FC5 CV90 Mk IV height unresolved, kept at 2.8 m.** There is no free
  drawing and no square-on photo. The official 2.8 m is an overall height.
  It is named for slice 15, not guessed.
- **FC6 CV90120 length kept at 6.95 m.** The 6.6 m on military-today and
  Tank Encyclopedia describes the prototype. The CV90120-T rides the Mk
  III/IV chassis, whose official length is 6.8 m, which is within 3 % of
  6.95 m.
- **FC7 Howitzer gun mounts move to the trunnion.** All four have pivots
  2-3 m behind the turret front and above its roof, with `muzzle_m` set to
  the whole tube.
  - They came in with the artillery roster commit (`d1518dc4f`) and have no
    recorded basis. The art already draws the gun near the real trunnion.
  - The muzzle reach comes from published overall lengths where the photos
    disagree, as on the 2S19, where the photo's barrel may be the vehicle
    behind.
  - The simulation fires every mount's rounds from its pivot plus its
    muzzle (`crates/sim/src/weapons/muzzle.rs:12-22`). These rows move where
    shells leave, so they are named digest changes for slice 15.
- **FC8 A box covers the body, including an armoured turret overhang.**
  - The PzH 2000's turret overhangs the hull rear by about 0.4 m, so its box
    grows from 7.87 to 8.33 m.
  - The M109's bustle makes 7.02 m against 6.8 m. That is 3.2 %, borderline,
    and kept.
  - The alternative is hull length only. It was rejected because the bustle
    is armour a shot can hit.
- **FC9 Thresholds kept as slice 07 set them:** max(0.10 m, 3 %) for hull
  dimensions and 0.15 m for pivots along x. The lane adds 0.30 m for a
  muzzle's reach, because the slice set none. Borderline rows are named in
  their JSON (BMP-2M pivot 0.2 m, M109 length 3.2 %).
- **FC10 BMP-2M turret ring at +0.1 m, from the square-on photo.** The 1981
  U.S. Army drawing puts it at -0.48 m, and the U.S. Army BMP-1 graphic puts
  its turret further forward still. The photo is a real vehicle, so it
  wins. The muzzle agrees in both the photo and the drawing, 2.48-2.50 m
  ahead of hull centre, and that is the stronger row.
- **FC11 Unresolved, kept at today's values:**
  - M109A7 width: 3.15 m (family) against 3.9 m (A7 sources, probably over
    the turret racks).
  - PLZ-05 box: no square-on reference. The single-source 6.1 m hull is
    rejected against the seven-wheel hull in the photos.
- **FC12 Ajax roof from the art.** No free drawing of the Ajax exists. The
  art's roof (2.57 m) stands, because the published 3.0 m height lands on
  the art's ORION sight top (3.04 m). The box is 2.6 m.
- **FC13 Ajax roof MG mount** at pivot `[0.15, -0.72, 2.78]` (the RWS body
  centre: yaw through the bearing, pitch through the cradle) and muzzle
  `[0.95, 0, 0.05]`. The values come from the art, as slice 07 Q8 asks.
- **FC14 References committed only under allowed licences.** A BMP-3 photo
  under CC BY-SA 1.0 (not on the list) was measured in `throwaway/` only.
  - The BMP-3 public-domain drawing is committed twice: in `bmp/` and, as
    the hull the BRM-3K is built on, in `brm/`.
  - Libraries hold files per family folder, and S13 keeps the two
    families' folders apart.
- **FC15 Overlay vehicle mode used locally, not committed.**
  - It keeps `wheel_*` and `track_*`, drops `dressing_*`, and aligns on the
    lower hull's rear and the ground (the bustle and gun spoil a bounding
    box alignment).
  - It accepts a known px/m from a scale bar.
  - Lane F-A owns the committed version; these two alignment options are
    offered to it.

### Lane F-C decision audit (slice 07, before commit)

Each entry gives the verdict, then the confidence that the user would have
made the same call. Entries are grouped by verdict, least confident first.
FC1-FC15 above give the reasoning; this audit adds the calls they left out.

**Review these first:** FA-1 (sight heads), FA-2 (no generated views), FA-3
(the CV9040C roof).

Needs the user:

- **FA-1 What happens to sight heads above a lowered roof (FC3).**
  - The scenario: the Puma's box drops from 3.6 m (its PERI sight's top) to
    3.1 m (its turret roof). The PERI head, 0.41 m across, now stands 0.6 m
    over the box.
  - The validator's dressing rule lets bulky parts reach only 0.3 m past the
    box. Above that, only thin parts (at most 0.15 m across, like an
    antenna) may rise. So the export fails on the PERI.
  - The same happens on the Ajax, the CV90120 and the M2A4, and marginally
    on the BMP-2M.
  - The gap: slice 07 assumed sights fit the allowance; for these units they
    don't.
  - The reach: every future lowered roof hits it, and so does the Leopard
    (Q4).
  - Provisional call (reversible): slice 15 lands the roof heights and adds
    one dressing rule for sight heads above the bulky allowance. To reverse
    it, set each box at the sight top minus 0.3 m instead.
  - Confidence: low.
- **FA-2 Q4 rung 3 (generated orthographic views) went unused.**
  - The scenario: the CV90 Mk IV and the PLZ-05 have no free drawing and no
    square-on photo. The ladder's last rung is a paid image-generator view,
    scaled to the published size and checked against a photo.
  - The lane left both units' heights and lengths at today's values, marked
    unresolved, instead of paying for generations.
  - A generated view only guesses the inner proportions, the same ones the
    published figures leave open. It would cost money and still be checked
    against the same oblique photos.
  - Provisional call: unresolved rows stay at today's frames. Run rung 3 for
    those two only if the family slice can't judge them from the quarter
    photos.
  - Confidence: medium.

Sound:

- **FA-3 The CV9040C roof at 2.50 m (FC4) is a reasoned middle, not a
  measurement.**
  - The scenario: the free plan shows the base Strf 9040's roof at 2.37 m.
    The C kit's published height grows by 0.21 m, but nothing says how much
    of that is roof.
  - 2.50 m is the midpoint, and it leaves the art untouched.
  - If the user prefers measured-only, the row is 2.37 m and the turret art
    drops 0.17 m.
  - Confidence: medium.
- **FA-4 Photos measured as drawings, outside the ladder's literal rungs
  (FC1).**
  - The ladder names drawings and generated views. The lane also measured
    square-on photos (BMP-2, 2S19, PzH 2000, M3A3), and committed those
    under allowed licences as references.
  - The guard: a photo counts only where a drawing or published figure
    cross-checks it. That guard is what caught the Bradley photo reading
    0.15 m high.
  - Confidence: medium-high.
- **FA-5 A base or earlier variant's drawing stands in for the roster
  variant.**
  - The Strf 9040 plan stands for the CV9040C, the M109A6 drawing for the
    A7, the BMP-2 for the BMP-2M, and the base 2S19 photo for the 2S19M2.
  - Each entry's note names what differs, for example the A7's six Bradley
    road wheels.
  - Only the shared parts (the hull, roof, turret shell and ring) are read
    from them.
  - Confidence: high.
- **FA-6 `eye_m` kept on every unit, even where the box drops.**
  - The contract defines the eye as the sight's height above the hull base
    (`catalog.rs:181`), not a fraction of the box.
  - The old values look like 0.9-0.94 times the old box heights. They still
    sit at or below each real sight, so lowering the box doesn't move what
    the crew sees from.
  - Confidence: medium-high.
- **FA-7 Mount numbers come from overall length where a photo disagrees.**
  - The 2S19's photo barrel reads 5.5 m from the trunnion. Its published
    11.917 m overall length gives 7.0 m.
  - The PzH 2000's muzzle is 7.1 m, the midpoint of a hull-rear and a
    turret-rear reading of the overall length.
  - Both errors are a tenth of the 2-3 m they replace.
  - Confidence: medium.
- **FA-8 The frame file layout.**
  - There is one JSON per family: `cv90` (it holds the CV90120, as slice 25
    does), `bradley`, `puma`, `ajax`, `bmp`, `brm`, `sp_howitzers`.
  - Each unit holds published, measured, model, overlays, verdicts (field,
    frame, verdict, new, source, why), dressing conflicts and notes, plus
    the family's answers to the slice's questions.
  - Overlay images are `<family>.<unit>-<view>.png` beside it.
  - The reach: slice 15 reads the verdict rows, and the coordinator's
    `frames.html` reads the images. The other lanes may shape theirs
    differently. The coordinator should merge on `verdicts[].verdict ==
    "fix"`.
  - Confidence: medium-high.
- **FA-9 Verification was the narrowest check.** The six touched libraries
  were checked with `checkReferences` (the code `asset check` runs per
  library), and all were clean. A full `asset check` needs a bake, which
  lanes don't do. Confidence: high.
- **FA-10 Thresholds recorded here (FC9), not in slice 07's file.** Three
  lanes share that file, while `decisions.md` merges by union. The
  coordinator copies the final values into the slice when it closes 07.
  Confidence: high.
- **FA-11 Reference files.**
  - The Mick Bell sheet is split into four view files at full resolution, so
    each keeps the scale bar's 236.6 px/m.
  - The U.S. Army GIFs are committed as PNG conversions.
  - The PzH 2000 and 2S19 photos were resized to the 1600 px limit.
  - Files are named with the existing `-n` suffix.
  - Confidence: high.
