> **Superseded.** This is the brief the three plan drafters worked from. The canonical plan is `../README.md`. Where the drafts corrected this brief, the correction is a synthesis decision (S1–S15) in `../decisions.md`. Paths below that say `scratch/tracked-lineup/` now live in `../assets/` or `throwaway/tracked-lineup/`.

# Brief: plan the tracked-armour model pass (spec drafting)

You are drafting a **sliced implementation plan** (a spec) for a repository at
the root of this checkout. Do not implement anything and do not edit any file
outside your own scratch notes. Return the plan inline.

Read first: `AGENTS.md`, `development.md` (skim the check ladder), `specs/README.md`,
`.agents/skills/make-3d-models/SKILL.md` (the per-model procedure every family
slice follows), and `.agents/skills/write-spec/SKILL.md` (the spec format; also
summarised at the end of this brief). The full planning map (four
quadrants, decision ledger, landmine cards) is
`scratch/tracked-lineup/map.html` — read its `<script>` data arrays (KK, L, UK,
N, C, CF, T); everything below summarises it.

## Goal

Improve the 3D models of every tracked armoured unit (37 models, 26 Blender
scripts under `packages/scene-assets/blender/roster/`) with the
`make-3d-models` skill. This is the first of several per-unit-group passes
(later: wheeled armour, helicopters, wheeled artillery/AD, trucks, infantry,
drones). The plan must let a fresh agent start at slice 1 without any chat
history, and must be executable end-to-end without approval stops.

Target look: **cinematic (Hollywood) realism, not museum realism** — correct
proportions and silhouette first; small fittings only where they read at the
zoomed-in battle camera or in cinematics (the closest the player ever sees a
tank).

## Scope: the 37 models

- Tanks: Abrams SEP v2 / SEP v2 Trophy / SEP v3 Trophy (`abrams.py`), M1E3
  (`m1e3.py`, built on abrams), M10 Booker (`m10.py`), Leopard 2A6/2A7V/2A8
  (`leopard.py`), Challenger 2 TES (`challenger.py`), Challenger 3
  (`challenger_3.py`), Leclerc XLR (`leclerc.py`), KF51 (`kf51.py`), CV90120
  (`cv90120.py`), T-72B3 (`t72.py`), T-80BVM (`t80.py`), T-90M (`t90.py`),
  ZTZ-99A (`type99.py`) — t80/t90/type99 build on t72.py — T-14 (`t14`→`armata.py`),
  ZTQ-15 (`type15`).
- T-15 (armata), Puma (`puma.py`).
- IFV/recon: Bradley M2A4 / M3A3 (`bradley.py`), CV9040C / CV90 Mk IV (`cv90.py`),
  BMP-2M / BMP-3 (`bmp.py`), BRM-3K (`brm.py`), Ajax (`ajax.py`).
- SP howitzers: M109A7, PzH 2000, 2S19M2, PLZ-05 (`sp_howitzers.py`, one script).
- AD and rockets: Gepard, Buk-M3 (`air_defence.py`, shared with wheeled Pantsir,
  Skyranger 30, NASAMS ×2), M270A2, MARS II (`rocket_artillery.py`, shared with
  HIMARS ×2, BM-21, Tornado-S). Wheeled siblings in shared scripts are
  re-exported and checked, not reworked.

## Decisions already made (carry as givens; don't re-litigate)

User decided:
- **Route:** procedural (code) pass. Plus a three-way cast-turret experiment on
  the T-72B3 dome: current vs smoothed in code (e.g. subdivision) vs a
  generated image-to-3D skin of the turret alone. Skins are adopted only if
  they clearly win.
- **Jet experiment:** smooth the F-35 and Su-35 skin *guides* in code and
  compare against their generated skins; if code ties or wins, moving all jets
  off the generator becomes its own follow-up spec (not this one).
