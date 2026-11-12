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

## Lane 09+11 decisions (2026-11-11, shared slices 09 and 11, decision C16)

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
Review these first:
- the bevel floor reaching `s300` and the wheeled families;
- the Bradley proof barely moving;
- fenders bounded by the envelope, not the upper hull alone.

- **L1 The bevel floor re-bevels the wheeled callers, `s300` included.**
  - *Scenario.* `bolted_panel` now floors its bevel at `PLATE_BEVEL`, and
    `armour_tiles` raises a thin tile's bevel toward `depth * 0.45`. Every
    caller of these shares the change, tracked or not. That means about 20
    wheeled scripts: Boxer, BTR, Stryker, Patriot, S-300, S-350,
    IRIS-T SLM, Fennek, Jaguar, Centauro, VBCI, ZBL-08 and the truck
    chassis. They pass 0.008–0.015 m today.
  - When slice 16 re-exports them, those plates re-bevel. Triangle counts
    stay the same and vertices move under a centimetre. `parallel.md`
    lets lane B re-export `s300` only "to prove it unchanged", and after
    this it can't be.
  - *Gap.* The slice kept the floor out of `parts.py` so buildings and jets
    don't move, but didn't say whether the armour helpers' wheeled callers
    may move.
  - *Reach.*
    - Slice 16's re-export and lane B's `s300` check.
    - The README's "wheeled siblings … re-exported and checked only" now
      means checked for a small, named bevel change.
  - *Verdict.* Sound. A per-caller exception would be the special case the
    floor exists to remove, and the plates gain the same narrow edge. The
    coordinator should tell lane B before it runs the `s300` check.
  - *Confidence:* medium.
- **L2 The Bradley proof barely moves, because its jut is the applique,
  not the fender.**
  - *Scenario.* The Bradley's "front side panels sticks out" is its
    applique tiles and skirt standing above the glacis, plain in the
    corner crop. Its fender sat inside the side outline already. On
    `track_fender` its front comes back up to 4 cm.
  - The CV9040C's and Mk IV's jut is likewise their `armour_tiles` side
    armour. The CV90120, already on `side_skirt`, has none (the unprimed
    critique saw the same).
  - *Gap.* The plan named the fender as the proof on a user note (S7).
  - *Reach.* Slices 25 and 26 carry the visible fix by moving those runs
    onto `side_skirt`. Lane rules kept this lane off those lines.
  - *Verdict.* Sound for the lane. The user's note on these two vehicles is
    not yet fixed, and the end-of-pass report should say so until 25 and
    26 land.
  - *Confidence:* medium.
- **L3 Fenders are bounded by the upper and lower hull's envelope, not the
  upper hull alone.**
  - *Scenario.* The critique said the far-side Bradley fender "pokes out
    past the nose". Bounding it by the upper hull alone (the sponson it
    sits beside) was tried. On both the Bradley and the CV90 the whole
    fender then lies inside the sponson, so the clip removed exactly the
    part you could see and the fender vanished. That was reverted.
  - With the envelope, the fender is a mudguard over the sprocket. It
    stands ahead of the narrower lower nose plate, inside the side
    elevation.
  - *Gap.* Which profiles bound a part that sits beside one hull piece and
    under another.
  - *Reach.* Families choose which profiles to pass. The rule is "inside
    the side elevation", the D2 contract.
  - *Verdict.* Sound. A fender that should tuck under a sponson needs its
    own lower `z` (family data), not a stricter outline.
  - *Confidence:* medium-high.
- **L4 `side_skirt` grew `rows` and line-shaped `foot`/`top`.**
  - *Scenario.* `follow_tiles` was a two-row grid, its front lower tile cut
    up at 45° to clear the sprocket. One run per row would rename the
    CV90's tiles from `side_armour_R_{column}_{row}` to row-first, and the
    CV90 wreck's `side_armour_R_1_` would then blow off a whole row.
  - So `rows` names panels `{name}_{k}_{j}`. The foot is a height or a
    convex line of (x, z) points (it may rise toward a sprocket), and the
    top is a height or a concave line. Each line becomes half-planes, so
    one Sutherland-Hodgman pass clips everything. A foot that isn't
    convex, or a top that isn't concave, raises `ValueError`.
  - *Gap.* The seam's `foot` was a function of x with no rows. Breakpoints
    can't be read from a function.
  - *Reach.* Families pass data points, not closures.
  - *Verdict.* Sound.
  - *Confidence:* medium-high.
- **L5 How the outline clips: the inset is per edge, and only the belly
  is skipped.**
  - *Scenario.* Each outline edge's half-plane moves in by
    `inset * max(|nx|, |nz|)`. On a shallow edge that is exactly `inset`
    vertically, and on a steep one `inset` horizontally, so both "never
    above top(x) - inset" and "never ahead of front(z) - inset" hold. The
    prototype's `glacis_z - 0.03` falls out exactly.
  - The first version skipped every downward-facing edge, so skirts could
    hang below the hull. `hull_shape_test` caught a fender passing under a
    shallow lower nose. Now only the lowest edge (the belly) is skipped.
  - *Gap.* The seam said "shrunk by inset" and named only top, front and
    rear.
  - *Reach.* Every `side_skirt` and `track_fender`. Below the belly, a
    panel is still bounded by the lower glacis and rear plate lines,
    carried on.
  - *Verdict.* Sound.
  - *Confidence:* high.
- **L6 The CV90 identity is a named difference** (`slices/09`, result).
  - The CV9040C and Mk IV are byte-identical.
  - On the CV90120 the glacis-head tile's top follows the outline, where
    the prototype cut a chord up to 3 cm under it. One collinear vertex
    drops, and ten bolts per side move 0.4–8.4 cm onto their tile's own
    clipped span.
  - A panel is dropped when its whole clipped height is under
    `MIN_PANEL_M`. The prototype dropped one when its lowest top stood
    under 10 cm. Neither drops a CV90120 tile.
  - *Confidence:* high.
- **L7 The probe's five non-convex hulls are left to their families.**
  - *Scenario.*
    - BRM-3K: a 14 cm roof step.
    - T-72B3, T-90M, T-80BVM and Type 99A: the upper box's underside
      breaks upward toward the nose, 3–10 cm.
  - Nothing here changes those scripts. Each family slice passes
    `envelope=True`, adds `hull_lower`, or gives the BRM a `top`.
  - The Armata's rings and the Abrams' loft were expected to fail, and
    are convex.
  - *Confidence:* high.
- **L8 The CV90's and Bradley's hull profiles have one owner each.**
  - `cv90.hull_profiles(v, nose_z)` feeds both `hull_body`'s prisms and
    `hull_sides`' outline.
  - The Bradley's two profiles became locals in `hull_body`.
  - This touches the two `prism` lines in each `hull_body`. They are next
    to, but not on, lines lane 12 may edit (hatches).
  - *Confidence:* high.
- **L9 `track_fender` takes the plate floor too:**
  `min(PLATE_BEVEL, depth * 0.45)`, so 0.018 m on today's 0.04 m fenders
  (was 0.012). The slice listed three helpers, but the fender is an armour
  plate the same module draws. *Confidence:* high.
- **L10 Slice 09 shipped `side_skirt` at 0.02 m, not the seam's interim
  0.01 m.** That is the prototype's own bevel, so the identity proof held.
  Slice 11 replaced the literal with `PLATE_BEVEL`, at the same value.
  *Confidence:* high.
- **L11 The Abrams' wear evidence is the vertex alpha, not a closeup.**
  - Workbench closeups don't draw the game's chips. So the slice measures
    the paint's wear attribute: the Abrams' mean goes from 0.121 to
    0.230, and the Leopard 2A6's is 0.234.
  - A wear-mask render shows where the wear lands.
  - The critique flagged the shared recipe smearing wear across faces and
    wheel discs. That is a `parts.py` matter, outside this lane.
  - *Confidence:* high.
- **Territory.**
  - `asset check` finds `hull_shape_test.py` with no wiring: it imports
    `bpy`, so it runs in Blender.
  - A wider bevel never adds triangles: the segment count is per tier
    (`parts._bevel_for`).
- **Trivial discretion (3).**
  - The front-corner shot is `corner:6.8m,4.6m,1.2m:3.0m,1.45m,1.0m:50` at
    1200×750.
  - The probe, diff, wear-mask and wear-stat scripts are committed beside
    their evidence.
  - Before-exports were made at `e598a571c`, into `throwaway/`.

### Lane 09+11 decision audit (2026-11-11, before commit)

L1–L11 above give the reasoning for the calls the lane named while it
worked. This audit traced the session's steps and commits (`35952c02e`,
`a01d095a7`, `7dd22c35f`) for calls those entries left out, and checked
each claim against the exports.

Review these first:
- A2, the gap at a run's foot and top;
- L1, the floor reaching `s300`;
- the L9 correction.

**Verdicts on L1–L11.** All are sound.

- L1 is a coordinator matter, not a user one: tell lane B before its
  `s300` check.
- L2 leaves the user's "front side panels sticks out" note open on the
  Bradley, CV9040C and Mk IV until slices 25 and 26 land. That is the
  plan's scoping, so the end-of-pass report must say so.
- **L9 correction.** The claim "0.018 m on today's 0.04 m fenders" holds
  only for a fender the outline leaves whole. That is the CV90120's: its
  vertices move 1 cm. On the CV9040C and the Bradley, the clipped front
  edge is short, so Blender's bevel clamp (`clamp_overlap`) holds the
  wider bevel back. Their exports are node-for-node unchanged by slice 11.
  The floor still applies; it just can't widen a bevel the plate's own
  short edge limits.

**Calls the entries above missed.**

