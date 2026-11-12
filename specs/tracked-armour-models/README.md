# Tracked armour models

Rebuild every tracked armoured unit's 3D model so it reads as its real vehicle
at the zoomed battle camera and in cinematics: 37 appearances from 25 export
scripts.

Silhouette and proportions come first, then a crisp, clean factory finish.
Each vehicle's body is a Blender script that Gemini 4 Argon writes in a Bento
session from the reference photos and our measurements (decisions A1–A6). The
coordinator measures it against the battle frame, and the family script wears
it through `scripted_body` (the [Argon pilot](slices/argon-pilot.md)). The pass
also shipped the jets' paint sheen and ran the code-versus-generator
experiments.

This is the first of several passes, one per group of units. The queue for the
rest is at the end of this file.

## Next Agent Prompt

**Status (2026-11-12):**
- The shared slices are merged: 00–12 and 15; 13 was cancelled (G1) and 14 was
  not needed.
- The Argon pilot is merged. The T-72B3 and Abrams SEP v2 wear Argon bodies,
  and the T-72's front corner was fixed after the user's note (edit 14).
- The slices 17–29 now follow the Argon recipe ([family
  template](slices/family-template.md)), in four parallel lanes A–D
  ([`parallel.md`](parallel.md)).

**Pickup:** slice 16 (coordinator), alongside the four lanes. Merge each lane's
vehicles as they land, bake, and push a milestone every three families.

You are the **coordinator**. Before you touch anything, read:
- `AGENTS.md`;
- `development.md`;
- `packages/scene-assets/blender/README.md`;
- this README;
- [`parallel.md`](parallel.md);
- [`slices/argon-pilot.md`](slices/argon-pilot.md);
- the newest sections of [`decisions.md`](decisions.md) (G, A, P, R).

The lanes are Opus subagents in their own worktrees. Brief each with:
- its slice files;
- the family template;
- the pilot's recipe;
- `parallel.md`.

Only you edit:
- the coordinator-owned files;
- `assets/catalog.json` and `fixtures/**`;
- the lint baseline;
- the runtime.

Only you bake and push.

Rules:
- **Argon:** at most two rounds per vehicle (A6). Check every session's model
  (A5).
- **Secrets:** never print or commit `FAL_KEY`.
- **Scratch:** scratch output goes in `throwaway/`, which is ignored. The
  repo's `scratch/` folder is not ignored.
- **Review checkpoints never block.** Open the shots with `preview-shots`,
  wait about 5 minutes, then decide on the evidence. Record the decision in
  `decisions.md`, close the shots and carry on.
- **The bar:** anything still below the bar when its slice ends is named to
  the user in your end-of-pass report.

**Before you end every pass,** update this section: the status, the date, the
exact pickup point, any blockers, and the ticks below.

Warnings:
- `bun run check` reads the baked runtime against the current frames
  (`web/tests/sceneAssets/unitFit.test.ts`). Never push art without the bake.
- Exports fail on any new lint issue. A vehicle on a scripted body must lint
  clean (P10), and its fixes go in its `build.py`.
- The art load grows about 3 MiB a vehicle (R5); the tripwire is 768 MiB.
  Read the bake's load line at every milestone.

TODO:

- [x] 00 lint truth ([slice](slices/00-lint-truth.md))
- [x] 01 baseline re-export ([slice](slices/01-baseline-reexport.md))
- [x] 02 split `vehicle_parts.py` ([slice](slices/02-split-vehicle-parts.md))
- [x] 03 cast-turret experiment ([slice](slices/03-cast-turret-experiment.md))
- [x] 04 seam experiment ([slice](slices/04-seam-experiment.md))
- [x] 05 jet paint sheen ([slice](slices/05-jet-sheen.md))
- [x] 06 jet guide experiment ([slice](slices/06-jet-guide-experiment.md))
- [x] 07 frame survey ([slice](slices/07-frame-survey.md))
- [x] 08 gun tube ([slice](slices/08-gun-tube.md))
- [x] 09 hull outline and side panels ([slice](slices/09-hull-outline-and-side-panels.md))
- [x] 10 track sag ([slice](slices/10-track-sag.md))
- [x] 11 plate edges ([slice](slices/11-plate-edges.md))
- [x] 12 closed hatches, no crew ([slice](slices/12-closed-hatches-no-crew.md))
- [x] ~~13 panel seams~~: cancelled by the generator pivot (G1) ([slice](slices/13-panel-seams.md))
- [x] ~~14 turret skin~~: not needed, the smoothed dome won (slice 03) ([slice](slices/14-turret-skin.md))
- [x] 15 frame fixes ([slice](slices/15-frame-fixes.md))
- [x] Argon pilot: T-72B3 and Abrams SEP v2 ([slice](slices/argon-pilot.md))
- [ ] 16 integration milestone ([slice](slices/16-integration-reexport.md)), then push
- [ ] Lane A: 17 T-72 family, 18 Armata, 19 ZTQ-15
- [ ] Lane B: 20 Challengers, 21 Abrams variants, M1E3 and M10, 22 Leopard, 23 Leclerc and KF51
- [ ] Lane C: 25 CV90 family, 26 Bradley, 27 Puma and Ajax, 28 BMP and BRM
- [ ] Lane D: 24 air defence and rockets, 29 howitzers
- [ ] 30 lineup and sweep ([slice](slices/30-lineup-and-sweep.md))
- [ ] 31 ship ([slice](slices/31-ship.md)), then push and promote

