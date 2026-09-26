# 00 — Setup, baselines, reuse manifest

**Status:** planned. **Depends on:** none. **Lane:** setup.

## Contract

Make the repo cheap to work in parallel, and measure the before state. Answer the questions the landmine sweep said to confirm before coding.

## API seam

- Git LFS:
  - `brew install git-lfs`, then `git lfs install --local --skip-smudge`;
  - a `.gitattributes` tracking `*.glb *.png *.jpg *.exr *.blend *.bin` under `assets/`, `specs/*/assets/` and `web/public/`;
  - commit the reference frames (`specs/battle-look/assets/reference/`, still untracked) through LFS.
- Prove the worktree recipe from `AGENTS.md`:
  - a harness-created worktree gets pointer files;
  - `git lfs pull --include=<path>` fetches exactly that path;
  - `web/node_modules` is symlinked;
  - `CARGO_TARGET_DIR` is shared.
- `specs/battle-look/assets/reuse-manifest.json`:
  - `source_repo`;
  - `runtime_imports_of_sibling: "none"`;
  - per-file entries: `source`, `source_commit`, `sha256`, `destination`, `mode` (`copy|adapted|technique`), `transitive_imports`, `local_changes`, `dropped`, `retained_tests`, `slice`, `landmines`;
  - `not_ported`;
  - `third_party` entries: `path`, `sha256`, `licence`, `url`, `retrieved`, `accepted_by`.
- `web/tests/reuseManifest.test.ts`:
  - every destination exists;
  - no import resolves into `../game`;
  - every third-party hash matches the file on disk.
- A frame-cost probe, `web/scenes/_frameCost.mjs`:
  - 1920×1080 on the production build;
  - frame-time distribution;
  - GPU timings via `timestamp-query` where headless Metal allows;
  - buffer and texture bytes;
  - publication bytes.

## What you can run or see

Row 0 of `frame-cost.md`: the current village at 1920×1080, from strategic, default and ground cameras.

## Verification

Confirm each of these, and record any failure as a blocker or a fix:
- The tank cannon auto-selects AP against armour while AP remains, and HE otherwise (decision Q9+). If not, fix it with a native test.
- The TypeGPU 0.12.5 `root["~unstable"].createCommandEncoder()` used by the ported code works in our build.
- Ask the user to accept the licences of the Quaternius Universal Animation Library and Universal Base Characters (CC0), recorded in the manifest. This blocks slices 03 and 21 only.
- Whether `timestamp-query` is available in headless Chromium on Metal. If not, frame cost reports CPU timings only, and says so.
- The manifest test passes on an empty manifest.

Record frame cost in [`frame-cost.md`](../frame-cost.md) for this slice.

## Decision budget

- **Delegated:** Probe output format; the manifest's field order.
- Anything else this slice has to invent is a spec gap. Record it in [`choices.md`](../choices.md) and resolve it here; don't improvise.

## Must stay green

Replay and digest parity. Every existing scene and test, except the pixel checks this slice retunes: each retune is recorded in `decisions.md`, never loosened silently. `bun run check` and `bun run verify` at closeout.

## Feedback that changes this slice

If LFS skip-smudge does not apply to harness-made worktrees, reslice 00 to use sparse checkout, which was decision D1's alternative.
