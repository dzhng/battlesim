# 08 · A destroyed carrier, and refunds

**Owns:** D11 (the ground row), D12 (vehicle search), D14, D15, L13. **Depends on:** 04, 07.

Load `tweak-mechanics` first.

## Contract
- **A ground carrier destroyed** calls `bail_out` with its `ground` row, inside `consequences` (`battle.rs:2779-2931`).
  - Do it after the wreck's resting place (`rest`, from `death_roll`, `movement/drive.rs:497-539`) is known, so survivors appear around the wreck.
  - Report a squad wiped out as a `UnitDeath` in the same `destroyed` list, with the carrier's killer as its source. Kill rewards and settlement then count passengers (D14).
  - The burst that killed the carrier must not also hit passengers: they are off the field until `bail_out` places them.
- **Refunds** (D15):
  - `retire_withdrawals` (`battle.rs:915-988`) retires and refunds every passenger along with its carrier, each at its own `entered.price` times its condition (`withdrawal.rs:41`).
  - A refund aimed at a passenger is refused with `Aboard`.

## What a human can see
Variant `destroyed` of the transport lab: an enemy tank kills a loaded APC; the squad appears beside the wreck, hurt and pinned.

## Verification
- `crates/sim/tests/transport.rs`:
  - `a_destroyed_carrier_bails_out_its_passengers_hurt_and_pinned`. It checks that the kill reward includes the passengers who died, that there are no duplicate ids or corpses, and that every passenger is accounted for once.
  - `refunding_a_loaded_carrier_refunds_its_passengers`.
- The transport scene, variant `destroyed`.
- Visual: unprimed `screenshot-critique` of the bail-out frame, then the non-blocking `preview-shots` checkpoint.

## Delegated
How the passenger deaths join the `destroyed` and settlement paths.

## Must stay green
Vehicle death without passengers (digests unchanged), withdrawal and refund of unloaded units, settlement.

## Feedback that would change it
The ground row's numbers (data).
