# One-renderer research record

[Whole-browser memory](memory.md) owns the later before/after process totals,
memory reductions and measurement limits. The WASM-only figures below describe
their frozen historical builds, not current total application memory.

## Contract and evidence identity

Replace the browser TypeGPU renderer with one Rust `wgpu` runtime shared with
native. Presentation remains authoritative for poses, visibility and batches;
publication layouts and battle digests must survive unchanged. Acceptance requires
production visuals and lifecycle behavior, not a microbenchmark win. A small
performance regression is acceptable for fewer owners and lower total complexity;
fewer maintained source lines have not been established.

Compare the same presentation history, assets, camera, viewport, adapter and
quality. Queue drain and compositor turns are not GPU timestamps. Startup,
encoding, GPU execution, owned payload, WASM capacity and source size answer
different questions and must remain separate.

The fixed original TypeGPU source baseline is
`5b52cabb57578d059eaa3a352bf2e38946742d4b`, preserved with isolated build output
under `throwaway/final-perf/typegpu-baseline/identity.json`. Runtime media are
shared read-only; the unchanged simulation artifact is reused. Current source
also includes cleanup after the measured rendering cohort: its timings are not
a fresh measurement of every final edit.

The production frame cohort is frozen by
`throwaway/final-perf/full-reel-final-1-identity.json`: HEAD
`69a01c6e4bb3fb4b358efac564bc112561ad0f2f`, dirty-patch SHA-256
`b48bc2dbc42c2c50b8afc1d26ad18b580f968b8c7792125015acf3e9895b4b35`, capture SHA-256
`eb26cc15bac18ed9ccd7bbfe5036a5f889d62c45941712c96ecd5910148b9687`. That identity
also records source/artifact hashes. Historical checkpoints include `ab00ccef5`
(native extraction parent), `a7c668458` (production integration incumbent), and
`c574cfd97` (canonical fog-input probe). These are distinct experiments, not a
single-factor progression whose timing differences can be causally attributed.

## Parameter-effect map