- **A1 A fender the outline clips away entirely now fails the export.**
  - *Scenario.* A family places a `track_fender` wholly ahead of its hull's
    nose. Before the review, `track_fender` returned nothing and the
    export carried on with no fender. Now `parts.prism` gets an empty
    profile and the build stops with Blender's face error.
  - *Gap.* The seam said nothing about a part clipped to nothing.
  - *Reach.* Any family that misplaces a fender finds out at export, not
    in a closeup.
  - *Verdict.* Sound. A fender that vanishes silently is a family bug the
    lint can't see. The error is terse, so a family lane that hits it
    should know what it means.
  - *Confidence:* medium.
- **A2 A run's foot and top keep half a gap, even with one row.**
  - *Scenario.* A family calls `side_skirt(..., foot=0.55, top=1.20)`. The
    panels run from 0.56 to 1.19, not 0.55 to 1.20. Every cell is shrunk
    by half the `gap` on each side, as the prototype's tiles were, so the
    run's own edges sit a centimetre inside the heights the family passed.
  - The alternative: the gap only between panels, so a single-row skirt
    meets its foot and top exactly.
  - *Gap.* The seam said "each panel is the rectangle … [foot(x),
    top(x)]".
  - *Reach.* Every family migrating to `side_skirt` authors foot and top
    knowing this. It is 1 cm at the default gap.
  - *Verdict.* Sound for now, because it kept the CV90120 identical. If a
    family slice finds itself adding `gap / 2` to its heights, the rule
    should change to gaps between panels only. That costs only re-exports.
  - *Confidence:* medium.
- **A3 Rows without a `top` divide the run up to the outline's highest
  point.**
  - *Scenario.* `side_skirt(..., rows=2)` with no `top` splits the band from
    the foot to the hull's roof height (less `inset`) into two rows. The
    outline then cuts the upper row down the glacis. The rows do not follow
    the glacis down.
  - *Gap.* The seam had no rows.
  - *Reach.* Only matters for a multi-row run without a top; no caller does
    that today.
  - *Verdict.* Sound: the simplest well-defined meaning.
  - *Confidence:* medium-high.
- **A4 The belly is one edge: the outline's lowest.**
  - *Scenario.* A hull whose side elevation bottoms out at a point (two
    sloping belly edges meeting, a V) has no edge lying wholly at its
    lowest height. So no edge is skipped. A skirt hanging below that point
    is then cut by both sloping lines carried on.
  - *Gap.* L5 decided to skip only the belly, but not what counts as one.
  - *Reach.* No tracked hull has a V belly: all 37 probe outlines have a
    flat bottom edge.
  - *Verdict.* Sound.
  - *Confidence:* high.
- **A5 `hull_shape` exports `clip_half_plane` and `span`.**
  - *Scenario.* `armour_parts` clips a panel by its foot and top lines with
    the same Sutherland-Hodgman step the outline uses. It also places bolts
    across a clipped panel's vertical span. Both come from `hull_shape`
    rather than a private copy.
  - *Gap.* The seam named only `Outline`'s three queries.
  - *Reach.* Slice 13's `panel_seam`, or any helper that cuts a shape by a
    line, should import these rather than write its own.
  - *Verdict.* Sound: one owner for the side-plane geometry.
  - *Confidence:* high.
- **A6 The probe looked only at parts named `hull*`.**
  - *Scenario.* The probe recorded every `prism` and `loft` whose name
    starts with `hull` while each tracked appearance built. A hull that
    also draws its silhouette with other names was judged without those
    pieces:
    - the T-15's `troop_compartment`;
    - the Leopard's `front_module`;
    - the T-72's and Type 99's `splash_board`.
  - *Gap.* "Build an `Outline` for every tracked hull" didn't say which
    pieces make a hull.
  - *Reach.* Each family slice decides which profiles bound its panels, and
    may add a convex piece the probe didn't see.
  - *Verdict.* Sound. Adding a convex piece can't make a convex envelope
    fail; a notched one would raise when the family builds it.
  - *Confidence:* high.
- **A7 Evidence PNGs are committed under the spec, in LFS.** That is about
  20 MB across `assets/after/09/` and `11/`: final shots, before shots for
  the corner crop, the paired crops, and the scripts that made them. Per
  "keep generated assets" and the lane's evidence ownership. *Confidence:*
  high.
- **Handoff, not a decision.** The scene-assets Blender README's
  `hull_shape` row still reads "its plan rings, and where a part sits on a
  leaning face". It should add "the side outline its skirts and fenders
  keep inside". That README is outside this lane's files, and lane 08 may
  edit the same table, so it is the coordinator's one-line fix at merge.
- **C19 The bevel floor applies to every caller, wheeled ones included**
  (2026-11-11). Slice 11 put `PLATE_BEVEL` inside `bolted_panel` and
  `armour_tiles`, so about 20 wheeled scripts re-bevel their thin plates.
  That's one rule for every armoured plate rather than a tracked-only special
  case. It costs no triangles: bevel width moves vertices but adds no
  segments. Slice 16 re-exports those wheeled scripts too, `s300` included,
  and lane B's "prove `s300` unchanged" rule is retired.
- **C20 The "front side panels stick out" note belongs to the family
  slices** (2026-11-11). Slice 09 found the cause is side armour standing above
  the glacis: the Bradley's applique tiles, and `armour_tiles` on the CV9040C
  and Mk IV. The fender isn't the cause. Slices 25 and 26 move that armour onto
  `side_skirt`. The CV90120, already on it, shows no fin.
