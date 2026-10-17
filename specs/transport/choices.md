# Transport: choices ledger

The calls the implementing agent made where the plan was silent, one entry per call. Each entry says:
- the slice it came from;
- the choice, and the alternative it beat;
- why;
- what it reaches;
- how to reverse it.

Decisions the user made live in [map.md](map.md); change them there.

## Slice 06 on_field audit

`Unit::alive()` remains the accounting predicate. World-facing readers use
`Unit::on_field()`, which excludes a living passenger in a carrier's
`Inside` or `Exiting` phase.

| Reader | Verdict | Reason |
|---|---|---|
| bodies, blasts, near-miss suppression, shoves, crowding, `clear_of`, treads | `on_field` | A passenger has no independent collision body. |
| sensing, fog sweep, hearing, radar, shouts | `on_field` | A closed carrier is the only observed source. |
| objectives and supply service | `on_field` | A passenger cannot capture or receive service through the hull. |
| encounter referee attackers and zone presence | `on_field` | Referee presence is physical presence. |
| unit cap, `Load`, casualty settlement and withdrawal accounting | `alive` | A passenger remains a living purchased unit until it exits or dies. |
| scripts, order validation, pursuit and weapon settlement | `alive` | These paths account for identity and health; target-facing paths separately reject or skip aboard units. |

## Slice 06

- **The carrier timer starts at `Entering(0)`.** Lowering the order and advancing the hold happen in the same tick, so starting at 0 keeps the full 1 s.
- **The first pass ran its own carrier timer in `carriage.rs`, and its `Host::building()` panicked on carrier holds.** The orchestrator's review sent it back.
  - The hold phase and timer now run only through `hold::advance`, with host hooks (`carriage::near_and_still`, `carriage::admit`).
  - `Host::building()` returns an `Option`.
  - Running out of seats at the end of the timer publishes `CapacityFull` in `ObservationFrame.refusals`.
- **Old parity scenarios got `rules.carriage` from the game fixture** before being re-blessed.
- **After its timer, a squad riding in a carrier has no body.** Drawing, picking and sound all take poses from one interpolated sample, and that sample leaves out own units riding in a carrier (`178927cf2`). Passengers stay in `frame.own`.
- **A Board approach ends once the carrier is within the hold's reach** (`reach_m` plus the hull). Before, the squad walked at the carrier's centre and stalled against its hull (`09fb08609`).
- **The lab's collect case uses the 3-man `test_at` team.** The 4-man `test_recon` doesn't fit the scout car's 3 seats.
- **The lab has no objective yet.** The slice asked for driving past an enemy and through an objective; only the enemy is there.

- **Carrier reach is 3 m.** Alternative: reuse the building's 4 m entry distance. Why: the slice delegates a reach below 4 m; the hull's footprint supplies the final approach room while the authored threshold remains a closer boarding distance. Reach: boarding and collection admission. Reverse: change `fixtures/game.json` and the carriage rule checks.
- **Test carriers explicitly clear inherited roster metadata.** Alternative: let `test_apc` and `test_scout_car` inherit `test_jeep`'s later test roster patches. Why: test-only carriers must not become extra skirmish cards when a test patches `test_jeep` into a deck. Reach: test catalogs and AI unit-selection fixtures. Reverse: remove the `roster: null` fields and update every deck test's candidate filtering.
- **Collect approaches to the carrier hull's reach envelope and boards when stopped.** Alternative: drive to the squad centroid exactly and rely on the generic route stop. Why: the carrier needs a physical hull-side approach and a stopped 1 s timer; the envelope avoids a permanent last-metre stall. Reach: `Collect` movement and the transport scene. Reverse: move the approach calculation into the carrier order owner.


## Slice 05
- **Recon prices stay the same after dropping a scout.**
  - Alternative: cut the price in proportion to the squad's size.
  - Why: the slice's default; nothing asked for retuning prices.
  - Reach: recon cards cost the same for three soldiers.
  - Reverse: edit the cost on the four rows in `fixtures/units/roster/*.json`.

## Slice 02
- **`Unit::garrisoned()` stays the name of the building-only query**, rather than being renamed.
  - Why: it still means "inside a building".
  - Reverse: rename it if carriers make the name ambiguous.
- **Building seat data lives in `Place::Building(Garrison)`, while `Hold` owns the host and phase.**
  - Alternative: flatten the seat fields into `Hold`.
  - Why: keeps building specifics in `garrison`.
