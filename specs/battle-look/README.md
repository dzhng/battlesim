# Battle look: a playable summer village in the WARNO style

Make the village battle look like WARNO in summer:
- painterly patchwork fields;
- dense 3D grass;
- a warm afternoon grade.

Take framing and battle scars from Defilade, effects from Broken Arrow and WARNO, and sharp urban fog of war from ARMAPHRACT. Units are real, animated, modern models. It is driven by new simulation rules: directional vision, an animation feed, ricochets and an authoritative ground layer.

**Done** means friends can play the full `/battle/village` encounter on this Mac, in the new look, at no less than 30 FPS at the default camera. The encounter has both armies and all five unit kinds.

**Status: 00–05, 09–12 done. Updated 2026-09-25.**

## Next Agent Prompt

You are implementing this spec in `/Users/david/dev/battlegame`. **Next pickup:** in flight: 06 (ricochets), 07 (ground rules), 13 (light). Next when they land: 08 after 07; 28 (adopt `math`), then 14, 16 and 18–19, after 13; 20 after 11 and 28. Frame cost is the benchmark's short run (`bun run --cwd web scene -- benchmark`). Open user calls, running provisionally: the village capture target at 6/10, and idle turret bearing (`choices.md`, orchestrator after slice 04); the fog look questions (slice 15).

1. Read these first:
   - [`unknowns-map.html`](unknowns-map.html): the settled interview and the landmine cards. Every decision in it is a given.
   - [`decisions.md`](decisions.md): what planning decided on top of the map.
   - [`research.md`](research.md): the external sources.
   - [`AGENTS.md`](../../AGENTS.md): the worktree recipe and the test ladder.
   - The slice file you are on.
2. Work the graph below in lanes. The simulation lane (04–08), the controls lane (09–10) and the asset lane (03, 11, 20–22) run in parallel with the renderer lane. No visual slice consumes a rule before that rule lands.
3. Spikes 01–03 are throwaway. They write a verdict to `spikes/<NN>.md` with measured numbers and never merge code. The slices that consume a spike rewrite the code properly, and do not start until the verdict exists.
4. Every slice follows the standing gates below.
5. Never modify `~/dev/game`. Copy from it only through [`assets/reuse-manifest.json`](assets/reuse-manifest.json), which slice 00 creates.
6. Put every provisional number in the fixture: rules in `fixtures/village.json` rule blocks, presentation in its `presentation` block, and the biome in `fixtures/biomes/summer.json`. Log tuning in `decisions.md`.
7. Before you end a pass, update this section:
   - status and date;
   - the exact next pickup;
   - the TODO checks;
   - blockers;
   - and any reslice.

   Put decisions you made where a slice was silent in [`choices.md`](choices.md).

### Global TODO

**Setup and spikes**
- [x] [00 Setup, baselines, manifest](slices/00-setup.md)
- [x] [01 Spike: renderer port feasibility](slices/01-spike-renderer-port.md): go, see [`spikes/01.md`](spikes/01.md)
- [x] [02 Spike: sight-fog technique](slices/02-spike-sight-fog.md): sight lights, see [`spikes/02.md`](spikes/02.md)
- [x] [03 Spike: rig, clips, tank articulation](slices/03-spike-rig-and-vehicle.md): the technique works, but the look criterion tripped (reads as toy-like), so the plan is resliced; see [`spikes/03.md`](spikes/03.md)

**Simulation lane**
- [x] [04 Sim: one sight shape](slices/04-sim-sight-shape.md)
- [x] [05 Sim: animation feed](slices/05-sim-animation-feed.md)
- [ ] [06 Sim: ricochets](slices/06-sim-ricochets.md)
- [ ] [07 Sim: ground layer rules](slices/07-sim-ground-rules.md)
- [ ] [08 Sim: ground delivery and resync](slices/08-sim-ground-delivery.md)

**Controls lane**
- [x] [09 Camera and keys](slices/09-camera-and-keys.md)
- [x] [10 Main menu and scripted benchmark](slices/10-menu-and-benchmark.md)

