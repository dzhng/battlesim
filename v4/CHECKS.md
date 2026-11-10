# v4 validation — partial, deliberately stopped
Source: `8eaf7a59b299739261854e84e886d99ce36c0afe`. A / `realistic` is the chosen code default.

Passed: native release build, `bun run build:wasm`, presentation suite, normal source commit hook (fmt, workspace clippy, file-size).
Focused proof checks: 24 jet + 17 GPU tests passed before the test-only split; split jet targets passed 17 + 7.

The remaining workspace run was stopped and the queued native/renderer GPU test step cancelled, following the final instruction to stop after the push and reserve GPU for a separate post capture. Full final suites are INCOMPLETE; this is not an all-tests-pass claim. No further C polish or GPU work.
