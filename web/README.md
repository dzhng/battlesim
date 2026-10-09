# Browser application

The browser consumes the simulation's per-side observation. Presentation may
interpolate known evidence, but it cannot query hidden battle state. The
[application entry](src/main.tsx) mounts the [battle lab](../apps/battle-lab/README.md),
which owns routes and composes the battle view.

## Authority and preparation

[Preparation](src/battle/prepare/) admits a generation request (a battle never names a
saved map: those are tests' and the menu's), fields the request's two factions on the map's admitted skirmish sites and hands the
prepared world to its battle authority. Expensive preparation stays off the page thread. The page
builds a plain geometry world from the same map for drawing, picking and camera
clearance; it does not run a second battle.

[The simulation adapter](src/battle/sim/README.md) owns ordered commands, worker lifetime,
bounded publications and packed observation decoding. [Input](src/battle/input/)
produces commands, and [presentation](src/battle/present/) reads admitted observations.
[Map adapters](src/maps/) fetch saved documents or resolve a generation request
through Rust's one admission path. Their listing policy, saved-map identity and
categories are explained in [fixtures](../fixtures/README.md#saved-maps).

The [benchmark](src/battle/benchmark/README.md) measures the actual simulation and
renderer with a declared workload and camera tour. A route selection or successful
startup alone does not prove its performance budget.

## Build and serving

[The web manifest](package.json) owns browser dependencies, WebAssembly compilation,
build and test commands; [the root manifest](../package.json) composes prerequisites.
Use the root [startup instructions](../README.md#running-it) for a first checkout.
[WebAssembly build and memory ownership](../crates/game-wasm/README.md) explains
generated bindings and how publications cross the boundary.

[Vite configuration](vite.config.ts) resolves source-only packages and apps against
this application's dependencies, keeps caches local to the checkout and serves
prepared [runtime assets](../assets/README.md). Development plugins provide the
editors' local APIs and asset rebaking. Those APIs and editor routes are development
facilities; the production build does not provide fixture editing.

[Vercel configuration](../vercel.json) owns deployed builds, SPA rewrites, isolation
headers and immutable caching for content-addressed assets. Keep serving isolation
consistent with Vite so local, preview and deployed pages follow the same
cross-origin isolation policy. Runtime assets must be materialized from Git LFS before a build can
serve them; committed pointer files are not media.

## Checks and evidence

[Vitest configuration](vitest.config.ts) and [tests](tests/) own fast browser-side
checks, including Node and DOM tests. Build WebAssembly before checks that consume
it; native editor executables are built by the root task runner when needed.
A focused invocation is `bun run --cwd web test -- tests/observation.test.ts`.

[The scene runner](scene.mjs) owns browser verification. Its usage comment and
`bun run --cwd web scene -- --list` expose the current scenes; the
[fixture registry](../apps/battle-lab/src/fixtures.json) is their one inventory.
A fixture and its [scene](scenes/) have matching identities. Files prefixed with
an underscore are shared harness helpers, not separately registered scenes.
Run a selected fixture with `bun run --cwd web scene -- <fixture-id>`.

The runner starts its own server unless configured otherwise; production timing
fixtures require its production build. A development URL cannot silently stand in
for that performance gate. Each scene owns the observations that prove its contract;
a screenshot proves appearance only after visual review. Evidence goes into ignored
`throwaway/evidence/`, rather than becoming a second set of fixtures.

Interface appearance is a regression gate. A scene can match a capture against
an approved picture ([baselines](scenes/_baseline.mjs), stored in LFS beside the
scenes); a mismatch fails and leaves the capture and a diff in its evidence. The
scenes that have pictures are the visual suite, run together by the runner's
`--visual`; a worktree fetches the pictures from LFS before running it. A
picture is approved by a person who has looked at it, by blessing that scene's
captures and reviewing them in the diff, never to make a red check pass. Every
component the player sees is drawn by a page whose scene pins it, or is named
with why it cannot be ([coverage](tests/uiCoverage.test.ts)); the
[UI gallery](../apps/battle-lab/src/routes/ui.tsx) draws what the panel workbench
and cursor lab do not. Pictures are pinned only from fixed test content, since
the roster grows. They are approved on the macOS development machine's browser
and fonts; another platform rasterizes text differently, and the answer there
is not a looser threshold.
Beside the pictures, the [containment check](scenes/_spills.mjs) asks of every
panel the geometric question a picture answers only once approved: does it
hold its content?

Every rendering job shares the machine's GPU. Concurrent sessions serialize through
one lock in the main checkout, for example
`lockf -k <main checkout>/throwaway/gpu.lock bun run --cwd web scene -- <fixture-id>`.
This avoids contention corrupting frame measurements.

[The asset CLI](asset.mjs) is file IO around [scene-assets](../packages/scene-assets/README.md).
Its bare invocation prints available commands. Audio preparation has a separate
[audio tool](../packages/battle-audio/README.md#recording-preparation), because audio
provenance and crops belong to the sound catalog rather than the appearance catalog.
