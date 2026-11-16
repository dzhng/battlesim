# Shared hosts: buildings and carriers admit squads alike

**Status:** shipped 2026-11-16.

Several squads share a host (a building or a carrier) while seats remain, one
seat per living soldier, squads whole. A squad gets in when it arrives and
fits; nothing is promised or refused at the command except a squad bigger
than the whole host. A squad that arrives to no room hears "TOO MANY TO FIT"
(`CapacityFull`) and its order ends where it stands. Squads sharing a house
fight from it as one garrison, and a gun that sees them at the windows can
fire on them.

This reverses the 2026-09-28 "a building takes one squad" decision
(`specs/done/battle-look/decisions.md`), which was taken for simplicity and
had deleted the waiting phase and slot sharing with it.

[decisions.md](decisions.md) is the final ledger: every decision the code
carries that the ask did not spell out, who made it, and how confident the
audit is that the user would agree. Two items there are still the user's:
the Paris-corner menu reel now plays with no casualties (N1), and whether
guns should aim at seen soldiers for any squad whose middle is blocked, not
only garrisons (N2). [unknowns-map.html](unknowns-map.html) is the
2026-11-16 walk that reached the user's decisions, kept as history. The
ledger's "slice" numbers name the build passes; the branch's commit
subjects carry them.

## Why it works this way

- **Arrival decides, so the relief manoeuvre works.** The user's own
  example fixed the rule: send squad B to a house squad A holds, order A
  out, and B walks in when it gets there. "This is GOOD, it's a feature."
  Promising seats at the command would refuse B, and so would any warning
  on a full house, so neither exists. The order planner never reads who is
  inside.
- **Leavers don't count when stepping in starts; everyone counts when it
  ends.** A host uses one timer for both directions, so a squad already
  stepping out is gone before the newcomer finishes stepping in. If the
  leaver turns back instead, the newcomer is the one refused, at the end of
  its timer.
- **Nothing waits.** A waiting phase was deleted in September and stays
  deleted: a squad refused on arrival ends its order, and the player hears
  why.
- **One building order.** There used to be the player's `OccupyBuilding`
  (one entrant plus companions parked nearby) and an `Order::Garrison` the
  encounter defenders sent. Arrival admission made the parking spots and the
  one-squad refusals meaningless, so one order remains, and every listed
  squad gets its own approach, spread apart when several go in together.
- **No route proof.** The old order replayed the battle ahead to prove the
  entrant would end up inside, which the relief manoeuvre fails by design
  (the house is full when the proof runs). An entry needs only that the
  side's navigation grid reaches the approach, as boarding a carrier always
  did, or that the squad already stands within entry reach; it finds out
  the rest on the way. The proof's private replay of the world went with
  it.
- **One window plan per building.** Each squad used to carry its own copy of
  its building's windows; with several squads the copies would drift. One
  plan per building lives on the battle, and each window remembers when a
  soldier last came to it, which is what the hold time after a swap reads.
- **Window swaps span the house.** A veteran expects the AT gunner at the
  window facing the tank, whoever's rifleman held it. A lone weapon's
  operator outranks one of many riflemen, whichever squad either is in.
- **A garrison is shot at its windows.** A gun aims at a squad's middle,
  and a garrison's soldiers are spread over windows round the house, so
  that middle can lie inside the walls, where every shot met the house.
  Sharing made it routine (two squads cover every facade), but a lone
  garrison could be just as unshootable. A gun tries each soldier its side
  saw at the windows instead, for every garrison, lone ones included. The
  rule is scoped to garrisons; widening it to any squad whose middle is
  blocked is open (ledger N2).
- **One mark per house.** A shared house draws one ring and one outline: the
  ring means "in this building", not a squad count. The IN BUILDING row
  shows the house's seats used of total, the carrier's SEATS word, so a
  player can see how full it is.

## Invariants

- **One owner for seats and admission, for both hosts.** `crate::hold`
  (`crates/sim/src/hold.rs`): `Seats` (total, staying, leaving),
  `Seats::free`, `admit` (the only place a hold begins, in its stepping-in
  phase, for both hosts) and `seated` (the end-of-timer check). There is no
  building-only or carrier-only copy. Command-time answers come from
  `hold::refuse_oversized` and `refuse_oversized_riders`.
- **One seat per living soldier; squads enter whole** (`hold::living`).
- **Start check counts squads stepping in or inside; end check counts
  everyone.** A refusal at either pops the order and publishes
  `CapacityFull`; nothing queues the squad.
- **One side in a building at a time.** Any hold on it by a living squad of
  another side, in any phase, makes an entrant give up its order silently.
- **Admission keeps a lone squad's host byte for byte as before.** Entry,
  seating, window swaps, reinforcement and the encounter defenders' entries
  were held identical for a host with one squad through every admission
  pass; a change that moves them is a change to this rule, not a re-record.
  The one deliberate exception is engagement: firing at a garrison's seen
  soldiers applies to lone garrisons too, and moved the street and
  Paris-corner scenes and the Paris-corner menu reel (ledger N1, S3).
