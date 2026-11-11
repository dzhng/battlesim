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

## Lane E2: slice 04 seam experiment (2026-11-11)

The evidence is in [`assets/experiments/04/verdict.md`](assets/experiments/04/verdict.md).

- **04 seams: grooves win (D1 settled).** The per-model baked normal+AO map
  did not clearly win. In an unprimed, unlabelled critique of five
  candidates, it ranked third and fourth on both models. Its surface scores
  were 0.5–2.5 points below the grooves. It also showed unique-UV artifacts
  along the seams. So S8 does not trigger, and no follow-up spec is opened.
  - **Surface scores** (battle zoom / cinematic):

    | Candidate | T-72B3 | Leopard 2A6 |
    |---|---|---|
    | grooves 2 × 2 cm | 6 / 6 | 5.5 / 5.5 |
    | grooves 1 cm × 6 mm | 6 / 6 | 5.5 / 5.5 |
    | bake from 2 × 2 cm | 6 / 5.5 | 5 / 4.5 |
    | bake from 1 cm × 6 mm | 5.5 / 5 | 4.5 / 3 |
    | control | 5 / 4 | 4 / 3.5 |

  - **The 1 cm × 6 mm grooves vanished at `battle_zoom` on the T-72B3.**
    Fewer than 0.1% of the glacis and side crop's pixels moved by 16 grey
    levels. That triggered the one deeper re-run, at 2 × 2 cm. It moved the
    most pixels on every crop: 0.29% glacis, 0.92% side and 3.36% Leopard
    cheek, by 16 levels or more. At native battle-zoom size, the Leopard's
    wedge joints read; the T-72's seams are only barely there.
  - **The cause, for slice 13.** `parts.finish` starts its occlusion rays 12 mm
    off the surface (`parts.py:664`). So a 6 mm groove floor gets about 4%
    darkening, and a 20 mm floor about 30%. At the closest battle camera one
    pixel covers about 2 cm.
- **04 costs.**
  - **Grooves.** The T-72B3, 36 seamed plates: tier 0 goes from 62,382 to
    66,588 triangles (+6.7%), and the GLB grows 3.2%. The Leopard 2A6, two
    wedges: 56,256 to 57,268 (+1.8%), GLB +1.2%. Tiers 1–3 are unchanged.
    That is about 120 tier-0 triangles per seamed plate.
  - **Bake.** Two textures (normal + AO) per part, or one atlas per model.
    - At 400 px/m it measured 14.2 M texels and 8.3 MB of PNG for 24 T-72
      plates.
    - A whole-model atlas at runtime RGBA8 is 8192², 268 MB per model, at
      400 px/m. At 100 px/m it is 2048², 16.8 MB per model, about 620 MB
      for the 37 appearances against the 640 MiB catalog tripwire.
    - It also needs a second UV, bundle support and a shader change.
- **04 signature for slice 13: `armour_parts.panel_seam(plate, points,
  width=0.02, depth=0.02)`.** It cuts a groove along a polyline into a plate's
  tier-0 mesh, inside a family's `build`, before `finish` bakes vertex colour.
  - **`plate`:** the part name its `box`, `prism` or `loft` was given. Only
    `<plate>_LOD0` is cut, because `battle_zoom` draws tier 0. Tiers 1–3 keep
    the plain plate.
  - **`points`:** two or more points in the plate's parent frame (the hull
    or turret node), on or within a few centimetres of one face, each snapped
    to the nearest surface. The polyline is closed when the first point equals
    the last.
  - **`width`, `depth`:** in metres. The defaults, 2 × 2 cm, are the
    candidate that read best at `battle_zoom`. `depth` must stay above the
    occlusion bake's 12 mm ray offset, or the groove gets no darkening, so
    use no less than 1.5 cm.
  - **Roles:** none of its own. The groove takes the plate's material (the
    paint role) and adds no part or node. `finish`'s occlusion darkens the
    floor, and the paint's edge-wear chip lights the lips. It returns
    nothing.
  - **Geometry:**
    - one swept box per segment, `width` wide, extended `width / 2` past
      each end so corners close;
    - cut with Blender's Manifold boolean solver;
    - the Exact solver deleted a closed, positive-volume Leopard wedge
      (148 → 2 triangles);
    - a cut that loses faces is a build error.
  - **Placement:**
    - **Seams follow the reference's real joints.** Inset lid rectangles on
      the T-72 side panels read as "cabinet doors"; its skirts show vertical
      joints only.
    - **Keep a seam about 10 cm or more clear of an existing edge or gap.**
      Lid lines 4.5 cm inside the glacis tile gaps merged with the gaps and
      vanished.
  - `weld_line` stays for raised welds.
- **04 finding for lane B (slice 22).** Every Leopard 2A6 render, the
  control included, shows hard-edged light and dark triangles across the
  wedge faces that follow its mesh triangulation. It is not a seam effect.

### Lane E2 decisions audit (slice 04)

These are the calls lane E2 made that the slice left open. They are grouped
by verdict. Within each group the least confident comes first.

**Review these first:** "panel_seam cuts tier 0 only", "the default is
2 × 2 cm", and "the bake's high source was the groove geometry".

#### Sound