- **Jet paint sheen (J1):** jets get the glossy/metallic sheen of real jet paint
  in sun. Today `textures_air.py:15-18,53` sets `AIR_ROUGH = 0.55` ("satin, not
  gloss"), shared with helicopters; chosen in commit f5a8a2b59 so a curved
  skin keeps its grey to the silhouette. Plan: separate lower roughness for
  jet schemes only (helicopters stay satin), tuned in the battle camera against
  the user's screenshot `scratch/tracked-lineup/jet-sheen-reference.png`;
  metalness stays 0 unless gloss alone can't give the sheen. Renderer already
  has PBR sun specular + PMREM sky reflections
  (`crates/renderer-core/src/shaders/modules/lighting.wgsl:150-169`). Ships
  with the jet experiment.
- **Budget:** the catalog-load tripwire (`packages/scene-assets/src/schema.ts:756-773`,
  640 MiB, 635.3 used) is raised freely to measured load + room; no startup
  requirement; trimming wrecks is out of scope.
- **Measurements:** free drawings go into the reference library
  (`assets/references/<family>/references.json`); else copyrighted drawings
  measured in scratch with only numbers committed + source note; else generated
  orthographic views recorded `generated`, scaled to published dimensions and
  checked against photos.
- **Frames:** battle frames found wrong get fixed, as a named digest-changing
  decision, with one balance sample at the end. Known suspects: Leopard 2A6
  height 3.0 m vs ~2.6 m in photos; Puma MELLS overhang; Ajax.
- **Ajax:** gains a roof machine-gun mount `on: autocannon` (like the
  Leopard's HMG `"on": "cannon"`; mechanism exists,
  `crates/contract/src/weapons.rs:286-289`). Named balance change.
- **Taste:** clean factory paint (no weathering); one clean scheme per nation
  (no camo work); crisp hard armour plates with a narrow light-catching bevel;
  panel lines/weld seams/bolt rows "painted in"; silhouette & proportions
  first; track sag between return rollers; hatches closed, **no crew** on any
  tracked model.
- **No backward compatibility, no migrations.** Hard cutovers; re-record
  fixtures/digests/baselines rather than adding shims.

Agent decided on the user's behalf (user opted in; still givens):
- **Order:** baseline re-export → split `vehicle_parts.py` → experiments (parallel)
  → measure all frames → shared parts → frame fixes (one bless chain) →
  families: T-72 family, Armata, Abrams+M10, Leopard, other tanks, IFVs,
  howitzers, AD+rockets → closeout. Opus subagents in parallel worktrees;
  only the coordinator edits shared modules and bakes. One commit per family;
  push milestones to staging and promote (`bun run promote:main`).
- **Bar:** per family, the make-3d-models critique bar (unprimed
  screenshot-critique: no score < 6, average ≥ 7), scores recorded; a lineup
  cohesion sheet at the end. No approval stops; anything still below the bar
  is named to the user.
- **Done per family:** critique bar AND every one of the user's lineup notes
  for that family visibly fixed in a before/after beside its reference.
- **Edge wear:** `chip=1.0` on every tracked model (Abrams family currently
  0.35 default, `abrams.py:502`; others 1.0).
- **M10:** exaggerate its nose and turret length-to-height ratio within the
  frame so it doesn't read as a small Abrams.
- **D1 seams:** experiment on T-72B3 and Leopard 2A6 — shallow geometry
  grooves darkened by the existing per-vertex AO bake vs a per-model baked
  detail map (normal+AO). Adopt the bake only if it clearly wins at the zoomed
  camera. (Textures tile per scheme in `textures.py`; per-model maps would be
  new machinery and bytes.)
- **D2 side panels:** one shared skirt/fender helper whose panels never jut past
  the hull's side outline (stop where the glacis starts or follow its slope);
  families stop drawing their own (`bradley.py:73`, `cv90.py:81`,
  `sp_howitzers.py:101-105`, and equivalents in ajax/puma).
- **D3 rockets:** draw the M270/MARS II turntable + elevation hinge at the rear
  of the launcher box (`rocket_artillery.py:164-168` has it at the front). The
  launcher stays stowed (fixed mount); raising it is out of scope.
- **D4 T-15:** keep the 30 mm; draw the barrel at its real length/shroud
  thickness; if that moves `muzzle_m` it joins the frame fixes.
- **Q8 reuse:** spec at `specs/tracked-armour-models/`; README ends with the
  queue of later passes; the make-3d-models skill gains only lasting lessons
  (never per-task notes).

## The user's lineup notes (each family's defect list)

- M1E3: back needs to shift up; front too pointy.
- M10: turret shape looks wrong.
- Challenger 2 TES and Challenger 3: turret shape wrong.
- KF51: turret and barrel shape wrong.
- T-80BVM: turret too small. T-90M: turret too far forward.
- ZTQ-15: turret shape wrong.
- T-15: gun barrel too small.
- Puma: body shape wrong; front side panels stick out.
- Bradley M2A4, M3A3, CV9040C, CV90 Mk IV, Ajax: front side panels stick out.
- BMP-2M, BRM-3K: upper front slopes down too much; lower front should slope up more.
- M109A7, PzH 2000, 2S19M2, PLZ-05: turret too big or body too small; front side panel sticks out.
- Buk-M3: needs more detail.
- M270A2, MARS II: launcher pivot belongs at the rear.
- Known before the walk: T-72 `gun_2a46` (`t72.py:208-222`) barrel built from
  sleeve sections with see-through gaps and no core (shared by T-80/T-90/Type 99);
  M10 reads as a small Abrams; Leopard/Puma/Ajax frame mismatches
  (`specs/done/unit-models/choices.md:119-243`).

Lineup renders and reference thumbnails: `scratch/tracked-lineup/thumbs/` and
`scratch/tracked-lineup/closeups/` (fq/side, shaded/matcap per model). They
should be copied into the spec's `assets/before/` as the "before" record.

## Landmines (from a sweep; must shape the plan)

1. **Frame-change chain:** Blender reads `fixtures/catalog.json`
   (`catalog_frames.py:23`), regenerated only by
   `BLESS_CATALOG=1 cargo test -p sim --test sim catalog::`. Wreck
   `footprint_half_m` in `assets/catalog.json` must equal the hull exactly
   (`validate.ts:1658-1667`). `track_gauge` is a fraction of half-width
   (`crates/presentation/src/fielded.rs:440`, `crates/sim/src/battle.rs:2139`).
   Hull caps `crates/contract/src/limits.rs:21-24`: tracked half-width ≤ 2.1
   (both Challengers at 2.1), half-length ≤ 4.8 (T-15 4.75). A shorter Leopard
   pushes its sight into the `dressing_*` allowance (0.3 m bulky / 0.15 m thin,
   `unitArt.ts:44`). Turret pivot moves need hatch re-checks
   (`specs/done/off-centre-turrets.md`); the 2A8 inherits the 2A7V frame.
   Frame data lives in `fixtures/units/roster/{us,europe,eastern}.json`.
2. **All 25 tracked receipts are stale** (`assets/source/roster/<family>/source-receipt.json`,
   LFS); nothing checks `source_sha256`. `--variant`/`--wreck` runs write no
   receipt (`vehicle_export.py:218`). Shared scripts re-export wheeled siblings.
3. **File-size ratchet** 32,000 bytes (`check-file-size.mjs:190-192`): over-budget
   files may not grow: `vehicle_parts.py` 51,243, `textures.py` 86,279,
   `vehicle_lint.py` 42,691. Nearly full: `parts.py` (~890 B left),
   `abrams.py` 29,056, `air_defence.py` 26,205, `wreckage.py` 25,753. Split by
   concept before growing.
4. **Lint allowlist hides defects:** global patterns `*_sleeve`,
   `fume_extractor`, `bore_evacuator` (`vehicle_lint_allowlist.json:29,61,66`)
   silence every check on matching parts (`vehicle_lint_core.py:136-142`) — this
   is how the T-72 barrel gap passed. Other lint gaps (sharp edges, no new
   machinery): only tier 0 checked; `dressing_*` skipped; same-top-node parts
   not checked against each other; crew added after lint; wrecks never linted.
5. **Lint tooling:** `--roster`/`--update-baseline` see only a shared script's
   first family folder → Buk, MARS II, PzH 2000, 2S19, PLZ-05 skipped;
   `--update-baseline` silently drops failing families and exits 0
   (`vehicle_lint.py:944-1032`). Easy fixes.
6. **15 of 25 tracked families never export-linted:** abrams, m1e3, ajax, bmp,
   bradley, brm, challenger, challenger_3, leclerc, puma, t15, t90, gepard,
   sp_howitzers, rocket_artillery. First linted export will surface old issues.
7. **Pictures to regenerate (once, closeout):** unit-roster baselines
   (`UPDATE_BASELINES=unit-roster`, `web/scenes/unit-roster.mjs:111,182`),
   tutorial shots (`UPDATE_BASELINES=ui`, `web/tutorial-shots.mjs:43,53`), menu
   reel footage (Abrams SEP v2 and T-72B3), `asset icons`
   (`web/tests/icons.test.ts:56-63`). Agent may bless after inspecting.
8. **Parity/balance:** `BLESS_PARITY=1`; `paris-corner-*.json` hold frozen Abrams
   data; balance sample `cargo run -p mapgen --release --example battle_sweep -- --seeds 10 --seconds 30` + `skirmish_tournament`.
9. **Blender** 5.2.1 via the asset CLI only (`web/asset.mjs:101-102,509-517`):
   `bun run --cwd web asset -- blender <script> -- [--variant=<id>] [--wreck]`.
   Traps: `rotation_euler`, magenta repack, relative paths, `noise_vector`.
10. **Parallel worktrees:** each needs its own `bun run build:wasm`; `asset bake`
    deletes runtime folders it didn't produce (`web/asset.mjs:288-291`) → lanes
    export source GLBs + receipts only, coordinator bakes once on the merged
    tree; `asset sheet` under `lockf -k <main>/throwaway/gpu.lock`. Removing crew
    drops the LFS rifleman pull (`vehicle_crew.py:31-44`).
11. **Python tests not wired:** `catalog_frames_test.py`,
    `vehicle_lint_logic_test.py`, `wreckage_test.py`, `vehicle_lint_test.py`
    run in no check; no CI. Decision: wire them into `asset check`.
12. **Jet paint matte on purpose** (see J1).
13. `test_tank_wreck` 12.2 MiB in the load — out of scope.

## Facts known

- Shared parts: `vehicle_parts.py` (`track_loop` 257-323 is a convex hull → no
  sag; `tracked_running_gear` 326+; `hatch`, `cupola(lid_open=)`, `bolted_panel`,
  `weld_line`, `armour_tiles`…), `parts.py` (`box(..., bevel=)`; bevels only in
  tiers 0–1, `parts.py:318-330`), `vehicle_export.py` (`materials(chip=)` 70-90,
  `Vehicle.head_out` crew 133-149, crew import 196-210, `run` 243,
  `run_disabled` 289).
- Triangle caps (`packages/scene-assets/src/unitArt.ts:53-62`): tracked_heavy
  120k/45k/13k/4k; tracked_medium 90k/34k/10k/3k. Largest tracked model is
  T-80BVM at 72,845 (tier 0). Room for bevels.
- Tools: `scripts/closeups.py` (Blender closeups of GLBs, no runtime catalog
  needed), `scripts/drawing_overlay.py`, unprimed `screenshot-critique`,
  `compare-screenshots`, `preview-shots`. `asset sheet` needs the whole runtime
  catalog pulled.
- Generated skins: Hunyuan3D 3.1 Pro on fal.ai (`tools/skin-generate.mjs`,
  `FAL_KEY` never printed/committed), fitted by `aircraft_skin.py` (orients by
  highest point = fin tip; for a turret that assumption breaks), pins in
  `blender/skins/*.json`, guides in `blender/guides/*.py`.
- Hit-rate check: Leopard 3.0→2.6 m drops vertical hit rate 75→71% @300 m,
  58→52% @500 m, 38→34% @850 m (aim at 75% of hull height, 3σ-truncated spread).
- User preferences (memories): questions up front then finish to 100%; numbers
  are guesses; fix easy issues proactively; expensive checks once at the end;
  fewer, bigger functional tests, no snapshot tests of data; Opus subagents for
  all 3D/visual work; push green milestones; skills are timeless; keep every
  generated asset (commit unpicked candidates); recompute over compat layers.

## What to return

A complete slice graph in this format:

- README sketch: goal, non-goals/firewalls, single-owner invariants, the
  review surfaces, a Next Agent Prompt (second person, status, pickup point,
  global TODO checklist pointing at slices).
- For each slice (`slices/NN-name.md`): the contract it unlocks; the API seam
  (module/function/data, ownership); what the human can run or see; the
  verification (named commands, closeups, screenshot-critique as the last
  check for any visual slice, compare-screenshots against the before/reference
  where one exists); for visual slices the **one slice variable** and the
  crop/view judging it plus visible wrongness explicitly out of scope; the
  decisions delegated to the implementer (everything else resolved); what must
  stay green; what feedback would change it; a non-blocking review checkpoint
  where useful (open shots with preview-shots, ~5 min window, then decide and
  record).
- Parallelism: which slices run as concurrent worktree lanes and who owns which
  files; merge order.
- Risks and the cheapest early check that kills each.
- Anything in this brief you think is wrong or under-specified, with evidence
  (file:line) from the repo.

Ground every claim in files you actually read (cite file:line). Be concrete.
