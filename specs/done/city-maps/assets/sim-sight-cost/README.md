> Frozen historical evidence. Original identities and rejected arms remain scoped to their source; current completion and user-authorized deferrals are recorded in [completion evidence](../../evidence.md).

# Frozen SA5 sight-cost evidence

This is the performance-only sight/fog comparison against the original simulation,
not acceptance of later lane rules, generated encounters, rendered frames or the
complete G0 gate. The manifest pins every included file, source
revision and referenced generated-map identity with SHA-256. Maps remain referenced
rather than copied here. No executables or installed profiling runner are included.

Baseline simulation is `e5cf680b`; baseline city placement adds only the force-size
harness in `2fcbb104`. Final uninstrumented source is `21ec37fb`, including direct
world page lookup, lazy occlusion tiles, remembered-body indexing and tail-first
complete union preflight. This was measured before integration with named movement,
garrison and forest-cover rule changes. Some captured reports label the source uncommitted; their unchanged production
bytes are pinned by the subsequent final source commit. Native reports use macOS
aarch64 release builds and process-retired instructions. Process CPU counters use the macOS Mach
timebase conversion in the source-pinned report helper.

| Arm | Before instructions G | Final instructions G | Identical final digest | Final RSS MiB |
|---|---:|---:|---|---:|
| Mixed Small, 6 km | 354.4 | 153.9 | `63211f98d8877be6` | 216 |
| Metro Large, 10 km | 463.7 | 142.0 | `487cd21bab9c7462` | 501 |
| Endurance, 5 battle minutes | 8581.9 | 1439.4 | `ce696ab655fff2d4` | 259 |

City probes run 30 battle seconds with 100 units per side and movement reaches of
0.03 / 0.018 map widths respectively (180 m). These bounded probes measure sight
with a full force; they do not prove that units complete an edge-to-edge crossing.
The final city maxima are 22.8 / 21.6 ms wall and 21.89 / 19.39 ms process CPU,
with zero whole-step wall ticks over 33 ms. All six final village quick digests
match the frozen baseline; final serial trials retire 1149 G instructions.

## Timing scope and failed arms

The final uninstrumented endurance whole-step maximum is 222 ms wall / 55.36 ms
process CPU, with 237 wall ticks over 33 ms (p95 27.8, p99 47.0). Its instruction
reduction is 83.2%; those whole-step clocks do not imply a whole-simulation 33 ms
pass. The final quick report records host load 23.84 / 22.87 / 24.16; startup and
wall clocks varied with the shared host. Instructions remain the cost comparison.

The separate final attributed endurance arm has the identical battle digest.
Its temporary patch brackets only both sides' `sense` calls and the due
`sweep_fog` call, including their knowledge/learning work. `sight::snapshot` occurs
before this bracket; other battle stages and publication are outside it. Across
all 9000 ticks, combined CPU peaks at 22.477 ms and bracket wall time at 29.766 ms;
sensing alone peaks at 1.699 ms and fog alone at 21.921 ms. No measured sight
bracket exceeds 33 ms in either clock. The raw per-tick file preserves every
sample. The summary arrays are `[tick, sense_cpu_ns, fog_cpu_ns, wall_ns]`.

The original captured attribution diff includes the then-uncommitted private-map
cleanup. The extracted `sight-tail-attribution.patch` contains only profiling
against final source `21ec37fb`; the historical absolute report-helper import
must be adjusted if reproducing in another checkout. Neither patch is installed
in production. Instrumented whole-step instruction totals include measurement
and logging overhead; the quantitative comparison above uses the separate
uninstrumented outputs.

The earlier forward-preflight attribution failed: its bracket also included
`sight::snapshot`, reached 36.330 ms CPU and had 13 CPU ticks over 33 ms. Its derived
summary retains every violating CPU row and hashes the complete raw source log,
which remains in the main checkout's `throwaway/city-maps-sa5/`. The included
loaded city core arms are earlier source through `c4aeff4d`: wall maxima 84 / 196
ms, and Large CPU 35.55 ms. They are failed timing evidence, not accepted final
runs. Final repeats and the exact tail-first proof are retained alongside them.

## Input and reproduction boundary

The two `*-map-SOURCES.json` files preserve the actual generation identities:
seed 4, generator `layout-3`, presets `layout-presets-5`, supplied-input hashes,
physical catalogue hash and canonical map hash. The manifest names the local
JSON locations and independently hashes their bytes. These prototype source
identities do not certify finished-source or visual acceptance.

The original tools are the existing examples: `city_report <map.json> 30
<reach> 100`, `village_report --quick --threads 1 --save <file> --compare <baseline>`,
and `endurance_report 5`. Their frozen source revision supplies the rules and
encounter. No new performance run was used to assemble this evidence. Final
combined lane checks, replay/wasm gates and integrated river timing belong to
the coordinator's closeout.

Raw evidence (reports, logs, patches, recordings and shots named above): tag `city-maps-evidence-2026-10-01`.