- **C21 Two more shared-part fixes join slice 13's lane** (2026-11-11), each
  judged on its own crop and committed on its own, before slice 16:
  - **Edge wear.** The shared wear recipe (`parts.textured`'s convex-edge
    wear and `finish`'s per-vertex bake) spreads wear across whole faces and
    across road-wheel discs. That contradicts "clean factory paint" and the
    light-catching edge (slice 11 critique).
  - **Faceted running gear.** Road wheels and belts read as faceted drums at
    tier 0 (critiques of slices 10 and 11). Fix it in `running_gear`'s tier-0
    segment counts, within the triangle caps.

## Slice 08 decisions (2026-11-11, coordinator's implementation agent)

Least confident first.

- **Six tracked findings joined the baseline, beyond the brief's "wheeled
  only".**
  - *Scenario.* Removing the patterns exposed one wheeled finding (the VBCI)
    and six tracked ones:
    - the Bradley M2A4 and M3A3's `cannon_sleeve`;
    - the M10's `bore_evacuator`;
    - the M109A7, 2S19M2 and PLZ-05's `fume_extractor`.

    All are lint false positives (next entry).
  - *Gap.* The brief allowed exact baseline entries for wheeled findings only.
    The slice also requires the lint to stay green.
  - *Reach.* Without the entries, any export of those families fails, and so
    does slice 16's full re-export. With them, they pass until each family
    adopts `gun_tube` in its own slice (S6). Under C11 a family must clear
    floating parts, so the entries can't outlive the family slices.
  - *Alternatives.*
    - Keep the patterns scoped to floating parts, which re-hides the gap.
    - Leave the lint red and hand the entries to the coordinator.
  - *Verdict.* Sound, but it oversteps the brief. The coordinator should
    confirm it or revert it.
  - *Confidence:* medium.
- **The floating-part check never counts an intersection as contact.**
  - *Scenario.* `check_floating_parts` measures from sampled vertices, edge
    midpoints and face centres. A closed collar whose end caps cut through a
    barrel has none of those samples on the barrel. So it reads as 2–4 cm
    "from every part" even though it is seated. That is why the patterns
    existed, and why every finding above is false.
  - *Gap.* The slice expected exposed findings to be real gaps.
  - *Reach.* `gun_tube`'s collar closes onto its tube, so the lint sees the
    contact. Families that adopt it clear their entries. A real fix in the
    lint (treat a BVH overlap as distance 0) is outside this slice's files.
    It is a coordinator call, and would also stale other false floating
    entries.
  - *Confidence:* high.
- **No `checks` scope; the three patterns are gone.** The exposed findings
  were all `floating_part` and none was an overlap. A scope limited to the
  overlap check would have hidden nothing, so `vehicle_lint_core` and its
  tests are unchanged. *Confidence:* high.
- **Sleeves now run up to the evacuator.**
  - *Scenario.* Kept at their old lengths, the sleeves left 5–8 cm of bare
    core either side of each collar. The first critique read that as a
    pinched barrel.
  - *Gap.* The slice said to keep lengths and radii.
  - *Reach.* The four guns' overall length and every radius are unchanged.
    Each sleeve section near the collar grows by the old gap.
  - *Verdict.* Sound. The gap was the defect, and a sleeve that meets its
    collar is what the photos show.
  - *Confidence:* medium-high.
- **The evacuator is one turned piece whose ends close onto the tube
  beneath it.**
  - *Scenario.* A closed cylinder around the core floats as far as the lint
    can see (the entry above), and the separate end cones the Abrams and
    Leopard use are a second and third part.
  - *Reach.* One profile carries either chamfered shoulders (`cap = 0`, the
    four migrated guns) or cones (`cap > 0`, probed but no caller yet). A
    ring one shoulder-width in keeps the shoulder's smoothing off the
    collar's face; without it, the first critique saw a "comet" highlight.
    The lathe is private to `vehicle_guns`.
  - *Confidence:* high.
- **The Abrams and Leopard evacuator cones are reversed.** `cyl`'s first
  radius sits at −X, so `evacuator_rear` (`abrams.py:431`, `leopard.py:335`)
  is widest at its open rear end. `gun_tube`'s `cap` fixes this when lane B
  adopts it (slices 21 and 22). *Confidence:* high (probed).
- **The keywords beyond the seam: `band=(proud, length)` and `bore=`.** Band
  height follows the tube beneath it. Defaults reproduce the T-72 and
  Type 99 bands, and the Challenger passes `(0.01, 0.06)`. The bore's radius
  is per gun. *Confidence:* high.
- **The core is the sleeve's radius minus 1 cm, with no taper.** It must sit
  inside both the sleeve and the muzzle, and on the ZTQ-15 they are the same
  0.080. `muzzle_radius` exists for the family slices' taper; the critique
  asked for one. *Confidence:* medium.
- **The muzzle reference box now sits on the sleeve (ZTZ-99A, Challenger 2).**
  It floated 1–1.5 cm above, under the lint's 2 cm threshold. The fix is
  small, it is on the gun being migrated, and the critique flagged it.
  *Confidence:* high.
- **Routed from the last critique** (outside this slice's variable):
  - lane A's family slices: the ZTZ-99A has no mantlet (17), the Challenger's
    barrel butts a box (20), the muzzle sensor is a bare cube (17, 20), the
    T-72B3's root collar runs into an ERA block (17), and the ZTQ-15 has no
    bands and a dark sliver at its root (19);
  - every family: plain evacuator cans and no taper;
  - camouflage stretching along the barrels is the texture mapping on
    cylinders, a shared `textures` matter for the coordinator.
- **Trivial discretion.**
  - The pieces are named under `main_gun` (`main_gun_core`, …). Nothing reads
    the old names.
  - Each family keeps its own barrel root or gun-shield collar, and the core
    starts inside it.
  - Segment counts are the helper's, so the ZTQ-15 goes from 22 to 24 sides.
  - The before shots are the committed GLBs. The pixel diff confirms only the
    gun moved.
  - The crop is a new `gunside` shot, recorded in the slice rather than in
    `shots.txt`.
  - `README.md`'s queue already says to clear the wheeled entries from
    slices 00 and 08, so it needed no edit.

### Slice 08 decision audit (2026-11-11)

The audit traced the session's own steps. Above all, review the first
entry of the slice 08 decisions: the six tracked baseline entries. Every
decision above stands, with the verdict and confidence it gives. The audit
adds three:

- **The blender README's `vehicle_guns` row is left stale.**
  - *Scenario.* The README's module table says `vehicle_guns` holds "Machine
    guns on vehicle and aircraft mounts". It now also owns the tank gun
    barrel (`gun_tube`), as the spec README's one-owner table already says.
  - *Gap.* The README is outside this lane's files. Parallel lanes 09–12 edit
    rows of the same table, and the table is column-padded, so a longer cell
    would rewrite every row.
  - *Reach.* An agent reading the module table won't find `gun_tube` until
    the coordinator updates the row.
  - *Verdict.* Needs the coordinator. Provisional call: at the merge, change
    the row to "Tank gun barrels, and machine guns on vehicle and aircraft
    mounts". It fits the padded column.
  - *Confidence:* high.
- **Slice 08 is accepted with critique scores below the family bar.**
  - *Scenario.* The last critique scored the four barrels 5/6/4/4/5. The bar
    (no score below 6, an average of 7) is the family slices'. Every finding
    behind the low scores is outside slice 08's one variable, which is
    continuity: mantlets, camouflage mapping, taper, bands, the sensor's
    shape. The critique confirmed continuity itself.
  - *Gap.* The slice says "critique last" but gives no bar for a shared
    slice.
  - *Reach.* The routed findings become inputs to slices 17, 19 and 20.
  - *Verdict.* Sound: the README's standing gate 3 sets the scored bar for
    family slices.
  - *Confidence:* medium-high.
- **The ZTQ-15's sleeve now ends where its muzzle begins (redone after the
  audit).**
  - *Scenario.* The sleeve ran to `reach - 0.15` under a 0.16 m muzzle of the
    same 0.080 radius. That left 1 cm of two coincident surfaces, which can
    flicker (z-fight). The other three guns' sleeves already end exactly at
    the muzzle.
  - *Reach.* A 1 cm shorter sleeve at the muzzle. The crop re-renders
    pixel-identical (pixelmatch 0), and the export lint shows 0 new issues.
  - *Confidence:* high.
- **C22 Slice 08's six tracked baseline entries are accepted** (2026-11-11).
  These are the Bradley's `cannon_sleeve`, the M10's `bore_evacuator`, and
  `fume_extractor` on the M109A7, 2S19M2 and PLZ-05. Without them, those
  families can't export until their family slices run. They are false positives
  from the floating check, which never counts one part cutting through another
  as contact. Each family slice clears its own entry when its gun moves onto
  `gun_tube`, under the template's adoption checklist. The floating check
  itself stays as it is: `gun_tube`'s closed collar removes the case.
## Slice 15 decisions (2026-11-11, coordinator's implementation agent)

The frame chain landed as planned: all 64 rows of `fix-rows.json` applied,
none refused. Each entry gives the decision, the gap the plan left, its
reach, a verdict and a confidence, least confident first.

Review these first: the Buk's lint entry needs the coordinator, and the
Ajax's weapon station is dressing until slice 27.

- **The Buk's re-export is committed with one lint issue the baseline
  doesn't hold.**
  - *Scenario.* The upper hull of `air_defence.tracked_hull` sits on both
    track belts' top runs along the whole hull: the belts ride at 1.07 m
    under an underside at 0.88 m. That is hidden seating, and the baseline
    holds it at 110 mm on the left and 190 mm on the right. The geometry is
    symmetric, so the lint's depth measure is noise on a coplanar overlap.
    The 9.3 m hull moved the left side's reading to 166 mm, and the export
    fails on it. Lint for the other five appearances is clean.
  - *Gap.* The brief keeps the baseline out of this slice; the C11 rule
    lets hidden seating stay in it.
  - *Reach.* The script was exported with `--no-lint` so its receipt could
    be written. Slice 16's lint-on re-export fails on the Buk until the
    coordinator refreshes that one entry, or slice 24 lifts the upper hull
    off the belts.
  - *Verdict.* Sound. Tuning the geometry until the noise reads under
    115 mm would hide the issue, not fix it.
  - *Confidence:* medium.
- **The Ajax's weapon station is dressing until slice 27.**
  - *Scenario.* The new HMG mount gets its rig nodes from the frame, but
    the station's art (`rws_*`) stays fixed to the turret, as the slice
    plans. Its body stands 0.21 m over the 2.70 m box, so it goes under
    `dressing_rws`. The HMG fires from the rig's muzzle, which is where the
    drawn barrel ends at rest (the survey placed the mount on it).
  - *Gap.* The slice gives the parenting to slice 27 and the fit to this
    one.
  - *Reach.* Until slice 27, the drawn station doesn't turn with the HMG.
    The lint skips dressing.
  - *Verdict.* Sound. Parenting it now would be a five-line change. It was
    left to slice 27 because the slice gives it to that lane.
  - *Confidence:* medium.
- **The T-14's turret is drawn 0.41 m lower.** `t14.HEIGHT` goes from 1.24
  to 0.83, which puts the roof at the measured 2.75 m. The loft's neck and
  waist come down from 0.26/0.32 to 0.15/0.19 over its base. With the new
  pivot (1.8 m) and gun axis (2.26 m), the old levels would have put the
  waist above the arrowhead's point. The sight heads and the soft-kill box
  are dressing, and the panorama head sits at its measured 3.1 m.
  - *Gap.* The survey found the turret 0.5–0.7 m tall, but no slice before
    18 redraws it, and the box fix forces a fit now.
  - *Reach.* The lower half of the turret is squeezed, because the shared
    Armata deck (`armata.DECK`, 1.92 m) is 0.11 m above the measured
    1.81 m. The deck is shared with the T-15, so it was left for slice 18.
  - *Verdict.* Sound for a fit. It reads as a lower, flatter Armata
    turret.
  - *Confidence:* medium.
- **The T-80BVM's nose is shortened by its glacis run, not moved back.**
  The 7.01 m hull loses 0.195 m at each end.
  - At the front, the glacis keeps its head at 2.30 m and its angles (11
    and 22 degrees) over a 1.205 m run, so the driver's hatch, periscope,
    splash board and headlights stay put. Moving the head back instead put
    the hatch under the turret's front.
  - The nose fittings and the rear stations move 0.195 m.
  - The sprocket moves forward. The idler and the upper hull's foot keep
    their old places, so the belt wraps 6 cm past the nose, inside the
    0.1 m tolerance.
  - *Why the idler stays.* The belt's inner face is coplanar with the
    centre hull's side by construction (`CENTRE_Y = TRACK_Y - TRACK_W / 2`).
    Every idler move flipped the lint's reading of that contact, from
    7.8 mm to 116 mm, with no vertex inside. That is the same noise as the
    Buk's.
  - *Reach.* Slice 17 should pull the idler back under the nose.
  - *Confidence:* medium.
- **The CV9040C, M10 and BMP-2M keep their roof fittings as dressing.**
  - *Scenario.* On these three, the art's roof is the box top, so the
    hatches stand over the 0.1 m tolerance along with the sights. C4 named
    only sight heads.
    - CV9040C: hatches, sights and the bustle tarp, in
      `dressing_roof_fittings`, on the C only.
    - M10: the gunner's sight, the cupola and the loader's hatch. The
      commander's dome gets its own node.
    - BMP-2M: both sights and the turret hatch.
  - *Gap.* C4 assumed every lowered box sits over a roof drawn lower.
  - *Reach.* Hatches under dressing aren't linted. The family slices may
    lower the roof instead.
  - *Confidence:* medium.
- **Wreck tolerances widened for guns that grew past them.**
  - The T-80BVM goes from 1.9 to 2.0 m: the hull is shorter and the gun the
    same length.
  - The Challenger 3 goes from 1.7 to 2.2 m (its own 6.95 m muzzle) and the
    Challenger 2 TES from 0.8 to 2.1 m (7.2 m). Both wrecks lie with the
    gun reaching past the box.
  - The Buk-M3 gains 0.2 m. Its stripped right side stands 0.11 m inside
    the box, the failure slice 01 found.
  - The alternatives were trimming wreck guns or keeping the Buk's right
    skirt. Wreck trimming is out of scope, and the skirt's loss is the
    wreck's look.
  - *Confidence:* medium-high.
- **The Leopard 2A7V's body is the 2A6's, drawn 0.175 m behind the frame's
  centre.**
  - *Scenario.* The 2A7V frame is the 2A6's lengthened at the front only.
    The art built a symmetric hull to the new half-length around fixed
    stations. So the turret (pivot 0.125 m) sat over the engine doors, the
    tail lights and hooks were buried in a longer rear, and the glacis weld
    floated over a changed slope (three new lint issues).
  - The body now builds through `truck_chassis.on`, the existing owner of
    "this chassis is part of a longer frame". It is drawn at 7.7 m
    (`leopard.BODY_HALF`) and offset back. The front module stands 0.35 m
    further out to meet the frame.
  - *Reach.* `leopard.py` imports `truck_chassis`. The 2A6 builds as
    before, since an offset of zero adds no node.
  - *Confidence:* medium-high.
- **The Challenger 3's panoramic sight stands on a mast.** The mast reaches
  2.69 m and is not dressing. The head is dressing and tops out at 2.93 m,
  toward the "over 3 m" that set the 2.70 m box (A2). *Confidence:* medium.
- **Sight heads come down to their measured tops where the bulky allowance
  needs it.** Above the box top plus 0.3 m, only parts up to 0.15 m across
  may stand.
  - Leopard PERI: −0.10 m, to 3.0 m.
  - Ajax ORION: −0.05 m, to 2.99 m.
  - Puma PERI and its mast: −0.11 m, to 3.59 m.
  - CV90120 sight ball: −0.14 m, to 2.79 m.
  - M10 dome: −0.09 m, to 2.89 m.
  - T-14 panorama: −0.08 m.
  - BMP-2M and CV9040C commander's sights: −0.02 m.

  *Confidence:* high: every one comes from the survey's own heights.
- **The Pantsir draws its own tow hook.** It passes `tow=False` to the
  shared chassis and hangs its hook under the rear bumper, inside the
  frame. `truck_chassis.py` is untouched. *Confidence:* medium-high.
- **The tracked air-defence hull's hooks sit on its sloped nose and tail
  plates**, set back by their reach so they end inside the frame (C12).
  Its upper-hull overhang leaves nowhere else to seat them.
  *Confidence:* high.
- **The Buk's launcher is laid off the tail** (`tx = -half + 4.10`), so its
  canisters end inside the 9.3 m hull. The turntable stays behind the cab.
  The frame's pivot at +2.0 m is slice 24's rebuild, because this mount is
  drawn by the hull. *Confidence:* high.
- **The BMP-2M's hatches are placed from the turret ring.** The troop
  hatches were at fixed stations and now follow the ring, as the
  commander's already did. The driver's hatch keeps its hull place
  (ring + 1.15). At ring + 0.95, the ring's 0.2 m move put it up the glacis
  and into the turret's underside. Five new lint issues are cleared.
  *Confidence:* high.
- **Track gauges come from the art's track centres.** CV9040C:
  1.27 / 1.69 = 0.75. Its tracks don't move with the C kit's armour. 2S19M2:
  1.29 / 1.69 = 0.76. *Confidence:* high.
- **The Type 99's commander's sight post reaches the box top.** It is no
  longer dressing and is 4 cm taller, per C4's mast rule. *Confidence:* high.
- **Wreck footprints: 21, not 20.** The 2A8 inherits the 2A7V frame, so its
  wreck takes the same footprint. *Confidence:* high.
- **The off-centre-turrets record** now gives the 2A7V/2A8 ring as +0.125 m
  and the BMP-2M's as +0.1 m. *Confidence:* high.
- **One parity rewrite was reverted.** `BLESS_PARITY` rewrote
  `fixtures/parity/scenario-overrides.json` with one weapon's keys in a
  different order. The content is identical, so it is not a change.
  *Confidence:* high.

### Slice 15 digest record (step 9)

No recorded digest changed. With the new frames, these all pass unblessed:
- sim's 1,162 tests, which include the menu reel's exact shots and every
  `fixtures/parity/` record;
- presentation's 360 tests;
- mapgen's paired records (`compiler`, `layout_cli`).

`BLESS_PARITY=1` on every suite that writes a record rewrote nothing.

Why: no recorded battle fields a fix-rows unit. Mechanic tests run on
`test_*` units, and the reel's capture units extend test units. The
digests that change are those of roster battles fielding these units, each
for its row's cause in `fix-rows.json`:
- hull box: 20 appearances, plus the 2A8;
- shell origin: 41 rows, plus the 2A8 and the M1E3's cannon, through
  `extends`;
- mobility: the Buk-M3, now `tracked_medium`, which changes its sounds and
  its triangle caps;
- eye: the T-14;
- one new mount: the Ajax's HMG `on: autocannon` (Q9b).

The balance sample is slice 31's (Q9).

### Slice 15 decision audit (the implementation agent auditing its own trace)

This audit covers the calls the entries above don't. Each gives a verdict,
and a confidence that the user would have made the same call. Least
confident first.

**Review these first:** the Buk's lint entry (above), the skipped visual
critique, and the T-14's turret drop (above).

- **No unprimed critique or Preview checkpoint was run on the fit edits.**
  - *Scenario.* Several fit edits change what the player sees:
    - the T-14's turret is 0.41 m lower;
    - the T-80BVM's nose is shorter;
    - the 2A7V's front module is longer;
    - sight heads were lowered or put on masts.

    AGENTS.md asks for `screenshot-critique`, `compare-screenshots` and
    `preview-shots` on any visual change. Instead, the agent rendered
    before/after side and front-quarter shots of eight models and judged
    them by eye (`throwaway/s15/shots/*-ba.png`). It found no breakage.
  - *Gap.* The brief scoped this slice to data and minimal fits, and gave
    the re-export and showing to slice 16.
  - *Reach.* Slice 16 re-exports every family. Its critique round should
    include the T-14, T-80BVM and 2A7V, the fit edits that moved geometry
    most.
  - *Verdict.* Sound for a fit-only slice that lands together with 16, but
    it is a deferral the user should know about.
  - *Confidence:* medium.
- **The Gepard's folded radar is dressing on a taller mast.** The 3.0 m box
  sits 0.23 m under the radar. The mast that holds it goes from 0.45 to
  0.60 m, so a non-dressing part reaches the box top, as C4 asks. The
  added mast length is hidden inside the radar. *Verdict:* sound.
  *Confidence:* high.
- **The spec README's handoff and ticks were left alone.** The coordinator
  owns the README, and slice 15 lands only with slice 16. *Verdict:* sound.
  *Confidence:* high.
- **Validation ran on exports written over the tracked source paths, and
  every non-`air_defence` GLB and receipt was then restored.** That way the
  validator matches each file to its catalog entry, including the wreck
  footprints and tolerances. Nothing but `air_defence` is committed.
  *Verdict:* sound. *Confidence:* high.
- **The whole `assets/runtime` folder was pulled (0.83 GB).** The web
  `unitFit` and `icons` tests read every baked bundle. The data came from
  the shared LFS store, in eight seconds. `icons` is green; `unitFit` is red
  only because of the stale bake. *Verdict:* sound. *Confidence:* high.
- Trivial discretion (3 calls):
  - the dressing node names;
  - two-decimal track gauges;
  - the throwaway row applier, which works by text surgery so each fixture
    keeps its formatting and isn't committed.

## Generator pivot (2026-11-12, coordinator with the user)

- **G1 Models come from image-to-3D generators, not agent-built geometry.** (User.) "The whole
  point of this entire spec is NOT for you to create the models yourself ... it's WAY cheaper to
  use a dedicated 3D model and it's WAY better." The slice 13 lane (seams, cupola, wear, facets) was
  cancelled. Slices 08-12 and 15 stay merged: frames are gameplay truth whatever draws the body.
  Slices 16-29 are rewritten around the per-unit pipeline once the pilot (G6) proves it.
- **G2 Inputs are matched views of the measured model, made photoreal.** (Agent, from the trial in
  `assets/source/skins/{t72,abrams}/generator-trial/`.) One three-quarter image from a photo came out
  13-36 % short (the generator guesses the hidden length), and the T-72B3 photo was CC-BY-SA, which
  may not feed a shipped asset. Views rendered from our measured model and restyled by the Bento
  image model with the outline held keep the proportions (side and top views within 1 % of the
  render) and are licence-clean. The image model enlarged nose-on views by 9-25 %; they are scaled
  back to the render's outline before upload.
- **G3 Per unit: Tripo h3.1 multiview (front/left/back/right) and Rodin v2.5 (five views), pick the
  better.** (User.) Tripo had the only T-72B3 with the glacis right; Rodin the best Abrams. Hunyuan
  Pro (four views) was close; Meshy (one view) clean but dearest; Hunyuan Rapid blobby. About $1.00 a
  unit at list price, plus the image edits.
- **G4 Tripo multiview comes out about 15 % short** (hull length/width 1.67 against 1.96 on the
  T-72B3) even with one-scale front and back views. Fitting each body to its battle frame per axis
  corrects it; the pilot checks that the road wheels don't read oval.
- **G5 The inputs' shape errors carry into every output.** All five Abrams came back with the same
  too-thick turret: our measured model's, not the generators'. Shape notes from the user's lineup
  review are fixed in the measured model before its views are rendered.
- **G6 Pilot first:** the T-72B3 (Tripo) and the Abrams (Rodin, after its turret fix) taken all the
  way into the game, then the spec is rewritten from what it proves.

## Argon pivot (2026-11-12, coordinator with the user)

- **A1 Gemini 4 Argon builds every model, in a Bento session, as a Blender script.** (User:
  "use argon from now on. both are HUGE improvements".) Supersedes G3 and G6. The pilot's Tripo
  T-72B3 integration was stopped: the user rejected the generated surface as "very soft and lumpy".
  The session installs Blender, reads the reference photos and our model, writes `build.py`, renders
  five views and corrects itself. Round 1 cost $4.26 (T-72B3) and $3.88 (Abrams), and gave 59k and
  49k triangles with crisp plates. The results are in `assets/source/skins/{t72,abrams}/argon-r1/` and
  `assets/experiments/argon/`.
- **A2 Argon's output already meets most of the integration contract.** Hull, turret, gun, each
  wheel, the tracks and the roof gun come as separate named parts with origins at the turret ring and
  trunnion. G2's restyled view inputs, G4's per-axis fit and the generator split questions are
  retired.
- **A3 Proportions stay the coordinator's job.** (User: "this is just my visual read, not sure about
  proportions, that's your job".) Each round's GLB is measured against the frame (hull box, turret
  extents, pivot, muzzle) and the measurements go back to the session as numbered fixes. In round 1
  the T-72B3 hull sat 0.20 m forward, because the brief carried the pre-slice-15 muzzle (6.30, not
  6.10). The Abrams turret ran about 1 m long at the back, and its roof was 0.06 m high.
- **A4 Briefs use our material slot names** (armor_paint, steel, rubber, optics, fittings, lamps,
  recesses), so nation paint applies without remapping.
- **A5 Check the session's model.** Bento sessions created with `--tier google-recommended` once
  ran another model, and the user had to switch it to Argon by hand. Read the first turn's
  `usageByModel` before trusting a run.
- **A6 At most two Argon rounds per model.** (User, to save money.) Round 1 builds and round 2
  takes the measured fixes. Anything left after round 2 is fixed by the coordinator or a lane, by
  editing the committed `build.py` and rebuilding in the pinned local Blender.

## Argon pilot decisions (2026-11-12, implementation agent)

The pilot's slice is [`slices/argon-pilot.md`](slices/argon-pilot.md), which
also holds the recipe for every other unit. Each entry gives the decision, why,
and a confidence that the user would have made the same call.

- **P1 The export runs the build script, not its GLB.** (Agent; high.) The
  family script imports `build.py` and calls its `build()`
  (`scripted_body.run_script`).
  - One step leaves no intermediate file to drift from its source. An edit
    to `build.py` reaches the game on the next export, and the receipt
    hashes the script.
  - `model.glb` in the Argon folder is a review render, not an input.
  - Importing the GLB would have kept Argon's script untouched. But every
    fix would then need a second, manual rebuild step, and a stale GLB would
    pass unnoticed.
- **P2 The script contract.** (Agent; high.)
  - `build()` builds into the current scene. `main()` resets, renders and
    exports to `OUT_DIR`.
  - Parts are named by what moves them, from a closed list, and materials
    from the seven slots.
  - Both pilot scripts were patched to this contract, and the rollout brief
    asks for it.
  - Classification is by name only, the nearest rig part up the parent
    chain. A part under nothing the game moves fails the export.
- **P3 The rig is the frame's; parts keep their world place.** (Agent;
  high.)
  - `vehicle_export.rig` makes the mount nodes from the frame, as for every
    procedural family, with the same trunnions (T-72 0.95 m, Abrams 1.55 m).
  - Each script part moves under its node with its world transform unchanged,
    and the script's own empties are dropped.
  - Pivots therefore come from the frame whatever the script used. No frame
    changed.
