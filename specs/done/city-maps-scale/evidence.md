# Identified scale admission evidence

Frozen source 82372fe7 Native/Wasm state, wire, real decoder and requested-heap
runs are complete on layout 9. The integrated hull correction has a separate
bounded native proof; no full latest-engine 9,000-tick run is claimed.
**Browser admission is GREEN** for the identified layout-9 throughput/contact
and corrected reset component proofs below. The original combined scene process
retains its old-reset failure; it is not relabelled green. Cost and memory scopes
remain separate measurements, not added joint maxima.

## Identity and workload

- Native measured source: `82372fe792a6fba327f7ea344b8b3ebc16f661ce`.
- Shared engine: `1d9d85aad2b391afb2809063e115f9248bcb9cb5fa8620f7592264b994f3498f`.
- Generated map: `bdadf296b14b34a17c8fa41b189ac4c67eb09a86849af9612f9faa0b347d3be9`.
- Rules: `b00f7d5e7f3db01f26cb576e201853da96e27b30d2d85e67f13a56b37676c3fa`.
- Physical catalogue: `6b0a5e8be69e9309bd16db843fa7ecaad672616874978ecfde0fd9332c3e6316`.
- Preset bytes: `8244e667d22fc320642e3ff329d389daf010aaf5fad891388ffc323a75989c61`.
- Generation config: `af26dc0ab98ce2eb4b5c6d559af7ec6586fa2fe822f10bf210942bfa828ef861`.

Metro Large, map seed 4, battle seed 4, layout-9 / layout-presets-8, 10 × 10 km,
100 living units per side at the start. Late adds 20,000 corpses and 2,000 wrecks.
The full world is retained around the production contact workload. The compiled
map has 12,933 buildings and 39,812 authored parts. Runtime body counts include
implicit forest bodies: 99,856 early / 101,856 late; they are not authored parts.

**Early identity:** factory scenario `e252223483d5b5c8590d26c0630ff8974cd692316069263eda442916586a29d4`;
materialized runtime scenario `e252223483d5b5c8590d26c0630ff8974cd692316069263eda442916586a29d4`;
runtime map `bdadf296b14b34a17c8fa41b189ac4c67eb09a86849af9612f9faa0b347d3be9`. Replay scenario digest
`87542f50ae552919`, config digest
`9fa788cba55aa2ce`. Initial state `8fcd1e3acbc55257`;
final state `a51675185057ec67`.

**Late identity:** factory scenario `2289f7a246fcfd6755f4e82c6d44098ff9fe9aed53da151628116bf8cab690c0`;
materialized runtime scenario `70bc7b7584beecad366f775f184a939415473b6043ebe2ab58b44bace563b048`;
runtime map `553e6e055b60a3f92bc825c05f2e199f066f90ffe57059314825b4d2f98f871b`. Replay scenario digest
`2e1696827b427e97`, config digest
`9fa788cba55aa2ce`. Initial state `a1c2946126938108`;
final state `e969311ff1945caa`.

Factory bytes and serialized/reparsed runtime identity are separately recorded;
both runtimes consume exact factory bytes. No hash is substituted for another.
The historical browser control source is `d805a1ea`, after incoming appearance
and ground-ledger changes. `current-main-input-equivalence.json` separately
proved exact frozen factory bytes at that checkpoint, with unchanged Rust engine
and map/preset/catalogue inputs. This transfers input identity, not native timing.
The hull correction is integrated at `9211c627`, engine `d58fb441…`. Fresh browser
controls identify source `98d81ef1d11601d3adc82ebd4aea69fb7e0c9fac` and Wasm SHA256
`78b4b9f547d43346c2cddebba08daaaaeacf5015ad18f395dbe61a7acf615963`.
Their actual preparation receipts retain Metro Large seed 4, layout-9 / presets-8,
the map/config/catalogue identities above and the full world in both arms. Frozen
complete evidence retains source `82372fe7` and engine `1d9d85aa…`.

Later quiet integration at `3768021bdd3e5c45282bdae3e9f22b8a2e1607ff` includes the
reset correction `fa331ca0` and parent layout-11 / presets-10 town-junction and
presentation work. The fingerprinted sim source/build, contract, game-wasm and
Cargo inputs are unchanged from `98d81ef1`; the simulation engine remains the
identified hull engine. Incoming generator, preset and presentation changes have
separate parent ownership. A fresh integrated Wasm build emits layout-11 and the
same hull-engine identity; that is generator readiness, not a new battle measurement. These receipts measure the frozen layout-9 reference,
not the current parent's layout 11, and no latest-layout input or performance
equivalence is claimed.

