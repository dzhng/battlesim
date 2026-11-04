# Whole-browser memory

The fixes remove duplicate source payloads and upload copies while preserving
the complete admitted catalog. This compares the original TypeGPU path, the
shared renderer before the fixes, and the fixed shared renderer. It is not an
isolated test of Rust versus TypeScript or `wgpu` versus TypeGPU.

## Measurement contract

The primary number sums macOS kernel **physical-footprint charges for one
isolated Chromium process tree**: browser, GPU process, pages with their workers,
and utility processes. It includes charged compressed and IOKit memory. It is
neither exact unique physical RAM nor an exhaustive measurement of driver
allocations outside those ledgers. Summed RSS is diagnostic because shared
mappings can be counted more than once.

An independent Python process enumerates descendants and reads
`proc_pid_rusage`, version 2, about every 200 ms. SDK offsets for resident size
and physical footprint are 64 and 72 bytes. Subinterval peaks and short-lived
processes can be missed. The process breakdown uses the median-footprint sample
in the second preceding a checkpoint, classified by Chromium's PID inventory.
Workers belong to renderer PIDs and are included without inspecting their
individual isolates.

All totals below are decimal GB. JS heap, backing stores, WASM capacity and
logical GPU payload overlap: **never add them to the process total**. WASM
capacity is an allocator high-water capacity, not live payload. Chrome's trace
`gpu/dawn` mostly reports Skia here, not the game's total WebGPU allocation.

Natural runs exclude tracing, screenshots, forced GC and worker inspection
until the settled checkpoint finishes. Main-isolate forced GC and page departure
are subsequent diagnostics. Main GC does not prove collection in every worker
or the GPU process. Detailed memory-infra cohorts are attribution only:
requesting their dumps can change subsequent memory behavior.

Host: Apple M6, Mac18,5, 24 GiB unified memory; macOS 27.0.1 (26A434).
Playwright Chromium 148.0.7778.96 uses hardware WebGPU, fresh headless processes,
1280×800 at DPR 1, disabled network cache and frozen production assets. The
repository launcher adds `--enable-unsafe-webgpu`, `--enable-features=WebGPU`,
and the evaluator adds `--mute-audio` to Playwright defaults. The canonical GPU
lock serializes checkouts. This task does not compile during measurements;
ordinary host activity is not disabled.
Startup sample gaps have medians of 205–209 ms and a maximum of 320 ms;
the matched replay medians are 207 ms and maximum gap is 311 ms.

## Predeployment comparison

The same small generated US battle, seed 1, pauses at tick 30 and settles for ten
seconds. Every row has digest `7446f004bf34b797`. This is loading before
deployment, with no fog eyes, not combat. TypeGPU admits 384 appearance names;
both shared builds admit 187. The TypeGPU comparison includes admission-policy
improvements already shipped in the port. The before/fixed shared comparison
keeps the catalog and logical GPU payload identical.

Three natural repeats per build are included. One TypeGPU attempt briefly
overlapped an orphaned browser from a cancelled owned job; exclude that attempt
and include its complete replacement rather than selecting a favorable repeat.

| Whole-tree footprint, GB | Original TypeGPU | Shared before fixes | Shared fixed |
| --- | ---: | ---: | ---: |
| Blank browser, mean | 0.268 | 0.269 | 0.271 |
| Application ready, mean | 6.978 | 9.579 | 5.874 |
| Settled, mean | 7.671 | 8.845 | 7.012 |
| Settled, all repeats | 7.678 / 7.699 / 7.635 | 8.861 / 8.831 / 8.842 | 7.681 / 6.441 / 6.914 |
| Sampled peak through settled, mean | 7.951 | 10.779 | 7.290 |
| Sampled peak, all repeats | 7.913 / 7.923 / 8.018 | 10.762 / 10.784 / 10.792 | 7.943 / 6.712 / 7.216 |
| After main-isolate forced GC, mean | 7.595 | 8.785 | 6.014 |
| After page departure, mean | 0.384 | 0.371 | 0.365 |

The fixes reduce mean settled charge by **1.833 GB (20.72%)** and sampled peak
by **3.489 GB (32.36%)** against the original shared renderer. Against TypeGPU,
the fixed path averages 8.59% less settled charge and 8.31% less peak charge in
this workload. Fixed repeats vary substantially: one retains about a gigabyte
more GPU-process charge until diagnostic GC. These means do not establish a
memory win in every run or on other browsers and hardware.