| Parameter or strategy | Observation and tradeoff | Verdict / scope |
| --- | --- | --- |
| TypeGPU authoring, exported WGSL/raw WebGPU, Rust BrowserWebGpu, Rust base + TypeGPU overlay canvas | Four 120-frame shared-shader controls matched with zero mean RGB difference. Establishes browser submission and separate-canvas composition; does not establish production performance or same-texture interop. | Backend feasibility proved; hybrid is not the final one-owner architecture. Evidence: `throwaway/one-renderer-spike/`. |
| Independently built Rust production-shaped graph; permanent TypeGPU fallback | Transport/lifecycle checks passed despite missing sky, material, fog and shadow behavior. Keeping either would preserve duplicate policy. | Reject as migration architecture; delete both after shared production parity. |
| Extract actual native resource owners and concrete shared `FrameRenderer` | Model/scenery admission alone still failed visual parity (MAE 48.86802, pixelmatch 0.79827). Reusing native sky/environment/ground/grass/scars/fog then matched street RGB MAE 0.00001004912 and all six native plates byte for byte. | Keep real shared owners; green bindings are insufficient evidence. `throwaway/evidence/one-renderer/{paris-240-review/fog-after,shared-frame-review,native-shared-frame-diff}/`. |
| Separate presentation/render WASM versus one module + common Runtime | Matched optimized exports-only spike: raw 5,035,788→4,952,412 B; gzip-9 1,631,450→1,567,702 B; Brotli-11 1,102,643→1,025,680 B. Empty-world memory capacity 3,145,728→2,031,616 B. Actual unified GPU held/changing/recovery/native gates subsequently passed. | Keep to delete cross-memory frame/fog/scar copies, callbacks and replay owners. Spike sizes are not final artifacts or full-battle residency; no isolated packaging frame-time effect. `throwaway/unified-wasm-spike/REPORT.md`, renderer evidence `unified-runtime-*`. |
| Reprepare held model/effect streams versus canonical frame-index/static-generation reuse | The pre-reuse implementation spent 223.5 ms total encoding over 120 held draws (three-repeat mean), not an isolated upload measurement. Reuse records one model/effect preparation across 121 held preparations; changing frames each prepare. Fog/scar work remains live. | Reject repeated held uploads; keep bounded reuse. `unified-runtime-{first,profile}/samples.json` under renderer evidence. |
| Canonical PNG decode and complete effect-atlas mips | Shared pixels/native plates preserved; removes separate decode/atlas policy and adds mip storage versus old base-only browser atlas. Dummy one-layer atlas and malformed streak record visibly failed. | Keep real atlas and actual presentation effect stream; zero effect words at a sampled tick prove transport only. |
| Entire installed catalog versus requested canonical source admission | Full catalog exhausted WASM address space. Requested world/model/grass/scenery admission fits. Waiting for observations deadlocked because publication starts after GPU readiness. | Reject whole-catalog admission and observation wait; stage topology, frame on CPU, then commit GPU scene atomically. |
| Immutable geometry shared per resolved bundle/corpse orientation | Generated-skirmish owned GPU payload 2,670,304,601→1,223,999,969 B (54.2% lower), unchanged textures and zero changed image pixels. Routing/placement metadata stays independent; diagnostics count unique vertices once at admission. | Keep. `throwaway/{skirmish-geometry-shared.log,skirmish-geometry-pixels.json,model-memory-audit.md}`. |
| Latest complete appearance generation and page GPU ownership | Catalog remains usable while replacements prepare; superseded work cannot commit or revive retired state. Current-request errors stay terminal. Detached sheets share production admission. | Keep; 19 viewport tests include red controls. `throwaway/viewport-lifecycle-{final-19,appearance-red,instances-red,rejection-red}.log`, `throwaway/review/admission-final-targeted/verdict.md`. |
| Canonical model ABI, ranges, poses and actual impostors | Shared packer preserves the 48-byte authored vertex ABI; held/static ranges survive rebuild. Appearance/tier admission rejects unknown meshes. Articulated, skinned and ghost paths retain their actual capabilities. Authored MRT impostor albedo/normal/mask bytes, metadata and hash match TypeGPU exactly. | Keep; synthetic single-bundle transport never established heterogeneous production parity. `throwaway/one-renderer-impostor/`, player-feature review evidence. |
| Shared inspector/quality/paint/readback controls | All eight views match; queued snapshot remains the original frame while another view renders. Metal can skip untouched tile resolves, requiring independent paint clearing before MSAA. | Keep owned snapshots and explicit clearing. `throwaway/evidence/one-renderer/player-feature-review/capture/`, `throwaway/review/player-core/`. |
| Typed pipeline catalog + composed authored WGSL | Eliminates TypeGPU runtime/authoring, expanded exports, descriptor JSON parser, validation crate and spike. Token import aliases factor collisions without changing expressions/order. | Keep; all 64 assembled shaders equivalent **after whitespace normalization**, not claimed byte-identical. |
| Owned allocation visitor and weak lifetime identities | Deduplicated handles measure payload without retaining resources. Teardown + queue drain + explicit collection leaves zero observed buffer/texture wrappers. | Keep diagnostics transient; this does not measure allocator, driver, process or canvas residency. |
| Direct browser output target instead of display composite | Proposed as an independent way to investigate GPU overhead. | Unmeasured; no benefit attributed to it. Preserve output/readback contracts if revisited. |

Paths abbreviated as “renderer evidence” above are under
`throwaway/evidence/one-renderer/`.

## Controls that narrowed visual failures

Incomplete fixtures were development probes, not production comparisons. A
captured run that omitted BattleFrame's separate static upload and loaded only
tree scenery was invalid. Resolving simulation ids as appearance ids, a fixed
one-megabyte instance buffer, truncated vertex attributes, flattened batch
ranges and prematurely consumed incremental fog all produced genuine admission
or transport defects. Canonical source resolution, growing owned buffers,
shared packing/ranges and full fog resend corrected them. Environment/fog
placeholders and neutral material rows were never accepted as production parity.

The terrain-hole experiments retained negative and null results: disabling
cascades, skipping fog-mask compose, moving fog before world, and depth bias 1
did not fix the artifact; removing terrain depth removed terrain entirely.
`depthCompare: always` restored the early probe but was only a workaround.
Copying the native environment view matrix fixed an ABI error without changing
the 47.034991 image distance. Bundle-row, vertex-colour, instance-stream,
static-world-colour and shadow-floor probes were also null. Reusing the full
native resource graph finally closed parity; those probes do not identify one
causal lighting factor.

The final common pipeline uses reverse-Z `greater-equal`. Reverting only this
descriptor with the same native binary and WGSL restores all six old images
byte for byte. Keeping the correct shared rule is a named correction: 11 pixels,
21 RGB channels out of 5,529,600 pixels change, maximum 4/255; camera words and
current-current repeats are bit-identical. No screenshot threshold was widened.
Evidence: `throwaway/native-depth-control-result.json`.

