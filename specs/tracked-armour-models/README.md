# Tracked armour models

Rebuild every tracked armoured unit's 3D model so it reads as its real vehicle
at the zoomed battle camera and in cinematics. That means 37 appearances from 25
export scripts, plus the shared `armata.py` library.

The order of priority is silhouette and proportions first, then a crisp, clean
factory finish. All of it is built procedurally with
[`make-3d-models`](../../.agents/skills/make-3d-models/SKILL.md). The pass also
ships the jets' paint sheen and runs three code-versus-generator experiments.

This is the first of several passes, one per group of units. The queue for the
rest is at the end of this file.

## Next Agent Prompt

**Status (2026-11-11):** planned. Nothing is implemented.

You are the **coordinator**. Start at
[`slices/00-lint-truth.md`](slices/00-lint-truth.md) on a branch off `staging`.

Before you touch anything, read:
- `AGENTS.md`;
- `development.md`;
- `packages/scene-assets/blender/README.md`;
- the `make-3d-models` skill;
- this README;
- [`parallel.md`](parallel.md);
- [`decisions.md`](decisions.md).

Work the slices in the order of the graph below. Run the lanes it marks
parallel as Opus subagents in their own worktrees, briefed with their slice
file, `parallel.md` and the family template. Only you edit the
coordinator-owned files, write the lint baseline, edit `assets/catalog.json`
or `fixtures/**`, bake, push, or edit this README.

Never print or commit `FAL_KEY`. Scratch output goes in `throwaway/`, which is
ignored. The repo's `scratch/` folder is not ignored, so don't write there.

There are no approval stops. Each review checkpoint works the same way: open
the shots with `preview-shots`, wait about 5 minutes, then decide on the
evidence, record the decision in `decisions.md`, close the shots and carry on.
Anything still below the bar when its slice ends is named to the user in your
end-of-pass report.

**Before you end every pass,** update this section: the status, the date, the
exact pickup point, any blockers, and the ticks below.

Warnings:
- `bun run check` reads the baked runtime against the current frames
  (`web/tests/sceneAssets/unitFit.test.ts:91-110`). A frame change is red until
  it is re-exported and baked, so slices 15 and 16 land and push together.
- Between slices 08 and 16, shared-helper changes land in source while only
  representative models are re-exported, so the committed GLBs lag the
  sources. Nothing pushes in that window. Slice 16 closes it by re-exporting
  everything.
- Exports fail on any new lint issue (`vehicle_export.py:56-67`). Slice 00
  makes the lint actually compare parts, and that will surface old issues.
  Slice 01 baselines them.

TODO:

- [ ] 00 lint truth ([slice](slices/00-lint-truth.md))
- [ ] 01 baseline re-export ([slice](slices/01-baseline-reexport.md)), then push
- [ ] 02 split `vehicle_parts.py` ([slice](slices/02-split-vehicle-parts.md)), then push
- [ ] 03 cast-turret experiment ([slice](slices/03-cast-turret-experiment.md)), lane E1
- [ ] 04 seam experiment ([slice](slices/04-seam-experiment.md)), lane E2
- [ ] 05 jet paint sheen ([slice](slices/05-jet-sheen.md)), lane J, then push
- [ ] 06 jet guide experiment ([slice](slices/06-jet-guide-experiment.md)), lane J
- [ ] 07 frame survey ([slice](slices/07-frame-survey.md)), lanes F-A/F-B/F-C
- [ ] 08 gun tube ([slice](slices/08-gun-tube.md))
- [ ] 09 hull outline and side panels ([slice](slices/09-hull-outline-and-side-panels.md))
- [ ] 10 track sag ([slice](slices/10-track-sag.md))
- [ ] 11 plate edges ([slice](slices/11-plate-edges.md))
- [ ] 12 closed hatches, no crew ([slice](slices/12-closed-hatches-no-crew.md))
- [ ] 13 panel seams ([slice](slices/13-panel-seams.md)), after 04
- [ ] 14 turret skin, only if 03 picks the skin ([slice](slices/14-turret-skin.md))
- [ ] 15 frame fixes ([slice](slices/15-frame-fixes.md)), after 07
- [ ] 16 integration re-export and bake ([slice](slices/16-integration-reexport.md)), then push
- [ ] Lane A: 17 T-72 family, 18 Armata, 19 ZTQ-15, 20 Challengers
- [ ] Lane B: 21 Abrams and M10, 22 Leopard, 23 Leclerc and KF51, 24 air defence and rockets
- [ ] Lane C: 25 CV90 family, 26 Bradley, 27 Puma and Ajax, 28 BMP and BRM, 29 howitzers
- [ ] 30 lineup and sweep ([slice](slices/30-lineup-and-sweep.md))
- [ ] 31 ship ([slice](slices/31-ship.md)), then push and promote

## Contracts this pass changes

Read these first. Each one crosses a component boundary.

- **Battle frames** (the hull box, eye height, mount pivots and muzzles in
  `fixtures/units/roster/*.json`). Frames that measure wrong are fixed in one
  chain (slice 15), and so is the Ajax's new roof machine gun mount
  `on: autocannon`. These changes alter battle digests on purpose, and each is
  named. Every other slice leaves frames, digests and replays alone.
- **Blender module layout.** `vehicle_parts.py` splits into eight modules by
  concept (slice 02). Callers import the owner directly; there is no facade.