**Asset lane**
- [x] [11 Appearance bundle format, validator, CLI](slices/11-bundle-format.md)

**Renderer lane**
- [x] [12 Renderer frame and passes](slices/12-renderer-frame.md)
- [ ] [13 Light: sky, sun shadows, grade](slices/13-light.md)
- [ ] [14 Fog geometry: sight lights](slices/14-fog-geometry.md)
- [ ] [15 Fog look and contact ghosts](slices/15-fog-look.md)
- [ ] [16 Summer terrain material](slices/16-summer-terrain.md)
- [ ] [17 Scars on the ground](slices/17-scars.md)
- [ ] [18 Grass](slices/18-grass.md)
- [ ] [19 Trees and scenery](slices/19-trees.md)
- [ ] [28 Adopt `math` as the one TypeScript math owner](slices/28-adopt-math.md): after 13, before 20 and 23

**Models**
- [ ] [20 Model workbench](slices/20-model-workbench.md)
- [ ] [21 Infantry models and clips](slices/21-infantry-models.md)
- [ ] [22 Vehicle and building models](slices/22-vehicle-building-models.md)

**Battle**
- [ ] [23 Soldiers in battle](slices/23-soldiers-in-battle.md)
- [ ] [24 Vehicles, buildings, ruins, wrecks in battle](slices/24-vehicles-in-battle.md)
- [ ] [25 Combat effects](slices/25-combat-effects.md)
- [ ] [26 Smoke, fire, dust](slices/26-smoke-fire-dust.md)
- [ ] [27 Playable village](slices/27-playable-village.md)

## Slice graph

```
00 ─┬─ 01 spike: port ─────── 12 frame ─ 13 light ─┬─ 14 fog geometry ─ 15 fog look
    │                                              ├─ 16 terrain ─┬─ 17 scars ◄── 08
    │                                              │              ├─ 18 grass
    │                                              │              └─ 19 trees
    ├─ 02 spike: sight fog ──────────────► 14 (with 04)
    ├─ 03 spike: rig/tank ─┬─ 11 bundle format ─ 20 workbench (needs 12) ─┬─ 21 infantry
    │                      └──────────────────────────────────────────────┴─ 22 vehicles+buildings
    ├─ 04 sight ─ 05 feed ─┬─ 06 ricochets
    │                      └─ 07 ground rules ─ 08 ground delivery
    └─ 09 camera+keys ─ 10 menu+benchmark

23 soldiers in battle  ◄ 05, 13, 21
24 vehicles in battle  ◄ 05, 13, 22
25 combat effects      ◄ 05, 06, 13
26 smoke, fire, dust   ◄ 25
27 playable village    ◄ every slice
28 adopt math          ◄ 13 (before 20, 23)
```

**Parallel after 00:**
- the three spikes;
- the simulation lane;
- the controls lane.

06 and 07 run in parallel and merge in turn through the publication layout; the second to merge rebaselines. 16–19 and 14–15 run in parallel. So do 21 with 22, and 23 with 24. GPU measurements run one at a time on a quiet machine.

## Standing gates (every slice inherits these)

**Contracts**
- Replay and digest parity stay green.
- New authoritative state and knowledge enter `Battle::digest`.
- Presentation reads only the published observation, plus public static geometry.
- One owner per concept.
- Hard cutover: no compatibility layers. Obsolete paths are deleted in the same slice as their replacement.

**Frame cost**

Every slice records frame cost at 1920×1080 into [`frame-cost.md`](frame-cost.md):
- frame-time p50/p95/p99;
- GPU pass timings where available;
- buffer and texture bytes;
- publication bytes.

Before slice 10 lands, use slice 00's probe. From 10 onward, use the benchmark's short run. There is no budget until slice 27's 30 FPS floor; the performance budget itself is a later spec.

**Visual verification**

