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

## Slice 00 decisions (2026-11-11, coordinator's implementation agent, audited)

Every entry landed in slice 00's commits. Each one gives the decision, the
gap the plan left, its reach, a verdict and a confidence (how sure the audit
is that the user would make the same call). They are ordered least confident
first. Review these first: the track-against-wheel overlaps, `asset check`
now needing Blender, and roster scripts run as `__main__`.

- **Track-against-wheel overlaps stay issues, not fixed and not allowlisted.**
  - *Scenario.* The overlap check now compares the track band (`track_L`)
    with each wheel (`wheel_L_1` and the rest), because they are separate
    bodies. On the T-72B3, about 36 of its 75 issues are the band against its
    own wheels:
    - road wheels 1 and 6 and the idler, up to 414 mm deep;
    - the return rollers, about 200 mm;
    - the sprocket teeth, about 36 mm.

    A real track wraps those wheels and touches them; it doesn't swallow
    them 40 cm deep. The depths probably come from how the check measures
    "inside". It casts rays and counts how many surfaces each ray crosses,
    and that goes wrong on a ring-shaped belt. Today these go into slice 01's
    baseline as known issues. The alternative is an allowlist pattern
    (`track_*` against `wheel_*`), which would also hide a real clash, such as
    a road wheel poking through the belt.
  - *Gap.* S1 decided the grouping, but not what to do with the running gear
    it newly compares.
  - *Reach.* Every tracked vehicle carries about 20–40 of these in the
    baseline. Slice 10 (track sag) is where they can be judged in a low side
    crop.
  - *Verdict.* Sound for now: the slice forbids new lint checks, and the
    baseline is a reversible holding place. Unverified: no one has looked at
    a render of these pairs.
  - *Confidence:* low.
- **`asset check` now needs the pinned Blender to pass.**
  - *Scenario.* Someone runs `bun run --cwd web asset -- check` on a machine
    without Blender 5.2.1. Before, the check only baked and compared, so it
    passed. Now it also runs `vehicle_lint_test.py` and `wreckage_test.py`
    inside Blender. Those two report "Blender not found", and the check
    fails.
  - *Gap.* S11 put the Blender tests in `asset check`, but didn't say what
    happens without Blender.
  - *Reach.* No CI runs `asset check` today, and every milestone push runs
    it on the coordinator's Mac, which has Blender. A CI job would need
    Blender installed.
  - *Verdict.* Sound. Skipping the Blender tests when Blender is missing
    would let a check pass that didn't check. If that is ever wanted, make
    the skip print loudly.
  - *Confidence:* medium.
- **The roster walk runs each roster script as `__main__`, with export
  captured.**
  - *Scenario.* `--roster` runs, for example, `roster/stryker_support.py` the
    way an export would. When it calls `run(...)`, `vehicle_export._export`
    sees that `vehicle_export.CAPTURE` is a list. It appends the variants,
    their paint scheme, the family's `build` function and its `looks`, then
    returns without building anything. The walk then builds each captured
    variant alone and lints it.
  - The plan's routes were reading the `appearances=` list or the catalog's
    `family_variants`. The old code guessed both by parsing the script's
    source text, about 180 lines. Capture gets exactly what the export would
    build, including both of `stryker_support.py`'s two `run` calls.
  - The cost: a script's whole `__main__` block executes during the lint. A
    future script that, say, wrote a file before calling `run` would write
    it during every lint.
  - *Gap.* How to enumerate was delegated. This is the choice.
  - *Reach.* A new roster script needs no lint registration. It must keep its
    `__main__` block to "work out arguments, call `run`". A script is walked
    when it imports `run` or `run_disabled` from `vehicle_export`.
    `lint_render.py` shares the same walk.
  - *Verdict.* Sound. Every one of the 79 scripts follows that shape today.
  - *Confidence:* medium-high.
- **`--variant=<id>` judges only that appearance.**
  - *Scenario.* You lint the T-72B3 alone while `roster/a10.py` is broken.
    The walk runs scripts in alphabetical order until one exports the T-72B3,
    so it runs `a10.py` first, and that fails. The run prints the failure,
    but the exit code reflects only the T-72B3: its new issues, or its own
    build failing. `--roster` still fails on the broken A-10.
  - *Gap.* "Exit non-zero … in both modes" named only `--roster` and
    `--update-baseline`.
  - *Reach.* Family lanes use `--variant` to reach zero issues, and an
    unrelated broken family can't block them.
  - *Verdict.* Sound.
  - *Confidence:* medium-high.
- **`asset check` finds the tests itself.**
  - *Scenario.* `check` runs every `*_test.py` at the top of
    `packages/scene-assets/blender/`. One whose source has a line starting
    `import bpy` or `from bpy` runs in the pinned Blender; the others run
    under `python3 -m unittest`. It prints one line per test after the bake
    results, prints a test's output only when it fails, and fails the check
    on any failure. A sixth test added there joins with no list to edit.
  - `city/facade_test.py`, in a subfolder, is not run. City work is outside
    this spec. Reverse it by making the search recursive.
  - *Gap.* Discovery against a fixed list of five.
  - *Reach.* Tests in that folder must exit non-zero when they fail; both
    Blender tests already do.
  - *Verdict.* Sound.
  - *Confidence:* high.
- **S1 is applied as the articulated grouping.**
  - *Scenario.* With each hull child its own body, the T-72B3 gets 361
    overlap issues. Most are a grille slat inside its frame, or a latch
    through its lid: fittings built from overlapping boxes on purpose.
  - With the articulated grouping it gets 75: turret against hull fittings,
    hull against running gear, and track against wheels. Under it, the hull
    and everything fixed to it is one body; each `wheel_*`, `track_*` or
    `gear_*` node on the hull is its own; and each node hung off the root
    (the node carrying `unit_id`, such as the turret) is its own, with all
    it carries.
  - A real clash between two hull fittings (a fender box sunk through a
    fender) is no longer caught. Critique and closeups cover that.
  - *Gap.* None: S1 gave the rule. Rule held: more than 30, mostly by design.
  - *Reach.* `vehicle_lint._part_group`, with `RUNNING_GEAR` naming the hull
    children that count as their own bodies. Family lanes reach "zero issues"
    against this grouping.
  - *Verdict.* Sound.
  - *Confidence:* high.
- **Disabled cards are linted in the roster walk.**
  - *Scenario.* `run_disabled` exports, the 13 drone and MQ-9 cards, already
    ran the lint when exported. The old walk never visited them, so the
    baseline could never hold their issues, and an issue would have failed
    their export. All 13 are clean today.
  - *Gap.* "Every appearance a script exports" didn't mention disabled cards.
  - *Reach.* A disabled card's model issues block its own export, as for any
    vehicle.
  - *Verdict.* Sound.
  - *Confidence:* high.
- **One owner builds a variant: `vehicle_export.build_variant`.**
  - *Scenario.* An export, the roster walk and `lint_render.py` each need "a
    fresh scene with this variant built in its paint". Before, the walk
    rebuilt it by hand, with a scheme guessed from the source text.
    `lint_render.py` had its own copy of that guess and also patched
    `_export` away.
  - Now all three call `build_variant(variant, scheme, build, looks)`. A fresh
    T-72B3 export after the change is byte-identical to one from before it.
  - *Gap.* Not named in the slice.
  - *Reach.* Anything that builds a variant outside an export calls this.
  - *Verdict.* Sound.
  - *Confidence:* high.
- **Two synthetic tests now put their second box on a turret.**
  - *Scenario.* `test_no_issues_with_separate_boxes` and
    `test_no_issues_with_touching_boxes` put both boxes on the hull. Under the
    articulated grouping, two boxes on the hull are never compared. Those
    tests would then pass even if touching boxes were wrongly flagged.
    Putting the second box on a `turret` node makes the check compare them
    again.
  - The new test also puts a bolt sunk into its plate on the same hull, and
    asserts that only the turret-against-hull pair is flagged.
  - *Gap.* Forced by the grouping.
  - *Reach.* None outside the test.
  - *Verdict.* Sound.
  - *Confidence:* high.
- **The old `--variant` mode, three dead helpers and a duplicated baseline
  loader were deleted.**
  - *Scenario.* The old `--variant` mode guessed the scheme as `olive_drab`,
    failed, swallowed the error, and printed nothing. The deleted code:
    - `_is_running_gear`, which had no callers;
    - `lint_current_scene`'s keyword ladder, which passed each check its own
      defaults;
    - `vehicle_lint.py`'s own `load_baseline`, `save_baseline`,
      `load_allowlist` and `save_allowlist`, which repeated
      `vehicle_lint_core`.
  - `vehicle_lint.py` went from 42,691 B to about 23,700 B, under the
    ratchet.
  - *Gap.* Housekeeping.
  - *Reach.* `vehicle_lint_core` is the only owner of baseline files.
  - *Verdict.* Sound.
  - *Confidence:* high.
- **Trivial discretion (3).**
  - The lint with no mode prints its usage and exits 2.
  - Duplicate appearance ids keep the first script's capture.
  - Summary lines read `Name [id, script]: <check> <n>, …; <n> new`.
- **The committed T-72B3 art lags its sources.** Territory, not a decision.
  Fresh exports repeat byte for byte, but don't match the committed GLB, so
  the sources changed after the last export. Slice 01's re-export absorbs it.

## Coordinator decisions during implementation

- **C1 Experiments and the survey start from the spec commit, not slice 02**
  (2026-11-11). Lanes E1 (03), E2 (04) and F-A/F-B/F-C (07) write only to
  `throwaway/`, their own `assets/experiments/` or `assets/frames/` folders,
  reference libraries and the overlay script. None of those collide with slices
  00–02. Starting early shortens the critical path. The survey overlays the
  GLBs committed before slice 01. Slice 01 only removes drift from stale
  helpers, and that drift doesn't affect frame measurements.
- **C2 Lanes save checkpoint shots instead of opening Preview** (2026-11-11).
  Each lane writes its shots into the spec. The coordinator opens them with
  `preview-shots` at milestones. That keeps several unattended agents from each
  opening windows, and the checkpoints still never block.
- **C3 Generated skins go through Bento's fal proxy** (2026-11-11). The user
  declined to hand over a fal key and asked for Bento instead. Hunyuan3D 3.1
  Pro is not on Bento's admitted fal list. Lane E1 tries it through the proxy
  first and falls back to the admitted default, `tripo3d/h3.1/image-to-3d`.
  `tools/skin-generate.mjs`, which calls fal directly with a key, is not used in
  this pass.