Capturing a TypeGPU canvas after device destruction produced stale compositor
pixels in changing attempt 1 and black source pixels in attempt 2. Only attempt
3 retained each final native-size canvas before disposal. A recovery run that
overlapped lint-staged hiding the new harness was likewise invalid. Browser
startup failures before any game stage, including GPU-disabled/Metal probes,
provide no renderer timing result. Those attempts remain failures of evidence,
not favorable repeats or zero-cost samples.

## Verified production browser tradeoff

Production BrowserScene admission; 1280×720; paced changing `paris-corner` replay;
three interleaved backend repeats. Retain source records 60–2459 of 2461, with
2400 CPU frames and 2400 GPU samples per backend/repeat (7200 each in aggregate).
GPU windows contain 120 draws; queue drain/reset occurs outside CPU spans.
Means weight every valid sample; window p95 remains per-window, not a pooled
full-reel p95. Final output pixels and camera/shadow records match exactly.

| Metric | TypeGPU | Shared Rust | Whole-path change |
| --- | ---: | ---: | ---: |
| CPU mean | 1.575698 ms | 1.130157 ms | 28.28% lower |
| GPU mean | 10.477891 ms | 10.697395 ms | 2.09% higher |
| Render-call mean | 0.705688 ms | 0.335453 ms | 83.1% of total CPU reduction occurs in this span |
| Advance/upload mean | 0.870010 ms | 0.794703 ms | Remaining measured CPU reduction |
| GPU owned payload | 504,652,345 B | 482,749,361 B | Payload comparison for this replay |

CPU savings occur in every repeat. TypeGPU/Rust per-repeat CPU means:
1.642029/1.168856, 1.579083/1.149602, 1.505981/1.072012 ms. GPU means:
10.711363/11.111207, 10.398391/10.508401, 10.323918/10.472577 ms.
TypeGPU uploads during advance while Rust also prepares within render; span
boundaries locate savings without isolating language, library or packaging.
Rust additionally reports 193,721,908 B decoded bundle payload and 2160 B CPU
snapshot payload; TypeGPU has no equivalent CPU-heap measurement.
Evidence: `throwaway/final-perf/full-reel-final-1/{samples,summary,visual}.json`.

Frozen production plate 240 retains three repeats of 120 measured draws after
60 warmups. CPU TypeGPU/Rust means are 0.402042/0.149042,
0.477125/0.227000, 1.384458/1.216375 ms; GPU means are 8.871818/9.115237,
9.090577/9.126200, 9.121775/9.167318 ms. Both CPU paths drift substantially;
retain all repeats rather than promote a favorable first run. Single-repeat
pilots (including effect-heavy plate 660) are development evidence.
Raw data: `throwaway/final-perf/frozen240-final-1/samples.json`,
`frozen240-pilot-1/`, `frozen-pilot-1/`, and `visual-critique.md` in final-perf.

Earlier serial 640×360 frozen controls showed CPU p50 0.390–0.410 versus
0.120–0.130 ms and GPU means 3.635–3.653 versus 3.945–3.955 ms. Unified-runtime
repeats had slower controls on both paths (CPU p50 0.595–0.735 versus
0.275–0.300 ms). The earlier 120-frame changing cohort reduced CPU mean
1.1920→0.6775 ms (43.2%), but GPU drift included a per-repeat reversal. These
historical paired cohorts support their bounded whole-path results; differences
between cohorts do not isolate module packaging or individual optimizations.
Raw evidence: renderer evidence `shared-frame-profile/samples.json`,
`unified-runtime-profile/samples.json`, `changing-plate240-attempt3/`.

## Native profiling correction and limits

Apple M6, AC power, release, 1280×720, global GPU lock, one binary SHA-256
`77b473a710a927cfd9a00d4c7369d366c57b091c61d75eb146c1bef0a9e6c5cc`.
Metal previously resolved stale end stamps inside the frame command buffer;
sparse historical GPU summaries (including the earlier 11.57 ms control) are
not complete reel measurements. Resolving after render completion and rejecting
incomplete aggregates gives 7200/7200 changing and 360/360 frozen positive,
finite indexed samples.

| Native input | GPU repeat means | All-sample GPU mean | CPU repeat means |
| --- | --- | ---: | --- |
| Changing reel | 9.006299 / 8.990898 / 10.534978 ms | 9.510725 ms | 1.311635 / 1.255988 / 2.028345 ms |
| Frozen plate 240 | 8.911300 / 8.593995 / 10.216316 ms | 9.240537 ms | 0.787957 / 0.359128 / 0.765125 ms |

