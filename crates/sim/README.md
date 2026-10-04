# Simulation

The simulation owns battle outcomes and what each side can know. Other components
issue commands or consume observations. [The crate boundary](src/lib.rs) names its
rule owners; [the battle authority](src/battle.rs) composes them. Shared data shapes
and units belong to [contract](../contract/src/lib.rs), with WebAssembly exposed by
[game-wasm](../game-wasm/src/lib.rs).

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

## Checks

[Tests](tests/) are composed by the integration-test entry point, so filtering
`cargo test -p sim --test sim <test-or-module>` gives a narrow deterministic check.
Use `cargo test -p sim` for the crate. Test the behavior first, then verify the
digest for changes intended to preserve outcomes. Full workspace gates and when
to run them are defined by the root [checking policy](../../README.md#checks).

## Reports and cost

[Examples](examples/) own native battle reports, fixture validation and focused
movement, flight, navigation and resource probes. Each executable's usage text or
module comment defines its inputs; the directory is the current tool inventory.
Run one through `cargo run -p sim --release --example <name> -- <arguments>`.
These are explicit developer experiments, not another runtime authority.

The village report provides quick comparison feedback and final battle digests.
The endurance report measures an accelerated soak; it is not a real-time rendered
FPS result. Shared [instruction measurement](examples/common/instructions.rs)
provides load-independent process cost on supported hosts; absence of that counter
is not zero cost. Preserve workload identity when comparing runs, and keep scratch
reports under `throwaway/`.

[The performance rationale](../../specs/done/city-stress-performance/README.md)
records accepted outcome-preserving reuse and its measured limits. Real rendered
cost belongs to the browser [benchmark](../../web/src/battle/benchmark/README.md).
