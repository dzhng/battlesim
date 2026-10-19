# Performance choices

## Sound

### Baseline the browser before selecting an intervention — 2026-10-19

Two clean production runs of the browser's existing `graphics-fps` reel were
recorded before any optimization was chosen. The reel covers the same two maps
used by the repository's native comparison. This keeps a low 1% result from one
map or one noisy run from becoming a renderer change. The alternative would be
to pick the largest historical candidate—GPU residency, static packing, or
startup hashing—without proving that it explains the current player-visible
symptom.

- **Gap:** the placeholder named candidates but had no current measurement.
- **Reach:** later passes must profile the reported CPU/frame-pacing symptom
  before changing a shared renderer pass.
- **Verdict:** sound. The two runs reproduced the trigger while keeping the
  existing production entry point and output contract.
- **Confidence:** high.

## Needs user

### Choose the first intervention after CPU/frame-pacing profiling — 2026-10-19

The repeated `market-town` 1% low is below the agreed threshold, but its GPU
mean is about 11 ms while CPU p99 reaches about 18–21 ms. That evidence points
away from GPU saturation but does not identify whether static model packing,
another main-thread span, or display pacing owns the spikes. The next pass must
profile that path, then the user chooses the candidate under the balanced
tradeoff policy. A reversible provisional call is to profile first and change no
renderer code until the dominant span is named.

- **Gap:** the original placeholder did not name an owner for the new trigger.
- **Reach:** this decides whether the next spec slice belongs in renderer CPU
  packing, the browser frame loop, or measurement infrastructure.
- **Verdict:** needs-user after profiling; provisional call is profile first.
- **Confidence:** high.

## Sound

### Do not promote static caching from an inconclusive profile — 2026-10-19

The first steady-state CPU profile did not show static-model packing as a
dominant span. Most of the sampled interval was idle, and the active samples
were divided among WASM presentation work, GPU buffer writes and garbage
collection. The safe decision is to keep the current static-buffer owner and
measure frame pacing and allocations together before changing it. The
alternative would be to introduce a two-buffer static cache because an older
estimate mentioned four milliseconds of repacking; that would add stale-record
risk without explaining this run's low 1% frames.

- **Gap:** the historical candidate estimate predated the current baseline.
- **Reach:** future renderer work must be tied to a measured dominant span, not
  to the largest old estimate.
- **Verdict:** sound. The profile is evidence against promoting the cache now.
- **Confidence:** high.

### Extend the trusted reel instead of creating a second timing harness — 2026-10-19

The first disposable pacing observer produced a cadence unlike the trusted
graphics-fps scene, so its frame gaps could not be used as evidence. The
measurement pass instead put long-task, GC, heap and frame-timestamp capture
inside the existing production reel. That adds no renderer behavior and keeps
the same warm-up, GPU lock and two-map contract. The alternative would be to
maintain a separate profiling page whose pacing could diverge again.

- **Gap:** the spec required an owner-level signal but the scene only wrote
  aggregate frame and GPU numbers.
- **Reach:** future candidates can be tied to the exact long frame without
  changing the acceptance route.
- **Verdict:** sound. The instrumented reel passed and recorded a 943 ms
  `market-town` long task.
- **Confidence:** high.

### Treat the long task as the trigger, not its unattributed label — 2026-10-19

The production reel now proves that a `market-town` frame gap is real: a
997-millisecond browser long task overlaps a 1,050-millisecond frame interval.
Chromium's Long Tasks API labels it only as an unattributed window task. The
safe decision is to keep the trigger and add targeted counters around the
renderer/frame loop; the label alone is not evidence for static packing,
loading, or pacing. The alternative would be to optimize whichever historical
candidate sounds closest to the symptom.

- **Gap:** browser attribution does not name the JavaScript owner.
- **Reach:** the next implementation slice must instrument the suspected owner
  before changing production behavior.
- **Verdict:** sound. The attribution is explicitly insufficient evidence.
- **Confidence:** high.

### Instrument the presentation boundary before changing renderer code — 2026-10-19

The owner counters showed that the long frame is not model packing or GPU draw:
the renderer work stayed under a few milliseconds, while the Rust-backed
`ScenePresentation.frame` call took over a second. The next measurement exposes
the presentation module's existing static-selection timing rather than adding
a second renderer cache or changing simulation behavior. The alternative would
be to optimize the renderer because the symptom first appeared in a graphics
reel.

- **Gap:** the browser profile could not attribute time inside the WASM call.
- **Reach:** this keeps the optimization at the presentation owner; static
  selection is now ruled out by its existing 6.5 ms metric.
- **Verdict:** sound. The boundary evidence points to presentation work.
- **Confidence:** high.

### Remove a timing hook that perturbed the reel — 2026-10-19

The Rust-side breakdown hook added a few floating-point timing calls and a
cross-boundary array on every frame. In the instrumented reel it changed the
115-second workload into an unbounded run, so its numbers could not be trusted
as evidence. The hook was removed; the JS boundary spans remain because they
pass the existing scene contract and already isolate the expensive presentation
call. The alternative would be to keep a diagnostic that changes the workload
it claims to measure.

- **Gap:** the spec did not define a perturbation budget for diagnostics.
- **Reach:** future measurements must preserve scene completion and pacing before
  their numbers are accepted.
- **Verdict:** sound. The hook violated the measurement contract and was deleted.
- **Confidence:** high.

### Keep the optimization target at Rust presentation frame generation — 2026-10-19

The successful boundary run placed the long frame inside the browser's call to
the shared Rust presentation frame. Static-world selection and GPU rendering
were only a few milliseconds, so the candidate is the loaded unit/effect frame
generation that builds poses, effects, batches and picks. No code change is
chosen yet because the next slice needs its own focused contract and must keep
the browser's presentation output unchanged. The alternative would be to keep
working on renderer caches after the renderer has already been measured as a
small part of the stall.

- **Gap:** the placeholder's candidates were renderer and asset oriented, while
  the measured trigger points into shared presentation code.
- **Reach:** the next spec slice belongs under `crates/presentation`, with the
  browser and native clients as consumers of the same output.
- **Verdict:** needs-user for exact intervention; recommended provisional target
  is loaded presentation frame generation.
- **Confidence:** medium.
