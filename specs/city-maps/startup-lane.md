# Startup lane: one prepared world, and a browser that holds a full map

A separate session works this lane in parallel with the others. It is engineering with numbers: time and memory from pressing Deploy to a playable battle, on the largest map. Nothing here is judged by eye.

## The contract

One battle builds the simulation's world once. Today it is built three times from the same map: by the encounter planner (to ask where forces can stand), by the battle, and by the page's main thread (for picking, ground height and camera clearance). After this lane:

- the preparation worker builds the world once; the planner and the battle use that one;
- the page receives a compact read-only export of the public static geometry and the queries it needs (ground and building picks by ray, surface and height at a point, camera clearance), and constructs no simulation world of its own;
- a side still learns damage and destruction only through its own observations: the export is the static map, which is public;
- replacing or cancelling a pending battle releases everything the abandoned one held.

Measured on Metro Large on this machine, before and after: instructions and time from Deploy to first playable frame, and peak memory of the tab. [C33](slices/C33-battle-preparation.md) is the slice; the budgets are in [scale direction](scale-direction.md#startup-and-loading).

## Ownership

| This lane owns | Others own (stay out) |
|---|---|
| `web/src/battle/prepare/`, the lab's world hooks (`apps/battle-lab/src/useStaticWorld.ts` and what reads it) | The menu and routes' look; `web/src/maps/` adapters except what preparation needs |
| `crates/game-wasm/`, `crates/sim/src/world/export.rs` | The rest of `crates/sim/src/` (the [scale lane](scale-lane.md) is changing tick cost there): a constructor seam in `battle.rs` or `encounter/` is a small named commit |
| `packages/battle-renderer/src/frame/fogVisibility.ts` for step 3 | The rest of the renderer (the [buildings](buildings-lane.md) and [ground](ground-lane.md) lanes) |
| This file's Status section | `README.md`'s Next Agent Prompt and TODO; `crates/mapgen/` |

No repo-wide renames.

## Work, in order

1. **Measure.** Where the time and memory go from Deploy to playable, per stage, on Mixed Small and Metro Large (`STARTUP_MAP=type:size:seed` on the `generated` scene prints the stages). Record the table here before changing anything.
2. **[C33 public preparation](slices/C33-battle-preparation.md):** one world, as in the contract above. Picking and clearance must give the same answers as before: matched rays before and after.
3. **[C20 renderer fog at scale](slices/C20-renderer-fog-at-scale.md):** fog cost follows the occluders within an eye's reach, not every known occluder on a 16,000-building map.
4. **The camera lab's map through the catalogue:** it is the last route that compiles its own map beside the resolver ([C58 outcome](slices/C58-offline-encounter.md#outcome), "not done").
5. **A native-against-Wasm pair with combat.** The only such check of the simulation itself is a four-unit move with no firing (`fixtures/parity/publication/stream.json`); a short battle with shooting is a stronger pair at the same cost.

## How to work

Read [`AGENTS.md`](../../AGENTS.md): narrow checks only, no full gate. Battle digests and replays must not move; picking, height and clearance answers must not move. Measure cost in instructions retired where the tool gives them, and say when a number is wall time on a loaded machine. Branch from main, merge main often, push small green passes. Add an Outcome to each slice file and decisions to [`choices.md`](choices.md) under a heading for the slice. Scratch output goes in gitignored `throwaway/`.

**One GPU, shared.** Several sessions are working at once. Run every scene through the GPU lock in the main checkout (the README's Checks section has the command), never two at a time, and keep heavy jobs to one at a time.

## Status

In progress on `codex/city-maps-startup`, from main `4efec25b`.

- C33: the preparation worker will become the battle authority. Reusing the
  prepared world matches direct initialization through 180 combat ticks and
  replay in the native regression. The page receives lossless public query data
  and typed exports; only the geometry/flight probes retain main-thread WorldView.
- C20: the per-eye occluder index and local invalidation are committed; CPU
  contracts pass. Hardware oracle, paired build cost and visual proof are queued.
- Camera catalogue: the saved map resolves exactly to its prior compiled geometry;
  nine catalogue/camera tests and web typecheck pass. Trajectories keep their original
  framing. The offline plan lives with the map instead of the runtime script.
- Combat pair: a separate 80-tick stationary fight observes real firing and impacts,
  matching native/Wasm digests, packed publications and decoded fog every tick. The
  original movement pair is unchanged. The user authorized a named native math
  exception: pinned combat angle/spread functions remove system-libm rounding drift.
  Corrected native output matches both original and rebuilt Wasm for this sample.
- Next: integrate latest main, complete startup and fog hardware measurements,
  review the lane's final diff and push green checkpoints. No full `check`/`verify`
  at lane closeout.

### Startup measurement

Baseline inputs are frozen in a detached checkout at `4efec25b`, before C33.
The generated scene measures the menu's Deploy-to-playable interval, including
renderer resources. Both arms use a fresh Chromium instance/context, local Vite
and the same seed and assets. Disk caches are uncontrolled; browser HTTP cache is
cold. Wall time is on a loaded machine. Chromium's own process IDs are sampled
at 500 ms: summed RSS can count shared pages twice, physical footprint is the
kernel's charged memory, and sampled retired instructions are a lower bound
because a process can exit between samples. These include browser and GPU process
cost; they do not isolate just the page's JS heap.

| Map (seed 1) | Arm | Map ms | Encounter ms | World ready ms | Renderer ms | Playable ms | Instructions G | Peak RSS MiB | Peak footprint MiB |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Mixed Small | before | queued | | | | | | | |
| Metro Large | before | queued | | | | | | | |
| Mixed Small | after | pending | | | | | | | |
| Metro Large | after | pending | | | | | | | |
