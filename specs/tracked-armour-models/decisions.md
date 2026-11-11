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