## Complete battle and delivery

Both actual Native/Wasm arms match every authoritative digest and raw publication
from construction through tick 9,000: 9,001 records per arm. Final resync and red
switch also match exactly. No active record violates the 19,800 B whole-record
limit. Snapshots are outside active-record statistics.

| Measurement | Early | Late |
|---|---:|---:|
| Active wire mean / p95 / max B | 1,054.6 / 3,240 / 17,152 | 1,975.6 / 4,864 / 19,328 |
| Cold blue snapshot B | 452,360 | 871,852 |
| Final blue resync B | 470,864 | 912,980 |
| Final red switch B | 467,648 | 908,676 |
| Final living / corpses / wrecks | 141 / 361 / 16 | 149 / 20,331 / 2,015 |
| Rounds launched | 32,893 | 107,653 |

Real production JS decoding processes all 9,001 records in each arm. Six complete
snapshot samples per arm compare semantics and raw five-byte ground marks;
retained prior snapshots remain exact. Malformed same-generation retry, final
resync and side-switch reconstruction pass. Oracle retention/staging is scratch,
not product allocation or decoder-throughput measurement. Raw carriers use u32
aliases at decoding and the worker's Float32Array same-type `.set` preserves bits.

## Native cost: plain System allocator, instructions retired

Production step and packing are bracketed separately; counter dispatch is
included. Paired IPC, stream/oracle hashing and report output lie outside these
brackets. Heap hooks are absent. Construction is separate. These actual costs
are not a before/after comparison across different input identities, and no
loaded clock or paired-harness elapsed time is browser throughput.

| Measurement | Early | Late |
|---|---:|---:|
| Construction G instructions | 23.155 | 26.179 |
| Step mean / p95 / max M instructions | 167.359 / 296.674 / 54018.237 | 284.103 / 463.595 / 83912.541 |
| Packing mean / p95 / max M instructions | 2.391 / 3.747 / 11.316 | 5.885 / 7.582 / 22.526 |

| Production phase | Early mean / max M instructions | Late mean / max M instructions |
|---|---:|---:|
| Orders | 13.964 / 53800.206 | 23.944 / 83598.727 |
| Navigation | 0.142 / 17.821 | 0.475 / 19.751 |
| Movement | 23.943 / 2326.083 | 35.332 / 2481.185 |
| Flight | 1.294 / 6.817 | 6.371 / 22.209 |
| Sight | 39.529 / 106.977 | 48.729 / 114.569 |
| Fog | 24.898 / 116.334 | 27.075 / 117.203 |
| Learning | 30.609 / 110.617 | 35.458 / 127.540 |
| Weapons | 22.359 / 122.698 | 77.599 / 214.250 |
| Observation | 10.376 / 25.331 | 27.681 / 40.679 |
| Other | 0.229 / 1.745 | 1.423 / 19.771 |

Phase p95/max values are not summed into an invented joint peak. Scripted order
certification spikes remain in the whole-step records; average cost does not
hide them or establish truthful movement admission for a refused journey.

## Memory: distinct owners and scopes

A separate 9,000-tick allocator diagnostic matches each System control's final
digest. It counts Rust requested layouts for Battle/Publisher. Active brackets
exclude logging/oracle staging. Reallocation overlap is the conservative old+new
request bound; it is not an observed allocator-backend/RSS peak. Construction
and active allocation peaks are distinct. Whole-lifecycle maxima below are maxima
across read/parse/construction/cold/active/resync/switch stages, never their sum.

| Requested native heap, B | Early | Late |
|---|---:|---:|
| read frozen runtime scenario (end live / stage peak / overlap) | 44,981,789 / 44,981,891 / 44,981,891 | 45,741,319 / 45,741,421 / 45,741,421 |
| parse runtime scenario (end live / stage peak / overlap) | 92,826,816 / 93,071,527 / 93,071,527 | 94,081,646 / 94,082,542 / 94,211,983 |
| production battle construction (end live / stage peak / overlap) | 324,475,794 / 391,880,412 / 425,270,620 | 338,006,206 / 403,991,411 / 436,890,099 |
| cold blue publication (end live / stage peak / overlap) | 277,821,923 / 277,821,923 / 277,821,923 | 292,252,343 / 292,252,343 / 292,252,343 |
| same side blue resync (end live / stage peak / overlap) | 290,499,816 / 290,500,464 / 290,517,616 | 305,421,197 / 305,421,845 / 305,479,193 |
| red view switch (end live / stage peak / overlap) | 290,515,104 / 290,515,752 / 290,968,112 | 305,458,021 / 305,458,669 / 306,330,521 |
| Active peak live requests | 418,616,216 | 457,699,995 |
| Active conservative realloc overlap | 418,616,216 | 458,088,603 |
| Whole-lifecycle peak live requests | 418,616,216 | 457,699,995 |
| Whole-lifecycle conservative realloc overlap | 425,270,620 | 458,088,603 |
| Final retained requests after Battle/Publisher drop | 1,721 | 1,720 |

