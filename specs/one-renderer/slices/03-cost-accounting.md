# Slice 03 — implementation cost accounting

**Status:** baseline captured 2026-10-27 after the shared browser asset proof; rerun after any further full-pass port.

The spike needs a cost comparison alongside its pixel and timing comparisons.
The accounting command is deliberately independent of the browser scene so it
can be rerun after a port:

```sh
node tools/one-renderer-accounting.mjs --write
```

It writes `throwaway/evidence/one-renderer/accounting.json` (ignored scratch
output) and reports physical lines, non-empty lines, code lines, source bytes,
generated payload bytes, the latest scene result, and an owner map. `code` is
non-empty lines after removing full-line comments; it is a comparison aid, not
a complexity score.

## Current baseline

| Owner | Files | Lines | Code lines | Source bytes |
| --- | ---: | ---: | ---: | ---: |
| Production browser renderer TypeScript (`packages/battle-renderer/src`) | 88 | 20,815 | 16,189 | 816,050 |
| Production browser shaders (`packages/battle-renderer/pipelines`) | 63 | 16,714 | 14,696 | 654,269 |
| Renderer primitives (`packages/renderer-core/src`) | 11 | 1,604 | 1,156 | 64,747 |
| Rust spike (`crates/renderer-wasm-spike/src`) | 5 | 2,255 | 2,154 | 86,819 |
| Shared Rust renderer core (`crates/renderer-core/src`) | 14 | 2,875 | 2,426 | 95,668 |
| Shared Rust browser shell (`crates/renderer-browser/src`) | 11 | 1,942 | 1,793 | 74,262 |
| Native renderer modules (`native/client/src`, pass/resource owners) | 15 | 4,026 | 3,427 | 141,925 |
| Native platform shell (`native/client/src`, desktop/assets/audio) | 7 | 1,935 | 1,529 | 70,560 |
| Spike browser harness (`oneRenderer*.ts*`) | 3 | 769 | 728 | 27,483 |
| Spike scene checker (`web/scenes/one-renderer.mjs`) | 1 | 138 | 134 | 5,350 |

The standalone Rust source is currently about 6% of the production renderer's
TypeScript plus WGSL physical lines. It now has a production-shaped pass graph,
but its shaders and material records are deliberately bounded approximations;
the ratio is not a maintenance win. The decisive accounting must instead add
the extracted native core and its two platform shells, then compare that total
against both the native client today and the browser TypeGPU renderer.

The first shared-core checkpoints have landed: target allocation, buffer/bind
helpers, pass debug-group orchestration, pipeline construction, reverse-Z depth
and world-target attachment policy now have one owner in `renderer-core`. The
current extracted core is 2,875 physical lines and the browser shell is 1,942
lines; the remaining native pass/resource modules measure 4,026 lines and the
native platform shell 1,935 lines. This is exact accounting of the current
split, not a forecast: material, fog, effects and post owners still have native
or browser-only pieces, so the shared-core total will grow as those owners move.

The generated shared-browser payload measures 1,341,133 bytes of WASM and 84,761
bytes of generated JavaScript. The standalone transport spike remains 201,249
bytes of WASM and 72,057 bytes of generated JavaScript. These figures do not
include the production battle WASM, the 3,960,005 compressed test-tank asset
payload, or shared application chunks.

## Ownership gap

## Exact current tradeoff

The shared work is additive until the browser TypeGPU renderer is deleted. The
current measured Rust side is `2,875` shared-core lines plus `1,942` browser-shell
lines, while the native renderer still retains `4,026` pass/resource lines and
`1,935` desktop-shell lines. In other words, the extraction has added a browser
shell and a model seam without yet removing a production pass owner. The
complexity payoff starts only when the remaining native pass modules move into
the shared owner and the equivalent browser TypeScript/WGSL owners are removed.

The earlier shared-browser asset checkpoint measured `41.1 ms` setup including
asset fetch/decompression/parse/upload, `1.6 ms` CPU encode for 120 frames and
`0.1 ms` queue/compositor boundary. After range-aware model caster routing and
the real device-loss recovery round trip, the current interleaved scene reports
`168.0 / 6.3 / 0.1 ms` for setup / CPU encode / queue boundary. It draws
37,002 indexed vertices and executes 484 model-caster passes while surviving
two target reallocations and one device recreation. The TypeGPU production control is
`154.0 / 32.8 / 140.0 ms` on the same run, while the standalone Rust graph is
`8.9 / 21.0 / 30.4 ms`; the standalone number is not a migration comparison
because its image distance is `47.03` and its material/shadow/fog/post owners
are incomplete.

The cost of the real asset seam is also explicit: 3,960,005 compressed wire
bytes, 4,453,752 decoded bundle bytes and 9,437,706 decoded texture bytes for
the test tank. The shared WASM artifact is 508,439 bytes plus 69,184 bytes of
generated JavaScript. These are the costs a browser Rust migration must pay in
addition to the existing presentation and application payloads.

| Contract | Browser owner | Rust spike owner | Evidence |
| --- | --- | --- | --- |
| Canvas, adapter, device and surface | BattleFrame caller and TypeGPU root | `Renderer` | Same canvas seam works |
| Terrain and authored tank | `worldPass` / `modelLayer` | `Renderer` / `ModelRenderer` | Terrain diff 0; model mean RGB diff 0.049 |
| Reverse-Z depth and resize | `SizedTargets` / `worldDepth` | Renderer-owned depth texture | Bounded resize/depth gate passes |
| Shadows, fog, effects, post and overlays | BattleFrame pass owners | Rust production-shaped graph; material/shadow/fog/overlay/post parity open | Image distance 47.03; unsupported counters explicit |
| Registry, generation, rebuild, reset and device-loss recovery | `GpuRegistry` / `SizedTargets` / frame disposal | Renderer-owned ad hoc resources | Full lifecycle parity unmeasured |

The production frame's pass order is shadows → depth → fog compute → world →
effects → fog mask/rim → post → overlays/composite → high-air plane. Rust now
consumes the same frozen camera, fitted cascade matrices, resolved sun/fog
records, authored tanks and static prop mesh for the production-shaped run. It
still does not own the full material table, shadow sampling, fog visibility,
overlay composition, post treatment or resource registry. It must repeat
resize, resource reset/device-loss, startup and matched-image gates before a
source-size advantage can be treated as comparative renderer complexity.

The latest bounded run records `154.0 / 32.8 / 140.0 ms` for production
BattleFrame setup / CPU encode / queue-drain boundary and `8.9 / 21.0 / 30.4 ms`
for the Rust production-shaped graph. Queue drain is a submission/compositor boundary,
not a GPU timestamp. The values are useful to detect regressions in the fixture
but are not a migration decision while the images differ materially.
The source-level GPU construction counts are another useful guard against a
false complexity comparison. They count constructor call sites (not live GPU
objects or draw calls):

| Owner | Pipeline constructors | Render-pass constructors | Texture constructors | Buffer constructors | Bind-group constructors |
| --- | ---: | ---: | ---: | ---: | ---: |
| Production browser renderer TypeScript | 52 | 19 | 12 | 48 | 31 |
| Rust spike | 5 | 9 | 5 | 7 | 11 |
| Spike browser harness | 5 | 4 | 2 | 7 | 4 |