The third repeat slows in both CPU and GPU and is retained; the cause is
unisolated. Separate counter submission/wait changes profiling pacing: CPU spans
exclude it; frame intervals include it. Native bypasses surface acquisition,
copy and presentation, while browser includes canvas output. Native repeats ran
after browser, not interleaved. Cross-platform or earlier sparse/native timing
comparisons establish no renderer speedup or language/backend cause.
Evidence: `throwaway/final-perf/native-*-corrected-{1,2,3}/`,
`native-corrected-summary.json`, `native-corrected-window-analysis.json`.

## Retained behavior, lifetime and accounting

Production foundation/geometry/river retain their unchanged spatial gates;
raw reference differences of 262/330/464 pixels are not byte identity. Workbench
C remains exact while A prepares and B supersedes it. Missing F-16 source fails
the strengthened aircraft test before authored admission passes. The complete
app journey preserves forests, input/engagement acknowledgement, restart/history/
replay digests, one GPU/audio owner and zero retained observed wrappers on both
menu returns. Evidence: `throwaway/production-appflow-lifetime-3.log`,
`production-high-air-red.log`, `production-world-admission-frame-green-3.log`,
`production-frame-review/{raw-comparison.json,critique.md}`.

Queued readback, inspection return, resize, surface reconfiguration and actual
device recreation preserve exact pixels through production admission; fog
seen/unseen samples stay [1,0]. Recovery restores retained knowledge before
canonical uploads; the prior wall-visibility error has a red control. Ghost
alpha/depth/fog assertions stay unchanged. Evidence:
`throwaway/final-perf/retained-browser-gates-3.log`,
`retained-browser-gates/renderer-lifecycle.json`,
`throwaway/fog-recovery-probe-{red-2,green}.log`, `throwaway/lab-cards-{red,green}.log`.

Terrain helpers pass 120,000 distance comparisons (worst absolute error
0.00007218 m), cubic/linear exact-bit agreement and teardown lifetime checks;
WebGPU errors remain empty. Grass mixture/exclusion/retuning/brightness/stability
probes and fog visibility/cost probes pass. A stale presented tick caused the
garrison probe failure; the existing presented-publication wait corrected the
test without weakening its assertion. The full fog journey's unchanged cost
guard passes after a separately committed HUD coherence fix. Evidence:
`throwaway/ground-query-artifact8-material-all.log`,
`production-grass-probes-artifact7.log`, `production-fog-probe-green.log`.

The tiny-target regression now uses the canonical presentation floor
`MIN_TARGET_PX = 64` through `renderer_core::render_size`. The observed failure
and green fix are retained in `throwaway/final-perf/tiny-target-observed-red-2.log`
and `tiny-target-green.log`; this focused result is not full browser closeout.

The earlier pre-source-repair startup comparison uses the original TypeGPU source and then-current
shared-browser production builds, three fresh processes/backend/route in
rotating order, cache disabled,1280×720. Every repeat succeeds without page errors.
These are application-readiness observations over loopback, not actual display
presentation, network latency or filesystem-cache coldness.

| Route | TypeGPU readiness seconds | Shared Rust readiness seconds | Mean TypeGPU/shared | Response body bytes before ready TypeGPU/shared |
| --- | --- | --- | --- | --- |
| Frame lab, low quality |15.937/14.095/14.253 |5.551/4.712/4.939 |14.762/5.068s |617,155,511/618,577,749 |
| Small generated battle |15.788/14.174/15.583 |8.067/8.086/7.623 |15.182/7.925s |674,662,924–678,276,364/577,746,514 |

Context-level completed request sizes include workers; only requests finished
before readiness count. Backend-dependent startup duration changes how much
background asset warming finishes, so this response closure is not solely
required renderer downloads. Loopback transfer is uncompressed. Frame-page
JavaScript bodies are831,457/654,928bytes; battle JavaScript898,067/666,992bytes.
The unchanged game WASM is8,151,438rawbytes (2,760,967gzip;1,828,014Brotli).
TypeGPU presentation versus unified renderer/presentation/audio WASM:
3,439,054/5,037,821rawbytes;1,178,781/1,717,593gzip;796,893/1,126,591Brotli.
These are artifact sizes, not live memory.

Main-realm unified battle WASM reaches1,952,055,296–1,952,579,584bytes capacity;
TypeGPU's presentation WASM56,360,960–61,079,552bytes. Frame lab is78,381,056
versus3,801,088bytes. Both also admit the page geometry/simulation module
(~9.57MBbattle/~9.70MBframe). Managed JS heap observations and individual
request rows remain in the raw data. None measures live Rust allocations,
worker memory, total process residency or driver storage. The GPU payload
sharing result above is a separate matched within-Rust experiment.

