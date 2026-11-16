# Shared hosts: buildings and transports admit squads alike

**Status:** in progress; slices 00 to 06 done; 07's leftovers, recordings and docs done, its labs being merged. **Updated:** 2026-11-16.

Several squads share a host (a building or a carrier) while seats remain, one
seat per living soldier, squads whole. A squad gets in when it arrives and
fits. Nothing is promised or refused at the command except a squad bigger than
the whole host. This reverses the 2026-09-28 "a building takes one squad"
decision (`specs/done/battle-look/decisions.md`).

## Next Agent Prompt

You are implementing this plan from slice 07. Read, in order: this file, the
[decisions ledger](decisions.md), the slice you are about to build, and, for
the history of how each decision was reached, the
[unknowns map](unknowns-map.html) (its plan and prompt sections are
superseded by this file). Before changing player text read
[`game-ui`](../../.agents/skills/game-ui/SKILL.md).

Next pickup: **the rest of [slice 07](slices/07-labs-docs-closeout.md).**
Its leftovers (the unread navigation rules, the tweak-mechanics principle),
recordings (the browser captures, the counts fixture and what reads them;
the menu reel confirmed unchanged) and docs are done (ledger, Slice 07
recordings and docs). What remains, in order:

1. Merge the labs part (garrison lab, transport controls fixture, their
   descriptions, the tutorial line), built in a separate checkout, and run
   its browser scenes and lab shot gates as the slice says.
2. Run the full check ladder and the balance report once.
3. Run the whole-spec [`review`](../../.agents/skills/review/SKILL.md).
4. Consolidate the ledger: fold the slice sections into durable decisions.
5. Run [`close-spec`](../../.agents/skills/close-spec/SKILL.md); when the plan
   moves to `specs/done/`, repoint the superseded note in
   `specs/done/battle-foundation/contracts.md`.

The digest gates run with one command from the repository root:

```sh
throwaway/shared-hosts/probe.sh
```

It records every case's per-tick digest for HEAD (key: short SHA, plus
`-dirty` when `crates/` or `fixtures/` differ, tests excepted) and compares it
with `BASELINE` (e2c946e0a9), grouped by gate: `UNCHANGED_SINGLE_HOSTS`
(scripted building entry and exit, window swap, reinforcement inside,
collapse; `transport/board` for `Board` and `Collect`; `transport/heli`),
`UNCHANGED_OTHER_SCENES` (every other shipped encounter, the defender ones
included) and `CONTESTED` (the garrison lab and transport controls demos).
It prints each case's first differing tick and exits 1 if anything outside
`CONTESTED` moved; each case's `.facts` log shows what it reached.
`probe.sh record <name>` and `probe.sh compare <a> <b>` run either half.
`throwaway/` is ignored, so the probe exists only in this checkout; another
checkout copies `throwaway/shared-hosts/` across.

Host seats and admission live in `crate::hold` (`Seats`, `admit`,
`Seats::free`); a building's hold publishes `Seats`' total as `hostSeats`.