Every **visual** slice:
1. names its **one visual variable** and a reference crop from [`assets/reference/`](assets/reference/), and lists the wrongness that is out of scope;
2. runs [compare-screenshots](../../.agents/skills/compare-screenshots/SKILL.md) against that crop: telemetry plus a less-wrong verdict, not a pixel match;
3. runs an **unprimed [screenshot-critique](../../.agents/skills/screenshot-critique/SKILL.md) as the last check before acceptance**. From slice 14 on, the critique prompt must ask: *"Could any dark region be mistaken for sun shadow, or any shadow for fog?"*;
4. opens a non-blocking human checkpoint with [preview-shots](../../.agents/skills/preview-shots/SKILL.md). Allow about 5 minutes and keep working meanwhile. If there is no answer, decide on the evidence, record it, and close Preview.

**Tests and review**
- Pixel checks tuned to flat shading are retuned one by one, each recorded in `decisions.md`. Never loosen one silently.
- Red/green tests at the seam first, with the narrowest runner.
- `bun run check` and `bun run verify` once at the end of the pass.
- The review closeout.
- A focused commit.

## Single owners (the end state reads as designed today)

| Concept | Owner |
|---|---|
| Sight shape and range by direction | `sim::sight`. Read by sensing, the fog sweep, and published for renderer fog. |
| What a unit is doing for animation (member ids, weapon pose, shots, blasts, impact detail) | Published by `sim::publication`. Poses are derived only in `battle-renderer`'s pose driver. |
| Round flight and ricochet | `sim::flight`, which calls damage's `ImpactResolver`. Penetration and face policy live once in `damage`. |
| Ground layer: craters and cosmetic wear | Rules in `sim::ground`, per-side learned cells in knowledge, delivery in one patch protocol. |
| Camera and input | Camera behaviour in `renderer-core::CameraController`. Command keys in `web/src/battle/input` `CommandBindings`. |
| Frame, passes and GPU lifetimes | `battle-renderer` `BattleFrame` plus one resource registry. |
| Fog in pixels | One `FogTerm` in every world material. Overlays composite after post, in display space. |
| Appearance bundles (schema, validation, loading, baking) | `packages/scene-assets`. There is one loader, for the workbench and the battle alike. |
| Biome look | `fixtures/biomes/summer.json`, read by terrain, grass and trees. |
| TypeScript vector, matrix, quaternion, shape, culling, noise, random and easing math | The npm `math` package (pmndrs), used per [`.agents/skills/math`](../../.agents/skills/math/SKILL.md). Slice 28 migrates the hand-rolled originals. |
| Performance measurement | The scripted benchmark from slice 10: the menu entry, the scene runner and `frame-cost.md`. |

`~/dev/game` is a copy source only, recorded per file with its own commit pin. It is never a runtime import. `proxies.ts` and `unitProxies.ts` die in slices 23–24. Picking keeps the simulation's boxes.

## Short-lived seams (each has a removal slice)

- `FogTerm` reads the old 8 m bitset from slice 12 until slice 14 replaces its source. Nothing else may read the bitset for drawing after that.
- Box proxies (`proxies.ts`, `unitProxies.ts`) stay until slices 23 and 24 delete them. Picking keeps the simulation's boxes permanently: that is a contract, not a seam.
- Spike code (01–03) never merges.

## Firewalls (out of scope)

- **Company of Heroes-style soldier movement (a future spec).** Soldiers in a squad will each move on their own and find cover. This spec must not block it. Presentation reads each soldier's published position, velocity and id, never a formation slot. No renderer, pose driver or animation code may assume soldiers keep formation offsets from the squad centre, or share one facing or one gait. Per-soldier cover postures (kneel or prone behind cover) must be expressible by the same `PoseDriver` inputs.
- Winter and desert biomes: winter is the next biome spec, on the same data.
- The HUD redesign.
- The performance budget itself: a later spec. Only the slice 27 floor is in scope.
- Battle-foundation's 16b late-state scale work.
- Playing as red: the player stays blue and red stays the defender bot.
- Moving labs beyond what the shared renderer changes.
- Aircraft.