- **The geometry lint's contract.** Slice 00 makes its overlap check compare
  parts at all. Today it never does on a real vehicle
  (`vehicle_lint.py:243-259`: every mesh resolves to the root, so every pair
  is skipped, `:400-401`). It also makes it walk every appearance a script
  exports, and fail when a build fails.
- **`asset check`** runs the five Blender-side Python tests (slice 00). It is
  not part of `bun run check`.
- **Aircraft paint.** Jets gain their own lower roughness, keyed on the
  aircraft being a jet (slice 05). Helicopters are unchanged.
- **Nothing else.** The renderer, the GLB node contract (`track_L/R`,
  `wheel_*`, `track_length_m`, `link_pitch_m`), bundle formats and the
  simulation are unchanged.

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
| Frames | `fixtures/units/roster/*.json`, then the blessed `fixtures/catalog.json` | art moving a frame silently |
| Wreck footprint, `track_gauge`, appearance mounts | `assets/catalog.json` (coordinator only) | an edit from a lane |
| Edge wear | `chip=1.0` on every tracked `run(...)` | a tracked family on another value |
| Jet paint roughness | one jet roughness, keyed on the aircraft being a jet | keyed on a paint scheme (schemes are shared with helicopters) |
| Lint baseline and allowlist | coordinator | edits from a lane |
| Decisions | [`decisions.md`](decisions.md) (`merge=union`) | decisions left in chat |

## Slice graph

```
00 lint truth ─ 01 baseline re-export ─ 02 split vehicle_parts      (coordinator, serial; push after 01 and 02)
                                          │
        ┌─────────────────────────────────┼──────────────────────────────────────────┐
  lanes from 02 (parallel)                │ coordinator, alongside the lanes          │
  03 cast turret (E1) ──────────────┐     │ 08 gun tube                               │
  04 seams (E2) ─────────────────┐  │     │ 09 hull outline + side panels             │
  05 jet sheen → 06 jet guides (J)  │     │ 10 track sag                              │
  07 frame survey (F-A, F-B, F-C) ─┼──┼──┐  │ 11 plate edges                            │
                                 │  │  │  │ 12 closed hatches, no crew                │
                                 └──┼──┼──┤ 13 panel seams      (needs 04)            │
                                    └──┼──┤ 14 turret skin      (only if 03 picks it) │
                                       └──┤ 15 frame fixes      (needs 07)            │
                                          │ 16 integration re-export + bake, push     │
                                          └─────────────┬─────────────────────────────┘
                     ┌──────────────────────────────────┼────────────────────────────────┐
           lane A (17→20)                         lane B (21→24)                    lane C (25→29)
     T-72 family, Armata, ZTQ-15,        Abrams+M10, Leopard, Leclerc+KF51,    CV90 family, Bradley, Puma+Ajax,
     Challengers                         air defence + rockets                 BMP+BRM, howitzers
                     └──────────────────────────────────┼────────────────────────────────┘
                                   30 lineup and sweep ─ 31 ship (bake, pictures, balance, full checks, push, promote)
```

The lanes follow import clusters, so each lane's files are disjoint from the
others':
- `t80`, `t90` and `type99` import `t72`;
- `t14` and `t15` import `armata`;
- `cv90120` imports `cv90` (`cv90120.py:29`);
- `m1e3` imports `abrams`.

Within each lane, the order follows the planned family order. Ownership,
merging and baking rules are in [`parallel.md`](parallel.md). Every family
slice follows [`slices/family-template.md`](slices/family-template.md).

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
| Exports aren't byte-repeatable, so identity proofs mean nothing | Export the T-72B3 twice and compare hashes | 00 |
| Lint grouping fix floods the baseline | Lint the T-72B3 alone first; apply the 30-issue rule | 00 |
| The split changes geometry | Roster lint output identical, and T-72B3+wreck, Boxer and Apache GLBs byte-identical | 02 |
| Turret skin needs aircraft fitting machinery | Fit by hand, and check bounds and pivot before judging looks; slice 14 is conditional | 03 |
| Jet roughness is lost in the bake | Bake one jet in the lane worktree and read its material roughness | 05 |
| Lineup notes are really frame problems | The survey classifies every note before any geometry work | 07 |
| Sag cuts into a wheel, which the lint can't see | Clearance clamp built into `track_path`, plus a low side crop of the T-72B3 | 10 |
| Non-convex hulls break `Outline` | Build an `Outline` for every tracked hull in a probe | 09 |
| Bevels exceed the triangle caps | Re-export the T-80BVM (largest: 72,845 of 120k) first | 11 |
| A Leopard frame fix pushes its sight past the dressing allowance | Measure the sight top against the new box before blessing | 15 |
| `bun run check` is red after the frame chain | 15 and 16 land and push together, with a bake | 15–16 |
| Ratchet blocks family growth (`abrams.py` 29 KB, `air_defence.py` 26 KB) | Split the family file by concept as the slice's first step | 21, 24 |
| A shared amendment lands after a family merged | Receipt freshness probe | 30 |
| Lanes collide | Lanes follow import clusters, plus a path-ownership diff at each merge | `parallel.md` |
| A model never reaches the bar | Three-round cap, then name it to the user | template |

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
7. **Jets off the generator,** only if slice 06 finds code ties or beats the
   skins.
8. **A per-model baked detail map,** only if slice 04 finds it clearly wins.
   It needs a second unique UV, bundle support and a shader change
   (`models_material.wgsl:35-43`).
