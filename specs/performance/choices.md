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