The early whole-lifecycle overlap is **425,270,620 B during construction**,
larger than its 418,616,216 B active peak. The late whole-lifecycle conservative
overlap is 458,088,603 B during active work. These figures do not include browser
page, worker JS, graphics, or a claim about process RSS.

| Other independently measured memory, B | Early | Late |
|---|---:|---:|
| Standalone Wasm Battle/Publisher linear-memory high-water capacity | 489,029,632 | 499,974,144 |
| Decoder current word/fog buffers peak | 290,928 | 779,508 |
| Decoder old+new distinct word/fog buffers peak | 579,088 | 1,555,108 |
| Two alternating worker-credit capacity bound | 1,375,584 | 2,656,684 |
| Credit growth old+new bound | 1,827,944 | 3,528,536 |
| Coarse two-credit maximum bound | 1,883,456 | 3,651,920 |
| Coarse credit growth maximum bound | 2,825,184 | 5,477,880 |

Wasm high-water is exposed linear-memory capacity, not live requested memory, and
excludes browser generation/page/renderer/JS. Decoder numbers count typed-buffer
ownership and replay the existing alternating doubling-credit rule. They exclude
JS objects, ground Map oracles, deliberately retained snapshots, captured files,
source and Wasm heap. The browser owns complete page/worker/runtime overlap.
No independent maxima are added into a fabricated simultaneous peak.

## Integrated hull correction — bounded native proof

Hull gathering now rejects a non-hull unit before member liveness, through the
existing pure query. It adds no cache, retained state, geometry policy or wire
change. The matched controls use source `9211c627`, candidate
engine `d58fb441…`, the same frozen layout-9 factory bytes and rules above, and
900 ticks per arm. Late whole Weapons improves 35.418%, exceeding the declared
5% hypothesis; construction plus stepping improves 7.813%. Early does not
regress in either measured bracket. All 901 construction/tick digests and both
complete side observation hashes match in each arm; serialized replay content
matches after excluding engine identity. This does not claim a new from-replay
execution or old-engine playback admission. The focused hull/leaning/weapon
checks remain separate from this cost proof. No complete latest-engine 9,000-tick
Native/Wasm, memory or browser result is inferred from the bounded comparison.

The frozen control engine is `1d9d85aa…`; both arms use rules `b00f7d5e…`.
Early factory/scenario bytes are `e2522234…`; late factory bytes are
`2289f7a2…`, with reloaded scenario `70bc7b75…`. Early/late physical map
identities are `bdadf296…` / `553e6e05…`.

| Counted work over 900 ticks | Early control / candidate G instructions | Late control / candidate G instructions |
| --- | ---: | ---: |
| Whole Weapons | 56.111 / 55.344 | 120.526 / 77.838 |
| All stepping phases | 356.067 / 355.192 | 560.004 / 514.219 |
| Construction | 23.230 / 23.208 | 26.229 / 26.212 |
| Construction plus stepping | 379.297 / 378.400 | 586.233 / 540.431 |

Counter reads are included; input parsing, complete-observation hashing and
output are outside those brackets. The unchanged control rejects at zero gain.
The public hull test covers interleaved live/dead vehicles, fallen/mixed infantry,
ordered IDs and geometry; removing vehicle liveness retains a dead jeep and
fails that control. Original raw receipts were recorded in the original scale
worktree's ignored `throwaway/hull-cost/`; this is retained historical evidence,
not a new current-build or browser cost claim.

## Reuse of the frozen full functional and requested-memory proof

The only production simulation change after `82372fe7` is the hull-presence check
before liveness. Liveness and hull mapping are pure. A non-hull row could not emit
a Hull before; a hull row keeps the same health predicate and geometry. The
returned Vec keeps its order, yielded rows and allocation sequence; no persistent
state, observation, publication or retained allocation is introduced. This logic,
the actual 901-record paired checks and focused public regressions justify reusing
the frozen full functional/requested-memory proof for this exact change and input.