## Slice 01 decisions (2026-11-11, coordinator's implementation agent)

Each entry gives the decision, the gap the plan left, its reach, a verdict
and a confidence, least confident first.

Review these first: the standard shots' framing (`battle_zoom`'s yaw and the
cinematic's lens), holding back `air_defence.py`, and how the tracked issues
were sorted into kinds.

- **The standard shots (`assets/shots.txt`).**
  - `fq` and `side` are `closeups.py`'s own `front-quarter` and `side-low`
    at 800×500. Rendering the committed T-72B3 with them reproduces the
    planning closeups at distance 0, so that is what the planning lineup
    used.
  - `battle_zoom` is 25 m out (`zoom_min`, `fixtures/presentation.json:30`)
    at pitch 0.22 rad (`pitch_curve`'s first point, `:33-36`), 45 degrees
    off the nose on the left, aimed at the ground under the origin. It
    renders at 1920×1080 with a 23.9 mm lens, which is `fov_y` 0.8 rad
    (`:28`) at 16:9. There a 10 m tank is about 500 px long, roughly 5 px
    per 10 cm. That is the scale at which to judge whether a fitting reads.
  - `cinematic` is 12 m out, 35 degrees off the nose, the eye 1.5 m up, on a
    70 mm lens at 1920×1080. It frames the front two-thirds of a tank. The
    slice asked for "8–12 m and a long lens", and the far end of that range
    keeps most of a tank in frame.
  - *Gap.* The yaw, the aspect and the cinematic's exact numbers were open.
  - *Verdict.* Sound.
  - *Confidence:* medium.
- **`air_defence.py`'s re-export is held back: its art stays as
  committed.**
  - *Scenario.* The bake's fit check compares each model's outer faces
    with its unit's hull frame (`half_extents_m`), and fails a face more than
    10 cm off. Re-exported, the Buk-M3, Gepard 1A2 and Pantsir-SM (a
    wheeled sibling) fail it. Their tow hooks reach
    12.7 cm past the hull frame, against a 10 cm tolerance
    (`fit.hull_extents`). The Buk's wreck also fails `fit.footprint`: its
    right side, stripped of its skirt, stands 11 cm inside the footprint.
    Their frames grew in the fixtures on 2026-10-15, after the art was last
    exported on 2026-10-08:
    - the Buk from 9.28 to 9.75 m and the Gepard from 6.82 to 7.11 m;
    - the Pantsir's rear by 13 cm.

    The scripts build the hull flush to the frame and hang the tow hooks
    past it (`air_defence.tracked_hull`, and the Pantsir's rear hook). The
    committed art passed because its hulls were shorter.
  - Seating the hooks within the frame is family geometry. On the tracked
    hull they hang from the upper hull's overhang, and moving them in
    leaves them floating. It also reaches the wheeled siblings, which this
    pass only re-exports. So the committed GLBs, wrecks and receipt of all
    six appearances stay as they were. The lint baseline still holds these
    appearances' issues as their sources build them.
  - *Gap.* The plan assumed every tracked script re-exports cleanly.
  - *Reach.* Lane B's slice 24 starts from art that lags its frames by
    those lengths. Its before/after will show the lengthening, and it must
    seat the hooks to pass the bake. The Pantsir's rear hook needs the same
    fix in the wheeled pass. The Buk and Gepard closeups in
    `assets/before/reexport/` are of the committed art.
  - *Verdict.* Sound for a baseline slice. The alternative was a
    geometry change to a lane-owned family, and a wheeled sibling, outside
    "trivial".
  - *Confidence:* medium. The coordinator may prefer to land the hook fix
    before slice 24.
- **How the 1,204 tracked issues were sorted into kinds.** The coordinator
  will set the family slices' "zero issues" rule from this, so the
  definitions matter.
  - *Scenario.* Each overlap joins two bodies: the hull, the turret, or a
    piece of running gear. The kind is read from that pair and the part
    names:
    ```
    floating, and the part sits inside another (a barrel in its receiver) -> other
    floating otherwise                                   -> real misplacement
    wheel against wheel on one side                      -> real misplacement
    hull shape (`hull_*`) or road arm against gear       -> running gear seated into hull
    any other hull fitting against gear (skirt, bin, light) -> real misplacement
    turret shell or ring against a hull shape            -> turret seated into hull
    anything else between hull and turret                -> fittings into armour
    ```
    The deepest issue of each kind, in up to five families, was rendered in
    context and alone to check what it looks like. "Fittings into armour" is
    split by side in the slice file, because the two halves look different:
    hull fittings under the turret's overhang are hidden, while the T-72
    family's ERA bricks sunk into fender boxes are visible.
  - *Gap.* The brief named four kinds (turret seated, fittings into armour,
    real misplacements, other) but no way to sort into them. "Running gear
    seated into hull" was added as a kind of its own: it is 58 % of the
    issues and would otherwise swamp "other".
  - *Reach.* The headline, "seven in ten issues never show on screen", rests
    on these rules plus about 30 rendered samples, not on judging all 1,204.
    A "real misplacement" by these rules is one the Blender README forbids
    or a part in the wrong place, not necessarily one a player would notice.
  - *Verdict.* Sound for setting a strictness level. A family lane judges its
    own issues in renders anyway.
  - *Confidence:* medium.
- **A track belt is not compared with the wheels on its own side.** This
  reverses slice 00's "Track-against-wheel overlaps stay issues".
  - *Scenario.* Running-gear closeups of the T-72B3 (hull hidden and shown)
    show a closed belt wrapping every wheel on its side. The road wheels sit
    on its lower run, it wraps the idler and sprocket, and the return rollers
    sit under its upper run. Nothing pokes through. A nearest-face inside
    test (the outward normal's sign, which a ring can't fool) finds no
    road-wheel, idler or roller vertex inside the belt. The 400 mm depths
    were the lint's inside test misreading points in the loop's hollow. That
    test (ray parity) casts rays from a point and calls it inside a part when
    the rays cross the part's surface an odd number of times, and a point in
    the empty middle of a ring-shaped belt can fool it. Only
    the sprocket's teeth sit 36 mm into the 80 mm belt, which is how they
    drive it.
  - The rule is `vehicle_lint_core.compared`: a body `track_L` or `track_R`
    is not compared with `wheel_L_*` or `wheel_R_*` on its own side. A belt
    is still compared with the hull, fenders, exhausts, skirts, the turret
    and the other side. `vehicle_lint_logic_test.py` pins it.
  - *Gap.* Slice 00 parked these pairs in the baseline without a render.
  - *Reach.* Roster-wide, the overlap count falls from 4,990 to 3,348, and
    the T-72B3's from 75 to 43. A wheel that really pokes through its own
    belt is no longer caught. `track_loop` builds the belt round every wheel
    it is given, so only a wheel left out of that list could do it, and
    closeups would show it.
  - *Verdict.* Sound. It makes the check do what `track_loop`'s docstring
    says the belt is. Unverified: other ring-shaped parts (turret ring
    guards) may also be measured too deep by the same ray-parity test.
    Their pairs stay compared; only the depth may read high.
  - *Confidence:* medium-high.
- **What counted as trivial: one fix, the SP howitzers' turret numbers.**
  - *Scenario.* The slice allows fixing "a part floating less than 2 cm, or
    an obvious misplacement". A floating part is only reported past 2 cm,
    so this was read as a gap within 2 cm of that threshold (under 4 cm), in
    the tracked scripts only. Wheeled siblings are re-exported, never
    reworked. That leaves three cases:
    - The turret number on the M109A7, PzH 2000, 2S19M2 and PLZ-05 hovered
      2.8–3.2 cm off the turret. It was placed at the foot's half-width, but
      at three quarters of the turret's height the flank has already leaned
      in toward the crown. It now sits on the flank at its own height and is
      tilted to the flank's lean (`sp_howitzers.turret`). Fixed.
    - `mg_barrel` on three of the howitzers sits inside its receiver.
    - The Gepard's `gun_v0_radar` is a sleeve round its barrel.

    The last two read as floating because the check measures surface
    distance, so a part contained inside another looks detached. That is
    the false positive the allowlist's barrel patterns already cover. They
    are baselined, not allowlisted, because the allowlist is coordinator
    policy.
  - The Buk's and Gepard's side skirts (7.2 cm clear of everything, with no
    brackets) and the T-72 family's ERA bricks sunk 10–12 cm into the fender
    boxes are real, but fixing them means drawing a hanger or moving armour.
    That is family work, so they are baselined.
  - *Gap.* The bound on "trivial" was delegated.
  - *Reach.* The four howitzers gain 144 tier-0 triangles each from the
    tilted stencil. Their closeups show the number on the flank.
  - *Verdict.* Sound.
  - *Confidence:* medium-high.
- **A fitting named after running gear belongs to the hull.**
  - *Scenario.* Categorising the issues turned up the Leopard's and BMP's
    `track_guard_L/R` fender plates as bodies of their own. Slice 00's
    grouping took any hull child starting `track_` as running gear, so a
    fender bolted to the hull was compared against the hull itself
    (`hull_upper` against `track_guard_L`, 43 mm). The GLB node contract
    names the belts exactly `track_L` and `track_R`, and
    `vehicle_lint_core.running_gear` now reads that: `track_L`, `track_R`,
    `wheel_*` and `gear_*`.
  - *Gap.* None in the plan. It's a bug in slice 00's grouping.
  - *Reach.* 30 false issues on the Leopards and BMPs leave the baseline. It
    also means `--update-baseline` ran twice, not once: first before the
    find, then after the fix. Only the second run's file is committed. The
    live exports were re-run afterwards, so their receipts hash the final
    lint sources. The wreck runs write no receipt, and the lint never
    changes a GLB's bytes, so they stand.
  - *Verdict.* Sound.
  - *Confidence:* high.
- **`closeups.py` takes metres.** A `--shot` value ending in `m` is metres
  in the model's own frame, not a fraction of its bounds.
  - *Scenario.* `battle_zoom` must keep the battle camera's real 25 m. A
    fraction of the bounds scales with each model's size, so the same shot
    would stand closer to a short IFV than to a long-gunned tank.
  - *Gap.* The slice asked for the battle camera's distance in a format that
    could not hold one.
  - *Reach.* A three-line change to the make-3d-models skill's script. The
    existing fraction shots are unchanged; the planning fq and side shots
    reproduce pixel for pixel.
  - *Verdict.* Sound.
  - *Confidence:* high.
- **Drift found by the re-export, named and not fixed.**
  - 117 of the 148 planning closeups (fq and side, shaded and matcap) are
    pixel-identical after the re-export. Most changed GLBs changed only
    below a pixel.
  - **The M270A2's and MARS II's three windscreen shutters moved 26 cm
    aft.** They no longer stand proud of the cab face and now lie over the
    windscreens, which reads better.
  - The Ajax's crew meshes lost one polygon (from the infantry source). The
    S-300 differs only by the last bit of one part's rotation and one UV
    accessor. Neither changes a pixel.
  - The SP howitzers' only change is this slice's turret-number fix.
  - *Verdict.* Every change makes the art match today's sources. None
    needs a fix here.
  - The `air_defence` script's drift was larger, and its re-export is held
    back; see "`air_defence.py`'s re-export is held back" above.
  - *Confidence:* high.
- **The exports ran after the baseline, with the lint on.**
  - *Scenario.* The slice lists the steps as re-export, then lint and fix,
    then baseline. Every export also lints and fails on any issue the
    baseline lacks. So exporting first would have needed `--no-lint`. Here,
    the trivial fix and the baseline came first, and every export then ran
    with the lint on. That doubles as the slice's check that "every export
    passes against the baseline it just wrote".
  - *Gap.* The batch order was delegated.
  - *Reach.* None beyond this slice. The lint builds from the scripts, not
    from the GLBs, so the order changes no result.
  - *Verdict.* Sound.
  - *Confidence:* high.
- **The unit icons were regenerated and are committed with the art.**
  - *Scenario.* `asset check` compares `assets/icons/units/*.svg`, each
    model's silhouette, against a fresh render of the GLBs. It flagged 11
    icons as stale, all of them re-exported models (the howitzers, the
    rocket launchers and their siblings, the BMP-3). `asset icons` rewrote
    them.
  - *Gap.* The slice didn't mention icons. The check forced them.
  - *Reach.* The player-facing unit icons change slightly for those 11.
  - *Verdict.* Sound: they follow the art.
  - *Confidence:* high.
- **Data found, not changed: the Buk-M3's unit is `wheeled_medium`.** The
  Buk is tracked, but `fixtures/catalog.json` gives its unit
  `vehicle_class: wheeled_medium`. That class picks its sounds and its
  triangle caps (70k at tier 0 rather than 120k). Fixtures are the
  coordinator's, and the class is a game-data decision, so it is reported,
  not changed.
- **Trivial discretion (2).**
  - `assets/before/reexport/` holds only the fq and side shots, the ones the
    planning closeups can be compared with. `battle_zoom` and `cinematic`
    are left to the family slices, which render them anyway (about 2 MB
    each at 1920×1080).
  - The roster walk's sampling probes and issue dumps stay in
    `throwaway/reexport/`.
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
## Lane F-B decisions (2026-11-11, slice 07 frame survey)

Lane F-B surveyed the Abrams, M1E3, M10, Leopards, Leclerc, KF51, Gepard,
Buk-M3, M270A2 and MARS II. The numbers are in
`assets/frames/<family>.json`. Who decided: the lane agent, under the slice's
delegation ("which source to trust", "the threshold tuning").

- **F-B1 Box top over a tall sight.** The convention puts the box at the roof
  and the sights in the 0.3 m bulky dressing allowance. But the Leopard's PERI
  (0.36–0.41 m over the roof), the M10's sight dome (about 0.4 m) and the
  Gepard's stowed search radar (0.5 m) stand past it, and none is thin. So the
  box top is the higher of the roof and the sight's top less 0.3 m. That puts
  the box within 0.06–0.11 m of the real roof, near the fix threshold, and
  each sight fits exactly.
  - Alternative: the box at the roof, with `bulky_m` raised to about 0.5 m or
    a sight clause in `unitArt.ts`. Rejected: that changes every unit's
    allowance (coordinator-owned) to suit three vehicles.
  - Reverse it by setting the box tops to the roofs in `leopard.json`,
    `m10.json` and `gepard.json` and raising the allowance.
- **F-B2 Which source wins.** Manufacturer data comes first, then a
  dimensioned drawing, then an undimensioned profile, then a photo scaled on a
  published length. Copyrighted drawings came from the-blueprints.com, the
  only place any of these vehicles is drawn. They were measured in
  `throwaway/fb/`, and only the numbers and links are committed. Commons holds
  clipart only, so rung 1 of Q4 found nothing. No generated view was made: in
  every family without a drawing, the open number was one a generated view
  would have to guess (the M10's width, the KF51's barrel, the Buk's layout
  and length).
- **F-B3 Muzzle threshold.** The slice set thresholds for hull dimensions and
  pivots only. A muzzle is fixed when its world point (pivot + muzzle) is off
  by more than 0.15 m on any axis. Final thresholds:
  - hull dimension: max(0.10 m, 3 %);
  - pivot x: 0.15 m;
  - muzzle point: 0.15 m per axis.
- **F-B4 The unsubtracted pivot.** On the Abrams (5.805 = 9.77 − 3.965), the
  Leopards (7.12 = 10.97 − 3.85) and the KF51 (6.1 = 10.0 − 3.9),
  `muzzle_m.x` equals the world x that the published gun-forward length
  implies. The pivot's x was never subtracted. The Abrams and Leopards are
  fixed. The KF51 is kept: its "about 10 m" is rounded, and an L51 on the
  Leopard 2 ring implies about 11 m, so the frame's 10.3 m sits between the
  two. Slice 23 measures the barrel. Frames with pivot x = 0 are unaffected.
  Lanes F-A and F-C may find the same pattern.
- **F-B5 Leopard 2A7V hull and gun.** The 2A7V gets the 2A8's 8.05 m hull,
  because the photos show the same bolt-on hull-front armour on both.
  - The module only adds hull at the front, so the turret points move 0.175 m
    aft in box coordinates.
  - The gun-forward length is 10.97 m (KNDS's 2A7 page and an earlier 2A8
    brochure). The 2A8 datasheet's 11.17 m is rejected: the same gun on the
    same ring can't reach 0.2 m further.
- **F-B6 Radars are sensors.** The Gepard's stowed search radar counts as
  dressing, like a sight (F-B1). So the box drops from 3.29 m (KMW, "radar
  retracted") to 3.0 m over a 2.75 m turret roof.
- **F-B7 A fixed mount's pivot.** The M270's launcher mount does not turn
  (`turret: false`), so only pivot + muzzle reaches the simulation. The pivot
  moves to the rear hinge pin anyway (D3, Q7), so the frame says where the art
  hinges the box. The muzzle moves to the pods' front face, which is the
  digest change.
- **F-B8 Leclerc drawing rejected.** By its own scale bar, the Leclerc
  drawing measures the width (3.60 m) and the overall length (9.94 m) right,
  but gives a hull of 7.21–7.43 m and a roof of 2.39 m. Published figures are
  6.88 m and 2.53 m, and the frame keeps them. The XLR's 3.71 m width is
  0.11 m over the frame, which is on the threshold, not past it, so it stays.
- **F-B9 Buk named, not fixed.**
  - The Buk-M3 extends `roster_profile_wheeled_ifv`, so the simulation drives
    a tracked vehicle on wheels. Changing the profile is a gameplay change
    beyond the frame survey.
  - Once tracked, its 4.876 m half-length breaks the 4.8 m cap. 9.3 m is the
    best-supported length.
  - The pivot fix (from x −2.45 to about +2.0, ±0.5 m) comes from photos with
    no scale. The real turret stands forward with the radar in front. The
    model has a cab at the front and the launcher at the rear.
- **F-B10 Overlay registration.** The committed `drawing_overlay.py` centres
  the model on the drawing's bounding box. That box includes antennas, so the
  vertical registration slips. The lane registered its overlays with a
  scratch script instead. It scales the drawing by its hull length, anchors
  it on the nose and the ground line, draws the frame box and prints 19
  stations. Each frame file records the registration (`overlays.views[]`), so
  F-A's committed vehicle mode can reproduce the stations. The lane's
  uncommitted vehicle-mode copy of `drawing_overlay.py` agreed on the Abrams:
  10.07 m model against 9.85 m drawing overall.

### Lane F-B decision audit (2026-11-11)

Verdicts on F-B1–F-B10, plus the calls the lane made without writing them
down. Least confident first.

- **F-B11 The frame-file schema (needs-user, low).** The slice listed what a
  frame file holds but not its shape, so lane F-B invented one:
  - `appearances.<id>` holds `published`, `frame`, `verdicts[]` and
    `inherits`. Each verdict is `{field, old, new, verdict, source,
    confidence, note}`.
  - Alongside sit `sources[]`, `overlays.views[]` (registration plus 19
    stations), `notes[]` (each with a `class`) and `answers`.

  Scenario: the coordinator builds `visualizations/frames.html` and slice 15's
  fix list from the three lanes' files. If F-A writes `"fix"` rows under
  another key, the page and the fix list need two readers.
  Provisional call: the coordinator adopts this shape, or maps the others
  onto it, before slice 15. To reverse, regenerate with the lane's scratch
  generator, which holds the data as plain Python.
- **F-B9 Buk pivot from unscaled photos (sound, low).** The real turret
  stands forward and the frame's stands aft: that much is certain, from two
  photos and the layout. The number (+2.0 ± 0.5 m) is not. The rebuilt
  `buk.py` (slice 24) draws its turret ring from the same photos, so slice 15
  should take the pivot from that art, not from this estimate.
- **F-B12 The M10's photo-scaled numbers (sound, medium).**
  - Scenario: no M10 drawing exists, so the side photos were scaled by
    assuming the 7.0 m hull is right (two sources say 6.85 m).
  - From that scale, the roof (2.39 and 2.58 m) and the barrel (5.45 m to the
    muzzle) were read.
  - A 2 % scale error moves the muzzle about 0.1 m, inside the threshold.
    The 0.8 m fix is far outside it.
- **F-B13 The Abrams roof gun (sound, medium).** The drawing's remote-station
  M2 sits 0.26 m higher and reaches 0.24 m further than the frame's HMG
  muzzle, so it was fixed under F-B3. Reach: it moves where the roof gun's
  rounds start, a named digest change.
- **F-B14 Gap lines point at spec paths (sound, medium).** Each family's
  `references.json` gap line names the numbers' home under
  `specs/tracked-armour-models/`. When the spec closes into `specs/done/`,
  those paths go stale. `close-spec` should rewrite them, or the gap lines
  should drop the path.
- **F-B15 How far the lane checked (sound, high).** The slice asks for
  `asset check` to pass. That command re-bakes every asset (it needs the wasm
  build and the whole runtime). The lane changed only gap strings, which
  only the reference contract reads, so it ran `checkReferences` on its ten
  libraries instead: all pass. The full check runs at the coordinator's next
  bake.
- **Verdicts on F-B1–F-B10:**
  - F-B1: sound, medium. It is a rule the user did not state. The Q4
    question shows they expected the PERI to fit or be named, and this does
    both.
  - F-B2, F-B3, F-B5, F-B7, F-B8, F-B10: sound, high. Each is within the
    slice's delegation.
  - F-B4: sound, high for the Abrams and Leopards, low for the KF51 keep.
  - F-B6: sound, medium.
  - F-B9: needs-user (the wheeled profile). Provisional call: leave it
    wheeled this pass. Reverse it by moving the Buk to a tracked profile with
    a 9.3 m hull.
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

## Coordinator decisions, slice 07b: reconciling the frame survey (2026-11-11)

Slice 07b merges lanes F-A, F-B and F-C into one list for slice 15. C4-C9
are the coordinator's calls; the implementation agent applied them and
recorded what it found. C10 is the agent's, made to keep the list
consistent. The numbers are in `assets/frames/<family>.json` and
`assets/frames/fix-rows.json`.

- **C4 One box-top rule for every lane.** The hull box top is
  max(the real hull or turret roof, the top of the tallest bulky dressing
  - 0.3 m). This is F-B1's rule. The `unitArt.ts` allowance stays at 0.3 m
  bulky and 0.15 m thin.
  - Why: the allowance stays untouched for every unit, and the box sits at
    most 0.3 m above the real roof. The art puts each sight head under
    `dressing_*`. This replaces FC3 and FA-1's "a dressing rule for sight
    heads", which would have widened the allowance for every unit.
  - Every height row was re-derived. Each family file's `heights` block
    holds the roof, the tallest bulky dressing, the rule's top and the box.
    Where no real sight top is published, the art's is used and labelled.
  - Rows that moved from the lanes' values:
    - Puma: 3.1 to 3.30 m (PERI 3.6 m).
    - Ajax: 2.6 to 2.70 m (ORION 3.0 m).
    - CV90120: 2.4 to 2.50 m (panoramic sight 2.8 m).
    - M2A4 and M3A3: 2.565 to 2.76 m (the commander's independent viewer,
      3.06 m in the art).
    - M109A7: 2.8 to 2.96 m (cupola and MG mount 3.26 m).
    - T-14: 2.75 to 2.80 m (panoramic sight 3.1 m).
  - Rows the rule added:
    - The Challenger 3 gets its own box at 2.70 m (half 1.35). Calibre
      Defence gives "over 3 m to the sight", and today it inherits the
      Challenger 2's 2.49 m.
    - The PLZ-05 goes from 3.1 to 3.25 m. Its roof MG mount (3.55 m
      overall) is bulky, as the M109A7's is. The roof itself is unmeasured,
      so confidence is low.
  - Kept by the rule:
    - CV9040C at 2.50 m: the rule reads 2.41-2.50 m over the C kit's
      unmeasured roof armour.
    - CV90 Mk IV at 2.8 m (rule 2.8-2.9 m, C7).
    - PzH 2000 at 3.06 m: the rule's 3.16 m is on the threshold, so the art
      must hold its tallest roof fitting to 3.36 m or draw it thin.
    - BMP-3, BRM-3K, Buk, T-15 and every F-A tank but the T-14 and
      ZTZ-99A.
  - **The consequence for the art.** `fit.hull_extents` holds the
    non-dressing art within 0.1 m of the box top (`validate.ts:1031-1037`).
    Where the box stands more than 0.1 m over the real roof, the art keeps a
    non-dressing part (the sight's mast or the cupola ring) up to the box top
    and puts only the head under `dressing_*`. This applies to 11
    appearances, listed in `fix-rows.json` under `derived.art_fit`. Slice 15
    makes those minimal fit edits.
  - **BMP-3 hull "about 0.2 m low" (F-C) is art.** The box top is the turret
    roof (drawing 2.37 m against the box's 2.4 m), and no frame number holds
    the deck height. Slice 28 raises the hull roof and the turret with it.
- **C5 `muzzle_m` is measured from the mount's pivot.**
  - The simulation fires from pivot + muzzle, confirmed in
    `crates/sim/src/weapons/muzzle.rs:12-22`: `placed_muzzle` adds the
    carrier's ring, the pivot round it, then `muzzle` turned by the mount's
    own bearing, which is pivot + muzzle at rest.
  - Every gun row in all three lanes was audited: fixed, kept, and those
    whose pivot moves. F-B's Abrams, Leopard and M10 fixes stand. The KF51
    stays as F-B left it: a rounded "about 10 m" is no clear error.
  - Corrections made in the audit:
    - Challenger 2: 7.1 to 7.2 m. 11.55 - 4.15 - 0.2, the same derivation
      as the Challenger 3's 6.95 m.
    - 2S19M2: kept 7.15 to fixed 6.99 m. 11.917 m overall less the trunnion
      at 1.35 m. Its world point was 0.16 m long.
    - CV9040C: kept 3.9 to fixed 3.6 m. On the Strf 9040 plan the muzzle is
      3.61 m from the ring. F-C had compared world points on two different
      hulls.
    - CV90 Mk IV: 3.9 to 3.7 m. The generated view and the photo both put
      the muzzle at the nose line.
    - BMP-3 100 mm: 3.8 to 3.35 m. On the FMSO drawing the muzzle stands
      0.07 m past the nose, and the frame's world point was 0.46 m past
      that.
    - M2A4 and M3A3 25 mm: 3.6 to 2.5 m. The square-on M3A3 photo puts the
      muzzle 1.98 m ahead of hull centre, behind the glacis break. The
      frame's world point overshot by 1.1 m. The barrel's height (about
      2.05 m against 2.47 m) is named for slice 26 and not fixed: it rests
      on one photo, taken in dust.
  - Kept on audit:
    - T-90M at 6.2 m: 0.10 m past the published 9.53 m, so FA13 is settled
      by keeping it.
    - ZTZ-99A at 7.2 m: it matches the unsubtracted pattern by coincidence,
      but the drawing measures 7.23 m from the pivot.
    - CV90120 at 5.8 m: within 0.1 m of 8.9-8.95 m gun forward.
  - Where a pivot moves, the muzzle was recomputed so the real muzzle point
    stays put: the 2A7V (box lengthened at the front only), the BMP-2M,
    the T-14 (gun axis re-measured), the four howitzers, the M270 and MARS
    II, and the Buk.
  - The four howitzer pivots (M109A7, PzH 2000, 2S19M2, PLZ-05) are
    placeholders today. Each move is a named shell-origin change.
- **C6 The Buk-M3 is tracked.** Its fixture moves to `roster_profile_ifv`,
  the profile the Gepard, M2A4, CV9040C and BMP-3 extend. The two profiles
  hold the same sensors and sound, so the change that matters is
  `mobility`: it goes from wheeled to the Gepard's tracked block. The road
  speed is 65 km/h, which the Buk-M3 shares, and there is no turning
  radius.
  - Hull length: 9.3 m (half 4.65 m), from Army Recognition's Buk-M3
    (9K317M) data sheet, the same figure as the M1/M2 TELAR's. The 8.0 m
    figure is one the same site repeats across the system's vehicles. 9.3 m
    is inside the tracked cap (half-length 4.8 m).
  - The launcher pivot stays F-B's photo estimate (+2.0 ± 0.5 m). Slice 15
    runs before slice 24 rebuilds `buk.py`, so the art can't supply it.
    The rebuilt art draws its ring at the frame's pivot.
  - Fixtures are untouched. The extends, mobility and hull rows are named
    in `fix-rows.json`.
- **C7 Generated orthographic views (Q4 rung 3).** Both views were made
  through the Bento model gateway (`openai/gpt-image-2.6-flare`), with a
  real photo as input. They are recorded as `generated` in their
  libraries.
  - The gateway ignores seeds, so `seed: 0` records none.
  - The first try at each failed: the Mk IV came back as a front quarter,
    and the PLZ-05 prompt was refused as violent. The second prompts
    asked for a "museum catalogue profile".
  - **CV90 Mk IV: usable, row kept.**
    - Scaled to 6.8 m, its road-wheel pitch matches the Strf 9040 plan's.
    - Against the side photo, its hull stands about 7 % taller, measured in
      road-wheel diameters.
    - Corrected, the roof is about 2.6 m and the sight head 3.1-3.25 m, so
      C4 puts the top at 2.8-2.9 m. Today's 2.8 m stands (FC5 resolved).
    - The view also fixes the Mk IV's muzzle (C5).
  - **PLZ-05: the view fails its check, row kept.**
    - Its gun reaches about 1.1 m past the nose, where the museum photos
      show several metres. No single published number scales the hull,
      turret and gun together.
    - The hull length stays unresolved (FC11). The height moves under C4
      on the published overall figure.
  - The PLZ-05 library's parade profile (`-side-2.jpg`) was a PLZ-07:
    Commons files it under PLZ-07 and its description says 122 mm. It is
    removed and named in the gaps. It was the photo the generated view
    would have been checked against.
- **C8 No non-free line art in the repository.** Every committed PNG in
  `assets/frames/` was checked against its drawing's licence, on the file
  page and at the source:
  - `bmp.bmp_2m-side.png`: the U.S. Army's 1981 BMP-2 graphic
    (PD-USGov-Military-Army). Free.
  - `bmp.bmp_3-side.png` and `brm.brm_3k-side.png`: the U.S. Army FMSO's
    BMP-3 line drawing, from Red Thrust Star, January 1996
    (PD-USGov-Military-Army). Free.
  - `cv90.*.png`: Mick Bell's Strf 9040 plan, CC BY 4.0. Commons has no
    licence review, but the rights holder's site says "The drawings are
    all released under the Creative Commons 4.0 Attribution licence".
    Free, with attribution.
  - `sp_howitzers.m109a7-side.png`: A proietti's M109 A6 drawing, CC BY-SA
    4.0, his own work after a U.S. Army equipment sheet. Free. The overlay
    is a derivative, so it is shared under the same licence.
  - The photos F-C measured are free too:
    - BMP-2: CC0.
    - 2S19: CC BY-SA 3.0 and CC BY 4.0.
    - PzH 2000: CC BY 2.0, confirmed by the Flickr review bot.
  - Nothing was removed. Each overlay's attribution and licence now sit
    beside it in its family file.
  - `visualizations/frames.html` is self-contained. Per unit it shows:
    - the model's outline in three views, rendered from the committed GLB
      through the asset CLI and split into hull, mount and dressing;
    - both boxes, the pivots and the muzzle points;
    - the C4 heights;
    - the station numbers;
    - the verdict rows and the user's notes.

    It carries no drawing line art.
- **C9 One schema.** Every `assets/frames/<family>.json` now has F-B's
  shape (F-B11):
  - at family level: `sources`, `overlays`, `notes` and `answers`;
  - per appearance: `published`, `current` (from the blessed catalog),
    `measured`, `heights` and `rows`.

  Each row is `{field, old, new, verdict, source, confidence, note}`, with
  verdicts keep, fix, unresolved, art-only and inherits.
  `assets/frames/fix-rows.json` is the flat list slice 15 applies: 64 rows
  in parent-first order. Each names its fixture, JSON path, old and new
  values, source, reason and digest consequence: hull box 20, shell origin
  41, mobility 2, eye 1. Rows go only where a fixture owns the field:
  - The 2A8 and the M1E3's cannon take their parents' fixes through
    `extends`. F-B's note that the M1E3 restates its cannon row was wrong.
  - The Abrams Trophy variants restate theirs, so each gets its own rows.
  - The Challenger 3 gains a box and a cannon muzzle of its own.

  `derived` lists the follow-on steps: wreck footprints, track gauge
  (CV9040C, 2S19M2), the Ajax mount draw and the art-fit list. Every hull
  row is inside the tracked caps (`crates/contract/src/limits.rs:21-24`).
  The generator checks this when it writes the file.
- **C10 One threshold set (agent's call, confidence medium).**
  - Hull dimensions: max(0.10 m, 3 %).
  - Pivots and muzzle points: 0.15 m on any axis (F-B3).
  - A value written in another convention is fixed whatever its size.

  F-C's 0.30 m muzzle reach and F-A's 3 % muzzle rule are retired. Under
  the shared rule the 2S19M2 and CV9040C muzzles flip to fix. Every other
  row keeps its lane's verdict.
  - Why: one list needs one rule. 0.15 m is the slice's own pivot
    threshold, and a muzzle is a point, as a pivot is.
  - To reverse it, put the 0.30 m reach back. The two rows then return to
    keep.

### Slice 07b decision audit (2026-11-11)

These are the calls the 07b agent made beyond C4-C9 as briefed. Each
names its verdict and how sure the audit is that the user would have made
the same call. Least confident first within each group.

**Review these first:** A1 (PLZ-05 height), A2 (Challenger 3 box), A3
(Bradley muzzle).

Needs the user:

- **A1 The PLZ-05 box rises 0.15 m on one published figure (low).**
  - The scenario: the only height source is Wikipedia's 3.55 m "overall,
    with the QJC-88 machine gun". No drawing or square-on photo exists, and
    the generated view failed its photo check.
  - C4 counts a roof MG mount as bulky dressing, as it does on the M109A7.
    So the box top becomes 3.55 - 0.3 = 3.25 m, against today's 3.1 m.
  - The alternative: treat the MG as thin, or as too poorly sourced, and
    keep 3.1 m unresolved.
  - The reach: a hull-box digest change, and slice 29 must put the MG
    mount under `dressing_*`. The art's non-dressing top is already
    3.17 m, so the hull fit passes either way.
  - Provisional call: apply it. To reverse it, drop the PLZ-05
    `half_extents_m` row from `fix-rows.json` and mark the row unresolved
    in `sp_howitzers.json`.
- **A2 The Challenger 3 gets its own 2.70 m box (medium-low).**
  - The scenario: the Challenger 3 inherits the Challenger 2's 2.49 m box.
    Calibre Defence says it stands "over 3 m to the sight". Under C4 a
    3.0 m sight puts the box top at 2.70 m, so the agent gave the Challenger
    3 its own `half_extents_m`.
  - The alternative: keep the inherited box and treat "over 3 m" as too
    vague.
  - The reach: one more hull-box digest change. Slice 20 must draw the
    commander's sight at about 3.0 m (the art has 2.76 m today), with a
    non-dressing mast reaching 2.6 m.
  - Provisional call: apply it. To reverse it, drop that row.

Sound:

- **A3 Two muzzle fixes no lane asked for: the Bradley 25 mm and the BMP-3
  100 mm (medium).**
  - The scenario: auditing every gun row (C5), the agent measured the
    square-on M3A3 photo. It puts the Bradley's muzzle 1.98 m ahead of hull
    centre, where the frame has 3.1 m. The M2A4 quarter photo agrees that
    the muzzle sits behind the glacis.
  - On the public-domain BMP-3 drawing, the 100 mm muzzle stands 0.07 m
    past the nose, where the frame has 0.53 m.
  - F-C had kept both rows ("no measurement contradicts them").
  - The reach: two shell-origin digest changes, plus shorter barrels in
    slices 26 and 28.
  - The Bradley barrel also reads 0.4 m lower than the frame in the photo.
    That is left unfixed and named, because it rests on one photo taken in
    dust.
- **A4 The art's sight top stands in where no real one is published
  (medium).**
  - The scenario: C4 needs the tallest bulky dressing's top. For the
    M2A4/M3A3's commander's independent viewer, and for several tanks, no
    figure exists. The agent used the committed GLB's height (the M2A4's,
    3.06 m) and labelled it "art".
  - The alternative: the gunner's sight (2.97 m, Jane's), which gives a
    2.67 m box instead of 2.76 m.
  - The reach: a family slice that redraws the viewer lower should revisit
    the box.
- **A5 The CV90 Mk IV muzzle fix rests partly on a generated view
  (medium-low).**
  - The scenario: the generated side view puts the muzzle at the nose line,
    and so does the real quarter photo. The library rule says no view
    rests on a generated image alone. The photo is the second witness.
  - The fix is 3.9 to 3.7 m.
- **A6 The Buk's tracked numbers are the Gepard's (medium).**
  - The scenario: no off-road speed or turn rate is published for the
    9A317M. The agent copied the Gepard's tracked block (28 km/h off-road,
    45°/s), whose 65 km/h road speed matches the Buk-M3's published one.
  - The reach: the Buk's mobility digest. These are balance numbers, so
    the balance sample at slice 31 is where they get judged.
- **A7 Generated references record `seed: 0` (medium).**
  - The scenario: the reference contract requires an integer seed for a
    generated view. The Bento gateway ignores seeds. The agent wrote 0 and
    said in the note that 0 means none.
  - The alternative: make `seed` optional in `references.ts`. That is a
    contract change outside this slice.
- **A8 The two artefacts are generated by scripts that aren't committed
  (medium).**
  - The scenario: `fix-rows.json` and `frames.html` were built from the
    family files and the GLB probe by scripts in the ignored `throwaway/`.
  - The reach: if a family file is edited before slice 15, its rows in
    `fix-rows.json` must be edited by hand to match.
  - Both artefacts are one-shot hand-offs, which is why the scripts weren't
    promoted into the repo.
- **A9 Rows go only to the fixture that owns the field (high).**
  - The scenario: the 2A8 extends the 2A7V, and the M1E3's cannon extends
    the SEP v3 Trophy's. So they take their parents' fixes, with no row of
    their own.
  - The Abrams Trophy variants restate their mounts, so each gets rows.
  - A JSON path is `units.<id>...mounts[id=X]...`. Where a unit inherits a
    value today, its row carries `inherited_old`.
- **A10 The PLZ-07 photo is removed from the PLZ-05 library (high).**
  Commons files the parade profile under PLZ-07, a 122 mm howitzer. It
  could only mislead.
- **A11 The CC BY-SA M109 overlay stays (medium).**
  - The scenario: it is a derivative of a free drawing, so it may be
    committed under the same licence. Its family file now records the
    attribution and licence.
  - The alternative: remove it to keep share-alike material out of the
    spec. The library already holds CC BY-SA photos, so nothing new is
    introduced.
- **A12 Slice 15 now points at `fix-rows.json` (high).** Its stale "known
  rows" (a Leopard drop to "about 2.6 m", the HMG and eye dropping with the
  roof) were replaced by the rows that need more than a value change.
- Trivial discretion (2 calls):
  - the page's colours and layout;
  - each family file keeps the lane's own notes after the coordinator's,
    prefixed "Lane:".

- **C11 How strict the lint is on family slices** (2026-11-11). Slice 01 sorted
  the 1,204 tracked issues by kind. Seven in ten are seating that never shows:
  running gear into the hull sides (698), hull fittings under the turret
  overhang (113), the turret into the roof (46). A strict zero rule would make
  every family rework hidden geometry beyond its notes.
  - A family slice must clear the visible kinds: turret fittings into hull
    fittings, skirts or bins into the belt or wheels, wheels overlapping
    wheels, and unbraced or floating parts.
  - Hidden seating may stay in the baseline.
  - The family template carries this.
- **C12 `air_defence.py` waits for slice 15** (2026-11-11). Its re-export
  failed the fit check on tow hooks 12.7 cm past frames that grew on
  2026-10-15. Slice 15 changes the Buk's and Gepard's frames anyway (C4, C6),
  so it seats the hooks as its minimal fit edit and checks the Buk's unit
  class follows its new tracked profile.
- **C13 The lint's track-by-name test moves to slice 10** (2026-11-11). Slice 10
  owns the track node contract, so the ground-contact check's second
  definition of a track goes there.
- **C14 Slice 04's winning cutter is committed** (2026-11-11). The groove cutter
  that was scored (`assets/experiments/04/scripts/seam_build.py`, with its
  render and probe scripts) moved from the lane's ignored folder into the
  spec, so slice 13 builds from the scored code.
## Slice 05 decisions (2026-11-11, lane J, audited)

Each entry gives the decision and its scenario, the gap the plan left, its
reach, a verdict and a confidence (how sure the audit is that the user would
have made the same call). Least confident first. **Review these first:**
J05-1 (the shipped finish is still below the critique bar), J05-2 (how "is a
jet" is read) and J05-3 (where the roughness lives).

- **J05-1 Ship `JET_ROUGH = 0.38`, below the critique bar, and name the gap.**
  - *Scenario.* A player zooms the battle camera onto an F-15EX flying past
    the sun.
    - With the old satin paint (roughness 0.55), its top surfaces are an even,
      chalky light grey.
    - At 0.38, the wings and spine catch a cool sky sheen and a sharper
      highlight on the curves.
    - The side views barely change. The shaded sides stay the dark,
      teal-tinted slate they already were. Into the sun there is still no
      broad glow like the reference's.
  - *The three critique rounds* (unprimed; sheen, grey at the silhouette,
    artifacts, overall):
    - 0.35 scored 5/4/4/4. It read as chrome from above, and the Su-35S
      skin's lumps showed as crumpled foil.
    - 0.42 scored 4/4/5/4: "no different" from satin.
    - 0.38 scored 5/4/5/5: "marginally better than the previous finish on
      sheen".

    Round 1 had no satin captures to compare against. Rounds 2 and 3 had them
    and quoted the owner's words ("glossy, metallic-looking sheen").
  - *Measurements.* The luma ladder (`assets/experiments/05/compare-telemetry.txt`)
    shows the trade:
    - below about 0.3, the highlight narrows and the grey *darkens* (the
      sun-facing wing's mean goes from 160 at satin to 143 at 0.15);
    - from 0.35 to 0.42, the wing keeps or gains brightness (161–164) and the
      spine highlight sharpens (p99 215 → 236–246).

    0.38 has the best critique scores, near the brightest broad sheen.
  - *What paint can't fix.* Every round named the same three gaps, and none
    of them is a roughness:
    - **The shaded sides go dark teal.** That was already there at satin and
      is unchanged within one luma level. It is the scene's sky and ground
      light, not the paint.
    - **There is no sun glow into the sun.** The low-orbit crops measure the
      same at 0.35, 0.38 and 0.42.
    - **The Su-35S's foil streaks.** These are the generated skin's ripples,
      which gloss reveals. Slice 06 found code reads cleaner.

    The rest of the gap to the reference is its brighter scene and grading:
    the reference's 95th-percentile luma is 201, ours 177.
  - *Gap.* The slice delegated the value, and the three-round cap ended the
    tuning.
  - *Reach.* Every fixed-wing jet. The value is one constant to retune when
    the lighting or grading pass, or "jets off the generator", lands.
  - *Verdict.* **Needs-user, provisionally shipped.** The user asked for a
    visible sheen, and 0.38 is the strongest that didn't read as chrome or
    foil to an unprimed eye. *Provisional call:* keep 0.38. *To reverse:*
    change `textures_air.JET_ROUGH` (0.42 is indistinguishable from satin;
    0.30 is glossier with more foil) and re-export the 18 fixed-wing scripts.
  - *Confidence:* low-medium.
- **J05-2 "Is a jet" is the unit's catalog flight model.**
  - *Scenario.* An Apache and an F-16 are both built through `A.jet`, and
    their schemes can be shared (S5). At export, `aircraft_jet.fixed_wing`
    looks up the unit type the appearance belongs to in
    `fixtures/catalog.json` and asks whether `mobility.air.flight` is
    `"fixed_wing"`. That's the same field the simulation flies it by. The
    F-16 gets the jet finish and the Apache keeps the satin.
  - A disabled card has no unit type, so it reads as not fixed-wing. The
    MQ-9, a turboprop drone, keeps the satin, and its GLB is unchanged.
  - *Gap.* The variant that `catalog_frames` hands an exporter carries only
    its frame. `fixed_wing` reaches the catalog through the private
    `catalog_frames._catalogs`.
  - *Shared-part request.* The cleaner owner is `catalog_frames._variant`,
    which is coordinator-owned: it should carry `flight`. Then
    `fixed_wing(v)` becomes `v.variant.get("flight") == "fixed_wing"` and the
    private import goes.
  - *Rejected keys.* "Has a skin" breaks when the jets leave the generator.
    "Has nozzles" fails because the A-10 and F-22 specs build their own. A
    flag in each fixed-wing script would repeat a label 18 times.
  - *Reach.* Any future fixed-wing unit gets the jet finish with no script
    change, an enabled MQ-9 included.
  - *Verdict.* Sound key. The private import is a bridge, removed with the
    request above.
  - *Confidence:* medium.
- **J05-3 The roughness rides a jet twin of each recipe, not the material
  factor.**
  - *Scenario.* The slice planned to set the paint material's roughness
    factor. On export, `textures.attach` writes the recipe's images and
    resets `roughnessFactor` to 1 on every worn material, so that factor
    would have been silently dropped. Reading the code caught it before any
    bake.
  - *Decision.* `textures_air.jet_finish(name)` registers `<name>_jet`, the
    same colours, relief and wear, with its ORM roughness scaled from
    `AIR_ROUGH` (0.55) to `JET_ROUGH`. `aircraft_jet.jet` points the
    airframe paint's material at the twin, by rewriting its entry in
    `parts.TEXTURED`, the material-to-recipe table `attach` reads.
    `aircraft_parts.py` doesn't grow (37,191 B).
  - *Scope.* Only the airframe paint is glossy. Stores, fittings, gear and
    the chipped edges (wear roughness 0.8) keep their finish.
  - *Cost.* Each jet scheme adds one ORM texture layer, nine in all
    (`us_gunship_grey`, `us_compass_grey`, `russian_air_blue`,
    `russian_air_grey`, `russian_helicopter_camo`, `chinese_air_grey`,
    `chinese_air_light_grey`, `nato_air_grey`, `french_air_camo`). The floor
    is 2,048. The twins' albedo and normal images are byte-identical, so they
    share layers. This is unverified by a full bake (J05-5).
  - *Alternative.* Teach `attach` to keep a non-default factor. That changes
    `textures.py` (coordinator-owned) for every worn material.
  - *Verdict.* Sound.
  - *Confidence:* medium-high.
- **J05-4 The bake check is a bundle read, not a full bake.**
  - *Scenario.* A full `asset bake` reads every source GLB, about 4 GB of
    LFS.
  - Instead, `asset validate` ran on the F-16C. It builds the bundle with
    the bake's own material code (`material.ts`, `drawnSurface`), with a
    local, uncommitted line that prints what the paint role draws. At the
    0.30 trial, `armor_paint` drew roughness 0.302 (0.553 before) at the
    same luminance, 0.178. The Apache drew 0.553.
  - Every final GLB's paint ORM image averages 0.379–0.380.
  - For the battle shots, a throwaway `asset bake-only` ran
    `bakeCatalog(…, only)` on three jets and spliced them into a local
    runtime. The baked bundle hashes changed, and the shots showed the
    change.
  - *Verdict.* Sound. The coordinator's milestone bake is the full proof, and
    it also counts the texture layers.
  - *Confidence:* high.
- **J05-5 Exports run with `--no-lint` until slice 01 lands.**
  - *Scenario.* This branch starts before slice 01's baseline. Slice 00's
    truthful lint flags existing overlaps, such as the F-16C's bay doors
    against its fuselage and the Apache's, as "new". So every export fails.
  - The lint only reads the scene, so the GLB bytes don't change. Its modules
    drop out of the receipts' `source_sha256`.
  - The coordinator's slice 16 re-export, with lint, rewrites those
    receipts.
  - *Verdict.* Sound.
  - *Confidence:* high.
- **J05-6 Only the live jets and their receipts are committed.**
  - The diff to each committed GLB is one buffer view, the paint's ORM image;
    the geometry and everything else are byte-identical. That covers 25
    appearances from 18 scripts.
  - Wrecks burn every material (`wreckage.burn`). All 50 jet wreck and debris
    GLBs re-export byte-identical, so they aren't touched.
  - The Apache's live, wreck and debris GLBs re-export byte-identical
    against the final code. Its receipt changed only in script hashes, so it
    isn't committed. Other helicopter receipts now lag the script hashes in
    the same way, and slice 16 absorbs that.
  - *Verdict.* Sound.
  - *Confidence:* high.
- **J05-7 The Su-35S was judged in `russian_air_blue`.** The slice's crop
  named `russian_air_grey`, but `flanker.py` paints the family
  `russian_air_blue`. *Verdict:* sound. *Confidence:* high.
- **J05-8 The battle-camera harness is throwaway.**
  - It is a local `jet-sheen` scene over `/labs/air-high`, at the high
    graphics profile. Its encounter is swapped for an F-15EX, a Su-35S, an
    F-16C, an Apache and three T-72s for sight of the fields.
  - It ran with a local `asset bake-only` command and a luma-crop script.
  - None of it is committed, because `web/**`, `fixtures/**` and `apps/**`
    are coordinator-owned. The patches are in the lane worktree's
    `throwaway/` (`harness.patch`, `bake-only.patch`, `jet-sheen.mjs`).
  - *Verdict.* Sound.
  - *Confidence:* high.

## Slice 06 decisions (2026-11-11, lane J)

- **J06-1 The verdict is that code wins on the surface for both airframes,
  so the follow-up is queued** (for the coordinator to add to the README).
  - *Scenario.* Each airframe was shown in closeups beside its shipped
    generated skin, labelled only A and B, swapped between the aircraft.
  - *Result.* One unprimed critique round:
    - the code's surface scores 6 on both, and the skins' score 4 (F-35A) and
      3 (Su-35S);
    - the means are 5.6 for code and 4.6 for skin, on both aircraft.
  - *What the critique said.* The skins have crinkled wing-root blends and
    rippled spines and leading-edge extensions ("crumpled fabric"). The
    code's fault is faceting along its bodies.
  - The sheet and scores are in `assets/experiments/06/README.md`.
  - *Gap.* None: Q1b set the rule.
  - *Reach.* "Jets off the generator" becomes a real follow-up spec. It would
    also retire the guide-to-skin pipeline: `skin_views`, `skin_register`,
    `tools/skin-generate.mjs`, the pins, and `aircraft_skin` with its
    `Probe` seating. That is the one-owner gain.
  - *Verdict.* Sound on the slice's terms. The bar was relative, and
    neither side reaches the make-3d-models bar yet.
  - *Confidence:* medium. That's one round and two airframes, in Blender
    closeups rather than the battle camera.
- **J06-2 The technique is a voxel union of the lofted bodies; the flying
  surfaces stay lofted.**
  - *How it works.* The guide's near-tier painted bodies are joined into one
    mesh: fuselage, belly, intakes, nacelles, booms and the gun bulge. The
    mesh is remeshed on a 25 mm voxel grid, which fuses overlapping bodies
    into one closed shell with filleted junctions, like the skin's blends.
    The shell is smoothed (factor 0.6, 12 passes) and decimated to 20,000
    triangles.
  - *What stayed out.* Gear doors, pylons, intake lips and canopy sills are
    left out of the union; the first try included them and they fused into
    blobs. Wings, strakes, stabilators and fins stay as their lofted
    aerofoils, since a voxel grid eats thin trailing edges.
  - *Alternatives.* The slice named two: subdivision, or a denser loft.
    Neither blends one body into another; they only smooth each body
    alone. The skin's whole advantage is the blend, so they would have
    tested the wrong thing.
  - *Gap.* The technique was delegated.
  - *Reach.* The follow-up spec inherits this as its starting point, with
    its known cost: faceting along the bodies, which a finer voxel or more
    smoothing addresses. `assets/experiments/06/smoothing.py` keeps the
    script.
  - *Verdict.* Sound.
  - *Confidence:* medium-high.
- **J06-3 The candidate gets the skin's triangle budget, not the loft's.**
  - *Scenario.* The first run decimated the merged bodies back to the
    loft's own count of about 5,000 triangles, far coarser than the skin.
    The run that was judged gives the bodies 20,000 triangles. That's the
    skin's near-tier 26,000 (`aircraft_skin.TRIS`), less what the lofted
    surfaces keep.
  - The near tiers that were judged:
    - F-35A: 38,204 for code against 50,165 for the skin;
    - Su-35S: 66,332 against 80,349.

    The code side has fewer triangles, so the result doesn't come from
    spending more.
  - *Verdict.* Sound.
  - *Confidence:* high.
- **J06-4 Slice 06 was judged under the 0.35 paint, not the shipped 0.38.**
  The shipped skins and the code candidates were exported while slice 05's
  tuning stood at 0.35. That run is a little glossier, so it shows the skins'
  ripples a little more than 0.38 does. Both sides wore the same paint, so
  the A/B comparison holds. *Verdict:* sound, and not worth a rerun.
  *Confidence:* medium-high.

- **C15 Variants carry their flight model** (2026-11-11). This was lane J's
  shared-part request. `catalog_frames._variant` now sets `flight` from the
  unit's `mobility.air.flight`, and `aircraft_jet.fixed_wing` reads it instead
  of importing the private `_catalogs`. Proof: the F-16C re-export is
  byte-identical (`62bf05b1…`).

## Slice 02 decisions (2026-11-11, coordinator's implementation agent)

Each entry gives the decision, the gap the plan left, its reach, a verdict
and a confidence, least confident first.

Review these first: every receipt now reads as stale, and the kernel's
`_arc` was made public too.

- **Every receipt now names a stale `vehicle_parts.py` and none of the new
  modules.**
  - *Scenario.* A receipt hashes each script the export loaded. After the
    split, `vehicle_parts.py` has new bytes, and the seven new modules are
    in no receipt. Every family that imports vehicle parts reads as stale
    until it re-exports, even though its GLBs are byte-identical.
  - *Gap.* The slice forbids re-exporting (its behaviour is identical), and
    says nothing of receipts.
  - *Reach.* Slice 30's receipt freshness probe will see every family as
    stale for this reason alone. Family lanes refresh their own receipts
    when they re-export. No check reads receipts today.
  - *Verdict.* Sound: re-exporting 79 scripts to refresh hashes would
    commit nothing but receipts.
  - *Confidence:* medium.
- **`_arc` became public as `arc`, beside `_place` becoming `place`.**
  - *Scenario.* `tow_hook`, `shackle` and `exhaust` (`hull_fittings`) call
    the kernel's arc of points. Importing an underscore name across modules
    says "private" while every part module depends on it.
  - *Gap.* The slice named only `_place`.
  - *Reach.* The kernel exports `place`, `wheel_node`, `arc` and
    `tube_part`; `_tube` stays private, used only by `tube_part`.
  - *Verdict.* Sound.
  - *Confidence:* high.
- **The track node contract moved to `running_gear`'s docstring.** The
  kernel keeps the general spinning-node contract (`wheel_*`, made by
  `wheel_node`, which the aircraft gear also uses) and points to
  `running_gear` for `track_L`/`track_R`, `track_length_m` and
  `link_pitch_m`. The slice gave both the kernel docstring and the track
  contract a home; this is how they split. *Confidence:* high.
- **Callers import each owner under the slice's two-letter aliases.**
  `import running_gear as RG` and the rest, alongside `VP` for the tier sets,
  `tube_part` and `wheel_node`. Every alias is as long as `VP`, so no
  wrapped argument line needed re-aligning, and the diff is the call sites
  alone. Imports of named functions (`wreckage.py`, `wreckage_test.py`,
  `aircraft_parts.py`) import from the owner directly. *Confidence:* high.
- **The lint proof also dumps every issue.** `vehicle_lint.py -- --roster`
  prints per-check counts per appearance and only the new issues in full, so
  an identical report could still hide a swapped issue. A throwaway wrapper
  ran the same walk (`vehicle_lint_roster.walk`) and wrote every issue of
  every appearance as JSON; before and after are byte-identical.
  *Confidence:* high.
- **The machine-gun proof is the UH-60M and Z-20, not the Apache.**
  - *Scenario.* The slice named the Apache for `aircraft_parts`' guns, but
    it carries only a chin gun (`aircraft_parts.chin_gun`), which never
    calls `vehicle_guns`. The door guns do (`door_gun`). `hawk.py`'s UH-60M
    (an M2) and Z-20 (a Kord) were exported after the split, and both are
    byte-identical to their committed GLBs. The Apache stays in the set.
  - *Gap.* The slice's choice of aircraft missed the path it meant to prove.
  - *Reach.* None beyond this proof.
  - *Verdict.* Sound.
  - *Confidence:* high.
- **Territory: the jets' lint needs their skins pulled.** Without
  `assets/source/skins/` in LFS, `--roster` fails 25 jet appearances ("is an
  LFS pointer"); a live export with crew needs its faction's rifle-squad
  source. The proof runs pulled both.
- **Trivial discretion (6).**
  - Each module keeps its functions in the original file's order, and the
    old section-header comments are dropped, since each module is one
    section.
  - New modules repeat their siblings' `sys.path.insert` and `# noqa: E402`
    imports. `hull_shape` imports nothing local, so it has neither.
  - The README's table gives each module's kind of part, not its roster.
    The module docstrings name what each holds, and the kernel's docstring
    points to the README rather than listing the modules.
  - The proof exports ran with `--no-lint`. The lint never changes a GLB's
    bytes (slice 01), and the roster walk proved the lint separately.
  - The Boxer APC's wreck joined the export set, because it is the only one
    that reaches `tyre_wheel` through `wreckage`'s thrown wheels.
  - `aircraft_parts.py` is lane J's file, and its one changed line is the
    `vehicle_guns` import the slice named.
- **C16 The shared slices 08–12 run as four parallel lanes** (2026-11-11). The
  lanes are 08; 09 with 11 (both edit `armour_parts.py`); 10; and 12. Each
  branches from the slice 02 merge (`e598a571c`).
  - They commit source code and evidence only, never GLBs or receipts. Slice 16
    re-exports every model once, so binary LFS files can't conflict between
    lanes. This is the short-lived seam the README warns about.
  - For this wave only, lane 08 alone edits the lint allowlist and baseline,
    for the exact wheeled entries its narrowing exposes.
  - Slice 13 follows lane 09+11, because it adds `panel_seam` to
    `armour_parts.py`.

## Slice 10 decisions (2026-11-11, coordinator's implementation agent, lane 10)

Each entry gives the decision, the gap the plan left, its reach, a verdict and
a confidence, least confident first.

Review these first: the roller peaks were kept against the critique, and a
belt hangs 1 cm over low rollers rather than resting on them.

- **The peak over each return roller stays.**
  - *Scenario.* The unprimed critique's top fix was to round the hard point
    where the belt crosses each roller. The belt wraps the roller's rim
    there: two sagging spans meet across a 10 cm roller (8 cm on the
    ZTQ-15) at about 14 degrees. A real track bends at a link pin over a
    small roller. A rounder top would lift the belt off the roller, and
    smoothing the corner would pull it into the roller.
  - *Gap.* The slice delegated the curve, not this corner.
  - *Reach.* Every tracked vehicle whose top run shows. On the three models
    here, it shows only in the gear-only crop.
  - *Verdict.* Sound, if the user pictures a steel track. If the user
    reads the peak as a fault, the cheap change is a larger effective radius
    for the wrap, about one link pitch.
  - *Confidence:* medium.
- **A belt hangs 1 cm over rollers that sit below its straight line, rather
  than resting on them.**
  - *Scenario.* A family sets its return rollers low, below the straight
    line from the idler's top to the sprocket's. The belt's outline is the
    hull round all the wheels, so those rollers fall inside it and carry
    nothing. The span from idler to sprocket hangs over them, and the clamp
    stops its sag 1 cm (`SAG_CLEARANCE`) above the highest of them. In the
    synthetic rig, a 5.8 m span that would sag 17 cm stops at 5 cm, so the
    rollers look like they almost carry the belt. The alternative is to let
    the sag land on the rollers, which then become supports, and hang the
    span in pieces between them. That would be the "rest on what it sags
    onto" rule.
  - *Gap.* The seam specified the clamp, so this behaviour is the plan's.
    The audit names it because what it looks like is new.
  - *Reach.* Any family whose rollers sit below that line. No roster survey
    was run; the three models here have their rollers above it.
  - *Verdict.* Sound, as the plan's rule. If a family slice finds a belt
    hovering visibly, switch to "a wheel the sag reaches becomes a support",
    the same split `_hang` already does for a wheel that touches.
  - *Confidence:* medium.
- **Points every 5 cm along a span (`SAG_STEP`), not 8 per span.**
  - *Scenario.* With 8 points, a 1.6 m span has a point every 18 cm. Where a
    steep sag leaves a support, the straight piece from the support to the
    first point cut up to 9 mm into the idler (the test caught it at sag
    0.2). At 5 cm, finer than the belt's finest rings (6 cm), the points
    pushed onto the rim trace the wrap.
  - *Gap.* Samples per span were delegated.
  - *Reach.* A tier-0 belt keeps its ring count, since rings are spaced by
    length. The path has about 220 points instead of about 135.
  - *Verdict.* Sound.
  - *Confidence:* high.
- **A wheel the straight span touches becomes a support, and the span splits
  there.**
  - *Scenario.* The convex hull drops collinear points. Rollers at equal
    heights (the Leopard's 0.38 m and −1.02 m pair, any rig with level
    rollers) left the middle roller inside one span, touching it, so the
    clamp kept that whole span straight. The test caught it.
  - *Gap.* The seam said "between two different wheels" and didn't name the
    case of a third wheel touching the span.
  - *Reach.* Any level row of rollers now sags span by span. "Touching"
    means within 0.1 mm (`SAG_TOUCH`), since the hull's points are rounded
    to 0.01 mm. A wheel 2 mm under the line is not a support: it holds the
    span straight through the clamp.
  - *Verdict.* Sound.
  - *Confidence:* high.
- **A point that would dip into any wheel is pushed out onto its rim
  (`_outside`).**
  - *Scenario.* A parabola leaves its support at an angle, so near the
    support it lies inside that wheel's grown circle. Rather than solving
    for the true departure point, each point is pushed radially out of any
    circle it enters. This is the wrap a sagging belt makes, give or take a
    slight crossing angle where it leaves the rim.
  - *Gap.* The seam's clamp covered only the wheels under a span, not the
    span's own supports.
  - *Reach.* At the default sag, two points per belt rest on a rim on
    these three models. At 0.06 it is 7–13.
  - *Verdict.* Sound.
  - *Confidence:* medium-high.
- **The sag is vertical, and the clearance is measured vertically.** The
  spans that sag are within 60 degrees of horizontal (normal z > 0.5), and
  gravity is vertical. The least gap to a wheel's circle is found exactly:
  the gap is convex in x, so it is least where the circle's slope matches
  the span's. *Confidence:* high.
- **Ground contact reads `vehicle_lint_core.TRACKS`, not a constant from
  `running_gear`.**
  - *Scenario.* The slice asked the check to use the running-gear node
    contract. `running_gear` imports `bmesh` and `parts`, so the lint's
    plain-Python core can't import it. The core's `TRACKS` is already the
    lint's one reading of the contract, used by the overlap grouping.
    `running_gear.track_loop` still spells the two names in its own guard.
  - *Gap.* "This module owns" didn't settle which module the lint may
    import.
  - *Reach.* The check now groups `track_L` and `track_R` and everything
    under them. The belt meshes are found through their node, and fenders
    named `track_*` drop out. The lint test's hovering belt now hangs under
    a `track_L` node, as every export builds it.
  - *Verdict.* Sound.
  - *Confidence:* medium-high.
- **The test runs in Blender.** `running_gear` can't be imported without
  `bpy`, and the test checks the built belt mesh, which is what the renderer
  gets, not just the path. `asset check` routes it by its `import bpy`.
  *Confidence:* high.
- **The belt's scroll is proved by its contract, not by eye.** The sheet's
  running-gear strip draws the belt too small to see its link marks move. So
  the check is the contract the renderer scrolls by: on every tier the belt's
  u runs from 0 to `track_length_m / link_pitch_m`, and the renderer's scroll
  code is untouched. The strip proves the sagged belt draws in the production
  renderer at every travel step, and the sprocket turns. *Confidence:*
  medium-high.
- **Territory: `asset sheet` in a fresh worktree** also needs
  `build:mechanics`, the runtime pulled (`asset pull`) and
  `assets/third-party/effects` from LFS. Without the first, the workbench
  route returns 500; without the effects, it aborts on "Invalid PNG
  signature". Without either, the sheet times out after 60 s with no reason
  given.
- **Trivial discretion (4).**
  - The closeup crop is two custom shots: the full length from 9 m, 0.75 m
    up, and the middle rollers from 4.2 m. Both are low side views of the
    left running gear alone (a throwaway script strips the GLB), plus the
    whole vehicle.
  - The evidence is the before/after pairs, the span crops, the whole
    vehicles and the sheet strip crop.
  - The test's rigs are synthetic, not a roster vehicle's numbers, so a
    retune can't break it.
  - The probe outside Blender stubs `bmesh`, `parts` and `vehicle_parts`;
    it is throwaway only.

## Slice 12 decisions (2026-11-11, lane for closed hatches and no crew)

Each entry gives the decision, the gap the plan left, its reach, a verdict
and a confidence, least confident first.

- **The Leopard's own commander's hatch became `RF.cupola`.**
  - *Scenario.* `leopard.commander_hatch` swung its lid open. Closed, it
    was `RF.cupola(radius=0.36)` in all but small details: periscope heads
    1 cm shorter, an arc step of 0.62 rad against the helper's 0.64, and the
    lid 0.34 m wide against the helper's 0.33 m.
  - *Gap.* The slice named only `cupola`'s own `lid_open`. The brief asked
    for every hatch left open by other means, and kept the lane to the crew
    and lid lines.
  - *Reach.* The parts were renamed from `commander_ring`, `_periscope_*` and
    `_lid` to `commander_cupola_*`. No wreck, lint baseline or allowlist
    names them. `leopard.py` is in no other lane's edit list this wave.
  - *Verdict.* Sound: closing it meant writing the helper's closed branch a
    second time, and the README's owner table gives cupolas to
    `roof_parts`.
  - *Confidence:* medium. Slice 22 may still want its own periscope arc.
- **The Abrams' own commander's hatch was closed in place, not replaced.**
  - *Scenario.* `abrams.commander_hatch` threw its lid rearward onto a
    stop. Its five periscopes span only the front arc (±1.0 rad), which
    `RF.cupola` can't draw. The lid now sits on the ring at its old size, and
    the stop is deleted.
  - *Gap.* As above. `abrams.py` is edited by other lanes this wave, so the
    diff stays on the lid lines.
  - *Reach.* Slice 21 owns any move to the shared helper.
  - *Verdict.* Sound.
  - *Confidence:* medium-high.
- **The test tank's open driver's lid is left and named.**
  - *Scenario.* `tank.py:559` builds `driver_lid_open`, swung up. It is test
    art and never game content, and it sits outside the tracked scope and
    this lane's files.
  - *Reach.* Slice 30's "`lid_open` anywhere" grep will hit the part name.
    The coordinator decides whether to close it or scope that grep to the
    tracked scripts.
  - *Confidence:* high that it is out of this lane.
- **The receipt half of the oracle is proved by construction, not by a
  receipt.**
  - *Scenario.* A `--variant` export writes no receipt, and the wave forbids
    committing receipts.
  - *Proof.* All 25 live appearances of the 16 changed scripts were
    exported, with the rifle-squad sources left as LFS pointers. A crewed
    export would load one of those, so these exports never reached
    `vehicle_crew`. `_receipt` hashes the modules in `sys.modules`, and
    `vehicle_crew` is imported only under `if v.crew`. Slice 16's receipts
    are the direct check.
  - *Confidence:* high.
- **The shared cupola's closed geometry was kept as it was.**
  - *Scenario.* The critique saw the Ajax's vision blocks as locking tabs
    with glass flecks: the closed lid (0.92 r) covers the inner half of each
    block. The cupolas that were already closed (T-90M, KF51, ZTZ-99A,
    howitzers) look the same.
  - *Gap.* The slice's one variable is open against closed.
  - *Reach.* It is left for the shared helper's owner or slice 27.
  - *Verdict.* Sound for this slice.
  - *Confidence:* medium.
- **The roof crop is a camera in metres.**
  - *Scenario.* `roof:4.2m,3.4m,7.4m:-0.4m,0m,2.2m:32` at 1000x700 is one
    view for all three models. Fractions of the bounds would follow each
    gun's reach instead of the turret.
  - *Gap.* `assets/shots.txt` has no roof shot.
  - *Reach.* Families may reuse it. It is not added to `shots.txt`, which
    is the coordinator's standard list.
  - *Confidence:* high.
- **Trivial discretion (3).**
  - Docstrings that described a rider were trimmed with their calls.
  - Locals left unused by the deletions were dropped (`pivot`, `gun_mount`,
    the Abrams' pivot unpack).
  - `cupola`'s docstring dropped the `dark` role, which only the lid stop
    used.

### Slice 12 decision audit (the lane auditing its own trace)

The six entries above stand as written. The trace found two more:

- **Wrecks were not exported.**
  - *Scenario.* A wreck export runs the same `build` as the live model,
    then the family's `wreck`. That removes parts by name prefix
    (`wreckage.remove`), and a prefix that matches nothing does nothing.
    `head_out` already did nothing on a wreck. No family's `wreck` names a
    part this slice renamed or deleted: the Leopard's `commander_*`, or the
    Abrams' `commander_lid_stop`.
  - *Reach.* The wreck GLBs change too, because their lids are now closed.
    Slice 16 re-exports them, and that export is their first run.
  - *Verdict.* Sound.
  - *Confidence:* medium-high.
- **Two old unused locals were left alone.**
  - *Scenario.* pyflakes flags `abrams.build`'s `m, hull` and
    `sp_howitzers.wreck`'s `half`. Both were there before this slice, and
    both sit outside its crew and lid lines.
  - *Gap.* "Fix easy issues on sight" meets this wave's rule to keep each
    diff to its own lines.
  - *Reach.* Families 21 and 29 own those lines.
  - *Verdict.* Sound under the wave's rules.
  - *Confidence:* medium.
- **C17 Slice 30's `lid_open` grep covers only the tracked roster scripts**
  (2026-11-11). The test tank's `driver_lid_open` part (`tank.py:559`) is test
  art, outside this pass.
- **C18 The shared cupola's vision blocks get fixed with slice 13**
  (2026-11-11). Slice 12's critique found that the closed lid covers the inner
  half of each vision block, so the blocks read as locking tabs with flecks of
  glass. The cupolas that were already closed (T-90M, KF51, ZTZ-99A, the
  howitzers) look the same, so this is a fault in `roof_parts.cupola`. It is a
  coordinator fix to a shared part, done in slice 13's lane before slice 16's
  re-export.