## Contracts this pass changes

Read these first. Each one crosses a component boundary.

- **A vehicle's body is a build script** (`assets/source/skins/<family>/argon-r2/build.py`).
  Its `build()` names parts by what moves them and paints with seven slots.
  `scripted_body.wear` fits it to the export contract (the pilot's
  "Contracts").
- **Battle frames** (`fixtures/units/roster/*.json`) changed once, in slice
  15, with named digest changes. Everything since leaves frames, digests and
  replays alone: fit is fixed in `build.py` (P9).
- **Blender module layout.** `vehicle_parts.py` split into eight modules by
  concept (slice 02); `scripted_body.py` joined them (the pilot).
- **The geometry lint** compares parts, walks every appearance, and fails
  when a build fails (slice 00).
- **`asset check`** runs the Blender-side Python tests (slice 00).
- **Aircraft paint.** Jets have their own lower roughness (slice 05).
- **Unchanged:** the renderer, the GLB node contract (`track_L/R`, `wheel_*`,
  `track_length_m`, `link_pitch_m`, mount nodes, `dressing_*`, `_LOD0..3`),
  bundle formats and the simulation.

## Goal and taste

The target is cinematic realism (Hollywood realism), not a museum piece. The
closest the player ever gets is the zoomed battle camera and cinematics, so
that is where every model is judged. The user's taste, from the planning walk:

- **Proportions and silhouette first.** Small fittings only where they read at
  that distance.
- **Clean factory paint.** One clean scheme per nation, as now. No weathering,
  mud or camouflage work.
- **Crisp, hard armour plates** with a narrow bevel that catches the light, and
  panel lines and weld seams painted in.
- **Track sag** between the return rollers.
- **Hatches closed, no crew** on any tracked model.
- **Colour grading is out of scope.** The user likes WARNO's grading, but that
  belongs to another spec.

**Done for a family** means two things:
1. It passes the critique bar: an unprimed `screenshot-critique` gives no score
   below 6 and an average of at least 7.
2. Every one of the user's lineup notes for that family visibly reads as fixed
   in a before/after shown beside its reference.

Each model gets at most three critique rounds. After that, whatever is still
below the bar is named to the user, and the work moves on.

## Scope

| Group | Models | Scripts |
|---|---|---|
| Tanks | Abrams SEP v2, SEP v2 Trophy, SEP v3 Trophy; M1E3; M10 Booker; Leopard 2A6, 2A7V, 2A8; Challenger 2 TES; Challenger 3; Leclerc XLR; KF51; CV90120; T-72B3; T-80BVM; T-90M; ZTZ-99A; T-14; ZTQ-15 | `abrams`, `m1e3`, `m10`, `leopard`, `challenger`, `challenger_3`, `leclerc`, `kf51`, `cv90120`, `t72`, `t80`, `t90`, `type99`, `t14` (+`armata`), `type15` |
| Heavy IFV | T-15, Puma | `t15` (+`armata`), `puma` |
| IFV and recon | Bradley M2A4, M3A3; CV9040C, CV90 Mk IV; BMP-2M, BMP-3; BRM-3K; Ajax | `bradley`, `cv90`, `bmp`, `brm`, `ajax` |
| SP howitzers | M109A7, PzH 2000, 2S19M2, PLZ-05 | `sp_howitzers` |
| Air defence and rockets | Gepard, Buk-M3; M270A2, MARS II | `air_defence`, `rocket_artillery` |