That is an explicit reuse argument, not a latest-engine 9,000-tick rerun, sampled
900-tick evidence promoted to 9,000 ticks, or a new allocator measurement. All full
record digests, wire distributions, native instruction figures and requested-heap
peaks above retain their old source/engine. The newer bounded native cost and actual
browser measurements have their own identities. Native execution cost changes;
engine identity changes; old-build replay admission is not transferred. Browser
page/process/GPU memory must be measured separately. A later input, layout or
simulation change requires its own review of whether this reuse still applies.

## Browser admission GREEN — identified component proofs

The actual hull-engine browser controls at source `98d81ef1` run both full generated
arms for 300 seconds. Early advances 8,515 ticks in 300.066 s (28.377090 Hz), late
7,823 ticks in 300.112 s (26.066935 Hz); **both meet the 25 Hz floor**. Public contact
checks pass with 61 / 68 observed own shooters. Chromium 148.0.7778.96 uses Apple
metal-3. The historical `d805a1ea` late control was 22.0042 Hz; these are separate
actual runs, not a wall-clock attribution of the hull instruction gain.

Worst sampled 10-second-window frame p95/p99 are 92.985 / 158.415 ms early and
106.965 / 168.180 ms late. They are maxima across sample distributions, not a
combined-run percentile or Large-city frame/tour admission. Actual preparation
reports 380,764,160 B / 384,172,032 B Wasm capacities early/late; these prepare/world
figures are separate from the frozen full Battle/Publisher linear-memory high-water
values above.

| Fresh browser memory measurement | Value and scope |
|---|---|
| Late user-agent-specific page + worker snapshot | 1,865,250,762 B (1,778.8 MiB); snapshot, not lifecycle maximum |
| Early / late sampled main-thread JS heap peaks | 1,470.590 / 1,520.206 MiB; 30 ten-second samples per arm, excludes worker heap |
| Early / late sampled GPU buffer-byte peaks | 375,929,580 / 375,675,520 B; measured separately from texture and heap peaks |
| Sampled GPU texture-byte peaks, both arms | 391,069,557 B; live device allocation accounting |
| Sampled runner/descendant RSS sum peak | 6,212,878,336 B (5.786 GiB); one-second sampling, 735 samples across endurance and benchmark |

The RSS record covers the dedicated scene runner and descendants, including Vite,
Chromium browser/renderer/worker/GPU processes. Its sum may double-count shared
pages and miss brief peaks; it is neither requested heap nor isolated page memory.
It records process exit 1 because the original reset fixture failed. UA-specific
memory, JS heap, GPU allocations and process RSS overlap and are not added together
or treated as interchangeable budgets. No measured native requested-memory result
is described as whole browser memory.

The original tick-90 reset fixture on this fresh source remains **RED**: 591 buffers
and 41 textures each time, identical texture bytes 391,069,557 B, but buffer bytes
373,618,300 / 373,618,876 / 373,619,132 B. Counts alone do not satisfy exact resource
stability. The corrected harness controls publication arrival and first draw
history through one-tick paused steps and an external RAF fence. Native page and
worker clocks remain real; only controlled reset callbacks use a settled native
clock. Ordinary callbacks retain native timestamps. Exact counts and bytes remain
the gate; no tolerance or weaker resource rule is accepted.

Three full generated corrected controls are exact at tick 90 / presentation clock
90, digest `2356bf791e79ba9f`, the same ordered 49 lying soldier IDs and camera,
with no pending buildings and equal buffer label/size/usage multiplicities. Each
has 591 buffers / 41 textures, 373,617,724 buffer bytes and 391,065,461 texture bytes.
The actual retained-16-B mutant keeps the same presentation but yields buffers
592 / 593 / 594 and bytes 373,617,740 / 373,617,756 / 373,617,772; it fails actual GPU
resource equality. Both runs terminate as expected. These controls identify the
same `78b4b9f…` Wasm as the full throughput run, helper SHA256
`57191c4c251dfdb0ff73d6ab219bbc985150107ed3f02cc4d8dba993476deeb5` and RAF seam SHA256
`d0efc6cde6b9018a237423ea33ffa415955305f46547883409e35c0258f3812f`.

The tracked harness retains the original `resetAllocations` GPU-count/byte shape
and adds `resetStates` containing `presentation` plus `gpu`. It uses the shared
`buildingsSettled` owner, which now explicitly awaits drawn frames and inspects
completed pending state. A focused red/green probe showed the old async polling
returned with roof/glass rows pending; the corrected owner returns only with none.
The presentation helper is unchanged; its comment now states that death-animation
starts depend on earlier drawn frames. No pose, renderer, authority or dependency
change is needed for this correction.

