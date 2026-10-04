# Simulation

The simulation owns battle outcomes and what each side can know. Other components
issue commands or consume observations. [The crate boundary](src/lib.rs) names its
rule owners; [the battle authority](src/battle.rs) composes them. Shared data shapes
and units belong to [contract](../contract/README.md), with WebAssembly exposed by
[game-wasm](../game-wasm/README.md).

## Determinism and evidence

Battle state belongs in [the digest](src/digest.rs). Equal digests mean equal
battles; replays must reproduce outcomes as well as presentation evidence. An
optimization that changes a digest has changed behavior, even if a picture looks
the same. Seeded randomness and iteration order are part of that contract.

[Publication](src/publication.rs) exposes only the selected side's observation.
Geometry, sensing, hearing and knowledge owners decide what that observation may
contain; the renderer and audio engine do not grant visibility or reveal identities.
Rules follow physical components, with the game's [first-principles policy](../../README.md#rules-from-first-principles)
explaining where deliberate cinematic exceptions belong.

Native fixture and map adapters load the same [authored inputs](../../fixtures/README.md)
that browser preparation admits. [Encounter planning](src/encounter/) asks ordinary
placement rules where its roster can stand; it does not invent a second movement model.
[Map analysis](src/map_analysis.rs) shares sampled sight calculations with generation
tools, without making the production generator depend on the simulation.

## Replay compatibility and build identity

[The build script](build.rs) computes the engine fingerprint used by replay
admission. It includes simulation, contract and binding sources, compiler identity,
Cargo inputs and semantic configuration. Presentation rebuilds and portable
native/WebAssembly target differences do not create another simulation identity.
The script owns the exact input set; [build-identity tests](tests/build_identity.rs)
pin those inclusions and exclusions.

A replay separately checks engine build, scenario and rules before running its
accepted commands. Those admission identities are not the current battle-state
digest: a matching executable does not make a different scenario compatible.
A mismatch is an explicit refusal, not a best-effort replay under new rules.

## Checks

[Tests](tests/) are composed by the integration-test entry point, so filtering
`cargo test -p sim --test sim <test-or-module>` gives a narrow deterministic check.
Use `cargo test -p sim` for the crate. Test the behavior first, then verify the
digest for changes intended to preserve outcomes. Full workspace gates and when
to run them are defined by the root [checking policy](../../README.md#checks).

## Reports and cost

[The native experiment guide](examples/README.md) distinguishes tactical
comparison, movement pictures, flight traces, route quality, resource probes and
machine-readable editor tools. It explains which claim each family can prove and
where its inputs and outputs are defined.

[The performance rationale](../../specs/done/city-stress-performance/README.md)
records accepted outcome-preserving reuse and its measured limits. Real rendered
cost belongs to the browser [benchmark](../../web/src/battle/benchmark/README.md).
