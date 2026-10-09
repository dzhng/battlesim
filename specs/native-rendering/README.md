# Native rendering and Godot evaluation

**Status:** evaluation checkpoint: browser capture/export, native semantic decoding, saved-map composition, report identity, and a hashed evidence manifest are implemented. Godot remains experimental because authored-material/fog parity and display-backed comparison remain unbuilt.
**Updated:** 2026-10-09.

## Next Agent Prompt

Next session: execute [follow-on-display-backed.md](follow-on-display-backed.md). Hydrate the authored catalog, render terrain/props/fog/units in a foreground Godot run, capture matched named cuts, and complete the visual/performance gates. The native reel now resolves `fixtures/maps` by default, treats map limits as explicit diagnostics (unset or zero loads the complete export; positive values bound a probe), and updates fog cells from each replay frame. Keep the interactive GDExtension seam and Rust simulation as authorities; keep `/benchmark?preset=city-contact` unchanged.

Evidence: focused client/reel/report/capture tests, TypeScript, production build, route smoke, Godot decoder test and current foreground browser plus Godot runs using regenerated per-scene captures. The browser completed the full reel and emitted 3,541 `market-town` and 1,291 `paris-corner` publication samples plus displayed-frame timelines. Godot consumed both current captures, validated their layouts, walked 1,676,322 raw publication words, fully replayed 4,832 publications (66,590 unit rows and 4,832 fog payloads), and consumed bounded terrain/road/building/prop geometry from both map exports. `comparison_ready=false` remains because authored materials, fog, the full catalog, and display-backed visual equivalence are not proven. The cuts are lifecycle and camera/world-placement evidence, not visual-equivalence evidence. Native replay/side-filter parity and IPC measurement pass. Godot's editor scan still reports the expected missing local extension dylib when the dylib is not built. Authored Godot rendering, visual comparison and performance verdict remain open.
The durable [evidence manifest](evidence-manifest.json) pins the current capture hashes, workload fingerprint, viewport and scene identities. The capture now also carries the displayed-frame timeline; Godot follows those actual camera frames rather than deriving presentation time from simulation ticks.
Godot's capture validator now parses the included layout object and consumes every raw publication word, reporting layout validity and total words; semantic observation decoding now reconstructs the Rust-packed group stream for native unit poses, validates non-packed replacement/copy carriers, and consumes fog plus ground payload metadata; bounded terrain, roads, building and prop geometry are also consumed from saved maps. Full-stream native semantic replay now passes both real captures: all 3,541 market-town and 1,291 paris-corner publications decode with zero invalid ground payloads. The fog layer now renders bounded cells from the captured visibility bitfield; authored materials and the full catalog remain open.
The browser contract now has a real Rust publication regression: raw capture words from three consecutive publications decode through a fresh `ObservationDecoder`, proving the wire representation is replayable before native semantic decoding is attempted.
The attempted headless Playwright capture did not produce a report within its timeout, so no browser capture files were promoted from that attempt.

The first foreground cut probe produced 24 PNGs and loaded the authored GLB, but its shots visibly show one centered house against the clear background (and the opening fade is black). This is a renderer-path proof, not a matched menu-world result; retain the cuts as evidence only until the full map composition is loaded.
The current Godot cut metrics confirm that limitation: several shots are transparent or single-colour (edge density 0), while the detailed authored shot is an isolated building against an incomplete field. An adversarial visual read is that the new unit decoder could still be invisible in the player view; the existing cuts do not prove unit readability or scene composition, so comparison remains blocked until a fresh semantic-unit cut set is captured.
The new foreground authored cuts improve this only partially: named shots are now opaque and show roads, map ground, authored silhouettes and colored unit markers, but edge density remains low (about 0.016–0.038) and the units are tiny at the captured framing. The adversarial read is that the scene still reads as sparse proxy geometry rather than the browser menu world; no browser counterpart was available for a matched crop, so the visual gate remains red.
The fresh unprimed critique confirms the gate is still red: one cut is fully black, the remaining cuts are dominated by flat green ground and navy sky, authored silhouettes are aliased at the horizon, and unit capsules are too small to read. The saved-map and fog contracts are improving, but no visual-equivalence claim is warranted until authored materials, camera framing, and browser control cuts are captured together.
The next foreground candidate set (23 of the 24 named shots) still fails that gate. A fresh reviewer found no visible authored buildings, road network, fog, shadows, or terrain relief; scene-01 shots repeat the same nearly empty frame, and the only visible units are tiny red/blue capsules. This evidence is retained as a candidate failure, not promoted to a comparison baseline.

## Goal

Determine whether a Godot desktop client is worth maintaining alongside the browser TypeGPU/WebGPU client, while preserving the Rust simulation as the one authority. The first evidence run targets the current Mac and keeps the design platform-neutral. The comparison uses the full main-menu battle reel, exposed as a graphics test in Settings for the client currently running.

The graphics test reports average FPS, 1% low, absolute minimum, maximum, raw frame intervals and a frame-time chart. It also records simulation and observation transfer, CPU submission, GPU execution, presentation, startup and shader-compilation costs separately. There is no hard performance threshold; the verdict weighs performance with visual target, startup, platform reach and maintenance cost.

## Fixed decisions