The actual tracked default saved-map scene passes all checks with one-second arms:
three exact tick-90 states at digest `66ed091b693826ee`, no lying IDs, 422 buffers /
41 textures and 338,322,576 / 261,786,461 buffer/texture bytes. This saved-map result
uses the earlier flat `resetStates` metadata form, before its mechanical
nesting under `presentation`; the GPU contract is the same. The actual tracked
full-generated retained-buffer mutant exercises the final nested metadata and
passes the presentation check at the same generated tick/clock/digest/49 IDs. Its
only failure is the unchanged resource gate: buffers 595 / 596 / 597 and bytes
373,617,788 / 373,617,804 / 373,617,820, exactly one further 16-B allocation per reset;
textures stay 41 / 391,065,461 B. The negative runner checks page errors are absent
and no unrelated gate fails, and terminates successfully after detecting the
expected resource failure. Its one-second early/late arms are setup controls,
not the 300-second throughput proof.

Corrected-reset receipts are `reset-controlled-current/{receipt,resets}.json`,
`reset-controlled-mutant/{receipt,resets}.json`, `reset-green-mutant-batch.log`,
`reset-tracked-default-green.log` with `reset-tracked-default-evidence/telemetry.json`,
and `reset-tracked-mutant.log` with `reset-tracked-mutant-evidence/telemetry.json`.
The reviewed tracked reset helper, shared lab helper and scene have SHA256s
`1082ef8c6c90a98267807cc7a11847387daad9cd4fe1a65cba9494fa825b44f8`,
`34548a333116648551864443ac3c9448d853eefaf3d8acd03874e60b738ed59d` and
`f751810c3fd03bbcd7e1e5dadf8265d89bfe49c22a1e288a5bba52547432b894`
respectively. These hashes identify the measured tracked reset version. Later
commit `9d1b1754` removes only its unused default tick argument; its sole caller
still passes 90. The final helper SHA256 is
`54021256418d089be353a6796ae2a7aa585ea59b9afc6d34e4f49c230128af29`.
This shape correction preserves the measured caller's behavior. Reset staging
controls draw history; the 300-second rates retain source `98d81ef1` and their
original full-run receipt. No new full throughput run
is inferred from a short reset check, and no GPU instrumentation ships.

The affected short village benchmark completes 60,024.615 ms with 2,985 frames and
49.730 FPS; frame p50/p95/p99 are 16.930 / 25.300 / 32.790 ms. CPU p95 is 1.885 ms;
GPU mean is 5.482 ms with worst sampled-window p95 21.040 ms. At 1920 × 1080, DPR 1,
the village-contact v1 / village-contact-6300-v1 tour advances tick 6,300 to 8,101
and passes the existing phase, drawn-camera, contact and audio checks. This is an
affected regression control, not Metro Large frame/tour admission. Its sampled heap,
buffer and texture peaks are 415,902,121 / 234,031,748 / 405,928,349 B, each reported
separately. No benchmark maxima are added to the endurance maxima.

Fresh receipts live under `browser-admission-hull/`: `endurance/telemetry.json`,
`benchmark/report.json`, `process-memory.json`, `source-receipt.json` and `scenes.log`.
The original combined scene run remains failed solely at its superseded reset
fixture. Admission combines the actual 300-second throughput/contact proof with
separately identified corrected-reset positive and negative controls; it does not
claim that original process passed. Native823, bounded hull921, browser98 and
tracked reset-helper identities remain separate. No latest-engine full 9,000-tick,
new heap gain or later parent layout result is claimed. Historical parent proofs
and later generator/presentation work retain their separate owners.

## Provenance and historical limits

Current machine-readable source: `admission-anchor3-summary.json`; paired,
actual-decoder and allocator terminal logs are named `admission-anchor3-*` in
ignored scale-lane scratch. Input equivalence is a separate current receipt.
Historical numeric controls remain canonical in parent C05/C06/C07 leaves.
Their distinct identities are not current-main performance comparisons. C54's
Mixed Medium seed-9 refusal and historically pending generated infantry journeys
remain explicit parent findings; fresh idle/refused extensions do not prove
arrival. Focused current corner/centroid arrival and replay checks prove their
local repaired contracts only. Browser screenshots are measurement diagnostics,
not baseline design references.
