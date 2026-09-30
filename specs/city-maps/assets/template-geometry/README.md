# Physical template evidence

These labelled asymmetric descriptors are API stimuli, not release sources or appearance. The [C00 proof](../../spikes/C00.md) owns their conclusions and limits. The village preservation test reads existing dimensions directly; floors, entrance data and bay phase absent from that source remain unresolved.

The current reports freeze native catalogue identity and complete materialization for equal-height compound, rotated and precise-f64 cases. The wasm test consumes the same inputs/reports and compares every emitted value. The rejected unequal-height arm has separate immutable evidence and never supplies a current expected result.

Run the small report from the checkout root with an isolated `CARGO_TARGET_DIR`:

```sh
cargo run -p contract --example template_report -- specs/city-maps/assets/template-geometry/asymmetric.json '{"translation":[10,20,5],"yaw":0.37}'
cargo test -p contract --test templates
bun run --cwd web test -- tests/templates.test.ts
```

Build wasm before the web gate. Rotated uses frame `{ "translation": [10,20,5], "yaw": -0.62 }`; precise-f64 uses the identity frame. `identity.json` pins the original inputs, reports, implementation and compiled wasm. `numeric-correction.json` pins the corrected admission proof; valid reports are unchanged. Rejected numeric inputs and original outputs have their own immutable receipt under `rejected-numeric/`. The SVG draws the materializer's emitted records, never independently derived physical geometry. These proofs do not replace real-source geometry/appearance fit or full-extent resource admission.