Raw results, summary, exact build SHA256 identities and compressed artifact
counts: `throwaway/final-perf/startup-final/{results,summary,identity,wasm-artifacts}.json`.
The earlier candidate-only startup pilot is retained separately at
`throwaway/final-perf/cold-startup/results.json` and does not replace this cohort.

Maintained production-source accounting against original control
`5b52cabb57578d059eaa3a352bf2e38946742d4b` is **+3709 code lines**:
24,151 added / 20,442 deleted. This counts changed physical nonblank lines,
separates full-line comments, includes authored WGSL and its composition index,
and excludes generated expansion. Both endpoints of renamed paths are counted
with `--no-renames`; deletion-only or destination-only totals misstate complexity.
Production comments: 1,901 added / 3,383 deleted. Tests/harness code: 7,008 added /
3,260 deleted; their full-line comments: 98 added / 194 deleted. Generated code
is separately 19,548 net lines removed. Config/data is separately +31 lines.
These are reproducible line classifications, not parser SLOC or a complexity
score. `throwaway/final-perf/complexity.json` retains per-file classifications;
`tools/renderer-accounting.mjs` inventories current source and artifact surfaces.

## Verification reconciliation

Full Rust workspace passed in 1881.29 s. The later complete web run executed
195 files / 1296 tests: 1295 passed and one replay-setup import failed transiently.
Its unchanged focused reproduction passes, as do independent original-URL,
isolated-cache and shared-cache controls. Network diagnostics were added without
claiming an unproven functional repair. Final source/interaction regressions add
one test file and five tests; focused viewport/source/kit/regional/replay coverage
passes. This is reconciled coverage, not one all-green full-suite run.
The first full browser run attempted all48registered fixtures in1776.66s with
27failed assertions. Do not repeat it unchanged.

Source/readiness repair development evidence (`throwaway/final-perf/`):

- `air-hover-oom-stack.log` identifies the original trap as allocation failure in
  `wgpu` mapped-buffer initialization. Its probe sees1,048,740,232B decoded bundle
  payload and unrelated game wrecks/city kits in a small scene. Later unsafe-alias
  errors are secondary. Browser transport now scopes cached art before canonical
  prop selection, following the native worn-art contract; selected units retain
  their possible wrecks even with dynamic map props disabled. Source changes and
  observer admission must match before a renderer is built.
- `appearance-scope-red.log` and `viewport-completion-red.log` retain failing
  controls. Fast focused tests currently cover cached source exclusion, wreck
  closure and successful frame completion. `air-hover-admission-matched.log`
  passes the complete scene, including the previously crashing roof transition.
- `source-completion-browser.log`: weapons, fog, fog-look, facade and scenario all
  pass. Garrison fog now uses the rendered side's actual eyes. The prior rim
  failure captured a fully transparent canvas during asynchronous rebuild; actual
  submission completion repairs the evidence without changing the fog shader or
  any threshold. Facade's bare comparison hides material surfaces instead of
  preserving its static panels via dynamic-model suppression.
- `remaining-admission-scenes.log`: transport, garrison, supply, ground, cursor,
  city-lineup and city-ruins pass after source/LFS repair. Ground and lineup were
  expensive; preserve these valid results. `selected-sources-allies-readouts.log`
  then passes allies/readouts. `ally-help-fine-sampling.log` passes the remaining
  allied help predicate after CPU reproduction proved its old10-second samples
  missed a1.4-second combat halt. Deadline,100m threshold and halted predicate
  are unchanged. Readout selection now waits for real layout before the same
  strict selected-above-all assertion. These unrelated harness repairs are
  separate commits.
- `readiness-source-review/verdict.md`: independent Codex review reproduces a
  CPU-only cancellation defect. It is fixed: internal quality/world rebuilds
  report through their own viewport lifetime; explicit rebuild callers still
  receive rejection. `viewport-cancellation-red.log` and
  `viewport-source-cancellation-green.log` retain proof. Current source/viewport/
  kit/regional coverage is46tests (`battle-sources-final-tests.log`); typecheck
  and scoped complexity checks pass. No new hook warnings remain.
- `air-hover-reference-comparison/findings.md`: all five neutral critique concerns
  also occur in original TypeGPU across47paired full frames/crops. All four
  lost-contact crops and both ghost full frames are raw-identical. Unrecorded
  animation clocks preclude a general exact-parity claim for rotors/muzzles.

