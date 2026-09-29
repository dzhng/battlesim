# Evidence and issue accounting

Base: `25c273a`; main checkout initially clean.

## Input (in progress)

- Casualty selection test failed with `[1,2]` after observation dropped unit2; now survivor move/fast/reverse/stop/policy commands all carry `[1]`, and empty selection sends nothing. Reconcile before committing handlers; reuse selected-unit memo for every capability lookup.
- Role widening consumer test failed with `[1,2]` instead of `[1,2,3]` when two catalog types share a role. Now click recognition retains its own transitions; external selection and box selection explicitly reset it. Removed selection-key tracking and helper-only copies of transferred tests.
- Targeted input suites:20 tests passed. Focused lint clean. Typecheck passed. Independent Codex review found no actionable regressions.
- No visual change claimed yet; main-route selection check remains at integration.

## Baselines

- Quick village report saved6 trials to `throwaway/implementation-review/baseline-quick.json`, raw log alongside.
- Five-minute endurance baseline completed:1779.5G step instructions,3013 rounds,digest `b8cb6f06d47b1436`. Raw log `throwaway/implementation-review/baseline-endurance.log`.

## Parallel lanes

- simulation worktree: authored-capacity repair0a6ae37 plus side-known pose/garrison and Rust test repairs in progress.
- rendering worktree: resize guard proof repaired; complete appearance transaction serialization, shader/schema cleanup and package test repairs in progress.
- lifecycle worktree: direct terminal replies, pause intent, viewport disposal, diagnostic histories and asset watcher in progress.

All remaining report findings and test repairs are still open until integrated evidence is recorded. No full gate has been rerun for the changed tree.
