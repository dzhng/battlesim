# 07 Workbench GIF and closeout

**Unlocks:** a picture of the mechanic, the docs, and the spec archived.

## Workbench GIF

- **Scene:** a sniper team in an open field and another at a forest edge, approached by a rifle squad, scouts and an MBT, plus an MBT driving through forest. Mark each observer's identification moment (an identified/unidentified overlay) and the HIDDEN state.
- **Owner:** `crates/sim/examples/movement_shots.rs` is the only scripted GIF tool, and it's near its size budget. Recommended: move its canvas into `crates/sim/examples/common/`, then add a small `concealment_shots` example that replays the system test's scenario. The alternative is adding the scenario to the movement scenario table with an identification check, if that ends up with less code.
- **Verification, in order:**
  1. Judge the GIF yourself against the system test's assertions.
  2. Run an unprimed `screenshot-critique` as the last check before accepting it.
  3. Open it for the user with `preview-shots`.
  
  This checkpoint is **non-blocking**. Wait about 5 minutes. If there's no response, decide on the evidence, record the decision here, close the preview, and continue.

## Docs

- `crates/sim/README.md`: the sensing contract, the sight bands, and the removed fallbacks.
- `fixtures/README.md`: the sight bands, the forest rule, and the frozen physical rules.
- `crates/sim/examples/README.md`: list the spotting report.

## Closeout

1. Run everything once (the full check ladder and the balance workload). This is the only full run in the spec.
2. Run `review` (refactor-clean, code-review, write-docs).
3. Archive with `close-spec`.
