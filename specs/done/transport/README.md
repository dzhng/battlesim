# Transport

**Status:** shipped 2026-10-17.

Infantry squads ride in APCs, IFVs, light vehicles and helicopters. A carrier
has 3, 8 or 16 seats. A squad boards in 1 s, rides hidden from the enemy, and
gets out only on an explicit Unload order. When a carrier dies, its passengers
bail out hurt and some die; a collapsing building throws its garrison out the
same way. The skirmish AI buys a carrier for each squad and moves its infantry
by transport.

[map.md](map.md) holds the decisions the user settled (D1–D24) and the
landmines found before building (L1–L14). [choices.md](choices.md) holds every
call the build made where the plan was silent, and what the closeout review
changed.

## Why it works this way

- **Seats come in three sizes.** The user asked for few numbers: 3 for light
  and recon vehicles and the MH-6, sized for the recon patrols and the ATGM, RPG
  and air-defence teams (recon patrols dropped from 4 to 3 so they fit), 8 for
  the largest squad, and 16 for the large transport helicopters. A
  carrier's seats are data on its catalog row (`capabilities.carriage.seats`);
  nothing else makes a unit a carrier. Supply trucks carry supplies, not
  people.
- **Getting in works like entering a building, faster.** One hold mechanism
  with data-driven timing: 2 s and 4 m for buildings, 1 s and 3 m for carriers.
  Riders can't fire, be seen, be selected or be ordered; the carrier's card
  shows SEATS n/N and its riders' compact cards.
- **Only destruction kills.** An explicit unload is all or nothing: with no room
  for every soldier to stand, it is refused. A destroyed host is different:
  everyone who gets out is hurt, with a death roll per soldier and death for
  anyone with no room to stand, and a downed helicopter may lose the whole
  squad. Buildings changed to match, so nobody
  walks out of a collapse either unhurt or dead.
- **Severity follows the host.** Buildings 25% death and 20–40 hp lost;
  ground vehicles 10% and 10–30; helicopters a 50% whole-squad loss, then 25%
  and 30–60. Survivors appear right beside the host (search 8 m for buildings,
  5 m otherwise); there is no getting-out animation.
- **U is the unload key, only on carriers.** U then a click drives there and
  unloads (Shift queues it); U twice unloads where the carrier stands and
  replaces its orders, like a move order. Squads in buildings keep the Leave
  button and get no hotkey.
- **The AI pairs by data.** It buys the smallest carrier that seats each
  squad, replaces whichever half of a pair dies, collects squads and drops them
  short of the fight. An empty carrier with weapons fights like any combat
  unit; an unarmed one waits for a squad.

## Invariants

- **One hold.** `sim::hold` (`crates/sim/src/hold.rs`) alone owns a unit being
  inside something: one host, one phase (`Entering`, `Inside`, `Exiting`), one
  timer whose length is the host's data, one cancellation rule, and the hold's
  digest. `garrison` adds only building matters; `carriage` only carrier
  matters.
- **Alive is accounting; on the field is the world.** `Unit::alive()` counts
  for the unit cap, casualties and the referee's survival test.
  `Unit::on_field()` (alive and not riding, `Hold::riding`) gates everything
  physical: collisions, blasts, sensing, fog, hearing, objectives, supply,
  exit-spot searches.
- **One bail-out.** Every forced exit from a destroyed host goes through
  `sim::bail_out::apply` with a row from `rules.bail_out` chosen by the host's
  physical kind (`BailOutHost::{Building, Ground, Air}`), never by role; a host
  that leaves the map takes its passengers with it (`bail_out::lost_with_host`).
  Each event and squad draws from its own seeded stream, never the combat
  stream.
- **One seat count.** `carriage::seats`, `seats_used` and `fits` serve orders,
  the simulation and the AI. A squad still getting in holds its seats.
- **One loading-height rule.** `carriage::at_loading_height`: a ground carrier
  always; a helicopter once settled at the hover it actually holds
  (`air::low_hover_settled`).
- **Passenger positions come from one place**: the carrier, or the falling
  wreck (`crash.position`), and only for squads riding inside. Boarding
  soldiers stand on the ground.
