# Native client

The native client is the desktop shell for the shared Rust `wgpu` renderer. It
runs the shared runtime's shaders and passes over the [presentation crate](../../crates/presentation/README.md)'s
`World` and `Frame`, and decides nothing per entity itself.

The [shared runtime](../../crates/renderer-core/README.md) owns GPU resources,
preparation and complete frame encoding. This crate retains desktop concerns:
`winit` surfaces, filesystem runtime loading, replay/capture and audio. Display
capture uses the common bounded readback protocol; the native export makes its
plates opaque. There is one production pass graph for both targets.

## Windows development

The native client is a Windows desktop binary built with the MSVC Rust target
and Visual Studio's C++ workload (the Windows SDK supplies `link.exe`). From a
Windows PowerShell checkout, run `powershell -ExecutionPolicy Bypass -File
tools/setup-windows.ps1 -Native`; the script also prepares the browser path.
The GNU target can be built from WSL after installing MinGW, but MSVC is the
supported Windows development target.

The browser and native clients share the same baked runtime files but load them
through different boundaries. Vite copies `assets/runtime` into the web build;
the native client reads it from disk. A packaged native build should place
`assets/runtime` above the executable or set `BATTLEGAME_RUNTIME` to its path.
The native resolver no longer depends on the WSL checkout path.

The browser is started with `bun run dev`. WSL should serve the app while
Windows Chrome supplies the real GPU; configure `.wslconfig` with
`networkingMode=mirrored` so Chrome can reach the WSL localhost server. WSL's
own software adapter is suitable for pipeline checks, not for the full battle.

## Presentation and platform boundaries

Replay resolves canonical presentation from the same admitted rules, map and
content identities as the browser. The renderer consumes its camera, placement,
visibility, tier, effect and retained-static records directly. Native cannot
substitute camera lists for caster lists, reconstruct hidden state, or relabel a
held frame after preparing it.
[Device reconstruction](../../crates/renderer-core/README.md#device-reconstruction) is owned by the shared runtime.
The [presentation README](../../crates/presentation/README.md) owns the records.

Native adapts desktop services around those owners. Audio commands come from
presentation; the desktop sink decodes runtime recordings and plays them through
`kira`. Missing Git LFS takes or tracks are refused by name. Offline replay sound
is rendered to a file or buffer, never played aloud by a test. Player paint is
absent when a replay supplies no UI geometry; shaders still read valid empty
resources.

## Running a parity scene

The binary replays one scene of a browser parity run (the control the
[spec](../../specs/done/native-rendering-parity/README.md) names). It refuses a
capture or a saved map that is not the one the manifest's identity hashes,
naming the file, and reads the capture and the manifest as strictly as the
browser does: a key the format lacks or leaves out refuses the file. It reads
the world from the repository, not from the run: the map, its ground, buildings
and props, and the units its encounter fields with their effects
(`presentation::fielded`). Then it frames every record through the presentation
crate and writes each plate the browser wrote, under the same name. A plate
whose counts differ from the browser's fails the run: it exits with an error
listing each difference after the report. Its usage text lists the flags. Run it
under the GPU lock.

The window opens borderless and off-screen by default, so nothing appears on
the user's screen. On macOS, `wgpu` presents nothing to a window that isn't
visible (off-screen, minimised or hidden all report `Occluded`), so those runs
draw unpaced; plates never depend on presenting.

## Timing a scene

`--bench <dir>` is native's half of the [frame-rate
comparison](../../web/src/battle/benchmark/README.md#the-frame-rate-comparison).
It draws every frame record of the scene back to back through the whole pass
list, framing each one as a plate is framed, and writes no plates. Each frame
records the same things as the browser's reel: its interval, its CPU time, and
its GPU time from the first timestamp marker pass's start to the last one's
end, as the browser takes it. The CPU time is split into
framing, preparing, encoding and submitting. Benchmark submission bypasses
surface acquisition, the window copy and presentation; browser measurements
include their shell's surface work. The report goes to `native-<scene>-<time>.json`
in the browser file's shape. The benchmark keeps at most two frames on the GPU,
as a swap chain would. Every changing frame is drawn, so static batches are
written only when they change. Run a release build under the GPU lock.

Metal can return stale end timestamps when counters resolve in the frame's own
command buffer. The benchmark waits for that frame to complete, then resolves
and reads its counters in a separate submission. These profiling waits affect
frame intervals and FPS; the recorded CPU phases exclude them. Compare CPU and
GPU cost per frame separately from this instrumented throughput. Raw GPU samples
align with the CPU samples, and incomplete coverage suppresses the aggregate
GPU time instead of averaging only successful readbacks.

`--bench-freeze-index <record> --bench-frames <draws>` instead frames every record
through the chosen zero-based index once, then redraws that held canonical input.
The timed loop still prepares, encodes and submits; it performs no new framing.
Reports omit the first sixty warmup draws and the final draw's incomplete frame
interval. The report identifies both retained draw and source-record endpoints.

## Checks

[Pipeline checks](tests/pipelines.rs) build every built-in pipeline of the shared
catalog on this machine's device. Most newer rendering checks under [tests/](tests/)
are synthetic GPU tests of `renderer-core`, including material, fog, overlay and
high-air contracts. They exercise the shared runtime rather than native replay.
[Benchmark CLI checks](tests/bench_cli.rs) reject invalid requests before window or
GPU admission; [runtime generation checks](tests/runtime_generation.rs) verify held
static reconstruction without reframing the battle.

`tests/counts.rs` replays a committed count-only browser run (`fixtures/parity/capture/counts`,
gzipped as written) and holds native's counts (`counts.rs`: the models, the
effects, the building modules, the props and the scenery, all the
presentation's) to the browser's on every frame it counted, on tick boundaries
and between; it needs no GPU. A replay check of a plate counts the same way.
The fixture-backed checks replay the run's world through what native reads (the
models, the props, the scenery and the sound) and need the Git LFS files each
names. Plates are judged against the control's, region by region, with the
spec's screenshot skills.