The following process rows are additive and use those same settled samples.
GPU-process charge includes staging, driver and resource overhead; page/worker
charge includes native allocators and WASM, not only JS.

| Settled process charge, mean GB | Original TypeGPU | Shared before fixes | Shared fixed |
| --- | ---: | ---: | ---: |
| Browser | 0.080 | 0.075 | 0.076 |
| GPU process | 3.970 | 3.245 | 3.560 |
| Pages and workers | 3.486 | 5.389 | 3.244 |
| Utilities | 0.134 | 0.136 | 0.132 |
| Total | 7.671 | 8.845 | 7.012 |
| Summed RSS, diagnostic only | 5.111 | 8.710 | 3.079 |

Rounding can make displayed rows differ from totals. RSS and footprint treat
compressed, shared and IOKit-charged memory differently; RSS must not replace
the primary ledger.

| Nonadditive settled diagnostic | Original TypeGPU | Shared before fixes | Shared fixed |
| --- | ---: | ---: | ---: |
| Main JS used heap, mean MB | 40.809 | 27.556 | 38.285 |
| Main JS committed heap, mean MB | 110.592 | 64.340 | 63.357 |
| Main backing storage, mean GB | 2.074 | 1.311 | 0.726 |
| Main presentation/render/audio WASM capacity, range GB | 0.075–0.083 | 1.891–1.892 | 1.061–1.062 |
| Additional page game WASM capacity, GB | 0.009568 | 0.009568 | 0.009568 |
| Logical live GPU buffers, GB | 2.878 | 1.194 | 1.194 |
| Logical live GPU textures, GB | 0.563 | 0.434 | 0.434 |

Isolate diagnostics omit individual workers; process totals include them.
Identical GPU descriptor bytes before/after the fixes establish that the saving
does not come from deleting drawable geometry or lowering texture resolution.

## Active battle

The fixed live `ally-help` journey passes all five assertions and finishes at
tick 7277, digest `6e84c5b9d99ba6e2`. Its main WASM capacity is 1,880,489,984 B,
versus the previous successful shared path's approximately 3.59 GB. Natural
whole-browser settled charge is 8.181 GB and sampled peak is 11.185 GB. These
are different quantities; this is not a 1.88 GB application.

The exact saved replay supplies this three-build comparison. Every arm reaches
tick 7277 and digest `6e84c5b9d99ba6e2`, without page errors. Each arm has one
natural run, rather than the startup cohort's three repeats. The harness requests
60-tick advances and waits for final presentation, then settles ten seconds.
Reaching the last tick takes 710 / 43 / 38 seconds respectively. This is a
maximum-speed replay of identical commands/history, not matched wall-clock
pacing, a displayed FPS comparison, or an isolated backend benchmark. Different
advance rates give GC and asynchronous admission different time to progress.

| Whole-tree footprint, GB | Original TypeGPU | Shared before fixes | Shared fixed |
| --- | ---: | ---: | ---: |
| Blank browser | 0.285 | 0.283 | 0.283 |
| Application ready | 7.268 | 10.056 | 6.163 |
| Settled at final tick | 8.785 | 12.409 | 7.826 |
| Sampled peak through settled | 9.514 | 16.212 | 11.600 |
| After main-isolate forced GC | 8.581 | 11.868 | 7.819 |
| After page departure | 0.402 | 0.395 | 0.393 |

The fixes reduce settled battle charge by **4.582 GB (36.93%)** and sampled
peak by **4.612 GB (28.45%)** against the original shared renderer. Relative to
TypeGPU, fixed settled charge is **10.91% lower**, but peak is still **21.93%
higher**. This remaining admission peak is a real tradeoff. The independent
fixed live journey's 11.185 GB peak also shows substantial memory pressure
outside accelerated replay; it is not a controlled TypeGPU live-play comparison.

| Settled process charge, GB | Original TypeGPU | Shared before fixes | Shared fixed |
| --- | ---: | ---: | ---: |
| Browser | 0.193 | 0.078 | 0.080 |
| GPU process | 4.152 | 3.961 | 2.924 |
| Pages and workers | 4.298 | 8.236 | 4.689 |
| Utilities | 0.143 | 0.133 | 0.133 |
| Total | 8.785 | 12.409 | 7.826 |
| Summed RSS, diagnostic only | 3.405 | 7.670 | 7.294 |

