# Air-war flight

**Status:** slices 00–14 landed; the jet scenes are restaged on the jet field and are being browser-verified. Next: 15 (roster), 16 (model pass), 17 (skirmish AI), 18 (panels), 19 (closeout).
**Updated:** 2026-10-14.

Fixed-wing jets fly as jets: high, always moving, circling their goals, attacking in runs, dogfighting in organic turning fights, and arriving and leaving by their own edge. Every planned jet is enabled, with a reviewed model and a historical loadout. [The map](map.md) holds the decisions (D-numbers) and their reasons; this README and the slices are the build plan for them.

## Next Agent Prompt

You are implementing this spec with `implement-spec`. Opus subagents write the code and do all 3D modelling and visual work; Codex (`codex exec`, `gpt-6.1-sol`, high) only takes short, non-visual tasks. You orchestrate, review every diff, run the gates and commit. Read [the map](map.md) once, then the slice you are picking up. Load `tweak-mechanics` before any rule change, `write-tests` before any behaviour change, and `game-ui` and `renderer` before any presentation slice.

- **Pickup:** browser-verify the restaged jet scenes, then slice 15; slices 16, 17 and 18 can run beside it. Lanes 16 (model pass) and 18 (panel follow-ups) can start in parallel at any time.
- **Worktrees:** symlink `web/node_modules` to the main checkout's, run `bun run build:wasm`, and `git lfs pull --include="assets/runtime/**,assets/third-party/effects/**,assets/third-party/textures/**"` before running any scene.
- **Merging a worktree:** commit its work on its branch first, and check `git log` shows the merge brought commits in. Merging an uncommitted branch is an empty, silent no-op.
- **Clean up merged worktrees:** after a branch merges, `git worktree remove` it and delete the branch. Each worktree carries its own multi-gigabyte Cargo `target/`, and leaving them filled the disk once.
- **Before ending each pass,** update this section: status, pickup and the checklist.

### Checklist
- [x] 00 [jet field and test kit](slices/00-jet-field-and-kit.md)
- [x] 01 [air rules split, one height owner](slices/01-air-rules-split.md)
- [x] 02 [jet flight law](slices/02-jet-flight-law.md)
- [x] 03 [steering: circle and edge](slices/03-steer-orbit-edge.md)
- [x] 04 [distances in 3D, layers from the catalog](slices/04-distances-3d.md)
- [x] 05 [carrier velocity](slices/05-carrier-velocity.md)
- [x] 06 [missiles: speed, lifetime, fuse, aim](slices/06-missiles.md)
- [x] 07 [bomb release and new rows](slices/07-release-and-rows.md)
- [x] 08 [attack passes](slices/08-pass-planner.md)
- [x] 09 [nose-on gun and self-firing weapons](slices/09-gun-and-autofire.md)
- [x] 10 [dogfights](slices/10-dogfight.md)
- [x] 11 [orders and reactions](slices/11-air-orders.md)
- [x] 12 [edge transit in the sim](slices/12-edge-transit-sim.md)
- [x] 13 [approach and departure effect](slices/13-transit-effect.md)
- [x] 14 [jet attitude](slices/14-jet-attitude.md)
- [ ] 15 [roster: cleanup, loadouts, enable every jet](slices/15-roster.md)
- [ ] 16 [aircraft model pass](slices/16-model-pass.md) (parallel lane)
- [x] 17 [skirmish AI and jets](slices/17-skirmish-ai.md)
- [x] 18 [panel follow-ups](slices/18-panel-follow-ups.md) (parallel lane)
- [ ] 19 [closeout](slices/19-closeout.md)

## User decisions made during the build

| Question | Answer (user, 2026-10-14) |
|---|---|
| **Q13:** Bought helicopters share today's four-sides spawn. Should they arrive the way jets will? | **No.** Helicopters keep their current arrival: the four-sides spawn, the off-map wait and `traversing`. Only jets change (slice 12). |
| **Q14:** Should the skirmish AI buy jets and answer enemy jets? | **Yes:** it buys high-air-capable units when it sees enemy `HighAir`, and jets join its reinforcement rotation under attack-move (slice 17). |
| **AA is standard (follow-up):** | Every standard opening includes at least one AA unit that can reach jets. Jets are tuned so that **a side without AA takes heavy damage before it recovers**: that is the balance target for slice 19, and the AI's opening models the habit (slice 17). |

## Slice graph

```
00 ─► 01 ─► 02 ─► 03 ─┬─► 08 ─► 11 ─► 17
            │         ├─► 10 ──┘
            │         ├─► 12 ─► 13
            │         └─► 14
            ├─► 04 ─────────┐
            └─► 05 ─► 06 ─► 07 ─► 08
                     09 (after 03, 05) ─► 10, 11
15 (after 07; its enable pass after 16's family is accepted)
16, 18: parallel lanes from day one
19: after everything
```