- Browser and Godot share simulation outcomes, observations, scene identities, camera intent and quality intent. Rendering implementations may differ where the result remains visually equivalent; pixel parity is not required.
- The browser remains the shipped baseline. Godot stays experimental until broad gameplay and visual-target parity is demonstrated.
- The first comparison is Godot versus the current browser renderer. Native JavaScript WebGPU and Rust `wgpu` are follow-up probes only if the evidence needs them.
- The full menu reel is the reference workload. Its menu maps, encounters, seeds, warm-up, catalog and camera shots are frozen by identity. Benchmark playback is independent of reduced-motion preferences and the live menu's simulation-clock pacing.
- The existing synthetic `/benchmark?preset=city-contact` remains a separate diagnostic and regression workload.
- The first Rust-to-Godot binding slice is an architecture spike. Candidate seams include direct native/GDExtension, offline replay/presentation capture, and a process boundary. No seam is promoted until it passes the same authority, lifecycle and measurement checks.

## Ownership and firewalls

- Rust simulation and contracts own rules, fixed ticks, visibility, observations, replay and digests. Godot must not add gameplay physics, navigation or fog authority.
- The browser simulation adapter owns borrowed publication copying, credits, decode and lifetime. Native probes must preserve the same bulk-transfer and teardown semantics without copying packed layouts into a second authority.
- `apps/battle-lab/src/menuReel.ts` and `fixtures/menu-backdrop.json` own the menu scene identities. The benchmark manifest may reference them, but must not create a second set of scenes.
- `packages/renderer-core` owns camera and depth conventions; the battle renderer owns observed-feed composition. No general cross-engine renderer abstraction is introduced before the spikes identify a seam worth sharing.
- Temporary probes stay in ignored evidence or a clearly isolated native experiment. Rejected bridges are deleted when their evidence is archived.

## Slice graph

1. [Baseline and graphics test](slices/01-baseline-and-graphics-test.md) — freeze the menu-reel workload, define the shared report and add the browser Settings test.
2. [Binding architecture spike](slices/02-binding-architecture-spike.md) — compare Rust-to-Godot seams with live authority and replay/digest checks.
3. [Matched Godot renderer](slices/03-godot-reference-render.md) — render the same captured workload in Godot and produce a comparable displayed-frame report.
4. [Verdict and reslice](slices/04-verdict-and-next-plan.md) — repeat runs, compare equivalent outputs and write the focused follow-on plan.

Native JavaScript WebGPU and Rust `wgpu` remain optional follow-up probes, owned by slice 04 if Godot's evidence exposes a host question.

## Verification map

Every visual slice must use [compare-screenshots](../../.agents/skills/compare-screenshots/SKILL.md) against the frozen browser reference and finish with an unprimed [screenshot-critique](../../.agents/skills/screenshot-critique/SKILL.md). Use [preview-shots](../../.agents/skills/preview-shots/SKILL.md) to show the evidence. Visual review is non-blocking: open the shots for roughly five minutes, then decide from the evidence, record the rationale and close the shots if no response arrives.

At closeout, run the full repository checks once, as required by `AGENTS.md`. During implementation use the narrowest relevant browser, Rust and native probe checks. Serialize GPU runs with the repository's GPU lock.

## Research to recheck in slice 02

Use primary documentation and pin exact versions in the evidence manifest:

- [Godot renderer architecture](https://docs.godotengine.org/en/stable/tutorials/rendering/renderers.html)
- [Godot GDExtension](https://docs.godotengine.org/en/stable/tutorials/scripting/gdextension/index.html)
- [godot-rust](https://godot-rust.github.io/book/)
- [Godot benchmark suite](https://github.com/godotengine/godot-benchmarks)
- [wgpu](https://github.com/gfx-rs/wgpu) and [wgpu-native](https://github.com/gfx-rs/wgpu-native)
- [TypeGPU](https://docs.swmansion.com/TypeGPU/)

External measurements are context only; the frozen Mac runs are the evidence.

## Global TODO

- [~] Add displayed-frame rate summary fields and tests — slice 01 (landed).
- [~] Freeze complete menu-reel identity, canonical capture and browser control — slice 01 (fingerprinted identity, downloadable report and hashed evidence manifest; named browser cuts and visual control review remain open).
- [~] Add Settings graphics test and report chart — slice 01 (route exists; lifecycle, report identity and visual verification remain).
- [~] Verify offline native seam and Godot headless toolchain — slice 02 (offline probe and live extension smoke load; real scenario transfer/parity remains).
- [x] Spike binding candidates and select a seam — slice 02 (GDExtension for interactive authority; presentation capture for renderer comparison; IPC retained as measured fallback).
- [~] Render the full menu reel in Godot — slice 03 (semantic unit decoding plus complete saved-map geometry and comparable report envelope added; authored materials, fog timeline updates, full catalog loading and visual comparison remain open).
- [~] Compare repeated distributions and reslice the chosen direction — slice 04 (provisional verdict: keep Godot experimental; repeated equivalent display-backed distributions remain open).

## Human review surface

The human reviews the frozen menu-reel shots, the browser report chart, the Godot matched shots, and the final comparison table. Feedback can change the reference workload, visual-equivalence allowance or whether Godot advances; it does not silently change the simulation authority or measurement contract.

## Handoff state

This branch is `t3code/explore-godot-renderer-spike`. The handoff includes the
browser graphics-test route, native GDExtension parity/IPC probes, the versioned
TypeScript capture contract, the Godot camera/workload probe, and the Godot
capture validator. Generated `.godot` caches, UID files, binaries and reports
were removed before handoff. The stopping point is after decoder checkpoint
`cb007edd`; no full-reel Godot renderer exists yet.

The first evaluation is now closed at the evidence boundary: native map geometry, full captured publication replay, and sampled unit playback are present, but `comparison_ready=false` is preserved because authored materials, fog rendering, and a display-backed comparison are absent. Continue in [follow-on-display-backed.md](follow-on-display-backed.md); do not use headless Godot or the synthetic proxy for performance conclusions.