At the sampled peak the fixed build charges 5.415 GB to the GPU process and
5.976 GB to pages/workers; the before build charges 7.080 / 8.918 GB. Both peaks
occur around the later source-admission transition, rather than at initial
readiness. These are samples at the total peak, not sums of independent process
maxima.

| Nonadditive settled diagnostic | Original TypeGPU | Shared before fixes | Shared fixed |
| --- | ---: | ---: | ---: |
| Main JS used heap, MB | 55.236 | 56.925 | 36.298 |
| Main JS committed heap, MB | 68.420 | 104.759 | 41.845 |
| Main backing storage, GB | 2.257 | 1.433 | 0.794 |
| Main presentation/render/audio WASM capacity, GB | 0.352 | 3.591 | 1.880 |
| Additional page game WASM capacity, GB | 0.015204 | 0.015204 | 0.015204 |
| Logical live GPU buffers, GB | 2.944 | 1.268 | 1.268 |
| Logical live GPU textures, GB | 0.607 | 0.477 | 0.477 |

Postmeasurement worker inspection succeeds for both worker targets in each
arm. Their WASM capacities are identical: 97,648,640 B and 29,032,448 B. Worker
JS used heaps total about 4.4–4.6 MB. These post-GC isolate diagnostics do not
replace the natural page/worker footprint or assert that capacity equals live
memory. GPU buffer payload differs by only 1,512 B between the shared arms at
the sampled final frame; their texture payload and resource counts match.

## Parameter-effect map

| Strategy | Observation | Disposition and tradeoff |
| --- | --- | --- |
| Intern decoded textures by immutable content identity | Admitted bundles repeat about 873 MB of texture bytes but contain about 45 MB of unique texels. Startup main WASM falls from about 1.89 to 1.06 GB. | Keep. Weak library entries do not retain retired texels; bundles and pending generations own them. |
| Initialize GPU buffers through the queue | Texture sharing plus queue uploads averages 6.905 GB settled / 8.816 GB peak, versus 8.845 / 10.779 before; GPU payload is unchanged. | Keep the measured combination. Staging still costs memory; these figures do not isolate additive effects. Native/browser use one upload owner. |
| Serialize geometry only during admission | Main backing storage falls from 1.311 to 0.726 GB; mean peak falls from the combination's 8.816 to 7.290 GB. | Keep. Parsed art remains the reconstruction source; admission serializes again instead of permanently retaining another mesh copy. Settled GPU variability prevents claiming a monotonic settled win over the combination. |
| Lazy property followed by object spread | The first getter trial still retains serialized geometry: spread evaluates the getter eagerly. | Discard that implementation. Preserve laziness through the complete source-construction path. |
| Runtime JS emulation of queue uploads | Validation/workload errors. | Invalid, not a memory win; use the actual implementation and lifecycle gates. |
| Trace dumps or forced GC inside the primary window | Later memory behavior changes. | Attribution only; exclude from natural confirmation. |

A draft shares immutable textures **without copying the active bundle map**.
Otherwise preparing it after another generation commits could restore stale
sources it never requested. `Library::empty_generation` encodes this distinction;
canonical source installation still has one transaction owner.

Further savings remain possible but unmeasured. Parsed browser art and decoded
Rust art coexist for reconstruction and tooling; eliminating one requires a
single reconstruction source through reload and external tools. The GPU catalog
also admits geometry before visibility; deferral must preserve immediate
purchase ghosts. Avoid a second lifetime protocol merely to move a counter.

## Evidence and attempts

Raw output lives in ignored `throwaway/final-perf/`. `full-memory.mjs`,
`memory-sampler.py`, `summarize-full-memory.py`, and `memory-report.py` retain
the evaluator and arithmetic. `memory-report.json` contains exact byte values
and per-repeat process groups; `memory-artifacts.json` contains emitted-asset
SHA256 manifests. They are research evidence, not another runtime abstraction.

- TypeGPU source `5b52cabb57578d059eaa3a352bf2e38946742d4b`, artifact
  `startup-final/typegpu-build`.
