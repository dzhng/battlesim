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

## Lane F-A, frame survey (slice 07, 2026-11-11)

T-72 family, Armata, ZTQ-15 and the Challengers. The numbers and their sources
are in `assets/frames/{t72,armata,ztq15,challengers}.json`.

- **FA1 Thresholds.** The slice's two stand: a hull dimension is fixed past
  max(0.10 m, 3 %), and a pivot past 0.15 m along x. The slice named none for
  a muzzle or a pivot height, so both take the hull-dimension rule. The T-72B3
  muzzle (0.20 m against a 0.19 m threshold) is the only marginal row.
- **FA2 Which source wins.** A published figure wins when a drawing's own
  proportions agree with it, and the drawing is then scaled by it. When two
  independent measurements (a drawing and a photo) agree against one
  unreferenced published figure, the measurements win, because the published
  heights for the ZTZ-99A ("hull height") and the T-14 (3.3 m, to the roof
  sensors) measure something other than the roof. Wikipedia's 13.5 m gun
  forward for the Challenger 2 is rejected: the L30A1 is 6.6 m, and the
  drawing agrees with 11.55 m.
- **FA3 No free drawings exist.** None of the nine models has a free
  three-view drawing on Commons or the open web, so the Q4 ladder's first step
  is empty. The drawings measured are the-blueprints.com sheets, which are
  copyrighted. They and the overlays made from them (which carry their line
  art) stay in the lane's `throwaway/`, and only numbers are committed. So
  `visualizations/frames.html` cannot embed this lane's overlays. The
  coordinator has to choose between showing only the station numbers and
  re-rendering the overlays without the drawing's lines.
- **FA4 The overlay's vehicle mode stands side and front views on the ground
  line.** Centring on the bounding box let a drawing's antenna lift its centre
  and float the model off its tracks.
- **FA5 T-90M: the pivot stays at x = 0 (question 1).** The T-72 hull's ring
  sits at mid-hull, and the T-90M shares it. The note is art: the turret is
  drawn about 0.75 m ahead of the drawing's, and its bustle is about 1.6 m short.
- **FA6 T-14 height: a large named change, at medium confidence.** The roof
  drops from 3.3 to 2.75 m, with the ring, gun axis, HMG and eye following it.
  Two drawings and a photo agree that the roof is 0.5-0.7 m below the frame's,
  but no two share a scale anchor. The T-15 shares the chassis but has no
  drawing, so its height is kept and named as unverified.
- **FA7 The Challenger 3 gets its own cannon muzzle.** It inherits the
  Challenger 2's frame. Its L55A1 reaches 11.3 m gun forward against the
  Challenger 2's 11.55 m, so it needs a muzzle override (6.95 m) once the
  Challenger 2's moves to 7.1 m.
- **FA8 ZTQ-15 and T-15 have no overlay.** No drawing of either exists. The
  ZTQ-15 gains a low-resolution CC-BY-4.0 side photo, which is enough for
  proportions. A generated orthographic view (the ladder's third step) is left
  to slices 18 and 19, which rebuild the turrets anyway.
- **FA9 Frame files are written as rows that slice 15 can apply.** Each
  `assets/frames/<family>.json` holds, for each unit, its `published`
  numbers, its current `frame`, what was `measured`, and a list of
  `verdicts`. A verdict is one row per frame number, for example
  `{"field": "cannon.muzzle_m[0]", "frame": 5.561, "verdict": "fix",
  "new": 7.1, "source": …, "why": …}`, followed by `notes`, `overlays` (the
  station readouts) and `questions`.
  - The slice listed the contents but not their shape. Slice 15 reads `new`
    straight into the roster, and `frames.html` reads the station arrays.
  - The four files are named `t72`, `armata`, `ztq15` and `challengers`,
    after the family slices 17-20.
  - Agent's call; confidence high.
- **FA10 Some drawings are scaled by the model's own road-wheel pitch,
  checked against a published number.** The T-90M sheet's published length
  and its proportions disagree by about 8 %, and the T-72B line drawing has
  no clean end-to-end span. Their scale comes from the spacing of their
  road-wheel centres, set to the model's spacing (0.81 m). A published width
  or gun-forward length then confirms it: the T-90M's width comes out at
  3.79 m against the published 3.78.
  - The catch is that the model scales the reference, which is circular if
    the model's wheel pitch is itself wrong. The independent check guards
    against that.
  - Agent's call; confidence medium.
- **FA11 The T-14's eye and HMG heights are derived, not measured.** Once its
  roof drops to 2.75 m, the HMG pivot is set to 2.8 m (its weapon station
  sits on the roof) and `eye_m` to 3.0 m (the commander's panoramic sight
  head, read at 2.9-3.1 m in the line drawing).
  - These follow from the roof fix rather than from a separate measurement.
    Slice 15 must check that `dressing_panorama` fits the 0.3 m bulky
    allowance over the new roof.
  - Agent's call; confidence medium-low.
- **FA12 The final thresholds live here, not in slice 07.** The slice asks
  for them in its own file, but that file is shared by three parallel lanes.
  Editing it would conflict at merge, so FA1 holds them and the coordinator
  may copy them into the slice.
  - Agent's call; confidence high.
- **FA13 Two near-threshold rows are left for slice 15 to name.**
  - The T-90M muzzle (6.2 m) is kept because it is inside 3 %. It shares the
    T-72B3's gun and hull, though, and that tank's muzzle is fixed to 6.1, so
    aligning both is a named choice.
  - The Challenger 2 pivot (+0.2 m) is kept at low confidence. The only
    evidence is the commander's cupola at -0.42 m in a top drawing, which puts
    the ring near the hull's middle.
  - Agent's call; confidence medium.
