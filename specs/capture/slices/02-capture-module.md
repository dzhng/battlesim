# 02 — Capture file and `capture` module

## Contract unlocked

A capture file at `fixtures/captures/<name>.json` with the shape in the [README](../README.md#contracts). The menu reads `fixtures/captures/menu.json`. `fixtures/menu-backdrop.json` is gone.

## Seam

- **`capture.ts` replaces `menuReel.ts`.** Find it with `git grep -l menuReel`. It owns `Capture`, `Scene`, `Reel`, `Shot`, `validateCapture` and `sampleReel`.
- **`validateCapture`:**
  - keeps every shot field, including `ease`, `logDistance` and `zoomDuration`;
  - refuses unknown fields by name;
  - refuses a map whose category isn't `capture`;
  - refuses a `follow` index outside the encounter's `units`.
- **Move the file** with `git mv`, so the reel's history follows it.
- **Example captures.** The four `captureConfig.ts` presets become `fixtures/captures/{default,top-down,ground-level,orbit}.json`, each on a capture map. Slice 05 deletes the TS presets, once the CLI reads files.
- **Benchmark reel.** `apps/battle-lab/src/benchmark/reelRun.ts` and `reelReport.ts` also import the reel. Move them to `capture.ts`. Their fingerprint hashes the whole reel, so expect it to change once; that's a named change, not a regression.

## Human can see

The menu plays from the new file, unchanged.

## Verification

- `web/tests/menuReel.test.ts` becomes `web/tests/capture.test.ts`. Its 9 tests carry over.
- Add red/green tests:
  - a shot round-trips `ease`, `logDistance` and `zoomDuration`;
  - a test map is refused by name;
  - an out-of-range follow is refused.
- Every file in `fixtures/captures/` validates.
- `cargo test -p sim --test sim menu_reel` stays green, since the file move must not change the battle.

## Delegated

Internal validator helpers, and type names other than the contract's.

## Stays green

Slice 01's tests and the interpolation tests.
