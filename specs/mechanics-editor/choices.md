# Mechanics editor choices

User decisions live in the README. Append only implementation decisions that
the specification did not settle, with scenario, reach, verdict and confidence.

## Sound

- **Medium confidence — interrupted-save ownership.** A journal in ignored scratch
  storage records the writer process and exact before/after contents. An active
  writer causes a retryable conflict; a departed writer is rolled back before
  serving readers. Conflicting outside edits stop recovery rather than being
  overwritten. This governs every multi-file editor save.
- **Medium confidence — local soldier names.** Selected-unit soldier overrides
  use a unit-prefixed catalog id. Restoring the final override removes the
  temporary soldier and restores inheritance when the Rust resolver proves the
  copied slot override is redundant. No catalog schema change is required.
- **High confidence — generation capture.** Development captures accepted rules
  before importing gameplay consumers. Explicit restarts reload the page, keeping
  route parameters; diagnostic resets retain the captured generation so one
  experiment remains reproducible. Production retains worker resets.
- **High confidence — native validation.** A small Cargo example admits supplied
  source text using the same Rules, catalog, supply and flight validation as battle
  startup. Source text reaches Rust intact so duplicate JSON keys cannot disappear
  through JavaScript parsing. Errors are returned to the editor without writes.
  Development builds this executable before serving the editor; requests run it
  directly so fixture edits do not trigger recompilation of embedded Rust fixtures.
- **High confidence — development cache ownership.** Vite's cache lives in each
  checkout's ignored scratch directory; dependencies remain shared. This avoids
  optimized modules from another worktree replacing this checkout's modules.