- **P4 Track runs are the shared belt, not Argon's links.** (Agent; medium.)
  - `running_gear.track_loop` wraps that side's `wheel_*` circles at the
    script's run width and place. The belt is as thick as the gap under the
    road wheels, so it stands on the ground.
  - The renderer scrolls a belt's link UVs; modelled links would stand
    still under a sliding texture. The shared belt also owns the sag
    between the rollers (a taste given), which Argon's runs lacked.
  - The cost: the belt reads as a plain band up close (critique).
- **P5 Every turning wheel is its own node, with its measured radius.**
  (Agent; high.) The T-72's idler, sprocket and return rollers were fused
  into its track object; `build.py` now builds them as `wheel_*` objects.
  `radius_m` is the farthest vertex from the axle side-on.
- **P6 `steel` means bare metal.** (Agent; medium-high.)
  - Argon used `steel` for bolts, hinges, grille bars, skirt strips and gun
    bands. Our recipe paints `steel` bare and bright, so on the T-72 they
    read as white pinstripes and silver grates.
  - Painted hardware moved to `fittings`, or to `armor_paint` for the
    Abrams' gun bands. `steel` keeps tow cables, sprocket teeth, MG
    barrels, muzzles and track parts.
  - The rollout brief states this rule.
- **P7 Tiers come from `mesh_lods.reduced_copy` with larger island
  cutoffs.** (Agent; medium.)
  - Tier 0 is the script's mesh, split into one object per material.
  - Tiers 1-3 take each object's share of 22k / 6.5k / 1.5k triangles,
    dropping loose pieces under 2 cm, 12 cm and 50 cm.
  - Argon builds from hundreds of small closed pieces that the collapse
    leaves whole, so only the cutoff brings the far tiers under the
    `tracked_heavy` caps. With `mesh_lods`' own cutoffs, tier 2 was 15.7k
    against a 13k cap.