- **The bake's high source was the groove geometry (confidence: medium).**
  - **What happened.** To make candidate "bake", lane E2 took the same
    seamed model the grooves used as the high-detail source. It baked that
    model onto the plain one's surfaces: a tangent-space normal map, and an
    occlusion map that only counts geometry within 10 cm. It used a fresh
    automatic UV unwrap per part, at 400 texels per metre.
  - **What that means.** A groove with square walls looks, from straight
    above, like a flat floor. So the normal map carried almost nothing, and
    the bake's only seam signal was its occlusion. The automatic unwrap cut
    islands along the seams, which caused the dashes and slashes the critique
    saw.
  - **The alternative.** A V-shaped or rounded groove as the high source,
    with an unwrap that keeps each plate face whole. That would give the
    normal map something to carry, and remove the island-edge artifacts.
  - **The gap.** The slice named "a baked normal+AO detail map" but no high
    source or unwrap.
  - **The reach.** The bake lost partly on artifacts that a second round
    might remove. It would not remove its cost, though: about 17 to 268 MB
    per model at the runtime's uncompressed RGBA8.
  - **Verdict: sound.** The one-round cap applies (Q7). The cost alone rules
    the bake out under the 640 MiB tripwire.
- **The default is 2 × 2 cm (confidence: medium).**
  - **What happened.** The critique scored the 1 cm × 6 mm grooves and the
    2 × 2 cm grooves equally, and preferred the thin ones in the 10 m view of
    the T-72. Lane E2 still picked 2 × 2 cm as the default.
  - **Why.** The slice's one variable is whether seams read at the closest
    battle camera, and there the deeper groove moved the most pixels on every
    crop. The thin groove's floor also sits above the occlusion bake's 12 mm
    ray offset, so it never darkens.
  - **The gap.** The slice delegated the dimensions.
  - **The reach.** It sets slice 13's default, and every family's seams with
    it.
  - **Verdict: sound.** It follows the slice's variable. A family can pass a
    thinner width for cinematic-only detail, keeping depth at 1.5 cm or more.
- **panel_seam cuts tier 0 only (confidence: medium).**
  - **What happened.** The signature cuts only `<plate>_LOD0`.
  - **What a player sees.** Tier 0 is drawn while the model's largest extent
    spans more than 150 px. For a T-72 at 1080p, that is closer than about
    80 m. Past that, seams disappear with tier 1. At that distance a 2 cm
    seam is a fraction of a pixel anyway.
  - **The alternative.** Cut tier 1 too: about 120 more triangles per plate
    on a tier that must stay much smaller than tier 0.
  - **The gap.** The slice gave no tier policy for seams.
  - **Verdict: sound.**
- **battle_zoom was defined here (confidence: medium).** Slice 01 owns
  `assets/shots.txt`, which doesn't exist on this lane's branch point. So
  lane E2 read the camera from `fixtures/presentation.json:28-35` and fixed
  the rest itself:
  - 25 m orbit, 0.22 rad pitch, 0.8 rad vertical field of view;
  - 1920×1080;
  - the camera 35° off the nose on the left;
  - the target 1 m above the hull's footprint centre;
  - the sun at 0.7 rad elevation, 70° left of the camera's bearing.

  It also set the "cinematic" shot: 1.5 m eye height, 10 m out, 60 mm lens.
  If slice 01's shot differs, slice 13's before/after must use slice 01's.
  **Verdict: sound.** The pitch, distance and field of view are the game's
  own; the bearing and sun are a judgement.
- **Blender, not the game renderer, judged every candidate (confidence:
  medium).** The bake can only be rendered in Blender (S8). So lane E2
  rendered all five candidates in Blender EEVEE, with the game shader's
  colour rule (albedo × vertex colour × `colour_scale`) rebuilt on the
  imported materials, to compare like with like. The grooves were never seen
  in the game's renderer; slice 13 is the first time. **Verdict: sound,** and
  named as unfinished in the slice.
- **The Manifold boolean as the groove geometry (confidence: medium).** The
  signature tells slice 13 to cut with a swept box per segment and Blender's
  Manifold solver. That costs about 120 tier-0 triangles per plate, because
  the cut leaves n-gons that `finish` triangulates and splits.
  - **The alternative.** Build grooves into each plate's own topology, as
    `armour_tiles` already does with its gaps. That is cheaper, but needs a
    face-aware helper per primitive.
  - **Verdict: sound.** The cost is measured and well inside the caps:
    66,588 of 120k on the T-72B3. Slice 13 may still choose topology if it is
    simpler.
- **The seam layout was a probe (confidence: high).** Lane E2 placed seams
  where they would test readability, not where the references show joints:
  inset lid rectangles on the glacis tiles and side panels, and joints
  across the Leopard wedge. The critique called the side-panel lids "cabinet
  doors". **Verdict: sound** for this experiment's variable. The lesson
  ("seams follow the reference's real joints") is in the signature.
- **Probe builds drop crew and skip lint (confidence: high).** The crew
  import needs an infantry GLB that isn't pulled, and slice 12 removes crew
  anyway. Lint can't see grooves. Neither touches the seam crops.
  **Verdict: sound.**
- **Evidence is committed in full (confidence: high).** That is about 25 MB of
  PNG in LFS: montages, frames, closeups and four bake maps. This follows
  "keep generated assets". The CC-BY-SA T-72 photo is cited, not copied.
  **Verdict: sound.**
