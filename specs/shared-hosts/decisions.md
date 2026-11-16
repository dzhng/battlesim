# Shared hosts: decisions ledger

Settled decisions, with why and what lost. Sources: the 2026-11-16 unknowns
walk with the user ([map](unknowns-map.html)) and four independent plan drafts
(fewest slices, risk first, seam quality, determinism) synthesised into the
[README](README.md). Add new entries at the bottom as implementation decides
things the plan left open.

## From the walk

- **Several squads share a host; overflow ends up outside** (user, Q1). The
  same for buildings and transports: the user widened the question from
  buildings to both hosts.
- **Seats are taken when a squad starts stepping in, never at the command**
  (Q2; agent on the user's behalf, then locked by the user's relief remark).
  First to arrive gets in. At the command only a squad bigger than the whole
  host is refused. *Why:* the user's own example, "I can have one squad walk
  over, then tell the other squad to leave, and by the time the other squad
  reaches it'll be empty. This is GOOD, it's a feature." Lost: promising seats
  at the command, which would refuse that manoeuvre.
- **A squad arriving to no room ends its order with `CapacityFull`, never
  waits** (user, Q3). The player hears why. Lost: a waiting phase (deleted in
  September; stays deleted).
- **One building order, `OccupyBuilding`** (territory, Q4). `Order::Garrison`,
  `OneSquadPerBuilding`, `BuildingOccupied` and companion gathering are
  deleted; the defender sends `OccupyBuilding`.
- **Window swaps span every squad in the building** (user, Q5). The AT gunner
  takes the window facing the tank whoever's rifleman held it. Lost: per-squad
  windows split at entry.
- **The start check ignores squads stepping out; the end check counts
  everyone** (user, L1). Equal in and out timers per host mean a squad that
  began leaving before you arrived is gone before you finish stepping in.
  Lost: leavers keeping their seats (makes the relief manoeuvre a timing
  trick).
- **One ring per house** (user, U1): the same ring whichever occupants are
  selected; it means "in this building", not a count.
- **The IN BUILDING row shows seats used/total** (user, U2), like the carrier
  Seats row.
- **No full-host cursor warning** (user, U3). It would discourage the relief
  manoeuvre.
- **The relief manoeuvre is a required end-to-end test** (user).
- **Labs show the new rule** (user, U4): two squads share, a third arrives to
  "no room", in the garrison lab and the transport controls fixture.
- **No backward compatibility, no migrations** (user). Hard cutover.
- **Reinforcement needs a free seat in the host**, both hosts (shown, card 2).

## From the drafts

- **Entries use the old `Garrison` lowering; the proof machinery is deleted.**
  Each listed squad's approach is `garrison::approach` from its queue's end (or
  where it stands), with the same reach the old order used; applying the order
  pushes `UnitOrder::Garrison { building, approach }` exactly as the old order
  did. A squad is kept if it is within entry reach or the side's grid reaches
  its approach. `movement/certify.rs`, the candidate search and `unproven` go.
  *Why:* `Board` has no proof, so this is the consistent rule; the defender's
  entries stay byte-identical by construction (the menu reel); and the old
  proof replays the battle and requires the entrant to end up inside, which
  the relief manoeuvre would fail. *Alternative (three of four drafts):* keep
  the bounded proof, route each squad, and change its success to "reached the
  building". Lost because it keeps 25 KB of machinery built for the companion
  plan and risks changing the defender's lowering. If grid reachability gives
  wrong answers for buildings behind rows, stop and reslice slice 01.
- **Window-change time stays per window, on the shared plan** (three drafts;
  corrects the map's card 3). Moving it onto soldiers changes the digest of
  every scene with a window swap, including empty windows' history.
- **The shared plan lives on `Battle`, keyed by the building's structure
  owner**, created on the first admission and dropped when no living squad
  holds the building. The digest folds it inside each occupant's hold digest
  exactly where `digest_place` folds it today. *Alternative:* an `Arc` shared
  between holds (one draft), lost because the per-window change time needs
  mutation.
- **No new selection interaction** (determinism draft). Overlapping rings keep
  today's nearest-centre pick; soldiers and deck cards still select single
  squads. *Alternative (three drafts):* clicking a held house selects every
  own squad inside. Lost to the scope firewall; revisit only if the user asks.
- **Collect refuses at the command only a squad bigger than every listed
  carrier.** Its overflow is decided on arrival. This also fixes a latent bug:
  an overflowing Collect today refuses whichever squad finishes its timer
  first, because later squads' reservations already count.
- **The building's end-of-timer failure now publishes `CapacityFull`** (it
  drops silently today). An enemy inside stays a silent give-up.
- **Riders are never reinforced** (`Unit::on_field` excludes them), so
  reinforcement only touches buildings and squads stepping in beside a
  carrier. At full capacity, supply spends neither stock nor a soldier id.
- **The building seat total is published on `HostRef::Building`; used is
  summed client-side** with the same phase rule as the carrier Seats row.
  Enemy panels gain no capacity.
- **`OccupyBuilding` goes only to infantry**, through the existing
  `commandReach` predicate, so a mixed selection doesn't refuse the whole
  order.
