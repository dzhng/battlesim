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

Implemented on `codex/city-maps-startup`, integrated with main through
`4c6c8d1b`. The startup lane's five steps are complete; broader city-maps gates
remain with their owning lanes.

- C33 retains one prepared simulation world in the authority worker. The page
  imports public static query data; matched picking, height, clearance and
  learned-foliage answers remain exact. Cancellation and replacement release
  the abandoned worker. The real browser generated battle/replay, refusal and
  native prepared/direct regressions pass.
- C20 preserves the horizon and whole-building visibility contracts while
  indexing nearby eyes and angular candidates. The 16,000-box synthetic rebuild
  falls from 37.831 to 1.182 ms, passing the unchanged 2 ms gate. Village cost,
  exact flags/vectors and building-corner pixels pass. Full S4/G0 admission is
  outside this count-stress proof; see [frame cost](frame-cost.md).
- The camera lab resolves its saved catalogue map and offline physical plan.
  Catalogue/compiler checks pass. Eight trajectory clearance summaries and
  twelve frozen observer/placement frames match exactly. Live ride frames stop
  at slightly different animation positions. Fresh visual review found no
  courtyard regression; the collapse capture does not visibly frame the fallen
  building, so only its numerical idle-recovery proof is claimed.
- The new 80-tick combat pair matches native/Wasm digest, packed publication and
  decoded fog every tick and observes firing and impacts. The original movement
  fixture is unchanged. The user explicitly authorized the named native math
  correction: pinned portable combat functions remove cross-target rounding
  drift. Upstream gravity and publication changes are reflected in the native
  expectations, without changing the parity inputs.
- Independent review's hidden initialization refusal is fixed and covered by a
  real browser regression. Focused native/web checks, typecheck and the generated,
  fog, camera and ground scenes pass. No full `check`/`verify` or balance run is claimed:
  the overall city-maps implementation is still active. Ground rendering keeps
  blue/red learned marks separate across full snapshots, deltas and side switches.

Final main integration preserves the developer city-stress early/late factory
and the mechanics editor's development-server plugin. Stress preparation imports
its actual scenario map, including synthetic late wrecks, rather than exporting
the original generated map. That developer fixture retains its existing temporary
placement world; the normal menu encounter still shares one prepared world with
the battle. The measurements below remain the frozen `42b4dfb7` comparison,
not new performance claims for these later upstream additions. Integration also
repairs main's stale replay-storage-key reference through the existing storage
owner; viewer-import and blocked-storage regressions fail before their fixes and
pass afterward. A development import cannot reload until persistence succeeds.

### Startup measurement

The final startup comparison freezes both arms at main `42b4dfb7`, with the
startup implementation applied only to the candidate. An earlier frozen
`4efec25b` baseline remains the C20 oracle and historical query reference.
The generated scene measures the menu's Deploy-to-playable interval, including
renderer resources. Both arms use a fresh Chromium instance/context, local Vite
and the same seed and assets. Disk caches are uncontrolled; browser HTTP cache is
initially empty before the cold menu visit; the menu can preload Wasm and
metadata before Deploy. Warm repeats menu-to-Deploy in that context. Local assets
are available without a network download. Wall time is on a loaded machine. Chromium's own process IDs are sampled
at 500 ms: summed RSS can count shared pages twice, physical footprint is the
kernel's charged memory, and sampled retired instructions are a lower bound
because a process can exit between samples. These include browser and GPU process
cost; they do not isolate just the page's JS heap. The sampler stops at the
first interactive frame, rather than the broader scene's four-tick running proof.
The page's Wasm capacity is also recorded: it cannot shrink, so its value at that
frame bounds the query module's peak linear allocation, including temporaries.
`publicBytes` counts transferred arrays plus the UTF-8 query payload, not the
resident index or JavaScript object overhead; the kernel footprint covers those.

Stage endpoints are cumulative milliseconds from Deploy; Map and Encounter
are individual preparation timings. Encounter includes world construction in
the candidate. Each row is one sample, not a repeated benchmark estimate.

| Map (seed 1) | Arm/cache | Map ms | Encounter ms | Prepared ms | World ready ms | Renderer ms | Playable ms | Instructions G | Peak RSS MiB | Peak footprint MiB |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Mixed Small | before/cold | 276 | 508 | 1463 | 2611 | 3323 | 3355 | 91.274 | 2447.7 | 2921.0 |
| Mixed Small | before/warm | 247 | 420 | 1035 | 1947 | 2669 | 2692 | 79.056 | 2591.4 | 3066.3 |
| Mixed Small | after/cold | 386 | 694 | 3374 | 5001 | 6197 | 6318 | 88.734 | 2363.0 | 3029.1 |
| Mixed Small | after/warm | 462 | 867 | 1969 | 3368 | 4500 | 4537 | 74.468 | 2415.2 | 2993.1 |
| Metro Large | before/cold | 862 | 1291 | 2820 | 4512 | 5313 | 5360 | 165.158 | 3009.6 | 3372.8 |
| Metro Large | before/warm | 848 | 1152 | 2372 | 3909 | 4665 | 4701 | 149.432 | 3753.9 | 4064.1 |
| Metro Large | after/cold | 1026 | 1720 | 3893 | 5510 | 6379 | 6440 | 143.584 | 2876.9 | 3584.1 |
| Metro Large | after/warm | 952 | 1497 | 3074 | 4374 | 5219 | 5266 | 141.289 | 3105.5 | 3377.7 |

Both arms resolve the same map identities and counts: Mixed Small has 3,080
buildings, Metro Large 9,909. All cold/warm starts pass the one-minute gate.
Retired instructions decrease in all four comparisons (about 3–13%); loaded-
machine wall time increases. Charged memory varies: no general memory reduction
is claimed. This is admission for these menu presets, not a complete dense-city
or 20 km G0 envelope verdict.

| Map | Arm | Worker Wasm capacity MiB | Page Wasm capacity MiB | Public transfer MiB |
| --- | --- | --- | --- | --- |
| Mixed Small | before | 110.25 | 44.94 | not separately exported |
| Mixed Small | after | 174.94 | 55.19 | 16.82 |
| Metro Large | before | 290.44 | 125.06 | not separately exported |
| Metro Large | after | 403.56 | 110.69 | 33.74 |

The worker now retains its prepared world; its reported Wasm capacity is therefore
larger than the old disposable planner worker. The page capacity is smaller on
Metro Large but slightly larger on Mixed Small: exact query data and temporary
lossless import have a cost. Capacity is not resident memory. Cold and warm
capacities agree within each arm. Candidate public export costs are 116/144 ms
on Mixed Small and 200/181 ms on Metro Large (cold/warm).

Raw reports, stage memory samples and captures are in ignored startup evidence.
The kernel process-lifetime high-water sums are conservative bounds, rather
than the simultaneous charged-memory peaks shown above. Camera and refusal
Preview checkpoints were non-blocking; the corresponding critique limits are
recorded explicitly rather than broadening their claims.
