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

The production feature keeps a fixed Blue/Red/Neutral topology. The existing two-side baseline passed observation and replay checks. The four-side candidate is refused by the current contract, and a disposable Neutral extension required changes across encounter, movement, and route planning before compiling; the first real Neutral battle then panicked in `Battle::sense` because `sightings` remained a two-entry array. The inventory found 172 side-indexed/fixed-two Rust owners before browser/UI consumers, plus fixed skirmish/objective/publication contracts. Arbitrary side ids/FFA are therefore deferred as a separate architecture project. No dynamic-side compatibility layer is carried into the builder.

## Runnable artifact

Native/WASM probe output containing the owner inventory, generated scenario digest, per-observer publication summary, accepted command log, takeover tick, and replay comparison. If the browser path cannot run in the probe, that is a failed generalization, not a reason to silently omit the client.

## Verification

- Rust contract and sim tests: duplicate side/slot ids, unowned units, per-player command rejection, neutral response, and replay equality.
- WASM/native paired run: same canonical setup and digest.
- Browser worker/client test: each observer receives only the records allowed by the selected topology.
- Measure changed production lines and retained owners after the spike; do not count disposable probe code as the end-state architecture.

## Human review

No player-facing visual change is accepted in this slice. The topology report, refusal test, and replay baseline are the review artifacts. If a diagnostic visualization is produced, open it with `preview-shots`; review is non-blocking.

## Delegated implementation choices

Choose the smallest internal representation that removes fixed-two assumptions without duplicating scenario, publication, or controller ownership. Do not choose a migration bridge, side aliases, or UI-only emulation. Record every naming and representation choice in `choices.md` with evidence.

## Must stay green

Existing targeted contract, sim, WASM, and browser authority/publication tests that the probe touches. Full repository checks wait for slice 09.