Some scripts also export wheeled models. Those siblings are re-exported and
checked, never reworked:
- `air_defence.py` also exports Pantsir, Skyranger 30 and NASAMS ×2.
- `rocket_artillery.py` also exports HIMARS ×2, BM-21 and Tornado-S.
- `s300.py:37` imports `rocket_artillery`, so it is re-exported to prove it is
  unchanged.

## Non-goals and firewalls

- **Other unit groups.** No wheeled armour, helicopters, trucks, infantry or
  drones beyond the siblings above.
- **Jets.** Jet work is limited to the paint sheen (05) and the F-35/Su-35
  guide experiment (06). Moving the jets off the generator is a follow-up spec.
- **Out of scope:**
  - raising the rocket launchers (they stay stowed on fixed mounts);
  - the T-15's 57 mm turret;
  - wreck trimming;
  - startup time;
  - `test_tank_wreck`.
- **No new lint checks.** Slice 00 fixes existing lint code so it does what its
  docstrings say. That is all. Closeups and critique cover lint's remaining
  blind spots: only tier 0 is checked, `dressing_*` is skipped, and wrecks and
  crew are never linted.
- **No per-model texture machinery this pass.** If the bake wins slice 04, it
  becomes a follow-up spec.
- **No backward compatibility and no migrations.** Fixtures, digests, baselines
  and parity records are re-recorded, never shimmed.
- **`vehicle_crew.py` and `Vehicle.head_out` stay.** The wheeled `centauro.py:191`
  still uses them. The wheeled pass decides their fate.

## One owner per concept

When the pass is done, the code should read as if designed this way from the
start, not with the pass bolted on. Slice 30 greps for the "never" column.

| Concept | Owner | Never |
|---|---|---|
| Part-level contracts: tiers, `place`, spinning nodes, tubes | `vehicle_parts.py` (the kernel, after 02) | redefined in another module |
| Wheels, track belt, sag | `running_gear.py` | a family-local belt or sag |
| Hull side outline and plan rings | `hull_shape.py` (`Outline`, `hull_plan`, `mirror_plan`, `on_side`) | a family's own `skirt_top`, `skirt_foot` or `follow_tiles` |
| Armour plates, skirts, fenders, seams, `PLATE_BEVEL` | `armour_parts.py` | a skirt or fender drawn with a raw `box` or `prism` in a tracked script |
| Roof fittings: hatches (closed only), cupolas, vision blocks, sensors | `roof_parts.py` | an open lid, or `head_out` in a tracked script |
| Gun barrels (core, sleeves, evacuator, muzzle) and machine-gun models | `vehicle_guns.py` (`gun_tube`, `browning_m2`, `kord`) | a barrel made of sections with no core |
| Hull fittings and stowage | `hull_fittings.py`, `stowage.py` | — |
| A vehicle's body | its `build.py` under `assets/source/skins/<family>/`, worn through `scripted_body.wear` | body geometry drawn in the family script beside a worn body |
| Frames | `fixtures/units/roster/*.json`, then the blessed `fixtures/catalog.json` | art moving a frame silently |
| Wreck footprint, `track_gauge`, appearance mounts | `assets/catalog.json` (coordinator only) | an edit from a lane |
| Edge wear | `chip=1.0` on every tracked `run(...)` | a tracked family on another value |
| Jet paint roughness | one jet roughness, keyed on the aircraft being a jet | keyed on a paint scheme (schemes are shared with helicopters) |
| Lint baseline and allowlist | coordinator | edits from a lane |
| Decisions | [`decisions.md`](decisions.md) (`merge=union`) | decisions left in chat |

## Slice graph

```
00–15 shared slices (merged) ─ Argon pilot (merged)
                                      │
   ┌──────────────────────┬───────────┴──────────┬───────────────────────┬──────────────────────┐
16 integration      lane A (17→19)          lane B (20→23)          lane C (25→28)          lane D (24, 29)
milestone, push     T-80, T-90, Type 99,    Challengers, Abrams     CV90s, Bradley,         Gepard, Buk,
(coordinator)       T-14, T-15, ZTQ-15      variants, M1E3, M10,    Puma, Ajax, BMP-2M,     M270/MARS II,
                                            Leopards, Leclerc, KF51 BMP-3/BRM-3K            four howitzers
   └──────────────────────┴───────────┬──────────┴───────────────────────┴──────────────────────┘
                     30 lineup and sweep ─ 31 ship (bake, pictures, full checks, push, promote)
```