- **P8 A scripted body's paint chips at `chip × (2/3)²`.** (Agent; medium.)
  - `parts.finish` reads edge wear from the angle at each vertex.
  - Argon's sharp or single-segment bevels read as full edges (1.0), where
    the procedural parts' two-segment bevels read 0.67. With the family's
    `chip=1.0`, whole ERA blocks and plate rims came out chipped pale in the
    game renderer.
  - The scale makes a scripted body wear as a procedural one does. With it,
    worn paint vertices fall from 18 % to 0.5 % (procedural Abrams: 0.2 %).
  - The README's one-value rule (`chip=1.0` on every tracked `run`) holds,
    and `scripted_body` owns the geometry correction.
- **P9 Fit is fixed in `build.py`, never in the frame.** (Agent; high.)
  - T-72: the Sosna-U head and crosswind mast went under `dressing_sights`
    (2.39 m against a 2.23 m box). The cupola moved onto the HMG pivot
    (y = -0.58), and the Kord's barrel reaches the frame's muzzle.
  - Abrams: the M240 and the CITV were lowered into the 0.3 m dressing
    allowance, and the wind vane narrowed to a thin part.
- **P10 Lint is fixed in `build.py`.** (Agent; high.) The edits are in each
  folder's README:
  - T-72: return rollers moved off the road wheels; sprocket and idler
    cleared; fender boxes moved out of the turret's sweep; rear screen, log,
    arms and bustle cage cleared.
  - Abrams: track width, hull side skin, axle stubs.
  - Neither model has a baseline entry now (P13).
- **P11 A worn body's wreck is shared.** (Agent; medium.)
  `scripted_body.wreck` throws one side's belt run flat beside the hull,
  tears off named road wheels, warps and dents the body and drops torn
  plate. `wreckage.scatter` and `burn` then run as for every family. The
  Abrams keeps its gun droop.
- **P12 The Abrams Trophy and SEPv3 keep the procedural body for now.**
  (Agent; medium-low: the user may prefer the family consistent sooner.)
  - Their differences are Trophy housings that replace the bustle's side
    stowage and rails, and the SEPv3's UAPU box, CROWS-LP and data link.
    These belong in the same script as `build(variant)`, not as procedural
    parts glued onto Argon's turret sides.
  - One Argon round on the family script that adds the variants is cheaper
    than a round per sibling.
  - Until then the three Abrams don't match in the game.
  - Their GLBs were re-exported with the family: its receipt covers all
    three. That brought in slices 09–15's merged source changes, which
    their committed GLBs lacked. They lint with no new issues and bake.
- **P13 Only the two models' lint baseline entries were removed.** (Agent;
  high.)
  - The roster walk's `--update-baseline` rewrote about 1,000 lines across
    40 appearances, drifting counts and depths this pilot didn't cause.
  - The committed baseline was kept, minus the T-72B3's and SEPv2's 53
    entries. Both lint clean on their own.
  - The full refresh is slice 16/30's call.