- **`Hold` has a temporary `Deref`/`DerefMut` to `Garrison`** so building code reads unchanged. It panics on a non-building host.
  - This is short-lived scaffolding: slice 06 removes it when `Host::Carrier` arrives, and building code then reaches its `Place` explicitly.
- **The digest proof was replay-based.** The garrison and buildings modules ran before and after (54 and 32 tests), the 1800 per-tick replay digests matched, and `cargo test -p sim` passed. A probe listing every battle's final digest wasn't practical through the private test helpers.

## Slice 01
- **A falling helicopter stops on a bridge deck only if the deck was below the airframe on the previous tick.** One flying under the deck falls to the riverbed.
  - Alternative: always stop on the deck.
  - Why: an airframe below a deck must not be caught by it.
- **Bridge slabs don't count as obstacles while the airframe falls,** so it can land on them. They also don't block the wreck's resting place on a deck. Other immovable props stay obstacles.
  - Reverse: `crash.rs`, `Crash::fall` and `resting_place`.

## Slice 03
- **The own row carries the host as `hostKind` plus the host id and geometry columns.** One layout serves building and carrier hosts, so slice 06 adds no new columns.
- **The refusal reason table is fixed and published,** so slices 06 and 07 add reasons without another layout change.
- **Hold progress stays a 0–1 fraction,** keeping the existing UI behaviour.
- **The captures were re-recorded from one new browser run** (`922f3416c`).
  - The market-town window has had no script since it was first made, so a scratch example (not committed) rebuilt it from a native replay. It was checked byte for byte against the run's layout and samples.
  - That window's content also moved, because commit `ee1651c09` had retimed the menu battle after the previous capture. The new window matches the current game.
  - The paris-corner poses moved because this run drew fewer frames (2461 against 4473); they are sampled at frame times.

## Slice 04
- **Bail-out rows are keyed by an enum of physical host kinds** (`BailOutHost`: building, ground, air) in `rules.bail_out`. The caller picks the row through `bail_out::row`; `hold::Host` only supplies the event's random key.
- **A survivor's spot must be reachable from where he stood,** with the destroyed host's own parts ignored. This keeps the old rule: no appearing behind a neighbouring wall.
  - The first pass dropped the rule. The orchestrator's review restored it, and a courtyard test now pins it.
- **Soldier spacing for forced exits stays 1 m** (`arrangement::ESCAPE_SPACING_M`).
- **Partial deaths go through the existing corpse and unit accounting;** the observation is unchanged.

## Slice 10
- **Riders are drawn inside the carrier's card** by one component, `PassengerCards`, used in battle and in the panel workbench. The stack is laid out, hidden and hovered as one card.
  - Alternative: give each rider its own callout and add a second layout rule for nesting.
  - Reach: a stack that doesn't fit hides as a whole.
- **A state row can be marked `compact`,** meaning it stays on compact cards. Only `seats` is marked.
  - The Seats row is a count, never turns amber, and keeps its words in far view.
- **`passengersOf(own, carrier)` (`panelRows.ts`) is the one place that works out who rides what,** from `aboardCarrier(u)` (`observationView.ts`). Squads still boarding or getting out count toward seats and appear in the stack. Riders are listed in the side's unit order.
- **A passenger's hold row depends on its host:** BOARDING or DISMOUNTING with a timer, and nothing while inside. Building rows are unchanged.
- **Riders dim to 0.8 of the carrier's brightness** and line up with the row words, 6 px apart.
- **Selection follows D10.**
  - The control drops any squad in a carrier hold, in any phase, from every selection path.
  - Clicking a rider's card selects the carrier.
  - The first pass let a rider's card select the rider; the orchestrator's review overturned it (`cc3e23474`).
- **The Seats icon is a generated car seat.** Five unprimed critiques went through a chair, a seated figure and a row of seats; the last critique still found the car seat unclear. **Open:** a clearer drawing is welcome.
- **The transport lab camera sits 150 m out** (it was 380 m), centred on the carriers.
- **The panel specimens give the test jeep 8 or 16 seats,** as the radar specimens already do.
- **Two fixes outside the slice, each in its own commit:** stale generated icons (`e096f25a9`), and the panels scene's run split under the complexity budget (`cc095450a`).
