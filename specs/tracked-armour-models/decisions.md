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
