# 04 · Window swaps and reinforcement span the building

**Unlocks:** squads sharing a house fight as one garrison, and a reinforcement
can't overfill a host.

## Seam

- **Allocation (`garrison/windows.rs`)** runs once per building over every
  occupant. Order requesters by a stable (unit index, member index) and keep
  today's free-window preference, distance and index tie-breaks, weapon
  priority, `window_hold_s` and `window_swap_s`. A soldier's claim on his
  window is judged by his own squad's weapon aims. A squad stepping out keeps
  its windows until it has left; its soldiers stay targets there.
- **Free seat** is host-wide: a window no living occupant holds, and room in
  the host's `Seats` (stepping-in reservations count).
- **Supply (`crates/sim/src/supply.rs`)** checks room when choosing the next
  need and again when delivering a replacement. With no room it spends
  neither stock nor a soldier id. Riders stay unserved (`on_field`).

## Tests (`crates/sim/tests/shared_hosts.rs`)

- `squads_sharing_a_house_fight_as_one_garrison`: two squads fill a house; the
  AT gunner of one takes the other squad's rifleman's window facing a tank;
  both respect the swap and hold times under alternating threats; a
  reinforcement is blocked while the house is full and delivered after a
  casualty frees a seat; a squad stepping in beside a carrier can't be
  reinforced past the carrier's seats. Replay it every tick.
- Rewrite `a_squad_reinforced_during_entry_never_enters_only_partly_seated`
  (`tests/buildings.rs`): the reinforcement is now held back, so the squad
  enters whole.

## Verification

- `UNCHANGED_SINGLE_HOSTS` (single-squad window fights and supply are
  byte-identical) and `UNCHANGED_OTHER_SCENES`.
- Named moving check `SHARED_OCCUPANCY_BEFORE_AFTER`: only contested runs
  move.

## Delegated

Internal data structures for the flattened occupant list.

## Must stay green

`garrison::` (including `an_atgm_gunner_trades_windows_with_a_rifleman_to_face_armour`
and `alternating_threats_do_not_shuffle_the_squad_faster_than_its_hold`),
`supply::`, `buildings::`, `garrison_sight::`.
