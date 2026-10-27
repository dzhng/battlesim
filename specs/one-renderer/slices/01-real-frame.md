# Slice 01 — presentation-fed real frame

**Status:** terrain, authored model, and bounded production BattleFrame
comparison complete; shared native/browser core extraction is the required next
slice.

## Contract

Render one frozen `presentation-wasm` frame through the existing native Rust
`wgpu` renderer core compiled for browser WASM while a direct browser WebGPU
control renders the same authored terrain mesh and camera record. The target is
code reuse: the browser build must call the same Rust pass/resource modules as
the native client, with only surface/device/assets/audio setup differing.

The completed bounded probe used a standalone Rust graph and therefore proves
only the transport seam. It is retained as evidence, not as the architecture
under decision.

The model pass loads the existing `test_tank` runtime bundle through the normal
catalog and gzip/texture codec, selects authored LOD1 mesh data, applies the
bundle's resting articulated palette, and sends the resulting geometry, UVs,
first authored albedo texture and two placement/tint records to both backends.
It remains a renderer-input spike; the presentation contract is unchanged.

## Required gates

- Same frozen publication, camera, canvas size, render scale, adapter and
  presentation digest on both paths.
- Pixel captures for the pass and a whole-frame capture where the pass is
  composed; inspect the subject crop and surrounding depth/lighting context.
- Interleaved repeats under the existing GPU lock. Report setup/startup,
  bundle bytes, CPU encoding, GPU duration by pass, submitted work, GPU
  buffers/textures, heap/RSS and resource-reset baselines separately.
- Resize, rebuild and device-loss/error handling must return to the baseline
  lifetime contract.
- Browser TypeGPU remains green throughout; no simulation digest, publication
  layout or side-filtered observation changes are allowed.
- Native and browser must share pass order, target ownership, bindings,
  resource lifetime and presentation-record handling. A second browser-only
  pass implementation fails this slice's architectural contract even if its
  screenshot passes.

## Terrain result

The ten-row `one-renderer` scene now compares the original four micro paths,
the real terrain pair, and the authored model pair. The terrain pair produced
mean RGB difference `0` in the captured canvases. A representative run
completed 120 frames per row:

| Path | Setup | Encode | GPU drain |
| --- | ---: | ---: | ---: |
| Real presentation terrain + browser WebGPU | 17.8 ms | 8.3 ms | 2.9 ms |
| Real presentation terrain + Rust `wgpu` WASM | 7.0 ms | 5.6 ms | 1.8 ms |

The Rust path is faster to encode in this pass, while browser GPU drain is
lower on the direct path. The two terrain crops are byte-identical. The pass
proves authored geometry and camera handoff, not the full terrain material,
fog, shadows, post, models or resource graph; its low-detail patch is a seam
fixture rather than a player-facing battle composition.

## Authored model result

The browser and Rust model canvases use the same 9-float vertex stream, u32
index stream, 8-float placement/tint records, albedo bytes and camera record.
The Rust `wgpu` path draws the same terrain first, then the indexed model
instances in a second pass, sharing a reverse-Z `depth32float` target with
`greater` comparison. The captured mean RGB difference is `0.049`, below the
scene's `0.1` gate.

The latest 120-frame run measured:

| Path | Setup | Encode | GPU/present boundary |
| --- | ---: | ---: | ---: |
| Authored tank mesh + browser WebGPU | 76.6 ms | 0.9 ms | 14.0 ms |
| Authored tank mesh + Rust `wgpu` WASM | 7.2 ms | 23.3 ms | 0.2 ms |

Browser setup includes the runtime bundle, texture decode and persistent GPU
buffer allocation. The Rust row
reuses those browser arrays, so the setup comparison is diagnostic rather than
a whole-page startup claim. Both model captures are byte-identical and the
scene captures the complete eight-row comparison at 1280×2300.

The harness now repeats the whole eight-row run three times, rotating the start
row each time, and reports averages. The latest interleaved averages are:

| Path | Setup | Encode | GPU/present boundary |
| --- | ---: | ---: | ---: |
| Real terrain + browser WebGPU | 6.4 ms | 5.9 ms | 4.0 ms |
| Real terrain + Rust `wgpu` WASM | 2.3 ms | 4.7 ms | 4.9 ms |
| Authored tank mesh + browser WebGPU | 17.7 ms | 0.4 ms | 12.6 ms |
| Authored tank mesh + Rust `wgpu` WASM | 4.0 ms | 15.5 ms | 39.6 ms |

The Rust capture waits for three compositor turns after its measured loop so a
repeated WASM surface is present before the screenshot. This stabilizes the
harness boundary, but is not a claim about GPU time.

## Production BattleFrame result

This result is a lower-bound transport diagnostic, not the shared-core verdict.
The next implementation slice must replace the standalone Rust row with the
native renderer core and rerun the comparison.

The final two rows exercise the actual browser `BattleFrame` constructor with
its light, reverse-Z targets, fog, shadow setup, effect atlas, static props,
overlays and post-processing passes. Rust consumes the authored tanks, static
prop mesh, fitted cascade matrices, resolved sun/fog records and a synthetic
effect record through `wgpu`. The browser fixture's live effect count is zero,
so effect content remains an explicit input mismatch. Rust executes the corresponding pass order while keeping
unsupported material and shadow features explicit in its stats.

| Path | Setup | Encode | GPU/present boundary |
| --- | ---: | ---: | ---: |
| Production BattleFrame + browser WebGPU | 154.0 ms | 32.8 ms | 140.0 ms |
| Battle inputs + Rust `wgpu` WASM (production-shaped graph) | 8.9 ms | 21.0 ms | 30.4 ms |

The captured full-frame mean RGB distance is `47.03`. The browser control shows
four textured tanks, static props, rings, lines, fog and post. Rust shows the
same authored tanks and prop mesh and executes 480 shadow passes plus 120
world/effect/fog/post passes, but its material lighting, shadow sampling, fog
visibility, overlays and post treatment remain incomplete. The Rust screenshot
is substantially darker and less detailed, so this is a concrete porting gap,
not a visual-parity result.

Use `compare-screenshots` against the TypeGPU control and run an unprimed
`screenshot-critique` on every candidate shot. The latest fresh review found no
clipping or depth-order defects in the tank pair, while noting that the
near-black palette and diagonal terrain boundary make this fixture weak for
material and scene-fidelity judgments. Open evidence with `preview-shots` for a
short non-blocking review window.

## Stop conditions

Stop and record a rejection if the first real pass needs a second owner for
camera/depth/lighting, cannot load the existing runtime asset without a new
format, exceeds the 30-second-to-playable startup budget, or changes the
presentation contract merely to satisfy the renderer.
