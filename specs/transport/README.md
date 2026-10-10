# Transport (placeholder)

**Status:** placeholder. Nothing is sliced or implemented yet.
**Updated:** 2026-10-10.

## Next Agent Prompt

You are picking up a placeholder for passenger transport. It covers trucks, APCs and IFVs that carry infantry, and transport helicopters. When the user starts this work, run `explore-unknowns`, then `write-spec`, and turn this page into a sliced plan. Read [helicopters](../done/helicopters/README.md) first, because this plan reuses its low hover. Update this section before you end each pass.

- [ ] Map the unknowns with the user (see the open questions below).
- [ ] Slice: one carriage contract (load, carry, unload) shared by ground vehicles and helicopters.
- [ ] Slice: a helicopter landing as a low hover. First fix the helicopters' U1 ([choices](../done/helicopters/choices.md)): an aircraft must stop exactly on its point, not bow up to 0.18 m past it.
- [ ] Hand the carrier rows in the roster their passenger capacity.

## Why it exists

No unit can carry passengers today. Unit state, catalog capabilities and commands define no loading, unloading, passenger identity or carried-unit accounting (`specs/done/unit-roster/capability-admission.md`). The command enum has no load or unload order (`crates/contract/src/command.rs`). Roster descriptions say so, for example the Stryker's "passenger carriage not implemented" (`fixtures/units/roster/us.json`).

[Helicopters](../done/helicopters/README.md) ship the transport types (UH-60, Mi-8, Chinook, NH90 and the others) as door-gun gunships. This plan gives them, and the ground carriers, their passengers.

## Decided by the user (2026-10-10, helicopter walk)

- **One plan for both.** Transport covers trucks and helicopters together; helicopters don't get a separate mechanic.
- **A helicopter never lands.** To load or unload, it sinks to a **low hover** close to the ground, but still above every ground unit. Ground units never move aside to clear a landing spot, and the airframe never clips into them.
- **The low hover is the resupply sink.** The [helicopter resupply sink](../done/helicopters/README.md) owns the motion: idle in a supply zone, sink low, climb on any order or engagement. Transport reuses it for loading and unloading. It does not get a second descent mechanism.

## Prior plan to fold in

[Battle-foundation slice 17](../battle-foundation/slices/17-transports.md) already sketched ground transport. It owns requirements L06, L07 and U01 in [requirements](../battle-foundation/requirements.md), and puts passenger membership in `sim::transport::load/unload/destroy`. It also lists checks worth keeping:
- capacity and whole-squad entry;
- passengers aboard don't fire, sense or capture;
- every passenger is sampled when the carrier is destroyed;
- a blocked unload is rejected visibly;
- no duplicate IDs or corpses.

That slice now points here. Carry its checks into this plan and add helicopters to them.

## Open questions for the walk

- Capacity: one squad per carrier, or a count of seats?
- Can passengers fire from inside? Do they die with the carrier, or bail out?
- Unload order: a point on the ground, or "drop here" while moving?
- How does the skirmish AI use transport?
