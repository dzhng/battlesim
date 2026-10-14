# Air-war flight

**Status:** slices 00, 01, 02 and 05 landed; 03 next.
**Updated:** 2026-10-14.

Fixed-wing jets fly as jets: high, always moving, circling their goals, attacking in runs, dogfighting in organic turning fights, and arriving and leaving by their own edge. Every planned jet is enabled, with a reviewed model and a historical loadout. [The map](map.md) holds the decisions (D-numbers) and their reasons; this README and the slices are the build plan for them.

## Next Agent Prompt

You are implementing this spec with `implement-spec-with-codex`. Codex writes the code (`codex exec`, model `gpt-6.1-sol`, `model_reasoning_effort=high`), while you orchestrate, review every diff, run the gates and commit. Read [the map](map.md) once, then the slice you are picking up. Load `tweak-mechanics` before any rule change, `write-tests` before any behaviour change, and `game-ui` and `renderer` before any presentation slice.

- **Pickup:** slice 03 next; slice 04 is in review in `../awf-s04`. Lanes 16 (model pass) and 18 (panel follow-ups) can start in parallel at any time.
- **Open user questions:** Q13 and Q14, with provisional answers below. They only block slices 12 and 17. If the user hasn't answered when you reach either slice, adopt the provisional answer, record it in [choices.md](choices.md), and keep going.
- **Worktrees:** symlink `web/node_modules` to the main checkout's, run `bun run build:wasm`, and `git lfs pull --include="assets/runtime/**,assets/third-party/effects/**,assets/third-party/textures/**"` before running any scene.
- **Before ending each pass,** update this section: status, pickup and the checklist.

### Checklist
- [x] 00 [jet field and test kit](slices/00-jet-field-and-kit.md)
- [x] 01 [air rules split, one height owner](slices/01-air-rules-split.md)
- [x] 02 [jet flight law](slices/02-jet-flight-law.md)
- [ ] 03 [steering: circle and edge](slices/03-steer-orbit-edge.md)
- [ ] 04 [distances in 3D, layers from the catalog](slices/04-distances-3d.md)
- [x] 05 [carrier velocity](slices/05-carrier-velocity.md)
- [ ] 06 [missiles: speed, lifetime, fuse, aim](slices/06-missiles.md)
- [ ] 07 [bomb release and new rows](slices/07-release-and-rows.md)
- [ ] 08 [attack passes](slices/08-pass-planner.md)
- [ ] 09 [nose-on gun and self-firing weapons](slices/09-gun-and-autofire.md)
- [ ] 10 [dogfights](slices/10-dogfight.md)
- [ ] 11 [orders and reactions](slices/11-air-orders.md)
- [ ] 12 [edge transit in the sim](slices/12-edge-transit-sim.md) (blocked on Q13, provisional answer A)
- [ ] 13 [approach and departure effect](slices/13-transit-effect.md)
- [ ] 14 [jet attitude](slices/14-jet-attitude.md)
- [ ] 15 [roster: cleanup, loadouts, enable every jet](slices/15-roster.md)
- [ ] 16 [aircraft model pass](slices/16-model-pass.md) (parallel lane)
- [ ] 17 [skirmish AI and jets](slices/17-skirmish-ai.md) (blocked on Q14, provisional answer A)
- [ ] 18 [panel follow-ups](slices/18-panel-follow-ups.md) (parallel lane)
- [ ] 19 [closeout](slices/19-closeout.md)

## Open user questions

| Question | Provisional answer, adopted if unanswered when its slice starts |
|---|---|
| **Q13:** Bought helicopters share today's four-sides spawn, which D35 deletes for jets. How do they arrive? | **A:** the same back-edge arrival as jets, at their own cruise height, with the same approach effect. One arrival rule for every aircraft. |
| **Q14:** The skirmish AI (`crates/sim/src/skirmish_ai/mod.rs`) buys helicopters and answers low-air threats, but ignores jets. | **A:** it buys high-air-capable units when it sees enemy `HighAir`, and jets join its reinforcement rotation under attack-move. |

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
- **`AirRules::cruise_agl_m(layer)`** is the only answer to "how high". The sim, the ghost and native capture all read it, and no literal heights remain anywhere. A shadow falls along the sun onto the ground, from the aircraft's actual height above that ground.
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
- **Every mechanism this spec replaces is deleted in the slice that replaces it:** the weave, `traversing`, the snap, the four-sides spawn, the entry-centre retirement, the crash clamp, the strafe ban, bomb-first, the height thresholds and `air_bomb_guided`. No compatibility layer. Fixtures and digests are re-recorded.

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
