# Simulation

The simulation owns battle outcomes and what each side can know. Other components
issue commands or consume observations. [The crate boundary](src/lib.rs) names its
rule owners; [the battle authority](src/battle.rs) composes them. Shared data shapes
and units belong to [contract](../contract/README.md), with WebAssembly exposed by
[game-wasm](../game-wasm/README.md).

## Determinism and evidence

Battle state belongs in [the digest](src/digest.rs). Equal digests mean equal
battles; replays must reproduce outcomes as well as presentation evidence. Seeded
choices use explicit integer widths before converting to platform-sized indices,
so native and WebAssembly make the same decisions. Every transcendental function
under `crates/` is `libm`'s, never the standard library's, for the same reason;
[the clippy configuration](../clippy.toml) refuses the others in library code. An
optimization that changes a digest has changed behavior, even if a picture looks
the same. Seeded randomness and iteration order are part of that contract. A body
still where the map authored it is held by one digest of the authored props, taken
when the world is built, so a tick's digest costs what changed, not the map's size.

Seed a draw by what it decides, never by when it is asked. A draw that settles a
decision (where a squad stands at its goal, where it steps out of a building) is
keyed by the unit and that decision, not by the tick. Then the decision lands the
same whenever it applies, and a preview queried every tick shows what the order
will do instead of reshuffling. The tick belongs only in a draw for an event that
happens on that tick, such as one shot's dispersion. A ghost drawn while the
player is still choosing the point (a purchase being placed) decides nothing: it
keeps one draw, so it slides with the cursor instead of reshuffling under it.

[Publication](src/publication.rs) exposes only the selected side's observation,
and [its reader](src/publication/reader.rs), beside it, is the one decoder every
client uses. A firing report's heard mask has one bit per weapon row, which caps
the rules at 64 weapon rows (`publication::MAX_WEAPON_ROWS`).
Geometry, sensing, hearing and knowledge owners decide what that observation may
contain; the renderer and audio engine do not grant visibility or reveal identities.
[Weapon origins](src/weapons.rs) follow the actual operator, occupied window or
claimed lean. An individual infantry mount may declare a bore offset; both its
pivot and muzzle turn with the operator bearing. Known enemy threat assessment
uses the observed center and declared offset, never a hidden operator position.

## Sensing

[Optical sensing](src/sensing.rs) identifies visibility samples through clear
lines of sight and foliage. A sample's target concealment is **ground × unit ×
moving**: ground takes the stronger of forest and garrison shelter, the unit
supplies its own concealment even in the open, and an in-progress move applies
the authored movement multiplier. Airborne units bypass all three factors.

Spotting adjusts that product, capped at full directional sight. At or above
one it multiplies the product, so good eyes can cancel concealment completely;
below one it raises the capped product to the reciprocal power, deepening
concealment without changing an unconcealed target. Geometry and foliage still
limit identification. The published **HIDDEN** flag instead describes the
target's situation: enough visibility samples must have both concealing ground
and net target concealment below one. Known exposure to a visible enemy's eyes
or an attacker also clears the publication's flag. A sniper can be hard to
identify in the open without being HIDDEN; movement can cost a vehicle HIDDEN
in forest.

Sight into ground, low air and high air is authored independently on each unit.
[Sensors](../contract/src/catalog/sensors.rs) owns the band lookup; zero air reach
means blind. Radar emission is an independent signature read by receivers and
anti-radiation weapons, with no sight override or body-based range table.

## Battle rules

Rules follow physical components, with the game's [first-principles policy](../../development.md#game-rules)
explaining where deliberate cinematic exceptions belong.

[Skirmish authority](src/skirmish.rs) owns player wallets, preparation and pending
reinforcements; [the player list](../contract/src/players.rs) says who plays.
A confirmed purchase reserves both its cost and a place under the player's unit
cap; physical entry converts that reservation to a living unit only when the road-edge footprint
is clear. Browser previews use side knowledge, while actual entry checks physical
bodies. These are different questions and must not expose private occupancy through
a free preview. [Match contracts](../contract/src/skirmish.rs) carry setup and the
side's observation through the existing publication owner.
[Entries](src/entries.rs) is the one owner of which of a side's roads a
reinforcement takes and where a unit leaves the map; the roads are the side's,
shared by its players.