### Final source lifetime and purchase responsiveness

Current-only art scoping fixed allocation failure but initially regressed the
first reinforcement preview: `roster-complete-current.log` retains the original
250ms ghost timeout. The complete source closure now includes enabled purchases
for the human's faction before play, initial units, explicitly referenced history
(corpses, wrecks, crashes, remembered contacts and air transits), pending units
and subsequently learned kinds. Learned kinds remain until this battle/observer
ends. The page's cached union still does not decide GPU residency. CPU catalog
inventory in `source-lifetime-review/` measures the unit bundles only: own-faction
US502,956,748B, Europe611,459,656B, Eastern572,844,176B versus all-three
1,530,645,548B; these omit textures/scenery/skeletons/kits and are not live memory.
`roster-own-faction.log` restores the original preview gate and three unchanged
approved pictures. Final registered confirmation is in
`source-lifetime-browser-confirmation.log`.

A fielding request now has stable identity for all its actual inputs. Only that
request's result can reach the viewport; a new appearance generation cannot
meet an earlier requested-kind topology while asynchronous resolution runs.
`battle-sources-history-red.log` proves lost source retention can fail; its
repaired46-test focused cohort passes. This is lifecycle coherence, not an
extension of the simulation's visibility.

### Roster effect-history control and intended picture change

The failed full roster journey paused at tick12; old frozen first-shot controls
paused at1and3. Entry-chevron pulse uses the presentation clock, so these were
not matched visual states. The registered journey now pins an explicit paused
tick60 before actions. Baseline thresholds stay unchanged.

A separate exact-tick control isolates the remaining dust difference:
`roster-fielding-control/` runs original TypeGPU, original TypeGPU with only its
faction dependency stabilized, and current shared Rust. All finish at tick423
with equal complete observations and settled clocks. The unchanged TypeGPU
world reloads117times and loses its dust; stabilizing that tuple yields8loads
and exactly the same16effect instance records as shared Rust (one world load).
The stable TypeGPU/shared final images have0pixelmatch differences at0.1threshold
but1132different raw channels, so this is not raw pixel identity. The observation
and effect arrays are exact. This control changes neither rendering algorithms
nor simulation outcomes. The earlier frozen production pair similarly had
0versus16effects; differing frame rates alone do not explain mover dust.

Keeping presentation history is the intended repair. The order-picture reference
changes deliberately to show its existing authored movement dust, rather than
continuously resetting the observer to preserve an accidentally empty effect
pool. Only that reference changes; the first three pictures remain identical.
The user was shown matched full views. Fresh neutral inspection
(`roster-fielding-control/visual-critique.md`, receipts check passes) found shared
wedge/label/hatching concerns but no candidate-specific depth/missing-model defect.
These established appearances are not redesigned during renderer migration.

Army's old digest pin was independently stale: original TypeGPU and frozen
shared production both produce`fdd604ec48fc8b26` at tick90
(`matched-digest.json`), caused by an earlier mount-digest schema addition. The
pin repair is a separate commit. Ruler CPU staging and browser confirmation
(`ruler-cpu/findings.md`, `ruler-covered-approach.log`) preserve sight-edge,
remembered-contact and actual right-click requirements. One covered waypoint
reaches them; the old scout died before identifying anything.

The performance/startup numbers above belong to their named frozen builds;
subsequent source/readiness repairs are not silently credited those exact results.
The final confirmation below supersedes the older startup figures for the
completed candidate.


### Final complete-generation and interaction repair

A new enemy source originally withdrew the running presentation while its art
loaded. `useBattleFielding` now retains one complete topology/map-art/model-art
snapshot for the same battle and observer, retiring it on a scope change. The
new delayed-resolution regression fails on the previous behavior and passes with
retention, stale-result cancellation and art-only refresh identity preserved.
Raw audio publications remain independent of visual admission.

The first retained-generation browser attempt exposed overlapping admission:
a feed listener decoded a complete asset draft, then canonical framing detected
new topology and rebuilt another renderer from the same catalog. Serial tracing
confirms capacity 1.98 → 3.77 GB, a second `load_bundle` sequence, then the 4 GiB
WASM ceiling. It was not a concurrency-only failure or an overlay-size defect.
Source/world/model/proxy reconciliation now has one frame-boundary owner; it
chooses full rebuild for changed topology and an asset-only transaction otherwise.
The old eager appearance/world listeners are removed. The regression was observed
red before this correction. The matched serial ally-help journey now passes,
with a retained capacity high-water of 3,590,717,440 B; this remains a substantial
memory cost, not a claim of low live residency. Evidence:
`ally-memory-{serial,traced,reconciled}/`, corresponding logs and
`world-appearance-double-admission-{red,green}.log`.

