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
the same. Seeded randomness and iteration order are part of that contract. A body
still where the map authored it is held by one digest of the authored props, taken
when the world is built, so a tick's digest costs what changed, not the map's size.

[Publication](src/publication.rs) exposes only the selected side's observation.
Geometry, sensing, hearing and knowledge owners decide what that observation may
contain; the renderer and audio engine do not grant visibility or reveal identities.
[Weapon origins](src/weapons.rs) follow the actual operator, occupied window or
claimed lean. An individual infantry mount may declare a bore offset; both its
pivot and muzzle turn with the operator bearing. Known enemy threat assessment
uses the observed center and declared offset, never a hidden operator position.

Rules follow physical components, with the game's [first-principles policy](../../README.md#rules-from-first-principles)
explaining where deliberate cinematic exceptions belong.

[Skirmish authority](src/skirmish.rs) owns player wallets, preparation and pending
reinforcements. A confirmed purchase reserves both its cost and a slot; physical
entry converts that reservation to a living unit only when the road-edge footprint
is clear. Browser previews use side knowledge, while actual entry checks physical
bodies. These are different questions and must not expose private occupancy through
a free preview. [Match contracts](../contract/src/skirmish.rs) carry setup and the
side's observation through the existing publication owner.

[Active protection](src/protection.rs) owns finite defensive charges and cooldowns.
Flight offers it only an imminent swept hull collision at the authored standoff;
ordinary projectiles, passes and cover remain ordinary flight events. Supply
service restores one charge from finite stock after offensive ammunition and
before vehicle health or soldiers. Protection state is part of the battle digest
and never becomes an offensive weapon row.

[Kill settlement](src/settlement.rs) consumes causal full-unit deaths and immutable
purchase receipts. Both sides settle against the same pre-tick bounty pools, so
simultaneous trades cannot depend on iteration order. Rewards affect the wallet;
they confer no identity, location or destruction knowledge. Projectile and
structural-collapse provenance belong to combat, while economic deduplication and
fractional carry belong to settlement.

[Objective authority](src/objectives.rs) uses physical eligible combat presence for
capture, independently of either side's contacts. Public objective progress reveals
no participant identity. Score and capture completion share deterministic finish
ordering; a finished match stops authoritative advancement rather than relying on
a browser timer.

Native fixture and map adapters load the same [authored inputs](../../fixtures/README.md)
that browser preparation admits. [Encounter planning](src/encounter/) asks ordinary
placement rules where its roster can stand; it does not invent a second movement model.
[Map analysis](src/map_analysis.rs) shares sampled sight calculations with generation
tools, without making the production generator depend on the simulation. Skirmish
admission returns selected sites together with route evidence. Native and browser
preparation carry those selected coordinates into gameplay; measurements of an
alternative are never permission to use the original unmeasured site.

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

A test of a mechanic runs on the test units (`test_*`, `fixtures/units/test`,
never game content), loaded by `fixtures::test_game`, and adds a fake unit when
it needs a shape they lack; it never names or walks the faction roster, which
grows every day. A test that needs the roster says why (the roster's own
resolution, a roster model's fit, the menu reel).
What the roster must satisfy is refused where the rules load
([catalog resolution](../contract/src/catalog.rs) and the rules' cross-section
checks), so loading it is its check.

Where a mechanic depends on how big or how extreme a unit can be, the extreme
is data: the rules' `hull_limits` bound every hull a battle may field, loading
refuses a unit past them, and the mechanic is proven against fake units built
at the limits (`fixtures::with_units_at_limits`: `test_limit_tracked`,
`test_limit_wheeled`). What holds at the limit holds
for every unit inside it. A test that needs a shape past the limits, to show
how a mechanic fails, lifts them explicitly (`fixtures::lift_hull_limits`).

## Reports and cost

[The native experiment guide](examples/README.md) distinguishes tactical
comparison, movement pictures, flight traces, route quality, resource probes and
machine-readable editor tools. It explains which claim each family can prove and
where its inputs and outputs are defined.

[The performance rationale](../../specs/done/city-stress-performance/README.md)
records accepted outcome-preserving reuse and its measured limits. Real rendered
cost belongs to the browser [benchmark](../../web/src/battle/benchmark/README.md).
