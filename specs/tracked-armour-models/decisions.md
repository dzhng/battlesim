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

## Lane E1: cast-turret experiment (slice 03, 2026-11-11)

Evidence: [`assets/experiments/03/`](assets/experiments/03/README.md).

- **E1-1 Generator: Tripo H3.1, because the proxy refused Hunyuan.**
  (Agent, under the coordinator's brief.)
  - The user declined to hand over a fal key, so every call went through
    Bento's fal service proxy, never `tools/skin-generate.mjs`.
  - Hunyuan3D 3.1 Pro (`fal-ai/hunyuan-3d/v3.1/pro/image-to-3d`) came back
    HTTP 404 twice, with no charge header. Bento's fal list doesn't admit it.
  - The fallback was Bento's admitted default, `tripo3d/h3.1/image-to-3d`.
    It takes a single image, so the generator guesses depth. That is the
    weakness `make-3d-models` names ("one photo means guessed
    proportions").
- **E1-2 Its input was the smoothed dome alone.** (Agent.) One shaded
  three-quarter view (front left, 25° up, grey on white, the `skin_views`
  look) of tier 0 of the smoothed candidate, with no ERA, fittings or gun.
  - Why the smoothed dome: a skin inherits its model, so the faceted
    current dome would have brought its facets back.
  - Why nothing else: code keeps every fitting (slice 14's seam).
- **E1-3 One paid call; the second was not spent.** (Agent.)
  - The call: geometry only (texture and PBR off), detailed geometry, a
    200,000-face limit; request `01a1cb19-c4ec-7901-a8cb-92a958086e76`.
  - The proxy reported `x-duet-charge-usd: 0` on submit and on fetch.
    The workspace ledger doesn't attribute charges to a service, so the
    real charge wasn't seen. fal lists this request at $0.40.
  - Why no second call: the skin can at best reproduce its guide, so its
    best case ties the smoothed candidate. A tie can't meet "clearly wins",
    so a second paid call couldn't change the verdict.
- **E1-4 Fit probe: passed, with no unexplained offsets.** (Territory; the
  fit was done by hand in `throwaway/`.)
  - The fit:
    - a quarter turn, because the generator's long axis is its Y;
    - a per-axis scale to the guide's extents: x 3.01, y 2.88, z 2.72 (a
      10% spread, explained by the single-view depth guess);
    - the base set on the guide's base.
  - The skin came out symmetric front to back, so it lost the dome's
    0.15 m-longer front half.
  - Every candidate seats the same way on the ring and mantlet. The base is
    at −0.04 m against the ring guard's top at −0.02 m. The mantlet cover
    sits mostly inside the dome by design (155–170 of its 216 vertices),
    its nearest point 0.002–0.011 m from the shell.
  - How far each strays from the current dome:

    | Candidate | Mean | p95 | Max |
    |---|---|---|---|
    | smoothed | 0.013 m | 0.031 m | 0.038 m |
    | skin | 0.083 m | 0.210 m | 0.250 m |

  - Where the skin strays: it has a vertical lip round its base and
    terraced walls, so its shoulders stand proud of the dome's line.
- **E1-5 The smoothing technique.** (Agent; delegated by the slice.)
  - The profile is a Hermite curve through `dome_rings`' own five
    stations, with four sub-rings between each pair.
  - The plan blends its front and rear radii smoothly across the flanks.
  - Shading is smooth, with 48 points a ring at tier 0 and the roof poked to
    a centre vertex.
  - Tier triangles: current 62,382 / 19,344 / 5,264 / 2,268; smoothed
    63,806 / 19,574 / 5,244 / 2,190.
- **E1-6 No candidate carries the head-out commander.** (Agent.) This
  follows taste T5, and it keeps the dome the only variable.
- **E1-7 The slice defined its own turret shots.** (Agent.) Slice 01's
  `assets/shots.txt` isn't on this branch, so the shots come from that
  slice's definitions. All yaws are 35–50° off the nose toward the right
  side, round the dome's centre.
  - `battle_zoom`: 25 m and 0.22 rad, from `fixtures/presentation.json`
    (`camera.zoom_min`, `pitch_curve[0]`), cropped by a 150 mm lens;
  - `cinematic`: eye 1.5 m up, 10 m out, 85 mm;
  - `ref_angle`: 20 m, 0.45 rad, 110 mm, about the reference photo's angle;
  - `dome_close`: 7 m, 0.55 rad, 60 mm.
  - When slice 01 lands its shot list, these crops remain the record.
- **E1-8 compare-screenshots can't separate the three by pixels.**
  (Territory.)
  - Against the reference photo's turret crop, all six pair distances lie
    between 0.716 and 0.732. The camera, light and background all differ
    from the photo, so the metrics are diagnostic only.
  - Landmarks, reference → candidates:
    - The reference shows a cast wall below the cheek ERA, with the bricks
      standing proud of it. Current and smoothed both keep that. The skin's
      wall bulges out to the bricks' faces, so they sink in.
    - The reference's roof is low and flattish. Current steps up it in
      polygon facets, smoothed rolls into it, and the skin domes it like a
      helmet.
- **E1-9 Verdict: the smoothed dome wins, and the skin does not clearly
  win.** (Unprimed critique, one round, unlabelled, in random order; read in
  [`critique.md`](assets/experiments/03/critique.md).)

  | Candidate | Silhouette | Surface | Fittings | Materials | Cohesion | Rank |
  |---|---|---|---|---|---|---|
  | smoothed | 5 | 7 | 5 | 5 | 6 | 1 |
  | current | 5 | 4 | 5 | 4 | 5 | 2 |
  | skin | 3 | 2 | 3 | 4 | 3 | 3 |

  - The skin ranks last. Its surface is 5 below the smoothed dome's, and its
    silhouette dropped from 5 to 3. The critique saw:
    - a balloon dome;
    - a crumpled, spiky lower wall;
    - ERA sunk to stubs.
  - My own look confirms all three. The spiky wall is the generator's
    terracing, which decimation sharpens.
  - **Slice 14 is not needed.**
  - **Slice 17** applies `assets/experiments/03/smoothed_dome.py` to
    `t72.dome_rings`:
    - Use `parts.mesh_part(..., smooth=True)`, because `loft` can't shade
      smooth.
    - The T-80 imports `dome_rings` (`t80.py:32`), so check it on the T-80
      too.
    - Then answer the critique's shape note, inside slice 17's critique
      rounds: the smoothed dome reads as "a soft, low bowl", while the
      reference has steeper lower walls below the ERA. Tune the five
      stations so a steeper lower band rolls into the roof through a
      tight, rounded shoulder. The critique saw the hint of that crease in
      the current dome, and the smoothing kept its stations.
  - Findings outside this slice's variable, which every candidate shares,
    go to slice 17's done list:
    - the smoke launchers sit front-centre and read like headlights;
    - the front ERA fans out like petals instead of a forward-jutting
      chevron;
    - the mantlet is a box, not a bulged fabric cover.
- **E1-10 The skin is pinned like a jet skin, and worn by nothing.**
  (Agent.) Its files: `assets/source/skins/t72/eastern_t_72_t_72b3_2016_turret.glb`,
  its input view beside it, and `packages/scene-assets/blender/skins/t72.json`.
  It is kept as a generated asset. No export loads it.
- **E1-11 The smoothed patch is committed as evidence, not only kept in
  `throwaway/`.** (Agent.)
  - The scenario: slice 17 opens on a fresh worktree. `throwaway/` is
    ignored, so its copy of the patch is gone. Slice 17 would have to
    rewrite the smoothing from this ledger's prose and could drift from what
    the critique scored.
  - What was done: the exact code that built `candidates/smoothed.glb` is
    committed in the lane's own folder, as
    `assets/experiments/03/smoothed_dome.py`. Only its docstring was edited
    after the run. No shipped script changed.
- **E1-12 The skin's triangles per tier: 6,000 / 2,000 / 500 / 160.**
  (Agent.)
  - The scenario: the generator returned 188,182 triangles. Each tier is
    collapsed, symmetrically across the turret's centreline, to a budget
    about three times the smoothed dome's. A collapse that tight can sharpen
    the generator's small terraces into spikes.
  - The critique's "spiky lower wall" may owe part of its strength to
    that.
  - Its other findings don't depend on triangle count:
    - the balloon profile;
    - the ERA sunk to stubs;
    - the front-to-back symmetry.
- **E1-13 The critique was told what to look at, but not what to find.**
  (Agent.) Its prompt said "judge the turret, especially its main cast body,
  against the reference". That is the slice's crop. It named no candidate,
  defect or expected winner.

### E1 audit: verdicts (the lane auditing its own decisions)

The least-confident calls, to review first: E1-3, E1-12 and E1-2.

- **needs-user**
  - **E1-3, the second call not spent** (medium confidence). It is an
    external cost and a call on how thorough to be.
    - The provisional call: stop at one call. The best a skin can do is
      reproduce its smooth guide, which ties the smoothed candidate, so no
      second call could meet "clearly wins".
    - To reverse it: one Meshy v7.1 call through the proxy, from the same
      guide's views.
- **sound**
  - **E1-12, the skin's triangle budget** (medium confidence). It may make
    the skin's surface look worse. Its silhouette loss doesn't depend on it,
    and the rule needs both, so the verdict holds.
  - **E1-2, the smoothed dome as the guide** (medium confidence). Feeding the
    faceted dome would have handed the skin its facets. The smoothed guide
    gives the skin its best case.
  - **E1-4, fitted to the guide's bounding box per axis** (medium
    confidence). It is the only fit that needs no unexplained offsets. A fit
    shaped to keep the ERA proud would be the reshaping machinery the slice
    rules out.
  - **E1-7, shots defined in the slice** (medium confidence). Slice 01's
    shot list isn't on this branch. These shots read the same camera
    config.
  - **E1-11, the patch committed** (high confidence). Slice 17 needs it,
    and it lies inside the lane's own folder.
  - **E1-1, Tripo as the fallback** (high confidence). The brief forced it.
  - **E1-6, no crew** (high confidence). Taste T5.
  - **E1-8, a landmark verdict over pixel metrics** (high confidence). The
    camera and light mismatch makes pixel distance diagnostic only.
  - **E1-13, the critique's scope** (high confidence).
  - **E1-10, the pin kept and worn by nothing** (high confidence). This is
    the saved "keep generated assets" preference.
  - **E1-9, the verdict** (high confidence). The skin ranks last, so the
    rule fails on both of its counts.
- **unsound:** none.