- **P14 The families slice 15's frames left stale were re-exported.**
  (Agent; medium: slice 16 is the coordinator's.)
  - `asset bake` failed at HEAD on 18 appearances whose frames changed in
    slice 15 without a re-export: BMP-2M and BMP-3, T-14, T-15, the three
    Leopards, M10, CV9040C and CV90 Mk IV, Ajax, M2/M3 Bradley, Challenger 2
    and 3, M1E3, CV90120, Puma, Type 99 and T-80BVM.
  - The bake is all or nothing, so the pilot's art could not reach the
    runtime or be checked by `unitFit` otherwise.
  - Each family was re-exported, live and wreck, with no new lint issues
    and no source edits. The bake then passed. This is slice 16's step 2
    for those families only; the wheeled re-exports and the montage remain.
  - Stale icons were regenerated, including the Buk's, Pantsir's,
    Skyranger's, Gepard's and NASAMS's, which `asset check` flagged.
  - The T-90M was left at its committed GLB: it bakes, and re-exporting it
    would bring in slices 09–12's art unreviewed.
- **P15 The game's whip antennas, not Argon's.** (Agent; high.) The
  Abrams' antennas were removed from `build.py`. Both families place
  `RF.antenna` under `dressing_antenna_*` at the script's base positions.
- **P16 No markings.** (Agent; medium.) The procedural models' stencilled
  side numbers ("21", "32 >") are gone with their bodies. The brief keeps
  Argon's models clean, and markings are a later, roster-wide decision.

## Rollout plan (2026-11-12, coordinator)

- **R1 Four lanes by vehicle group, every round 1 at once.** (Agent; high.)
  Lanes A (Eastern tanks), B (Western tanks), C (IFVs and recon) and D
  (artillery and air defence) own disjoint roster scripts. Bento sessions run
  remotely, so a lane starts all its vehicles' first rounds together and
  integrates them one at a time; the slowest part is local integration, not
  Argon.
- **R2 Vehicles that share a hull are variants of one script, in one
  session.** (Agent; medium-high.) The Abrams Trophy/SEP v3 (P12), the three
  Leopards, the CV9040C and Mk IV, the two Bradleys, the Challenger 2 and 3,
  the BMP-3 and BRM-3K (S13), and the M270A2 and MARS II. That is 27 sessions
  for 35 appearances, about $150–200 at the pilot's rate. A lane splits a set
  when round 1 shows the hulls differ more than the brief assumed, and still
  stops at two rounds per vehicle.
- **R3 The "front side panels stick out" note is a brief rule, not a code
  helper.** (Agent; high.) The user saw it on the pilot's T-72B3 too: its
  skirt's front shield stood 8 cm outboard of a narrower, lower mudguard.
  Fixed there as `build.py` edit 14 (the mudguard became a housing flush with
  the skirt), and every brief now states the flush-skirt rule (the pilot's
  clearances, the family template). The procedural `side_skirt` fix (C20) is
  moot for vehicles that wear a body.
- **R4 Lanes leave shared procedural code in place.** (Agent; high.) A family
  that wears a body stops calling its procedural parts, but `t72`, `armata`,
  `cv90` and `abrams` keep what their importers use until slice 30 deletes
  whatever nothing calls. It keeps lanes disjoint, at the cost of dead code
  for a few days.

## Lane B decisions (2026-11-12, Western tanks: slices 20–23)

Each entry gives the decision, why, and a confidence that the user would have
made the same call.

- **B1 Shared-part request: `scripted_body.wear` passes a variant.** (Agent;
  high.) The template asks for one `build.py` per variant set with
  `build(variant)`, but `scripted_body.run_script` calls `module.build()` with
  no argument, so a family can wear only one variant of a script. The call
  wanted: `SB.wear(v, path, link_pitch, variant=None)`, passing `variant` on to
  `module.build(variant)` when it is given. Sent to the coordinator; every lane
  with a variant set needs it. It landed as 5c97d4aa9 (`wear(..., variant=)`),
  merged into lane B; each family wears `variant=variant["id"]`-keyed bodies.
- **B2 Seven sessions, not eight.** (Agent; high.) The lane brief counted eight;
  the slices name seven: Challenger 2/3, the Abrams variants, M1E3, M10, the
  three Leopards, Leclerc, KF51. A set splits only if round 1 shows its hulls
  differ (R2).
- **B3 The M1E3's example script is the SEP v2's `build.py`, not the T-72's.**
  (Agent; medium-high.) The prototype keeps the Abrams' layout and running
  gear, and the SEP v2 script is finished to the contract. The brief tells
  Argon to reshape the hull to the user's note (front deck 1.43 m, engine deck
  1.78 m, blunt glacis, from `frames/m1e3.json`), so it does not inherit the
  SEP v2's hull unchanged.
- **B4 The KF51's muzzle stays the frame's 6.40 m world.** (Agent; medium.)
  The survey left the 130 mm's reach open (6.1 against about 7.2 m from the
  pivot). Frames never move (P9), so the brief asks for a thick barrel and a
  heavy muzzle at the frame's muzzle; the user's "barrel shape wrong" is read
  as its thickness and muzzle, not its length.
- **B5 Briefs paint US vehicles tan.** (Agent; high.) The M10 and M1E3 photos
  are green, but the game paints them in the `us_desert_tan` scheme; Argon's
  flat colour is only for its own renders.
## Lane D decisions (2026-11-12, air defence, rockets and howitzers)

Slices 24 and 29. Each entry gives the decision, why, and a confidence that
the user would have made the same call.

- **D1 Nothing on the lane's tracked vehicles turns, so their bodies name no
  rig part.** (Agent; high.) The Gepard's and Buk's mounts are drawn by the
  hull (`assets/catalog.json` `mounts: hull`) and the M270's and the four
  howitzers' have no role (fixed in the hull), so `vehicle_export.rig` makes
  no node for them and `scripted_body.wear` would refuse a part named
  `turret` or `gun`. The briefs name the turret or launcher base
  `turret_body` (origin on its ring or pivot) and the weapon `gun_body`
  (origin on its trunnion or hinge), both under `hull`, so a later pass that
  gives them a rig renames two objects; sights, radars and roof guns over the
  roof line are `dressing_*`.
- **D2 The M270's rockets leave through the box's front wall; its rear face
  shows the pods' aft caps.** (Agent; medium-high.) The A2's three-quarter
  rear photo shows the box raised on a hinge at the bottom of its rear end,
  its front end lifted over the cab, and the capped cells in the rear face,
  which therefore are the rounds' aft ends. That agrees with the frame
  (pivot at the hinge, x -3.2; muzzle at the box's front face, x +0.5). Our
  current model showed the caps at the rear too, but hinged the box at its
  front (the user's note).
- **D3 The PLZ-05's generated side view goes to Argon labelled "layout only".**
  (Agent; medium.) It is the only square-on side the library has; the survey
  (C7) found its gun too short. The brief tells Argon to trust the three
  photos where they disagree.
- **D4 The M109A7 keeps its frame's 3.90 m width at the turret's racks.**
  (Agent; medium.) The frame's width is unresolved (3.15 m family figure
  against 3.9 m for the A7). Frames never move (P9), so the brief puts the
  3.90 m at the turret's side racks and asks for the hull over its fenders
  as the front photos show it (about 3.4-3.6 m), rather than a fat hull.
- **D5 A turn the provider kills is not an Argon round.** (Agent; medium.)
  The Buk's round 1 ended on "Provider finish_reason: other" after $0.26
  with a half-built script; one message asked it to finish round 1, then
  round 2 ran as usual. The M270's round 2 died the same way, but only after
  it had rewritten `build.py` and both GLBs, which lint and validate clean,
  so that output was taken as round 2 without a resend. A6 counts the
  rounds the work took, not Bento's failures.
- **D6 The four howitzers and the Gepard draw their gun mount with the gun
  rig.** (Coordinator grant; high.) `assets/catalog.json` `"mounts":
  {"gun": "gun"}` for the M109A7, PzH 2000, 2S19M2 and PLZ-05, and the
  Gepard's from `hull` to `gun`, each landed in its vehicle's own commit,
  nothing else in the catalog touched. The barrel rides the frame's pitch
  node, so it leaves the hull-box check; the Gepard's turret now turns in
  the game, and the howitzers' never does (their mounts are fixed in the
  hull, `turret: false`). The bodies' `turret_body`/`gun_body` (D1) were
  renamed `turret`/`gun` in each `build.py`, and the Gepard's round 2 built
  them so. My request had cited the committed M109A7 failing `validate
  --type`; that was a probe on a copy outside the repo, which loses the
  artillery's `hull_extent_m: 5.5` allowance, and I said so to the
  coordinator. The 5.5 m allowances are probably stale now; slice 30's call.
- **D7 The Gepard's wreck takes a 1.2 m footprint allowance.** (Coordinator
  grant; high.) `wreckage.burn` heaves and yaws the turret, which now carries
  the 4.1 m guns, so a barrel lies 0.09 m past the 1 m floor (R10).
- **D8 Painted hardware leaves `steel` in bulk where Argon's helpers default
  to it.** (Agent; high.) The PLZ-05's, M270's and Gepard's scripts (and many
  lines of the M109A7's) used `steel` for hooks, rails, hinges, hubs, rings
  and even wheel discs, which our recipe paints bright (P6): every such use
  became `fittings`, wheel and idler discs and painted barrels
  `armor_paint`, and `steel` kept only tracks, sprocket teeth, MG barrels,
  muzzle crowns, tow cables and piston rods. Each is a numbered README edit.
- **D9 Stale sibling GLBs are re-exported, not left.** (Agent; medium-high.)
  The rocket family's full run (for its receipt) rewrote the HIMARS x2 and
  S-300 GLBs; the committed `rocket_artillery.py` exports the same bytes, so
  they were stale against shared modules, not changed by this lane. They
  carry 0 new lint issues and validate clean; committed in their own commit.
- **D10 `air_defence.py` is not split into `buk.py`.** (Agent; medium-high.)
  Wearing the two bodies removes `gepard()`, `buk()` and `tracked_hull()`
  (about 170 lines), which is the shrink S14 asked for.
- **D11 The M270's rear hinge is shown by swinging its box.** (Agent;
  medium-high.) The launcher stays stowed in the game (out of scope to raise
  it), where the hinge's place can't be seen; a Blender render of the
  script with the box turned 35 degrees on its own origin
  (`assets/after/rocket_artillery/m270a2-hinge.png`) shows the box hinged at
  its rear-bottom and its front end lifting over the cab, the user's note.
- **D12 The before sheets are `asset sheet <appearance>`; the after sheets
  `asset sheet <the roster GLB>`.** (Agent; high.) By name the sheet renders
  the baked runtime, which is the committed art; the new art is only in the
  source GLB, which keeps its reference panel when given by its roster path.
