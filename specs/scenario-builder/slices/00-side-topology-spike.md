# 00 — Side topology and controller spike

## Contract unlocked

Determine whether arbitrary combat side ids and FFA are a clean generalization of the current Blue/Red runtime. The verdict must be based on the real end-to-end path, not a source estimate.

## Seam

Use a disposable contract/simulation/WASM/browser probe that introduces player-slot ownership separately from combat side identity. The probe must exercise:

- scenario admission and unit ownership;
- native and WASM `Battle` construction and one or more ticks;
- side-scoped publication and browser decoding;
- commands accepted only for the issuing player slot's units;
- Blue/Red/Neutral plus at least one 4-side FFA candidate;
- passive neutral/empty-slot behavior and active AI takeover;
- deterministic scenario/replay digest.

Inventory every fixed `[2]`, `Side::ALL`, literal `blue|red`, side-indexed objective/economy/referee path, and renderer/input assumption encountered in the real path. The probe may use test units and in-memory maps; it must not add fixtures or a compatibility wrapper.

## Verdict

Keep arbitrary side ids only if the resulting owners are one coherent topology, the full probe stays understandable, hidden-state isolation passes for every observer, replay remains deterministic, and no fixed-two compatibility layer is needed. Otherwise hard-cut the production contract to Blue/Red plus a first-class passive Neutral slot and record arbitrary FFA as deferred. The verdict is about final code shape and behavior, not minimizing spike effort.

## Runnable artifact

Native/WASM probe output containing the owner inventory, generated scenario digest, per-observer publication summary, accepted command log, takeover tick, and replay comparison. If the browser path cannot run in the probe, that is a failed generalization, not a reason to silently omit the client.

## Verification

- Rust contract and sim tests: duplicate side/slot ids, unowned units, per-player command rejection, neutral response, and replay equality.
- WASM/native paired run: same canonical setup and digest.
- Browser worker/client test: each observer receives only the records allowed by the selected topology.
- Measure changed production lines and retained owners after the spike; do not count disposable probe code as the end-state architecture.

## Human review

No player-facing visual change is accepted in this slice. A topology report is the review artifact. If a diagnostic visualization is produced, open it with `preview-shots`; review is non-blocking.

## Delegated implementation choices

Choose the smallest internal representation that removes fixed-two assumptions without duplicating scenario, publication, or controller ownership. Do not choose a migration bridge, side aliases, or UI-only emulation. Record every naming and representation choice in `choices.md` with evidence.

## Must stay green

Existing targeted contract, sim, WASM, and browser authority/publication tests that the probe touches. Full repository checks wait for slice 09.