- Shared before source `d9a08f88b7050fb238badf77fb297c019a50bed1` (the following
  `966bde248f0d5eff0e6b876f35f4bbaa07660516` changes docs only), artifact
  `startup-confirmation-final/wgpu-build`.
- Fixed source `e56cb964c45c98dffc123b36ea5a72825a42e11f` (commit-hook
  formatting does not change the measured program); artifact
  `memory-lazy-preserved-build`; renderer WASM SHA256
  `dcc7e500d286888d963d4fda34b4b28d2dd778552838977ef4b2f0ce5af9966a`.
- Runtime catalog SHA256
  `d98c65c9341bd3c4d06990b80ae78af8d94de2a9393b7bb8c0c2a876e69075c7`.

Compiled-assets manifests hash sorted emitted records (path, length, SHA256),
encoded as compact JSON with sorted keys, excluding runtime symlinks. Hashes:
`f4129cd5fcc3a1d6c5cd09214c81e31e11a9ccb7af2eff9ccf02ba20fee46417`
(TypeGPU), `8648d98adecb2c0ab527065eae984f207d2e6373690dcd6524088ed876bf986f`
(shared before), and
`a527fcbaa44b49a715ba01248658947f8a5b4da7e2cebfe5f8ff7013a92013be` (fixed).

| Evidence directory | Role and disposition |
| --- | --- |
| `full-memory-natural-confirmation`, `full-memory-typegpu-replacement` | Primary TypeGPU/before/texture+queue cohort. Exclude only contaminated TypeGPU repeat zero; include its full replacement. |
| `full-memory-lazy-confirmation` | All three planned final natural repeats, including the high GPU-charge repeat. |
| `full-memory-lazy-preserved-pilot` | Development only, excluded from confirmation means. |
| `full-memory-lazy-pilot` | Eager-spread implementation trial, not the final candidate. |
| `full-memory-pilot`, `full-memory-pilot-v2`, `full-memory-combined-pilot`, `full-memory-confirmation`, `full-memory-texture-sharing`, `full-memory-queue-probe` | Development/attribution only. Early ticks differ, tracing perturbs subsequent stages, and queue emulation / texture-only battle fail gates. None enters natural confirmation. |
| `full-memory-ally-control`, `full-memory-ally-control-v3`, `full-memory-journey-confirmation` | Failed, cancelled or incomplete live journeys. No complete memory total is inferred. |
| `full-memory-live-candidate`, `full-memory-replay` | Successful fixed live journey and matched replay cohort. |

Initial attempts allow 240 seconds. The saved TypeGPU replay needs longer
because its harness advances about ten ticks per second; the replay cohort
allows 1,100 seconds per arm and the independent sampler has a 1,200-second
ceiling. Missing/cancelled results never count as zero.

## Verification

The decoded-texture regression fails against the old ownership (40 bytes for
two copies) and passes with one 20-byte mip payload, unchanged texels, catalog
replacement and retirement. The empty-generation assertion also excludes
unrequested old source names. Four pipeline ABI/entry tests and six focused web
admission tests pass; TypeScript and strict Clippy for core/browser/native pass.
Eleven native GPU tests cover marks, frame views, high air, impostors and probes.

Browser frame checks pass resize/rebuild resource return, collapsed targets,
owned readbacks, surface reconfiguration, actual device recreation and current
fog restoration. Roster checks pass authored ghosts, purchase, deployment and
movement. The first browser run mistakenly served a frozen production build to
checks requiring development endpoints; discard that setup and use the normal
harness. The corrected roster run initially encountered an unfetched LFS
reference; fetching that exact reference and rerunning passes without changing
any threshold or approved picture.

Approved foundation/geometry/river and roster spatial image gates pass. Raw
images retain small capture-state/raster differences; do not claim byte identity.
The matched shared-before/fixed replay has full-frame grayscale MAE 0.00002
and edge-energy ratio 1.00000. A fresh unprimed reviewer inspects every full
pair plus purchase/order/town crops and finds no candidate-specific visual
regression. Crowded cyan labels in the replay town are a shared historical
readability concern. Evidence and the receipt-checked critique live in
`memory-visual/`. Independent code reviews find no remaining actionable defects.

The measured fixes add 154 net production lines, mostly propagating the queue
through existing constructors. They add no service, dependency, persistent
schema, secondary rendering path or background cleanup mechanism. The only new
cache is a weak content-identity index owned by the existing source library.
