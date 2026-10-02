# Physical template evidence

The labelled asymmetric descriptors in [`fixtures/parity/templates/`](../../../../fixtures/parity/templates/) are API stimuli, not release sources or appearance. The [C00 proof](../../spikes/C00.md) owns their conclusions and limits. The village preservation test reads existing dimensions directly; floors, entrance data and bay phase absent from that source remain unresolved.

The report for the equal-height compound freezes native catalogue identity and complete materialization; the native compiler test and the Wasm test both read it. The rotated and precise-f64 reports, which only the Wasm test read, are at tag `data-files-before-prune-2026-10-01`. These proofs do not replace real-source geometry and appearance fit or full-extent resource admission.

```sh
cargo run -p contract --example template_report -- fixtures/parity/templates/asymmetric.json '{"translation":[10,20,5],"yaw":0.37}'
cargo test -p contract --test templates
bun run --cwd web test -- tests/templates.test.ts   # build the Wasm first
```

Three rejected arms keep their original outputs as failures, never as current expectations:

- [Rejected numeric geometry](rejected-numeric/README.md): offsets and float lattices that slipped malformed geometry past admission.
- [Rejected unequal-height join](rejected-unequal-join/README.md): a facade join that hid the taller box's upper wall.
- [Runtime rotation](runtime-rotation/README.md): native and Wasm normals that differed by one ULP, fixed with a shared software evaluator.

Raw evidence (identities, numeric-correction proof, overlay SVG): tag `city-maps-evidence-2026-09-30`.
