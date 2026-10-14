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
bounded publications and handing them to the Rust publication reader. [Input](src/battle/input/)
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

`bun run setup` installs the web dependencies and Playwright's managed
Chromium, then checks the `ffmpeg` and `ffprobe` tools used by the audio asset
tests. If dependencies are already present but the browser executable is
missing, run `bun run --cwd web install-browsers`; check the audio tools with
`bun run --cwd web check-prerequisites`. On macOS install the audio tools with
`brew install ffmpeg`. On WSL install them with
`sudo apt-get update && sudo apt-get install -y ffmpeg`. The browser harness
reports its repair command instead of exposing a raw Playwright executable
error. WSL checks prefer headless Windows Chrome when `.wslconfig` has
`networkingMode=mirrored`; they fall back to managed Chromium when Windows
Chrome is unavailable. macOS uses managed Chromium.

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

Vercel Analytics is mounted by [`src/main.tsx`](src/main.tsx) at the application root
of a Vercel build (only Vercel serves its script), so deployed page views and client-side route changes use the
same single browser entry.

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
The runner prepares missing native fixture helpers automatically in the
worktree's Cargo `target/` directory; `throwaway/` is reserved for evidence
and other disposable output. Set `CARGO_TARGET_DIR` when a separate build
directory is needed.

The runner starts its own server unless configured otherwise; production timing
fixtures require its production build. A development URL cannot silently stand in
for that performance gate. Each scene owns the observations that prove its contract;
a screenshot proves appearance only after visual review. Evidence goes into ignored
`throwaway/evidence/`, rather than becoming a second set of fixtures.
The runner treats WebGPU validation, device-loss and adapter failures as fixture
failures at the first browser diagnostic, including scenes with their own readiness
loops. It keeps that first message and suppresses follow-on invalid-resource noise;
readiness timeouts include the page's published state so a slow or refused adapter
does not look like an unexplained hang.

Every check that draws gets its browser from [the browser choice](browser.mjs):
Playwright's Chromium, or inside WSL headless Windows Chrome, because WSL's own
Chromium has only a software GPU. Its header names the setting that keeps
Chromium.

Interface appearance is a regression gate. A scene can match a capture against
an approved picture ([baselines](scenes/_baseline.mjs), stored in LFS beside the
scenes); a mismatch fails and leaves the capture and a diff in its evidence.
The match forgives where a thing is drawn, a pixel or two off, and never what
is drawn: a new, missing or recoloured thing fails however small, so the
tolerance is a distance, never a share of the frame. The
scenes that have pictures are the visual suite, run together by the runner's
`--visual`; a worktree fetches the pictures from LFS before running it. A
picture is approved by a person who has looked at it, by blessing that scene's
captures and reviewing them in the diff, never to make a red check pass. Every
component the player sees is drawn by a page whose scene pins it, or is named
with why it cannot be ([coverage](tests/uiCoverage.test.ts)); the
[UI gallery](../apps/battle-lab/src/routes/ui.tsx) draws what the panel workbench
and cursor lab do not. Pictures are pinned only from fixed test content, since
the roster grows. macOS is the canonical visual-check machine: pictures are
approved, and the visual suite is judged, on the macOS development machine's
browser and fonts. Another platform rasterizes text and GPU output differently,
so a mismatch there is not a regression, and the answer is not a looser
threshold or a second set of pictures.
Beside the pictures, the [containment check](scenes/_spills.mjs) asks of every
panel the geometric question a picture answers only once approved: does it
hold its content?

The scene runner executes independent fixtures with bounded overlap (two at a
time by default) so a long browser scene does not serialize the whole gate.
Timing and parity fixtures are marked serial and still run alone because GPU
contention would invalidate their measurements. Set `SCENE_CONCURRENCY=1` for a
fully serialized run, or lower it on a constrained GPU. Separate rendering jobs
still share the machine's GPU; use the repository lock when running two jobs,
for example `lockf -k <main checkout>/throwaway/gpu.lock bun run --cwd web scene -- <fixture-id>`
(on Linux, `flock` in place of `lockf`).
Parallel runs use a 60 s lab-readiness allowance; serial runs keep the 30 s
fail-fast allowance. `SCENE_READINESS_TIMEOUT_MS` overrides either value when a
machine needs a different startup budget.
The web `verify` script selects the mechanics gate (`--verify`) and the test-only
low graphics profile for ordinary scenes by default. Extended renderer, audio
and generated-map scenes remain registered and run through `bun run --cwd web
scene`; authored screenshot galleries run through `bun run --cwd web scene --
--visual`. Set `SCENE_GRAPHICS_QUALITY=high` to exercise authored detail in the
mechanics gate.
Timing and parity fixtures always use their own high/current-quality contract.

[The asset CLI](asset.mjs) is file IO around [scene-assets](../packages/scene-assets/README.md).
Its bare invocation prints available commands. Audio preparation has a separate
[audio tool](../packages/battle-audio/README.md#recording-preparation), because audio
provenance and crops belong to the sound catalog rather than the appearance catalog.

## Runtime audio sharp edge

The Vercel project must have **Git LFS** enabled in its dashboard. Runtime music and voice MP3s are tracked LFS assets; without that setting a deployment serves pointer files instead of media.
