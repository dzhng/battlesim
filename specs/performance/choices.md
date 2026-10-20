# Performance choices

## Sound

### Reuse loaded-frame scratch vectors — 2026-10-19

The first accepted Rust presentation pass keeps three temporary collections on
`Loaded` and clears/repopulates them each frame. They hold high-air shadows,
indoor unit ids and allied unit ids, exactly in the same order as the old local
vectors. This means a frame still sees the same shadows and x-ray colours while
avoiding repeated heap allocation; the unbuilt alternative would have changed
the algorithm or cached visible records with a stale-data risk.

- **Gap:** the slice named the Rust owner but did not prescribe the first safe
  intervention.
- **Reach:** later loaded-frame work should preserve reusable scratch ownership
  where lifetimes are frame-local and output order is observable.
- **Verdict:** sound. The change is output-equivalent and bounded to temporary
  storage; presentation tests cover the contract.
- **Confidence:** high.

### Treat the bounded browser failure as a harness blocker — 2026-10-19

The timeout validation stalled even with `FPS_SCENES=paris-corner`, then failed
at 120 seconds before the graphics page reached `results`. Because this happens
without the trigger map, it does not justify blaming the Rust pass or claiming
an FPS change. The next action is to diagnose page completion or adapter setup;
the alternative would be to keep optimizing against a run that never produced
timing samples.

- **Gap:** the spec required a completed browser acceptance run but did not
  identify a terminal verdict for a cross-map graphics-page stall.
- **Reach:** browser evidence remains the final trigger gate; native A/B is
  interim evidence only.
- **Verdict:** sound. The failure is explicit and reproducible.
- **Confidence:** high.

### Make the graphics page publish its own timeout result — 2026-10-19

The external scene runner could detect a stalled page, but the page itself
remained on a blank running view. The route now changes to its existing results
view after the same two-minute deadline and records an incomplete-test reason.
That keeps the page and runner on one terminal contract; the alternative was a
runner-only timeout that left the page unable to explain what happened.

- **Gap:** the plan bounded the harness poll but did not require the page to
  publish the same terminal state.
- **Reach:** future benchmark failures are visible in the app and machine
  readable through `window.__graphicsTest`.
- **Verdict:** sound. Successful runs and timing thresholds are unchanged.
- **Confidence:** high.

### Reject transit scratch reuse after a controlled A/B — 2026-10-19

The transit builder returns a short list of aircraft poses each frame, so it
was a plausible allocation candidate. Identical native replay runs showed no
gain and a small regression in both CPU percentiles. The change was removed;
the alternative would have kept extra state in `Loaded` without evidence that
it helped the trigger.

- **Gap:** the loaded-frame slice did not rank which temporary vector to test
  after the first scratch pass.
- **Reach:** future allocation work must have a controlled A/B before it adds
  another reusable owner.
- **Verdict:** sound rejection. The candidate does not earn added complexity on
  the current workload.
- **Confidence:** high.

### Rebuild ignored WASM before diagnosing tracked lifecycle code — 2026-10-19

The browser reported a Rust ownership error during teardown while every scene
failed to produce a frame. Rebuilding the ignored presentation WASM artifacts
from the current Rust source removed the error, and the frame scene then passed
its hardware adapter and resource checks. The tracked TypeScript/Rust lifecycle
was left unchanged; the alternative would have added a workaround for a stale
local binary and risked hiding a real ownership bug.

- **Gap:** the performance handoff did not state that generated WASM must be
  rebuilt when switching branches or replaying a source change.
- **Reach:** future browser evidence must start from artifacts built from the
  current checkout before blaming presentation ownership.
- **Verdict:** sound. The fix changed only generated local artifacts and the
  completing reel provides the consumer-level proof.
- **Confidence:** high.

### Scale the reel watchdog to authored workload — 2026-10-19

The first watchdog used a fixed two-minute deadline. A valid two-map reel took
longer than that while the page was actively rendering, so the guard reported a
false failure. The page now derives its deadline from the selected scenes' shot
durations plus one minute of setup margin; the runner keeps a larger diagnostic
ceiling. A genuinely stalled run still terminates, while longer valid reels are
allowed to finish.

- **Gap:** the initial timeout did not account for multiple selected scenes.
- **Reach:** future benchmark variants remain bounded without embedding a
  workload-specific constant that can reject valid timing evidence.
- **Verdict:** sound. The final two-map reel completed under the scaled bound.
- **Confidence:** high.