Submitted interaction state includes pick boxes, own/enemy positions, observation,
clock and projection camera. Skipped surfaces leave it unchanged. Optional frame
fields retain staged values across a skip. A pose-only external-model change
requires reframing before submission; static review found the ordering issue,
and a regression observed the old pose before the repair. A separate regression
caught and fixed a React redraw effect accidentally returning a boolean cleanup.
Evidence: `skipped-picking-red.log`, `viewport-pose-framing-{red,green}.log`
(the green result is included in `viewport-final-review-fixes-green.log`),
`viewport-effect-cleanup-{red,green}.log`, `viewport-reviewed-final-green.log`.

The final independent focused review exited zero; its sole actionable finding
was pose-only framing, corrected above. The final frame scene preserves all
three approved images at zero differences and passes surface/device recovery.
The registered roster/allies/ally-help runs pass; the street vehicle tour now
checks the active catalog for reserved proxy assets rather than treating all
canonical listed instances as proxies. Source and viewport focused web coverage,
plus the replay-refusal regression, is green. The original full web run retained
one transient dynamic-import failure; unchanged focused and independent cold/cache
reproductions pass. Commit `1be68209e` adds failed-response/request diagnostics
without inventing an unproven functional fix.

The earlier 28.28% renderer CPU reduction was measured on a fully loaded frozen
replay before bundle-sharing and these source repairs. It cannot be attributed
to a loading freeze, the later Arc change or a stabilized React dependency.
The render span locates 83.1% of the saving but isolates no internal cause.


### Clean final startup confirmation

Candidate source: `d9a08f88b7050fb238badf77fb297c019a50bed1`, with the original
TypeGPU production build frozen. Three fresh browser processes per backend/route,
rotating order, 1280×720, network cache disabled. All twelve rows finish without
page errors. This task finished compilation before the cohort, held the canonical
GPU lock throughout, and retained the host process snapshot. System/background
activity was not artificially disabled. This is loopback application readiness,
not display presentation, external-network latency or filesystem/driver coldness.

| Route | TypeGPU seconds, all repeats | Shared Rust seconds, all repeats | Mean TypeGPU/shared |
| --- | --- | --- | --- |
| Frame lab, low quality | 14.981 / 13.859 / 13.117 | 5.109 / 5.305 / 4.549 | 13.986 / 4.988 s |
| Small generated US battle | 15.300 / 14.010 / 14.815 | 10.110 / 8.796 / 9.505 | 14.708 / 9.470 s |

Completed response bodies before readiness are 617,155,511 / 618,572,081 B for
the frame route, TypeGPU/shared. Battle is 667,436,044–674,662,924 / 425,107,014 B.
These context-level request closures include workers and any background warming
that finishes before readiness; they are not solely required renderer download.
Loopback asset transfer is uncompressed.

Main-realm renderer/presentation/audio WASM capacity in battle is
1,891,762,176–1,893,007,360 B, versus 56,360,960–60,882,944 B for TypeGPU's
presentation/audio module. Frame capacity is 57,606,144 versus 3,801,088 B.
Both admit the additional unchanged game module (~9.57 MB battle / ~9.70 MB
frame). TypeGPU also holds model/texture data in JavaScript; comparing these WASM
capacities is not a total application-memory comparison. They exclude worker,
managed-heap, external-array, allocator-live, process and driver residency.
The separate long-battle transition's 3.59 GB high-water remains material.

All rows, screenshots, timestamps and build identity are retained in
`throwaway/final-perf/startup-confirmation-final/`. Earlier
`startup-shared-bundles-final/` overlapped compilation during its first pair and
remains development evidence; all rows are retained, not selectively averaged.
Final artifact SHA256 and raw/gzip/Brotli sizes are in the confirmation's
`artifacts.json`; the inventory tool provides the current ownership surfaces.


Final standalone WASM artifact sizes (gzip level 9, default Brotli quality 11):

| Artifact | Raw bytes | Gzip bytes | Brotli bytes |
| --- | ---: | ---: | ---: |
| Unchanged game module, both builds | 8,151,438 | 2,758,109 | 1,828,014 |
| Original TypeGPU presentation/audio module | 3,439,054 | 1,177,157 | 796,893 |
| Shared renderer/presentation/audio module | 5,030,966 | 1,716,754 | 1,125,194 |