Within a lane, every vehicle's round 1 starts at once (the sessions run
remotely), and integration runs one vehicle at a time. Lanes own disjoint
roster scripts; no lane deletes procedural code another family imports
(`parallel.md`). Every family slice follows
[`slices/family-template.md`](slices/family-template.md).

## Standing gates

Every visual slice inherits these.

1. **Look at the picture.** Render closeups through the asset CLI:
   - command: `bun run --cwd web asset -- blender <abs>/.agents/skills/make-3d-models/scripts/closeups.py -- <out> <glbs> --shot …`;
   - shots: the standard list in `assets/shots.txt` (written in slice 01);
   - both shaded and matcap.
2. **Compare.** Wherever a before shot or reference exists, run
   [`compare-screenshots`](../../.agents/skills/compare-screenshots/SKILL.md) on
   the candidate against it. Its telemetry and less-wrong verdict are recorded
   in the slice.
3. **Critique last.** The final check before a visual slice is accepted is an
   unprimed [`screenshot-critique`](../../.agents/skills/screenshot-critique/SKILL.md)
   of the slice's own crop.
   - For family slices, it scores silhouette, surface, fittings, materials and
     cohesion out of 10, and lists at most five fixes.
   - The scores go in the slice file.
4. **Show.** At review checkpoints, open the shots with
   [`preview-shots`](../../.agents/skills/preview-shots/SKILL.md). These
   checkpoints never block.

The proof ladder is the one in `development.md`. Use the narrowest check that
answers the question while you work. Run everything once, at slice 31.

## Review surfaces

- `assets/before/`: the planning lineup. It holds:
  - `thumbs/`: reference, front-quarter and side views for every model;
  - `closeups/`: front-quarter and side shots, shaded and matcap;
  - `cropped/`;
  - `refmap.json`: which reference photo each model was judged against, with
    its licence.
- `assets/references/jet-sheen-reference.png`: the user's screenshot for
  slice 05.
- `assets/after/<family>/`: before/after/reference shots for each family.
- `assets/experiments/<03|04|05|06>/`: candidates and verdict sheets.
- `assets/frames/<family>.json`, with `visualizations/frames.html`: the frame
  survey.
- `visualizations/map.html`: the planning map with its four quadrants, the
  decision ledger and the landmine cards. Its "Plan + prompt" tab is superseded
  by this README.
- `visualizations/lineup.html`: the cohesion sheet, built at slice 30.

## Risks and the cheapest check that kills each

| Risk | Cheapest early check | Where |
|---|---|---|
| A session runs another model than Argon | Read the first turn's `usageByModel` | every session (A5) |
| Argon's proportions drift from the frame | Measure round 1's GLB against the frame before round 2 | template |
| A lint or fit problem needs a third Argon round | Run the integration once after round 1 and send its findings with round 2 | template |
| Denser bodies overflow the art load | Raise the limit in slice 16; read the bake's load line at every milestone | 16 |
| Lanes collide on an imported family script | Lanes never edit `t72`, `armata` or another lane's script; slice 30 sweeps dead code | `parallel.md` |
| A model never reaches the bar | Three critique rounds, then name it to the user | template |

## Queue of later passes

Each later pass starts with an `explore-unknowns` walk and its own spec.

1. **Wheeled armour:** Stryker, LAV, Boxer, VBCI, BTR, ZBL-08, Centauro,
   Jaguar, ACV, Fennek, VBL, Tigr, Humvee.
   - Confirm whether "no crew" carries over. If it does, delete
     `vehicle_crew.py` and `Vehicle.head_out` when Centauro drops them.
   - Clear the wheeled lint entries that slices 00 and 08 force into the
     baseline.
2. **Helicopters.**
3. **Wheeled artillery and air defence:** Caesar, HIMARS, Patriot, S-300/350,
   IRIS-T, Pantsir, Skyranger, NASAMS.
4. **Trucks:** HEMTT, MAN HX, Ural.
5. **Infantry.**
6. **Drones.**
7. **Jets off the generator.** Slice 06 found code-smoothed guides beat the
   generated skins on both airframes (surface 6 against 4 and 3). The code's
   remaining faults are faceting and fittings. Gloss also shows the skins'
   ripples (slice 05).
8. **A per-model baked detail map,** only if slice 04 finds it clearly wins.
   It needs a second unique UV, bundle support and a shader change
   (`models_material.wgsl:35-43`).