### Retain scratch reuse on native CPU evidence — 2026-10-19

The same captured replay ran before and after the pass. Reusing the three
frame-local vectors lowered native CPU p50 and p95 while counts and GPU work
stayed the same. The browser reel is still required for the player-facing
claim, but the controlled native A/B is enough to keep this output-equivalent
change instead of reverting it because the first browser attempt hung.

- **Gap:** the plan did not say whether native parity evidence could retain a
  pass while browser acceptance was pending.
- **Reach:** future shared presentation changes can use the native replay as an
  interim CPU guard, while keeping the browser reel as the final trigger gate.
- **Verdict:** sound. The A/B used identical captured input and changed only
  temporary storage ownership.
- **Confidence:** high.

### Bound graphics-reel completion — 2026-10-19

The graphics scene previously polled forever after starting a run. When the
page stalled before its `results` state, the measurement consumed the session
without producing evidence. The scene now fails after two minutes with an
explicit diagnostic; a successful run still uses the same timing and output
contract. The alternative was to leave the infinite wait and rely on the outer
900-second runner timeout, which hides the cause and delays the next experiment.

- **Gap:** the measurement contract required a completing reel but had no
  scene-local terminal verdict for a hung page.
- **Reach:** future performance runs are bounded and can distinguish a harness
  stall from a slow but valid frame workload.
- **Verdict:** sound. The timeout changes only failure handling, not measured
  frames or acceptance thresholds.
- **Confidence:** high.

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

### Do not claim a browser gain from the incomplete reel — 2026-10-19

The post-change graphics reel reached `market-town` but did not finish within
five minutes, so it cannot compare average FPS or 1% lows to the trusted runs.
The result is treated as missing evidence rather than as an improvement or a
regression. A completed production reel remains the acceptance seam; if it
cannot complete, the next pass must diagnose the harness/runtime condition
before selecting another optimization.

- **Gap:** the slice required a browser comparison but did not define a
  completion timeout for an invalid run.
- **Reach:** later performance decisions cannot use partial or hung reels as
  evidence.
- **Verdict:** sound. Stopping an unbounded diagnostic run protects the
  measurement contract.
- **Confidence:** high.
### Accept the current reel and keep XL stress diagnostic — 2026-10-19

The generated Metro XL workload confirms that 100 units per side is a valid
5v5-scale stress case, but its 20 FPS live and 17 FPS late-state cadence is a
separate scale bound from the player-facing graphics reel. The user accepts the
reel's repeated `market-town` 1% low around 39 FPS for the current target. No
additional cache or renderer machinery is justified without a new measured
player-facing symptom; the XL stress scene remains available for future scale
work.

### Use bounded internal render scale for low-quality production rendering — 2026-10-20

The XL target is dominated by GPU fill and multisampled render targets. A
renderer-wide 1× MSAA change is incompatible with the existing multisampled
depth contract and was reverted. A 0.75× internal canvas scale reduced GPU mean
from 41.2 to 35.8 ms live and from 48.8 to 45.7 ms late, while preserving
simulation digests and lowering texture memory. The HUD keeps CSS resolution.
The same scale is now the production Low profile; Medium and High remain at 1×.

- **Gap:** Low quality had no quality-controlled fill-rate lever.
- **Reach:** production Low profile and the minimum endurance profile.
- **Verdict:** sound after the low-quality graphics reel completed at 90.9 FPS
  average and 51.2 FPS 1% low on the measured adapter.
- **Confidence:** high for the measured adapter, medium across hardware.


### Make terrain and shadow culling the XL performance target — 2026-10-20

The XL GPU floor is not entirely camera-bounded. Post-processing is tied to the
render target, but the terrain mesh is submitted as one full-map draw and each
shadow cascade submits the terrain caster plus its selected casters. A camera
frustum clips the result after submission; it does not make a huge terrain
mesh or an off-view shadow caster free. Props and scenery already use chunked
presentation walks, so they are a secondary target.

- **Gap:** the prior handoff called the entire backbone camera-bounded and could
  have led us to optimize the wrong owner.
- **Reach:** browser and native world/shadow passes, with identical presentation
  records and visual output.
- **Verdict:** make terrain tiling/culling and shadow-caster bounding the active
  next slice; measure each separately first. Preserve post as a screen-size
  baseline.
- **Confidence:** high for the submission path, medium for the eventual split
  until pass-specific probes run.