Slices that share a seam stay on one lane. These pairs can run concurrently:
- 02 with 04;
- 05 with 03;
- 06 with 12;
- 10 with 08 once 09 has landed;
- 13 and 14 with anything after 03.

## Single-owner invariants

Every slice inherits these:
- **High air is an elevation layer.** It is the one layer that bends physics (altitude lowered from about 1,000 m to about 250 m for play), so its special treatments (the shadow beneath, cards drawn on top, ...) key on the HighAir layer, never on "jet" or a unit type.
- **`AirRules::cruise_agl_m(layer)`** is the only answer to "how high". The sim, the ghost and native capture all read it, and no literal heights remain anywhere. A helicopter's shadow is cast along the sun from its actual height; a jet at altitude casts none, and instead shows a soft silhouette shadow on the ground directly beneath it (choices.md).
- **`crates/sim/src/air/`** owns fixed-wing behaviour:
  - `jet.rs` is the only flight law; nothing else writes a jet's position, heading or speed.
  - `movement/air.rs` handles helicopters only.
  - Orders reach a jet's flight only through `air/orders.rs` → `air/steer.rs`. Weapons never turn a jet.
- **`air/pass.rs`** owns pass phases and pass-weapon choice. **`flight/release.rs`** owns where an unpowered round lands.
- **`air/dogfight.rs`** is the only place that writes `dogfight_target`. The break distance lives in the rules.
- **`air/edge.rs`** owns back edges, entry and exit. The sim never holds a position off the map: off-map flight exists only as the published `AirTransit` record and the presentation effect drawn from it, for its own side only.
- **Distance between two bodies is 3D,** through one helper. The layer, from the catalog or `interception`, chooses the rule row. Height never chooses a layer.
- **A round's starting velocity is set in one place,** at launch.
- **One row per airframe.** No generic or duplicate rows, and no fixed-wing entries in the model manifest.
- **Every mechanism this spec replaces is deleted in the slice that replaces it:** the weave, the jets' `traversing`, snap and four-sides spawn, the entry-centre retirement, the crash clamp, the strafe ban, bomb-first, the height thresholds and `air_bomb_guided`. No compatibility layer. Fixtures and digests are re-recorded.

## Verification rules

- **Write the test first** (`write-tests`). A red behavioural sim test pins every claim before its lab scene is rewritten, and each scene's header names that test. The last plan shipped "circle at a location order" with only a scene behind it.
- **Tests pin mechanisms, never balance outcomes.** Who escapes, who wins and the prices are tuned by eye and in sample runs.
- **Every numeric value is a starting guess** that lives in data: rules or rows, never literals in code. The exception is the **hard 100 m floor** (D20).
- **Run the narrowest check** per pass: the slice's own tests and its one or two scenes. The full test and scene run, and the balance report, run once, in 19.
- **Visual slices name one visual variable and its crop.** Each one:
  - uses [compare-screenshots](../../.agents/skills/compare-screenshots/SKILL.md) whenever there is a target (a before shot, a reference, or the behaviour mock `throwaway/air-war-flight/flight-mock.html`);
  - runs [screenshot-critique](../../.agents/skills/screenshot-critique/SKILL.md) unprimed as the last check;
  - opens the shots with [preview-shots](../../.agents/skills/preview-shots/SKILL.md) as a **non-blocking** checkpoint: about 5 minutes for the user to respond, then decide on the evidence, record the decision and continue.
- **Codex passes run sandboxed and never see a browser.** The orchestrator browser-verifies every visual slice on the production route, and that includes a pixel diff against the shot from before the change.

## Panel follow-ups

These are owned by slice 18. The reference shot is `web/scenes/baselines/panels/panels-aircraft-states.png`.
- **Radar dish icon.** At the panels' real size it is a small blob that only reads as a dish when zoomed. The UNKNOWN, RADAR CONTACT and BEARING rows all share it, and in red on the UNKNOWN row it loses most of its detail.
- **UNKNOWN row alignment.** It starts a few pixels left of the rows below it.
- **Dogfight icon.** Two chevrons meeting nose to nose can pass for a bow tie.

## Pointers

- [The map](map.md): the settled ground, decisions D1–D36, loadouts L1, context and landmines.
- [Choices](choices.md): decisions made where the spec was silent, appended each pass.
- The closed parent spec, [air war](../done/air-war/README.md).
- The behaviour mock: `throwaway/air-war-flight/flight-mock.html`. It is scratch; copy it into `visualizations/` if a slice uses it as a target.
