# Battle benchmark

One recording path measures a real simulation and renderer while a fixed camera tour flies. A preset names the work; it is never a fallback for a missing map or a failed preparation.

The benchmark (`/benchmark`, or by name `?preset=city-contact`) uses the existing simulation-owned central contact stress on a complete generated world. Its opening fighters are concentrated while reserves and logistics remain at the rear. Waves command the surviving fixed roster; their schedule alone does not establish sustained combat. Its synthetic starts and seeded orders are explicit; it does not measure player deployment or replace infantry travel. The map enters through the normal map source and preparation worker, and that worker becomes the battle authority.

A report records the selected workload fingerprint and exact applied tour. Generated reports also retain the checked request, returned map identity, derived map extents and stress identity. The fight stays at the playable centre; the overview frames the rendered landscape, including its visual-only margin. The existing display environment is drawn and counted too. Compare rows only when those inputs agree; a generator, physical catalogue, margin or rule change creates a different control even if the preset name stays the same.

Leaving preparation closes its worker. Once ready, timing starts after the declared warm tick and the first drawn frame; a cancellation produces a partial result rather than a complete performance verdict. Selecting and preparing the workload is a functional proof; the full timed window remains the frame-cost gate.

## The graphics test and its capture

The graphics test (`/graphics-test`) plays the menu reel and times it. It is also the control a second client draws against, so it records what it drew rather than what it meant to draw. A run of some scenes (`?scenes=`) reports the workload of those scenes, never the whole reel's.

Each scene writes two files, each with one strict reader that refuses a stale shape rather than migrating it:

- [The capture](presentationCapture.ts) (`battle-presentation-capture`): the observed publications and, for every drawn frame, the view the rig actually drew (after clamping, clearance and lag) in sim space, the interpolated sim time and the canvas size. The browser frame is not reproducible across runs (the simulation ticks in real time), so a plate is only ever compared with the frame records of its own run.
- [The manifest](visualPresentation.ts) (`native-visual-presentation`): the battle named by the hashes of the map and capture files, and what the frames were drawn with: lens, light, the terrain and grass tuning tables, the layers drawn, and the ground's and grass's GPU tables as hashed artifacts. An artifact is exactly the words the page uploaded, with a layout derived from the GPU struct it fills ([`wordTables.ts`](../../../../packages/battle-renderer/src/frame/wordTables.ts)), so neither can drift from the shader. The grass params are one frame's; their view fields are recomputed from each frame record. Each plate has a row of grass counts: the clumps that frame grew, by the screen cell their roots project into.

  Buildings and scenery are static per map, so the manifest carries their placement rather than a second client placing them: every building's module instances in each state its template has (intact, ruin, gutted), and every forest tree, understorey shrub, backdrop tree and piece of forest-floor dressing, each expanded by the owner the browser draws from and laid out as records ([`buildingTables.ts`](../../../../packages/battle-renderer/src/models/buildingTables.ts), [`sceneryTables.ts`](../../../../packages/battle-renderer/src/scenery/sceneryTables.ts)). Beside them are the kit modules and appearances the instances name, and the numbers each tier rule reads. A forest tree names its trunk's body, so a published fall finds it. Which buildings have fallen, which trees are felled and which ground is cleared stay per frame: a second client derives them from the publications. Each plate also counts what its frame drew, from the layers themselves: building modules per kit module and tier, scenery per population, appearance and tier, and the props its side knows (wrecks, shoved bodies, rubble) per appearance and tier.

`?plates=1` reads each shot's middle frame back in the task that drew it (the viewport's `FramePlate`): the canvas without the page's HUD or fade veil, the grass it grew, and the same frame drawn again offscreen lit without its grass. That picture is named after the plate by rule (`presentationCapture.ts`), and the canvas and the next frame are untouched. Only a plated run may copy from the canvas or export tables, so other frames pay nothing; a plated run is not a timing measure. The [`native-parity-capture` scene](../../../scenes/native-parity-capture.mjs) runs it unattended and writes each scene's files to `throwaway/native-parity/`.

## The frame-rate comparison

The browser and the native client are compared on the same reel scenes, at
1280×720, from the same numbers computed by the same code. Each client
records its frames raw, and [`frameComparison.ts`](frameComparison.ts) (run as
`node web/fps-compare.mjs <run dir>`) turns every run into average, 1%-low,
minimum and maximum FPS and p50/p95/p99/max frame time
(`frameTimeSummary`). It writes one row per client and scene: the median run
of several, with the spread of their averages. Only the interval clock
differs between the clients.

- **Browser.** The [`graphics-fps` scene](../../../scenes/graphics-fps.mjs)
  runs the graphics test without plates on a production build, headless, and
  writes the test's own recording: each drawn frame's `requestAnimationFrame`
  interval and main-thread time, the frame's GPU time from `timestamp-query`,
  and the in-game frame-rate counter's readings, which must agree with the
  recording's average.
- **Native.** The [native client's bench](../../../../native/godot/README.md#the-frame-rate-bench)
  draws a capture run's frame records and writes the same raw frames.
- **Same pacing, no presentation.** Neither client presents to the display:
  the browser runs headless and native minimised. Chromium's
  `--disable-frame-rate-limit` hangs the graphics test in battle preparation
  (inside the model layer's appearance install), and `--disable-gpu-vsync`
  alone leaves `requestAnimationFrame` at the display's 120 Hz, so the
  browser stays paced and native runs uncapped. The cap doesn't bind while
  the browser's frames take longer than 8.3 ms; where it would, compare CPU
  and GPU time per frame instead. Presenting to a shown window is a separate
  cost, measured on its own.
- **Power.** On battery the headless browser holds exactly 30 FPS whatever
  its frames cost, so every comparison runs plugged in.
- **GPU traces.** [`gpu-trace.mjs`](../../../gpu-trace.mjs) reads a Metal
  System Trace of either client: per-frame GPU busy and idle time, encoder
  times, and other processes' share of the GPU. Tracing slows the frames it
  records, so a trace gives proportions and gaps, never absolute ms.
- **Conditions.** Every run records its power source, build, adapter and
  pacing. Compare runs on the same power source; three runs per side, with
  the spread explained before any comparison.
- **Same drawing, not yet.** The browser also runs the live simulation
  worker and the reel's script, while native replays recorded publications.
  Native draws only the layers it has ported, so its frames hold less.
