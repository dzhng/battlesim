# v4 validation — in progress
Source: `8eaf7a59b299739261854e84e886d99ce36c0afe`.

Passed: native release build, `bun run build:wasm`, presentation tests, source commit hook (fmt, workspace clippy, file-size).
Focused proof checks before the test-only split: 24 jet tests and 17 GPU tests passed; split jet targets then passed 17 + 7.

Full remaining workspace and native/renderer suites are running under nohup on the Mini. This is not a claim that every test passed. PR #33's v4 comment will be updated with final results.