The final shared module SHA256 is
`8fb6fa618b8ed5d77d4a5f604a3c4ddaea5ecb9b45861cb7a6aa918c5639abf8`;
the original presentation module is
`d1f93c80b684d43d863b1b812ef1d7a80745e1d75630939c5aaff6362990fbdd`.
The identical game module is
`2815673c48f8a7c0cce424833d5f8cce94947bdda47f7917c6ba10b61e911825`.
These individual compressed sizes do not include JavaScript, textures or
transport headers and do not replace the observed route request closure.

The close-spec architecture audit found only a wording qualification: browser
surface presentation supplies a final display-copy shader, while shared core
owns production scene shaders/pipeline policy. The archive now says so. The
independent evidence audit recomputed all CPU/GPU aggregates and checked startup,
artifact identities, memory and visual controls. Its provenance finding is
resolved by retaining the matched roster full views and representative air-hover
full frames in the archive; copy SHA256 proofs confirm they are the cited controls.
All archive/root-pointer links resolve. The layout-size checkpoint split changes
no descriptors: nine complete visibility-layout expressions are unchanged after
whitespace normalization, and all four pipeline ABI/entry tests pass.

## Cumulative PR review

The cumulative review covers the original TypeGPU control through the full
shared renderer port and the subsequent memory fixes. Independent scopes audit
core/native, browser/presentation, and tests/build/deployment, in addition to a
separate cumulative Codex diff review. Their detailed coverage and dismissed
suspicions remain in `throwaway/final-perf/pr16-review/`.

The structural cleanup removes the second native display readback implementation
in favor of the shared bounded snapshot owner, preserving opaque native exports.
It removes abandoned JavaScript scar-upload interfaces and their WASM wrappers,
unused browser glass/fog-shader values, old spike pipeline-export diagnostics,
unconsumed target-generation/debug-group scaffolding and unused dependencies.
Canonical shadow camera sizing and cascade count come from presentation. The
remaining fog/grass query bindings have real diagnostic or configuration users;
the production renderer still consumes Rust records directly.

All obsolete doc links to deleted owners are repaired, and native documentation
now describes the completed boundary rather than an incremental port. Exact
benchmark numbers have one canonical home in research; the architecture README
states the principles and points to the measurements. Trailing blank lines are
removed from shader modules with all 485 canonical composition blocks unchanged.

The code review finds no confirmed gameplay, shader, lifecycle, packaging or
build regression. After cleanup, the full web suite passes 196 files / 1,301
tests. Strict workspace Clippy, all-target workspace checking, rebuilt combined browser WASM, TypeScript,
five shared memory/ABI tests, eleven native GPU tests and the registered browser
frame lifecycle/device-recovery scene pass. Six native capture plates retain the
existing raster variation (21 differing RGBA channels, maximum delta four across
all plates). A fresh unprimed reference comparison finds no material browser
visual regression. The earlier full Rust workspace run remains valid except for
the focused renderer paths rechecked here; this review does not claim a new full
Rust suite run.

The final web run initially exposed intermittent replay setup and navigation
worker tracking failures. Browser harnesses now own separate Vite optimizer
caches. GPU-less storage setup uses an isolated origin page with Vite's React
preamble; the navigation check waits for the lobby preview to release its worker.
The importer uses resolved React imports rather than a hardcoded optimizer path,
and the asset-serving build owns its temporary entry. The four cold-cache browser
cases pass together, with assertions and time limits retained. A focused review
also fixes temporary-cache cleanup on Vite startup/close failure. Presentation's
lab-probe contract moves to its own type module to keep the browser binding within
the existing file-size budget, retaining its public type export.

The diagnostic test audit does identify narrower surviving coverage for
asynchronous timestamp reset/saturation, exact terrain/water/scenery issued-work
counts, Windows shader compile-cost shape, and mixed-weight vertex quantization.
No runtime defect is demonstrated by these gaps. Future changes to those owners
should pin those contracts in Rust, rather than recreating deleted TypeGPU mocks.

The remaining memory overlap across complete asset generations is an explicit
architectural cost, not hidden by the cleanup. Memory, CPU and GPU measurements
continue to belong to their frozen artifacts; no new performance effect is
attributed to this review.

Cumulative source accounting after this review adds 24,126 and deletes 20,512
noncomment production lines, a net increase of 3,614. This uses the same physical
line classification and original TypeGPU baseline as the frozen accounting;
tests, configuration, docs and generated expansions remain separate. The
review's ownership cleanup reduces that increase without turning the port into
a source-line saving.