Checklist (each item is its slice's whole contract):

- [x] 00 · The relief test, red, and the digest probe baseline ([slice](slices/00-proof-and-probes.md))
- [x] 01 · One building order: contract cutover and every squad routed in ([slice](slices/01-one-building-order.md))
- [x] 01a · Spread entries: several squads in one order take approach points apart ([slice](slices/01a-spread-entries.md))
- [x] 02 · One window plan per building, outcomes unchanged ([slice](slices/02-one-window-plan.md))
- [x] 03 · Admission on arrival for both hosts; the relief test goes green ([slice](slices/03-admission-on-arrival.md))
- [x] 04 · Window swaps and reinforcement span the building ([slice](slices/04-building-wide-occupancy.md))
- [x] 05 · Seat total published; the IN BUILDING row reads used/total ([slice](slices/05-seats-on-the-panel.md))
- [x] 06 · One ring and one outline per house ([slice](slices/06-one-mark-per-house.md))
- [ ] 07 · Labs, recordings, player text, docs and closeout ([slice](slices/07-labs-docs-closeout.md))

Warnings:

- Every decision in the ledger is settled. If the work shows one is wrong,
  stop, record the evidence in the ledger, and reslice. Don't patch around it.
- Each slice lists facts to confirm first. They were read from the code by
  four independent drafts but not run; confirm them before building on them.
- The digest gates are the proof of this plan. A digest that moves outside the
  slice's named moving check is a bug to chase, never a re-record.
- Update this section (status, pickup point, checklist) before you end a pass.

## Contracts after the change

| Contract | Today | After |
|---|---|---|
| `Order` (`crates/contract/src/command.rs`) | `Garrison { units, building }`; `OccupyBuilding { units, building, gesture, facing }` | `Garrison` deleted. `OccupyBuilding { units, building, gesture }` |
| `BuildingPreviewRequest` | `{ units, building, facing, queued }` | `{ units, building, queued }` |
| `BuildingPlacement` | `{ building, entrant: Option<BuildingEntry>, destinations, unproven }` | `{ building, entries: Vec<BuildingEntry> }` |
| `OrderError` | `CapacityFull`, `OneSquadPerBuilding`, `BuildingOccupied` | `CapacityFull` only: bigger than the whole host (at the command), or no room on arrival |
| `HostRef::Building` (`crates/contract/src/observation.rs`) | `{ id, center, half }` | adds `seats: u32`, the building's seat total |
| Publication | refusal-name table lists both deleted reasons | both names removed; one host-seats field added (slice 05) |
| `UnitOrder::Garrison { building, approach }` (internal) | — | unchanged, digest tag unchanged; it is what every entry applies |

No backward compatibility and no migrations (user's call): old `garrison`
orders in scripts, labs and fixtures are rewritten, never aliased.

## One owner per concept

| Concept | Owner | Deleted |
|---|---|---|
| Host seats (total, taken, free) for both hosts | `crate::hold` | `carriage::seats_used`, `carriage::fits`, `garrison::held_by_another`, the silent slot-count drop in `hold::advance`, Board's closest-first bookkeeping in `battle.rs` |
| Admission (start check and end check) | `crate::hold`, one function that is the only constructor of `Phase::Entering` | the hold construction in `hold::advance` and in `carriage::start_boarding` as separate paths |
| The building's window plan and its allocation | new `crates/sim/src/garrison/windows.rs`, one plan per building owned by `Battle` | the per-squad copy in `Garrison.slots` |
| Where a squad stands to enter | `garrison::approach` | `occupy`'s `points`, `Candidate`, `rank`, `RouteJob`, `gather`, `EntryAction`, the selected-holder logic, and `movement/certify.rs` (its only caller is `occupy`) |
| The building order | `OccupyBuilding` | `Order::Garrison` |

End state: the code reads as if hosts had always been shared. There is no
buildings-only and carriers-only version of seats or admission, and no
transitional scaffolding survives a slice.

## Invariants

- One seat per living soldier; squads enter whole.
- A squad's start check counts squads stepping in or inside and ignores those
  stepping out; its end check counts everyone.
- Nothing waits: a squad refused on arrival ends its order where it stands.
- One side in a building at a time, checked when stepping in starts; an enemy
  inside makes the entrant give up silently, as today.
- A host with one squad behaves byte-for-byte as before.
- Carrier and building timer cadence is unchanged (a carrier hold starts and
  advances in the same tick; a building hold starts at `Entering(1)`).
- Same-tick arrivals resolve in unit order; no test relies on that order.

## Slice graph

```
00 → 01 → 02 → 01a → 03 → 04 → 05 → 06 → 07
```

Serial on purpose: each behavioural cut must be the only possible cause of the
first digest it moves.

## Digest gates

Slice 00 records the baseline in `throwaway/shared-hosts/` (ignored). Every
slice compares against it:

- `UNCHANGED_SINGLE_HOSTS`: single-occupant buildings and carriers, identical
  digest at every tick.
- `UNCHANGED_OTHER_SCENES`: every encounter except the contested garrison lab
  and transport controls fixture, including both menu reels (their event-log
  tests in `crates/sim/tests/menu_reel.rs` stay green and are never blessed).
- `LIVE_SCRIPT_REPLAY`: the replay-every-tick tests stay green.

Each slice names its one moving check, if it has one.

## Standing gates

- Tests pin mechanics, never config values: capacities and timings come from
  the test's own scenario or the published seat total.
- New sim tests go in `crates/sim/tests/shared_hosts.rs`. `tests/garrison.rs`,
  `tests/transport.rs` and `battle.rs` are over the file-size budget and may
  only shrink.
- Any slice that produces a shot runs
  [`compare-screenshots`](../../.agents/skills/compare-screenshots/SKILL.md)
  before against after, then an unprimed
  [`screenshot-critique`](../../.agents/skills/screenshot-critique/SKILL.md)
  as its last check, then shows the user with
  [`preview-shots`](../../.agents/skills/preview-shots/SKILL.md). Human
  review is non-blocking: about five minutes, then decide on the evidence,
  record it in the ledger, close the shots, and go on.
- The whole check ladder and the balance report run once, in slice 07.

## Scope firewalls

Not touched: `seat_plan`, `seats_for_state` and `capacity_soldiers`; carrier
sizes, unloading and bail-out; exit spots and collapse; encounter posts (one
squad per building stays a posting choice); skirmish AI strategy; selection
and picking (no new interaction); the helicopter's low hover for a pending
Board. No waiting phase, no command-time seat promises, no full-host cursor
warning.

## Risks

1. **The menu reel and street encounters run defender garrisons.** Slice 01
   keeps their entry byte-identical by applying exactly the old `Garrison`
   lowering.
2. **Moving the window plan off the unit** touches about ten readers and the
   digest fold. Slice 02 isolates it behind `UNCHANGED_SINGLE_HOSTS`.
3. **Losing route proofs.** A building the side can't reach yields no entry
   rather than an "unproven" one. Slice 01 checks the cursor still reads
   correctly.
4. **Wide cutover.** Rust contract, TypeScript protocol, voice lines and
   publication vectors change together in slice 01.