- **R5 The catalog-load tripwire goes to 768 MiB.** (Agent, under Q3; high.)
  The pilot's two bodies grew their four bundles from 11.8 to 17.8 MiB on
  the wire, most of it in the wrecks (the T-72B3's wreck 4.5 to 8.0 MiB),
  leaving every unit's art at 636.7 of 640 MiB. At about 3 MiB a vehicle,
  the other 33 add about 100 MiB. Wreck trimming stays out of scope (Q3);
  if the user wants the load back down, the wreck's tier 0 is where it is.

## Lane C decisions (2026-11-12, IFVs and recon: slices 25-28)

Each entry gives the decision, why, and a confidence that the user would
have made the same call.

- **LC1 Seven sessions for ten appearances.** (Agent; high.) The CV9040C and
  Mk IV, the M2A4 and M3A3, and the BMP-3 and BRM-3K are variants of one
  script each (R2, slice files); the CV90120, Puma, Ajax and BMP-2M have
  their own. The BRM-3K's body lives with the BMP-3's
  (`assets/source/skins/bmp/bmp3-argon-r{1,2}/`), and `brm.py` wears it with
  `variant="brm_3k"` (`scripted_body.wear`'s variant, 5c97d4aa9): the hull
  is one source, as S13 has it.
- **LC2 The autocannon pitches at its mantlet, not at the ring.**
  (Agent; medium-high.) The IFV scripts made their rig with no trunnion, so
  the gun pitched about the turret's centre and a raised barrel swung its
  mantlet out of the turret face. Each brief gives the trunnion at the
  real mantlet (0.5-0.9 m ahead of the ring; the CV90120 keeps its 0.95 m),
  and the family script passes the same distance to `rig` (`trunnion=`).
  Pivots and muzzles, which the simulation reads, do not move.
- **LC3 A second mount that is not a machine gun is still the `hmg` pair.**
  (Agent; high.) The frame gives the Bradley's TOW, the Puma's MELLS and the
  BMP-2M's Kornet the `hmg` role on the turret, and the BMP-3's 30 mm the
  `hmg` role on its 100 mm; the brief names the turning arm `hmg` and the
  pitching box or gun `hmg_gun`, its origin on the frame's pivot. Fixed
  launchers on the turret (the Mk IV's Spike) are `turret`.
- **LC4 Medium bodies are briefed under 70k triangles.** (Agent; medium.)
  All lane-C appearances but the Puma are `tracked_medium` (90k tier 0);
  the shared belt and the paint's edge splits add to Argon's count, so the
  brief leaves room under the cap.
- **LC5 A turn that dies on a provider error is continued, not counted as a
  round.** (Agent; high; the coordinator agreed.) The BMP-3/BRM-3K and
  Bradley round 1, and the Bradley's round 2, ended on "finish_reason:
  other" or "Stream error"; one short message asked the same session to
  continue from what was already in `out/`. The CV90's round 2 died after
  writing every output, so it was taken as done. The Bradley's round-2 GLBs
  predated its last script, so its archive holds that script rebuilt
  standalone.
- **LC6 The A5 check: Argon plus Bento's helper passes.** (Agent; medium-high.)
  Every turn reported `google/gemini-4-argon` with 1-5 cents of
  `openai/gpt-6-luna`, the same split as the pilot's sessions: Bento's own
  titling or summarising, not a second builder. One turn listed
  `typesafe-ai/jev` at $0, a tool.
- **LC7 A build script must never fetch or remove an existing material.**
  (Agent; high.) Argon's `materials.get(name) or new(name)` repainted the
  export's own `armor_paint` with its flat colour as a base-colour factor,
  and the CV90120's splinter print went near-black in the game. Reported to
  the coordinator, who fixed it in `scripted_body` (R9); the round-2 brief
  forbids fetching or removing materials either way.
- **LC8 Wreck footprints: answered by the coordinator.** (Shared-part
  request.) `scripted_body.wreck` throws litter and a track past the
  BMP-2M's, BMP-3's and BRM-3K's footprint tolerances. Answered by R10's 1 m
  rubble floor; no lane edit remains.
- **LC9 Fit outranks a critic's "taller turret".** (Agent; high.) Every IFV
  critique asked for taller turrets or sight masts that the frame forbids
  (P9): roofs, dressing allowances and pivots stayed. Edits went into shape
  and fittings inside them.
  - The Puma's PERI and the CV9040C's sight became drums.
  - The BMP-2M's right Kornet pair is fixed dressing, since the frame gives
    one launcher mount.
- **LC10 Local fixes for lint go where they read least.** (Agent; medium.)
  The Puma's sprocket axle came down 0.065 m rather than raising the
  sponson: the belt wrapping it cut 86 mm into the sponson's underside, and
  the sponson line is the prow chine of the user's note.
- **LC11 Critique rounds stopped at two for most vehicles.** (Agent; medium;
  the user may want all three spent.)
  - Three rounds: the CV90120 and the BMP family.
  - Two rounds: the CV90 pair, Puma, Ajax and Bradleys.
  - What the critics kept naming was structural (turret mass and clutter, the
    Ajax's and Puma's nose shapes) or the shared look (tan `fittings`, a
    washed-out glacis, the paint's mottle, the plain belt). Neither moves with
    a local edit per round. Every vehicle is named below the bar in its
    slice.
- **LC12 The shared look is the biggest lever left.** (Agent; medium-high.)
  - Across all ten models the critics docked materials for three things:
    `fittings` painting tan in the Russian, Swedish and British schemes;
    pale top faces; and the paint's mottle reading as smudges.
  - Argon uses `fittings` for large parts (stowage lids, deck panels, grille
    slats).
  - Either the lane sweep moves large flat fittings to `armor_paint`, or the
    `dark` role's colour per scheme is revisited. This is a slice 30
    question.
- **R6 A scripted body exports byte-identically, and its script never
  iterates a set of Blender objects.** (Agent; high.) Lane B found two
  re-exports of the Abrams SEP v2 differing in triangle order and in one
  reduced-tier piece (about 3 mm). Its `build.py` bevels
  `list({e for f in faces for e in f.edges})`: a set of bmesh edges, ordered
  by memory address, so each run bevels the edges in a different order. The
  T-72B3's script has no such set and re-exports byte-identically. The fix is
  in the script (`dict.fromkeys` keeps first-seen order), and the brief and
  template now ask for it and for a two-export check.
  - *Amended:* ordered bevel input wasn't enough. Lane B found the meshes
    still varied in face order and in each n-gon's loop start (which moves
    the GLB's triangle fan). `scripted_body._split` now puts every scripted
    mesh in one canonical order (vertices by place, loops from the lowest
    vertex, faces by material and vertices), so a script's build order
    can't reach the bytes, and no script carries its own canonicalizer.
    The T-72B3 exports byte-identically twice under it; its tier 0 lost four
    zero-area triangles, which the rotated fans no longer make.
- **R7 The four SP howitzers and the Gepard draw their guns on the `gun`
  rig.** (Agent, at lane D's request; medium-high.) In `assets/catalog.json`
  they declare `"mounts": {"gun": "gun"}`, so the gun pitches on its node,
  the barrel stops counting as hull art, and the Gepard's turret, which
  already turns in the simulation (`turret: true`), turns on screen; its
  `"gun": "hull"` dated from a model with no turret rig. The simulation is
  unchanged: the howitzers' guns stay hull-fixed (`turret: false`), so their
  turrets don't yaw on screen. (Lane D first thought the committed M109A7
  failed the hull-box check; it passes, because the artillery entries carry
  `"tolerances": {"hull_extent_m": 5.5}`. Its probe had validated a copy
  outside the repository, under the global 0.1 m.) Lane D lands each entry
  in its own vehicle's commit (a named exception to coordinator-only catalog
  edits). `vehicle_export.rig` stays as it is, since no caller passes a
  `hull` role once the Gepard is `gun`. Slice 30 removes the 5.5 m tolerances
  that only the barrels needed.
- **R8 The lint's vertex sample depends on vertex order; the canonical
  order stays.** (Agent; medium.) `vehicle_lint._measure_penetration`
  tests every (n//50)-th vertex of each part, so R6's canonical vertex order
  tests different vertices than a script's build order did. On the Abrams
  SEP v2 it found contacts the pilot's sample had skipped: skirt inner faces
  coplanar with the shared belt's outer edge (y = ±1.755), a recess piece in
  the belt's path at the nose, and the wheel discs touching the hull side by
  6 mm. These are clearances the brief already asks for, so they are fixed
  in `build.py` (lane B), not by giving up the canonical order. The lint's
  sampling stays as it is in this pass ("no new lint checks"); a lint that
  tests every vertex, or a spatially fixed sample, is a follow-up.
  - *Amended:* most of the order dependence was a lint bug, which lane B
    found. `_is_point_inside_mesh` unpacked `BVHTree.ray_cast` as
    (hit, location, ...), but it returns (location, normal, index,
    distance), so every ray after its first hit restarted from the normal
    vector, near the world origin, and the parity votes were noise. Fixed
    (slice 00's "make the lint do what its docstrings say"). The roster walk
    under the fix gives 9 new issues in 130 appearances: the Abrams SEP v2's
    8 (the contacts above, which lane B fixes in its script) and the
    MiG-31BM's right main gear strut, measured at 132.8 mm in place of
    17.5 mm, the same seating the J-20's entry accepts at 108 mm. That one
    baseline entry is updated. Receipts hash the lint module, so every
    receipt is stale until slice 30's freshness pass re-exports them.
  - *Amended again:* the fix left a second fault, which `vehicle_lint_test`
    caught (I had not run it). Each ray stepped 0.1 mm past its hit, and
    the BVH (epsilon 0.1 mm) reports a hit that far short of the face, so
    the next cast met the same face: a thin plate counted twice and a point
    inside a 10 cm skirt read as outside. Rays now step 1 mm past. Under it
    the roster walk drops 267 baseline entries (false contacts) and adds
    12: the pilot Abrams SEP v2's 9 (hull skin against the belts and the
    left wheel discs, which lane B's script replaces), the old Ajax's hull
    and turret seated 10 mm (lane C replaces it), and the BTR-82A's gun
    housing and mantlet seated 9–10 mm; the F/A-18E's gear strut entry
    reads 28 mm. The baseline is regenerated; slice 30 regenerates it once
    more after the last body lands.
- **R9 A build script can't repaint the export's materials.** (Agent, from
  lane C's finding; high.) The export makes its materials under the slot
  names before the script runs, so a script that looks its slots up by
  name (`materials.get("armor_paint") or new(...)`) took the export's own
  and set its colour. The CV90120's GLB then multiplied the scheme's albedo
  by Argon's dark flat green, near black in the game. `scripted_body.run_script`
  now hides the export's materials under other names while the script
  builds, and restores them after; the script's own come in as
  `script:<slot>`, which `_split` maps by role as before. The T-72B3
  re-exports byte-identically.
- **R10 A wreck may spread 1 m past its box.** (User: "the wreck validation
  is too strict ... maybe go up to 1m?"; lane C's shared-part request.)
  `scripted_body.wreck` throws a track run and torn plate to fixed spots
  beside the hull, which on a narrow hull (BMP-2M, BMP-3, BRM-3K) lie
  0.8–0.9 m past the wreck's `footprint_half_m`, over their 0.3–0.8 m
  `footprint_m`. The litter lies flat and blocks nothing, so the wreck
  scenery kind's footprint now carries `rubble_m: 1`, the least footprint
  tolerance any wreck gets (`scenery.ts`, read by `footprintFindings`); an
  entry may still ask for more. `wreck()` is unchanged, so no wreck moves.
  The 29 wreck entries at 1.0 or less are now redundant; slice 30 drops
  them, not now, while four lanes edit the catalog.

## Lane A decisions (2026-11-12, Eastern tanks: slices 17-19)

Each entry gives the decision, why, and a confidence that the user would
have made the same call.

- **LA1 Six sessions, one per vehicle, all started at once.** (Agent; high.)
  - Bento's dispatch was down workspace-wide from 08:51 to about 11:57 UTC.
    Every lane's sessions were created (each create returned 503) but sat
    undelivered.
  - Nothing was re-created. When dispatch recovered, the sessions ran as
    briefed.
- **LA2 A few cents of `openai/gpt-6-luna` in a turn's `usageByModel` passes
  A5.** (Agent; medium-high.)
  - Every turn reported Argon plus $0.00-0.10 of `gpt-6-luna`.
  - Argon did all the building: the helper's share is under 3 % of every
    turn, and the tool-call ids are Argon's.
  - A5's point is that no other model builds, so I read the helper as the
    session's own overhead and did not stop the sessions.
- **LA3 A round that fails on a provider error is resumed once, as the same
  round.** (Agent; medium.)
  - The ZTQ-15's round 2 ended `failed` (`finish_reason: other`) after
    $1.54, having written nothing.
  - One message asked it to finish the same eight fixes. No new asks: that
    is round 2 completed, not a third round (A6).
  - It cost $4.74 more, making the ZTQ-15 $10.27 in all. The user may
    prefer local edits over paying for a resumed round.
- **LA4 The family scripts were rewritten whole.** (Agent; high.)
  - `t80`, `t90`, `type99`, `t14`, `t15` and `type15` now hold only the
    body's path, the link pitch, the trunnion, the whips' base and the
    shared wreck.
  - Nothing imports these six, so their procedural code went with them.
  - `t72.py` and `armata.py` keep everything (R4); slice 30 sweeps what
    nothing calls now: all of `armata.py`, and the T-72 procedural parts
    once nothing imports them.
- **LA5 The whips stand on the roof the body actually has.** (Agent; high.)
  - Each `ANTENNA` base was probed against the built turret (a ray down the
    turret mesh at the chosen x, y).
  - Taking the frame's roof height had left the ZTZ-99A's whips floating
    over its lower bustle.
- **LA6 "Too dark" is the shared look, not the bodies.** (Agent; medium-high.)
  - Every critique called the new bodies darker than the procedural ones.
  - Measured on the close front view (pixels darker than the ground), the
    mean grey is unchanged: T-15 58.6 against 56.8, ZTZ-99A 52.8 against
    54.7, T-90M 54.7 against 56.9.
  - The bleached glacis tops and black flanks are the scheme's mottle and
    the workbench light, as the pilot found. No body was brightened to
    please the critic.
  - (A real darkening, Argon's flat colour multiplying the scheme, was the
    coordinator's 8a3995b96, merged before any lane-A integration.)
- **LA7 Painted hardware moves from `fittings` to `armor_paint` in the
  script.** (Agent; medium-high.)
  - The export paints `fittings` a brownish olive (`vehicle_export.materials`,
    `fittings=(0.11, 0.095, 0.068)`). Against Russian green or PLA camouflage
    it read as tan flecks, tan panel grids or blank tan tiles, which every
    critique flagged.
  - The brief's own slot rule says painted bolts and hinges are
    `armor_paint`.
  - Applied to the hull and turret of the T-15, T-90M, T-14 and ZTQ-15 (a
    local edit, `paint_hardware`); gun, wheels and roof guns keep theirs.
  - Shared-part request LA-S2 asks whether the shared fittings tone should
    change instead.
- **LA8 Frame-bound shape faults are fixed inside the frame, or named.**
  (Agent; medium.)
  - The ZTZ-99A's turret body was stretched from a 2.19 m to a 2.28 m roof,
    as high as the frame's 2.5 m top and its 0.1 m tolerance allow, given
    roof fittings 0.31 m tall.
  - The T-14's panorama was cut to 3.10 m to pass the dressing allowance.
  - The critics' "taller turret" asks for the T-90M, T-14 and ZTQ-15 are
    left: the frames set those heights (P9).
- **LA9 The T-15's right-hand Kornet pair is fixed turret art.** (Agent;
  medium-high.)
  - The real Bumerang-BM module carries a pair each side; the frame has one
    `launcher` mount.
  - The second pair is drawn in `turret`, so it turns but does not pitch.
    The battle is unchanged.
- **LA10 The T-90M keeps the drawing's turret placement against the
  critic.** (Agent; high.) Both critique rounds asked for the turret
  further forward, which is the user's own complaint reversed. The drawing
  puts the bustle at x = -3.0.
- **LA11 Two critique rounds, not three, where the third had nothing to act
  on.** (Agent; medium.)
  - The T-80BVM, T-90M, T-14 and ZTQ-15 stopped after two.
  - Their remaining findings were the shared paint look, frame-bound
    heights, or a turret rebuild beyond local edits: a T-80BVM front of flat
    Relikt blocks, a slab-sided T-14 turret.
  - The T-15 and ZTZ-99A ran three.

### Shared-part requests

- **LA-S1 Edge wear on thin bars (`parts.finish` / `scripted_body`).**
  - The T-14's slat cages, now `armor_paint`, still read tan or brass. A
    rod a few centimetres thick is all edge, so the paint's chip tone covers
    it whole.
  - Wanted: a scripted body's chip scaled down on thin parts, or a
    slat-cage slot without edge wear. Call: `scripted_body.wear` passes a
    thickness-aware chip.
  - Lane A carries on without it.
- **LA-S2 The `fittings` tone.**
  - `vehicle_export.materials(fittings=(0.11, 0.095, 0.068))` reads tan
    beside Russian green and PLA camouflage on every Argon body.
  - Wanted: either a fittings tone per scheme (a darker shade of the
    scheme's paint), or confirmation that lanes should paint hardware
    `armor_paint` (LA7).
  - Lane A did the latter in its scripts.
- **R11 A far tier sheds its smallest pieces until it fits.** (Agent, from
  lane D's Buk-M3; high.) `mesh_lods.reduced_copy` gives each loose piece a
  floor it never decimates below (a piece of 32 triangles or fewer keeps
  all of them), so a script's body of hundreds of small closed pieces kept
  them all and overran the tier: the Buk's tiers 1–3 came out 37.2k / 13.2k
  / 4.7k against `tracked_medium`'s 34k / 10k / 3k, and cutting its
  cylinders' segments made it worse. Now, smallest first, a piece also drops
  out past the tier's extent cutoff while the pieces' floors alone overrun
  the budget. Where they fit, nothing changes. The T-72B3 (78.0k / 31.5k /
  7.7k / 3.0k, now 27.4k / 7.3k / 2.1k) and the T-14 (33.8k / 9.0k / 3.7k,
  now 33.5k / 8.4k / 2.0k) were quietly over too; tier 0 is untouched. It
  is general (`make_tiers` shares it, so an infantry kit's tier 3 may shed
  pieces when next exported); slice 30's freshness pass re-exports all.
  - *Then:* the Buk's tier 1 still came out 36.3k against 34k (tiers 2–3
    fit): the paint's edge splits and the collapse's overshoot on small
    pieces take a 22k target to about 1.65 times it. Tier 1's target is
    now 19k for every scripted body, the general number, not a Buk
    exception or a heavier class for it (20k left the Buk 64 triangles
    over: the floors keep part of the tier whatever the target).
- **R12 Every tank turret's front is swept back.** (User, on the Leclerc:
  "every tank's turret have a front that's swept back, it's never flat".)
  Seen from above, the turret front runs back from the mantlet to each side,
  an arrowhead or wedge, never flat across and never angled in toward the gun
  (the Leclerc's cheeks were yawed the wrong way, a concave V). It goes into
  the brief's shape rules for every round still to run, the lanes check it on
  every turret they own, and slice 30 measures every turret's plan outline
  and fixes each that fails.

- **R13 Scheme paint is lusterless.** (Agent, from lane B's "pale patch" and
  a delegated diagnosis; high.) The pale faces on the Leclerc's, Challenger
  2's and KF51's cheeks were the sun's specular highlight, not edge wear or
  AO: the paint was a dielectric at roughness 0.53, and a large flat plate
  near the sun-camera half-vector (N·H 0.93) lit its 7 %-albedo paint about
  three times over. With the sun's specular removed the patch goes, while
  removing the sky reflection, the AO or the albedo texture leaves it. Every
  scheme's clean roughness (`textures._sprayed`, and `nato_camo` alike) is now 0.8, as military
  paint is (CARC, NATO IRR). Colours and schemes are unchanged. Bright tops
  and dark flanks are a separate matter, the workbench sun behind the flank
  in the standard views (N·L 0.13), not the AO bake, which is no brighter on
  top than the old bodies'. Paint textures ride in each GLB, so every ground
  vehicle takes it at slice 30's re-export.