- **The relation is published once, on the rider** (`OwnUnit.hold`); seats used
  is derived. The enemy never learns of passengers: no identified field, no
  sound, no capture.
- **The simulation owns the queue.** Which queued step unloads is published
  (`goal_unloads`, `queue_unloads`), so the browser keeps no order memory of
  its own.
- **Per-unit refusals:** one found at order time is returned in
  `CommandAck.refused` and also lands in `ObservationFrame.refusals`, with those
  found later. The callouts read the frame; the voice answers from the ack and
  skips the frame's copy of the same order, so nothing is said twice.
- **Transport state folds into the digest only when present,** so battles
  without it kept their digests apart from the two named outcome changes
  (recon at 3, collapse as bail-out).

## Where it lives

- Contract: `crates/contract/src/catalog.rs` (`Carriage`),
  `scenario.rs` (`CarriageRules`, `BailOutHost`, `BailOutRow`), `command.rs` (`Order::Board`,
  `Collect`, `Unload`, `Refusal`, `OrderError::{Aboard, NotACarrier,
  NoRoomToUnload}`), `observation.rs` (`HoldState`, `HostRef`).
- Simulation: `crates/sim/src/{hold,carriage,bail_out,garrison}.rs`;
  `carriage::passengers` (who rides inside), `arrangement::ground_taken` (what takes up ground), and `bail_out_carrier` in `battle.rs`; the AI in
  `crates/sim/src/skirmish_ai/transport.rs` with tuning in
  `fixtures/skirmish-ai.json`.
- Browser: `aboardCarrier`, `carrierOf`, `passengersOf`, `inBuilding` in
  `web/src/battle/sim/observationView.ts`; the Seats row in
  `web/src/battle/present/panelRows.ts` and the rider cards in `readouts.tsx`; U and the Unload button in
  `commandBindings.ts` and `useUnitControl.ts`; refusals in
  `rejectedOrder.tsx` and `apps/battle-lab/src/unitResponses.ts`.
- Voices: the `board`, `unload` and `no_room_to_unload` lines in
  `fixtures/voices.json`.
- Proof: `crates/sim/tests/transport.rs` (the scenario tests), the garrison
  collapse tests in `crates/sim/tests/garrison.rs`, the AI tests in
  `crates/sim/src/skirmish_ai/tests.rs`, and the transport lab
  (`/labs/transport`, `web/scenes/transport.mjs`, variants board, controls and
  heli).

## Dead ends and divergences

- **Codex first, then Claude subagents.** Most simulation slices were built by
  Codex; two stalled runs made the user switch to Claude subagents to finish.
- **The browser remembered its own unload points** (slice 11's
  `unloadPoints.ts`). That made it a second owner of the queue; the simulation
  now publishes the flag.
- **A separate carrier timer, a panicking `Host::building()`, and silent
  capacity refusals** all appeared in first passes and were removed: one hold,
  one timer, explicit refusals.
- **Collect once pulled every listed squad onto the hull,** and boarding
  soldiers floated to a hovering helicopter's height. A rendering bug was
  suspected; the cause was the position copy, now limited to riders.
- **The referee briefly treated riding infantry as gone,** defeating an
  attacker whose squads were all aboard; survival is accounting (`alive`).
- **Helicopter boarding waited forever over props** when "settled" meant height
  over the bare surface; it now means the hover the helicopter holds.
- **The market-town menu reel was re-recorded** for the building bail-out and
  again for per-squad bail-out rolls (main's capture reel holds a garrison
  collapse at tick 2093). Every event before that tick, the tank kills at 87
  and 295 included, is unchanged.
- **Riding toward the fight:** with objectives alone, the AI walked most
  squads across the map after the first sighted enemy, so a squad sent at a far
  enemy rides too. With objectives alone, only 4 of 14 squads ever rode.

## Open

- 31 voice takes failed the transcript check and are left out of the packs;
  [choices.md](choices.md) lists them for a listening pass.
- An armed carrier deep in a fight breaks off to collect a squad; prefer idle
  carriers if it reads badly in play. The AI's `ride_min_m`, `reaim_m` and
  `unload_share` are first guesses.
- The shot-down helicopter has tests but no lab shots.
