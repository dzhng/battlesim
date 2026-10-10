# Ground capability admission

This is the slice-01 source audit and the gate for [ground content admission](slices/03-ground-content.md). All records in [the authored roster](../../fixtures/units/roster/) are initially planned. A supported rule path is permission to prepare physical data, not proof that a named unit's body, weapon envelope or model already admits. No game mechanic was added by this audit.

## What existing ground combat supports

[Catalog bodies and mobility](../../crates/contract/src/catalog.rs) admit soldier squads and tracked/wheeled hulls. Sensors, armor, physical mounts and inheritance are existing owners. [Weapon definitions](../../crates/contract/src/weapons.rs) support finite or unlimited ammunition, magazines/bursts, minimum range, armor penetration and explosive damage. [Flight admission](../../crates/contract/src/ballistics.rs) couples speed, range, gravity, scatter and lifetime; a starter range is not independently sufficient.

[Weapon selection and guidance admission](../../crates/sim/src/weapons.rs) require a currently identified target and the launcher's physical optical line for guided fire. [Guided flight](../../crates/sim/src/flight/mod.rs) steers toward a commanded point without gravity; [the solver](../../crates/sim/src/flight/solve.rs) selects direct/indirect ballistic arcs. These are direct guidance and ballistic arcs, not a top-attack acquisition/climb/descent contract. In particular, an indirect ballistic arc must not be presented as authentic Javelin top attack.

[Infantry mounts](../../crates/contract/src/catalog.rs) distinguish a special weapon that transfers to a surviving carrier from ordinary equipment lost with its soldier. Operator appearance and active/carried equipment have an existing binding contract. [Catalog tests](../../crates/sim/tests/catalog.rs) and [weapon tests](../../crates/sim/tests/weapons.rs) exercise handoff and one-gun-at-a-time operator behavior. Named infantry equipment reuses these owners and the shared soldier rig.

[Supply](../../crates/sim/src/supply.rs) already restores finite ammunition, hull health and missing soldiers from a deployed source's finite stock, with deterministic service order and recipient restrictions. Every new finite weapon row needs its existing stock-price validation. Supply currently has no APS service item.

## Phase-one verdicts

The following groups cover the ground families and loadouts authored in the roster. Exact platform identities, memberships, prices and packs live in the fixture records, rather than a second inventory here.

| Family/loadout group | Slice-03 verdict | Admission still required |
|---|---|---|
| Light/elite recon patrols; rifle and assault squads | Supported ground profiles | Soldier slots, rifle/grenade envelopes, recon sensing and shared rig/equipment bindings. |
| Marksman and heavy-sniper teams | Supported direct-fire profiles | Finite magazine/round loads, stock prices, physical equipment and recognizable M110/SVD/M107/ASVK weapon identity where authored. |
| TOW teams and TOW-equipped Bradley/Stryker/LAV variants; Kornet team and Kornet-equipped BMP/Armata variants | Supported direct guided ground fire | Launcher optical LOS/support, admitted motor/range/lifetime, finite service price, mounts and carrier/hull fit. No fire-and-forget promise follows from the platform name. |
| RPG-7/RPG-29 teams | Supported unguided direct ground fire | Ballistic envelope and finite supply price. No unsupported guided motor is added to an unguided row. |
| Javelin/Akeron teams and Akeron-equipped Jaguar (superseded: [ground admission](../../ground-admission/README.md) adds sight-supported top attack) | Authentic top attack deferred | Existing direct guidance is technically reusable under starter balance, but cannot prove authentic top attack. Keep disabled if that capability is promised; any admission for the limited direct-guidance role must explicitly describe that limit and record the slice-03 decision. Do not advertise final trajectory or fire-and-forget behavior. |
| Spike-equipped Puma | Supported limited direct-guidance ground profile | Reuse current optical guidance, with an accurate limited-role description; no unstated top-attack or fire-and-forget capability. If its final description promises those modes, it stays disabled pending the matching capability. |
| HMG scout cars, wheeled reconnaissance and ground carriers | Supported ground weapons | Platform dimensions, steering/turning fit, sensors, mounts and model identity. Carriage is excluded as explained below. |
| Autocannon reconnaissance/IFV variants and BMP-3's additional HE gun | Supported direct-fire ground profiles | Independently admitted physical guns, ammunition choices, magazines, blast/armor behavior, service prices and articulated mount/model fit. A multi-gun platform uses existing mount ownership, not a platform-name special case. |
| Non-Trophy tank variants and light tanks | Supported ground gun profiles | AP/HE rows and capacities, hull/armor/mobility, muzzle/pivot positions and source-backed named models. Prototypes are permitted identities, not permission to invent extra capabilities. |
| Trophy-equipped Abrams, Leopard and Challenger variants (superseded: APS shipped; M1E3 and Challenger 3 are admitted by [ground admission](../../ground-admission/README.md)) | Blocked until slice 10 | Existing capabilities have deployment/supply only. Swept interception, threat classification, charges/cooldown and a stock-priced APS service item must ship before these variants become available. Ordinary weapon admission cannot erase their named hardware. |
| HEMTT, MAN HX and Ural resupply families | Supported finite logistics | Truck bodies, setup/packing and service profile, finite stock and named models; preserve existing service restrictions and order. |
| MANPADS teams; dedicated air-defense support variants | Deferred | Ground hulls and ballistic gun rows do not prove airborne target classes, anti-air guidance or sensing. Dedicated AA units stay disabled, including gun-only platforms. |
| Non-resupply support: artillery, mortars and rocket/precision batteries | Deferred | Existing indirect arc support does not admit these first-phase roles. Their targeting, range, observation, entry and service contracts belong to later capability work. |
| Drone recon/attack; helicopters; aircraft/EW | Deferred | Current mobility has ground foot/tracked/wheeled variants only. Aircraft/drone movement, sensing, combat/mission rules, entry and EW are separate capability work. |

## Carriage and model gates

Neither [unit state](../../crates/sim/src/units.rs), [catalog capabilities](../../crates/contract/src/catalog.rs) nor [commands](../../crates/contract/src/command.rs) defines passenger loading, unloading, passenger identity or carried-unit accounting. Garrison entry is building occupancy, not passenger carriage. Ground carrier platforms may serve as their supported ground weapons only; their authored descriptions explicitly say passenger carriage is not implemented. Do not create a load command or silently remove a purchased squad's identity to enable a carrier card.

Every phase-one hull needs measured dimensions, ground origin, eye and mount frames; infantry needs the existing rig/socket contract and its actual equipment. These physical/model facts remain OPEN research until the coordinator freezes the [model manifest](model-production.md). [Scene-assets](../../packages/scene-assets/README.md) owns bake, physical fit, moving-node bindings and runtime appearance validation. The audit does not bless generic tank/jeep art as a finished named platform.

## Narrow proof when enabling a row

Use the existing catalog smoke test to instantiate, move and fire admitted rows; planned entries never enter that iteration. Use focused flight/weapon checks for new envelopes and guidance, and [supply tests](../../crates/sim/tests/supply.rs) for finite resource service. A top-attack, carriage or APS claim requires its own behavioral test and actual implementation, not an added catalog label. Keep disabled metadata free of physical instantiation and required runtime model downloads.