[The skirmish AI](src/skirmish_ai/) plays one player from its side's
observation, and everything it decides is an ordinary command.
[Team planning](src/skirmish_ai/team/) is the side's level of it: it spreads
the side's AI players over lanes and objectives and sends help between them.

[Active protection](src/protection.rs) owns finite defensive charges and cooldowns.
Flight offers it only an imminent swept hull collision at the authored standoff;
ordinary projectiles, passes and cover remain ordinary flight events. Supply
service restores one charge from finite stock after offensive ammunition and
before vehicle health or soldiers, then flares, at the service rules' flare
rate and price (`service.flare_service_s`, `service.stock_per_flare`). A jet
never stands still in reach, so it rearms flares only by withdrawing.
Protection state is part of the battle digest and never becomes an offensive
weapon row.

[Kill settlement](src/settlement.rs) consumes causal full-unit deaths and immutable
purchase receipts. Both sides settle against the same pre-tick bounty pools, so
simultaneous trades cannot depend on iteration order. Rewards affect the wallet;
they confer no identity, location or destruction knowledge. Projectile and
structural-collapse provenance belong to combat, while economic deduplication and
fractional carry belong to settlement.

[Holds](src/hold.rs) own a unit being inside something, a building or a carrier,
with one entering and exiting timer and one cancellation rule; each host only
supplies its timing and where its occupants stand. [Carriage](src/carriage.rs)
owns seats, boarding and unloading, which squads ride inside (one still getting
in stands beside the carrier and shares none of its fate), and the one test of
whether a carrier is low and still enough to load. [Bail-out](src/bail_out.rs)
is the one forced exit from a destroyed carrier or a collapsing building: a
rules row per kind of host sets the injury and death odds, each squad drawing
from its own seed for the event, never from the combat stream. An explicit
unload never kills; with no room to stand it is refused. `Unit::alive()` is
accounting (unit cap, casualties, the referee's survival test);
`Unit::on_field()` is the world (collisions, blasts, sensing, hearing,
objectives, supply, the ground soldiers take up), so a rider is never seen, hit,
counted on an objective or in anyone's way.

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
to run them are defined by the [check ladder](../../development.md#check-ladder).

A test of a mechanic runs on the test units (`test_*`, `fixtures/units/test`,
never game content), loaded by `fixtures::test_game`, and adds a fake unit when
it needs a shape they lack; it never names or walks the faction roster, which
grows every day. The rules fixture carries no ground: a test that needs a map
asks for a saved `test` map by name (`fixtures::with_map`, which refuses a
`capture` map), builds an analytic one, or generates one from a seed. An analytic map's
buildings come from the generator's one template catalogue, so a test garrisons,
shells and walks round the buildings a battle places. A test that needs the roster says why (the roster's own
resolution, a roster model's fit, the menu reel).
What the roster must satisfy is refused where the rules load
([catalog resolution](../contract/src/catalog.rs) and the rules' cross-section
checks), so loading it is its check.

An aircraft is a hull with no ground footprint (`Unit::ground_footprint` is
`None`): traffic, shoving, cover, treads, forest lanes and forest concealment
never see it. Its motion (`units::Motion::Air`) never reaches ground
navigation; a site that only a ground mover reaches asks `Unit::ground`.

An aircraft flies in the altitude layer its row names. High air is the one
layer that bends physics: its heights are lowered from a real jet's for play,
and what follows from flying there (its cruise height and hard floor, no sight
of the ground) keys on the layer (`Unit::layer`), never on a kind of aircraft.
How an airframe flies is its row's flight model (`Unit::fixed_wing`), never
read off the layer: rotors hover (`movement::air`); wings never stop, attack
in passes, dogfight only another winged aircraft in their own layer, and arrive and leave by the
back edge, since they can't land at the base (`air`). Loading refuses a pair
no flight law flies yet.

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
