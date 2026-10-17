# 06 · Boarding, and "on the field"

**Owns:** D1, D5, D6 (boarding), D7 (simulation), D10, D23, L1, L2, L3, L9 (capability, test units), L14. **Depends on:** 02, 03. **Outcome change:** none for battles without carriers.

Load `tweak-mechanics` first. This is the riskiest slice; probe the risk first (below).

## Contract
- **Catalog:** `Capabilities.carriage: Option<Carriage { seats }>`, checked in the per-type `check` (`catalog.rs:1608`): 3, 8 or 16 only, and never on a squad. Mirrors: `packages/scene-assets/src/units.ts:222-245`, `apps/mechanics-editor/src/fields.ts:298-330`.
- **Rules:** `carriage { board_s: 1, reach_m, unload_search_m: 5 }`.
- **Commands:** `Board` and `Collect` (README contracts). Per-unit refusals use slice 03's `Refusal`.
  - If the selected squads need more seats than are free, the closest whole squads that fit board, and each other squad gets `CapacityFull` in `CommandAck.refused`.
  - `validate_units` (`battle.rs:1712`) refuses any order aimed at a passenger with `Aboard { unit }`.
- **Hold:** `Host::Carrier(UnitId)` joins the hold.
  - Its timing comes from `rules.carriage`.
  - Both units must be stopped and within `reach_m` for the 1 s timer to run; any other order cancels it, as for buildings.
  - The seat check runs again when the timer ends. Supply can add a soldier during the timer, the same way garrison re-checks at `garrison.rs:549`.
  - `Board` makes the squad follow a moving carrier, using the target-tracking approach that `Attack` uses (`battle.rs:2935-2975`). `Collect` drives the carrier to the squads, which hold where they are.
- **On the field:** `Unit::on_field()` is alive and not riding in a carrier (phase `Inside`, or `Exiting` of a carrier).
- **Passenger positions:** copied from the carrier every tick, in one place (L2).
- **New test units only:** `test_apc` (8 seats) and `test_scout_car` (3 seats) in `fixtures/units/test/vehicles.json`. Never give seats to `test_jeep` or `test_heli`: menu units and panel specimens extend them.

## Seam
- New `crates/sim/src/carriage.rs`: seats, riding along, and the validation for `Board` and `Collect`. It shares the hold.
- **The `on_field` audit.**
  1. Temporarily rename `Unit::alive` (`units.rs:600`) so the compiler lists every caller (`Soldier::alive`, `units.rs:130`, is a different function).
  2. Give each caller a verdict: stays `alive` (accounting) or becomes `on_field` (the world).
  3. Record the verdict table in [choices.md](../choices.md).

  At minimum these become `on_field`:
  - colliders (`bodies`, `battle.rs:2492-2533`; the soldier slot index must still advance for skipped soldiers);
  - blasts and near-miss suppression (`damage.rs:513-530, 735-779`);
  - vehicle shoves, crowding and `clear_of` (`movement/soldier.rs:287, 1025-1060`), and treads (`battle.rs:2374`);
  - sensing (`sensing.rs:235, 246`), the fog sweep (`battle.rs:3364`), hearing (`hearing.rs:33-50`), radar (`knowledge.rs:297`), shouts (`shouts.rs:264, 479`);
  - objectives (`objectives.rs:62-71`), supply service (`supply.rs:143-190`), and the encounter referee's attackers and zone presence (`encounter/referee.rs:33-41`).

  These stay `alive`: the unit cap (`battle.rs:3483, 4646`, `skirmish.rs:97, 120`), `Load` (`battle.rs:1287`), casualties and settlement.
- **Observation:** passengers stay in `frame.own` with `hold.host = Carrier`. Nothing about them enters the enemy's view. Shared presentation skips drawing them, by extending the `indoor` filter at `presentation.rs:1493`; native inherits this.
- **Digest:** carrier holds fold into `Unit::digest` only when present.
- **The transport lab** (created here): fixture `transport` in `apps/battle-lab/src/fixtures.json` (with a `describe`), route `apps/battle-lab/src/routes/transport.tsx` registered in `router.tsx`, scene `web/scenes/transport.mjs` (one-to-one with the fixture), and a saved `test` map under `fixtures/maps/`. Variant `board`: an APC and a scout car, a rifle squad and a 3-man team, scripted to board from both directions, drive past an enemy and through an objective.

## Risk probe (do first)
Before wiring orders, set one squad's hold to a carrier by hand in a test battle beside an enemy squad and an objective. Run 10 s. Assert that the passenger squad is never hit, seen, heard or resupplied, and never captures. If any of those can't be made false through `on_field`, stop and reslice: the audit approach is wrong.

## What a human can see
`/labs/transport?variant=board`: squads walk to the carriers, wait a second, vanish, and ride; the enemy panel shows only the carrier.

## Verification
- `crates/sim/tests/transport.rs` (new):
  - `a_squad_boards_a_stopped_carrier_and_leaves_the_field`: after the timer, the squad isn't blasted, heard, sensed, resupplied or capturing, and orders to it are refused as `Aboard`. It rides at the carrier's position.
  - `squads_beyond_the_seats_are_refused_closest_first`.
  - `a_carrier_drives_to_a_squad_and_takes_it_aboard`.
  - `the_enemy_sees_only_the_carrier`.
  - `transport_orders_replay_every_tick_exactly`.
- Catalog test: seats outside 3/8/16, or seats on a squad, are refused.
- The transport scene (`variant=board`) asserts boarding through `obs`.
- Rung 2: `cargo test -p sim`, then the `transport` scene.
- Digest probe: every battle without a carrier keeps its digest.
- Visual: the lab shot gets an unprimed `screenshot-critique`. There is no before shot. Then the non-blocking `preview-shots` checkpoint.

## Delegated
- `reach_m`, as long as it is under the building's 4 m.
- Whether seats count living members only (default: yes).
- How `Board` and `Collect` name their mover.
- Test unit ids and the lab's camera.

## Must stay green
Every digest of battles without carriers, garrison behaviour, the unit cap, the menu reel, catalog admission.

## Feedback that would change it
The boarding distance, or the feel of squads chasing a moving carrier.