- **Cadence.** A carrier hold starts and advances in the same tick; a
  building hold starts one step into its timer. Same-tick arrivals resolve
  in unit order, and no test relies on that order.
- **One window plan per building, keyed by its structure owner** (the part
  that stands for the whole building), opened by the first squad seated and
  closed once a tick when no living squad holds the building
  (`garrison::Windows`, `crates/sim/src/garrison/windows.rs`). The battle
  digest folds it inside each occupant's hold.
- **A building's seat total** is its plan's length while a living squad
  holds it, else the windows its standing state offers (`seats_for_state`).
  Admission, reinforcement and the published `hostSeats` all read this one
  number.
- **A reinforcement needs a free seat in its host**, counted the same way as
  the end check, so it can never make a squad fail its own entry. Riders are
  never reinforced (`Unit::on_field`).
- **Fog is kept when firing at a garrison:** only soldiers in the side's own
  track are aimed at, and the building's shell still stops every round.

## Contracts

- `Order::OccupyBuilding { units, building, gesture }` is the one building
  order; `Order::Garrison`, `OrderError::OneSquadPerBuilding` and
  `OrderError::BuildingOccupied` are gone (`crates/contract/src/command.rs`).
  Its `gesture` lets a double right-click hurry the squads' walk in, as
  `Board`'s hurries the walk to a carrier (ledger S38).
- The preview is `BuildingPreviewRequest { units, building, queued }`,
  answered by `BuildingPlacement { building, entries }`, one
  `BuildingEntry { unit, approach }` per squad that can get there.
- `CapacityFull` means bigger than the whole host (at the command) or no
  room (on arrival or at the end of the timer).
- `HostRef::Building` carries `seats`, the seat total
  (`crates/contract/src/observation.rs`), published on the own unit row as
  `hostSeats`; seats used is summed from the side's holds by the client.
- No backward compatibility: old `garrison` orders were rewritten, never
  aliased.

## Where it lives

- Seats, admission, command refusals, published seat total:
  `crates/sim/src/hold.rs`.
- The building order's plan (approaches, spreading, reach):
  `garrison::occupy` (`crates/sim/src/garrison/occupy.rs`) and
  `garrison::approach` (`crates/sim/src/garrison.rs`).
- Window plans, seating, swaps (`Windows::allocate`), free seats:
  `crates/sim/src/garrison/windows.rs`.
- Firing at a garrison's seen soldiers: `seen_soldiers` and the fallback in
  `crates/sim/src/weapons/assessment.rs`.
- Reinforcement room: `supply::service` (`crates/sim/src/supply.rs`).
- The IN BUILDING row: `web/src/battle/present/panelRows.ts`. One ring and
  one outline per house: `oneRingPerHouse` (ranked by `ringRank`) and
  `heldBuildingsLayer` in `apps/battle-lab/src/battleOverlay.ts`. Cards of units on one spot:
  `oneSpot` in `web/src/battle/present/readoutLayout.ts`.
- Tests: `crates/sim/tests/shared_hosts.rs` (the relief manoeuvre,
  `a_squad_ordered_into_a_held_house_walks_in_when_its_holder_leaves`; both
  hosts sharing and a latecomer refused; every listed squad walking in) and
  its submodules `shared_hosts/occupancy.rs` (one garrison's swaps and
  reinforcement) and `shared_hosts/engaged.rs` (a tank firing on a shared
  garrison); "squads on one spot all show" in
  `web/tests/readoutLayout.test.ts`. In the browser, the garrison lab
  (`web/scenes/garrison.mjs`) and transport controls
  (`web/scenes/transport.mjs`) show two squads sharing and a third finding no
  room.

## Dead ends

- **Seats promised at the command.** Refuses the relief manoeuvre.
- **Leavers keeping their seats.** Makes the relief manoeuvre a timing
  trick.
- **A bounded route proof with "reached the building" as success** (three of
  four plan drafts). Kept about 25 KB of machinery built for companion
  gathering and risked changing the defenders' entries.
- **Window-change time on soldiers** (the walk map's first plan). Would have
  changed every scene with a window swap.
- **Requests ordered strictly by unit and member index.** Reorders a lone
  squad's own window requests.
- **Spacing entries by the formation's ring size.** Around a lab-sized house
  no two points are that far apart, so every later squad fell back to the
  shared nearest point.
- **A scripted ground attack for the garrison lab's tank.** A workaround for
  garrisons nobody could engage; the engagement fix replaced it and the tank
  is back on fire at will.
- **Aiming at seen soldiers for any blocked squad.** Tried, and it moved
  squads in the open and several recordings; left for the user (ledger N2).

## Visual provenance

No reference images drove this plan. Its pictures were before-and-after
shots of the garrison lab, the panels lab and the transport controls lab,
judged against the previous build, and they lived in the ignored scratch
folder. The approved panels and UI scene pictures were re-approved where
the row text and refusal callout changed, and a new approved panels picture
(`panels-shared-house`) shows a shared house's row.
