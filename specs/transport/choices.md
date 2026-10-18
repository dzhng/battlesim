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

## Slice 07
- **The unload spot search starts at the carrier's centre** and ignores only the carrier's own hull.
- **Unloading in place always replaces the queue, even when it is refused;** the carrier stops at once.
- **Unloading an empty carrier is dropped as a no-op,** so the order can't get stuck.

## Slice 08
- **Bail-out starts at the carrier's wreck rest point.** The wreck is ignored only for reachability, since soldiers start inside it; survivors always stand outside every solid footprint.
  - The first pass let survivors stand inside the wreck. The orchestrator's review fixed it (`a2e6f55d2`), and the slice's missing tests were added there.
- **Only squads wiped out entirely enter `destroyed`,** which pays the kill reward. Soldiers lost from a squad that survives follow the existing corpse accounting.
- **A withdrawal refund includes only living passengers with a purchase receipt.**

## Slice 11
- **The board, collect and unload intents fire only in the default (move) mode.** In attack-move or reverse, right-clicking a unit falls through to a move.
- **An armed Unload has its own "unload" cursor badge.** Pressing the Unload button again while armed is the same as pressing U again.
- **The unload mark is the ordinary move line plus two chevrons off each side of the destination,** set perpendicular to the final facing.
- **A per-unit refusal shows beside that unit's card,** or its carrier's card for a rider, for 2.2 s. `capacity_full` reads TOO MANY TO FIT.
- **A riding squad's army-deck card is dimmed, carries the seat mark, and selects its carrier.**
- **The browser remembers its own accepted unload points (`unloadPoints.ts`),** because the publication doesn't say which queued step unloads.
  - Unsound: it makes the browser a second owner of the queue. Slice 11b replaces it with a published flag.
- **Two fixes outside the slice:** squads aboard no longer draw order marks; the garrison, fog and cover-sheet scenes read `hold`, not the retired `garrison` field.
- **Lint debt:** cleared by main's `af245b425` (the scenes' orphaned values were checks that had moved elsewhere), taken in the main merge.

## Slice 12
- **Three new response lines, `board`, `unload` and `no_room_to_unload`, for every voice set,** in both moods with two takes each. Collect speaks `board`. `no_room_to_unload` has its own line, not the generic refusal.
- **Board, collect and unload count as journeys,** so a trip that ends also gets the "arrived" line.
- **Sets that never carry anyone (fighter and attack-helicopter pilots, tank crews) still got the lines.** No order ever triggers them; they keep every set complete.
- **A take that fails the transcript check is left out of the pack, never packed.** Every line and mood still has at least one packed take: 725 packed, 31 left out.
- **One rewording:** the attack-helicopter set's "Troops boarding." became "Troops, get aboard." (it kept being heard as "Troops balding"). Four retakes passed.
- **Left out, for a listening pass** (set, line, mood, take: what the transcript heard):
  - cn_fighter_pilot no_room_to_unload/stressed/1: "Nowhere to add. Call out"
  - cn_ifv_crew, de_ifv_crew, it_ifv_crew, ru_ifv_crew unload/stressed/1 ("Dismount, go, go!"): heard as "Guess mount", "This round", "Yes, mount", "This round"
  - cn_transport_helicopter_pilot board/steady/1: "Terms abort"
  - de_atgm_team, ru_atgm_team, us_manpads_team board/stressed/0 ("Board, quickly!"): "Lord Brickley", "Move quickly!", "Report quickly"
  - de_marksman_squad board/steady/0: "Loading now"; ru_marksman_squad board/steady/0: "[laughs]"
  - de_recon_vehicle_crew board/steady/1: "Squatter board"
  - de_rifle_squad board/steady/0: "Loading"; us_assault_squad board/steady/0: "Warning!"
  - de_transport_helicopter_pilot board/steady/0: "Load in troops"; unload/steady/1: "We're here to unload"
  - fr_attack_helicopter_pilot unload/steady/1: "It's crystal clear"
  - fr_tank_crew no_room_to_unload/steady/1: "Enable no-hear ground"
  - ru_attack_helicopter_pilot board/steady/0: "Loading tubes"
  - ru_fighter_pilot, us_fighter_pilot no_room_to_unload/stressed/1: "Go ahead and land. Pull up", "Tell where to land. Call up!"
  - ru_manpads_team no_room_to_unload/stressed/1: "You are to get out!"
  - ru_recon_infantry board/steady/0: "Boarding complete."; board/stressed/0: "Mort, now"; unload/stressed/0: "Belt now"
  - ru_rifle_squad board/stressed/0: "Mort, move!"
  - ru_supply_driver unload/steady/1: "Passenger assault"
  - ru_transport_helicopter_pilot no_room_to_unload/stressed/1: "Two tine, pull away"
  - us_recon_infantry, us_sniper_team board/stressed/0: "Bored now"
  - us_rifle_squad no_room_to_unload/steady/0: "[outro jingle]"
- **Not done:** listening to the new lines in the sound workbench for tone and length against the building lines. Only a person can judge that.

## Main merge (team skirmish, capture)
- **Riders inside a carrier don't block a skirmish entry site;** main's per-entry dispatch counts only units on the field.
- **A retiring carrier still takes its passengers,** each refunded to its own buying player under main's per-player wallets.
- **The capture re-record waits for slice 11b,** which changes the publication layout again, so the captures are recorded once.

## Slice 09
- **One rule decides whether a carrier can load or unload:** `carriage::at_loading_height`. A ground carrier always can; a helicopter can only when settled at low hover, meaning zero velocity and exactly at low-hover height. Boarding and the start of an unload both ask it.
- **A Board order sent to a helicopter brings it down to low hover,** before any boarding timer exists. The timer needs the helicopter already low, so waiting for the timer would never start.
- **A shot-down helicopter's passengers fall with it** (they follow `crash.position`) and bail out where it hits the ground, with the air row. Each squad gets its own `squad_loss` roll, seeded with that squad's id. A shared roll would kill or spare every squad aboard together. The ground row has no squad loss, so ground draws are unchanged.
- **A helicopter that leaves the map with passengers kills them** (`DeathCause::LeftMap`, credited to the helicopter's killer). Their corpses stay off the map at the crash point. A retiring helicopter takes its passengers along, refunded.
- **One `Battle::passengers_of(carrier)` lookup** replaces four copies.
- **Test fixture:** `test_transport_heli` (16 seats). `test_heli` still carries no one.
- **Risk:** boarding needs the exact low-hover height over the surface. Where a building or tree holds a helicopter higher, a Board order can wait forever with no refusal. The lab's helicopter variant will show whether this matters.
- **The market-town menu reel was re-recorded** (`BLESS_MENU_REEL=1`). Main's reel puts a garrison collapse at tick 2093, and the building bail-out lets two soldiers live where the old coin flip killed them. Every shot still opens on a living unit; the footage after that tick needs the user's look.

## Slice 13
- **Seats sit on each roster row, not on the shared base profiles.** Those profiles also cover vehicles that carry no one (M-SHORAD, Skyranger, Patriot, NASAMS, Pantsir, S-300, Buk, Mi-28, Ajax).
- **D3's list is read literally:** 32 carriers (5 × 16, 20 × 8, 7 × 3). Only the variants D3 names carry anyone:
  - the MH-6M yes, the AH-6M no;
  - the Mi-35M yes, the Mi-28NM no;
  - the Stryker ICV, Dragoon and RV yes; the ATGM, Mortar and M-SHORAD no;
  - the LAV-AT, Gepard, Ajax, Fennek and Jaguar no.
- **"Passenger carriage not implemented" became "infantry carrier"** in 18 descriptions. The air-defence rows lose the clause.
- **The orchestrator checked the seat table by hand** against D3; no test pins it.
- **The skirmish parity records keep their frozen catalog text,** old descriptions included. They are test inputs; the digests didn't move.
- **The purchase picker shows no seat count:** its cards have no capability line.

## Slice 14
- **One owner, `skirmish_ai/transport.rs`.** It picks a carrier by data: the fewest seats that fit the squad, then the cheapest, then id. Every roster squad is 2, 3, 6 or 8 strong, so the AI never buys a 16-seat helicopter as a taxi; there is no separate rule against it.
- **Pairs are rebuilt at each decision, not stored.** A squad aboard, boarding or leaving a carrier belongs to it. A carrier already driving to a squad stays its ride, which stops the AI swapping squads between carriers.
- **The carrier collects the squad (Collect)** rather than the squad walking back to it.
- **Where it unloads:** short of the objective, out of reach of the enemies it has identified. With no enemy identified, that is 20% of the approach short (`unload_share`).
- **Rides toward the fight:** a squad sent after a far-off identified enemy rides too. With objectives alone, only 4 of 14 squads ever rode.
- **Empty carriers:** an armed one fights through the ordinary combat orders; an unarmed one waits. Pairing still claims an armed carrier first.
- **The purchase rotation counts an armed carrier as a combat vehicle and never buys an unarmed one.** Only pairing buys unarmed carriers.
- **New tuning** in `fixtures/skirmish-ai.json`: `unload_share` 0.2, `ride_min_m` 120 (shorter trips walk) and `reaim_m` 30. All three are first guesses.
- **Played probe, seed 7, 360 s:**
  - 17 carriers for 16 squads per side;
  - Blue 30 collects and 51 unloads, Red 19 and 35;
  - 14 empty armed carriers fought on each side;
  - up to 5 squads aboard at once;
  - no order refused.
- **Risks:**
  - An armed carrier in a fight breaks off to fetch a squad. The adjustment, if it looks wrong, is to prefer idle carriers.
  - An in-place unload refused for lack of room is retried at every decision.
- **The lab's `heli` variant** shows one board-and-unload trip: hover at 6.1 m, SEATS 8/16 in flight, all 8 soldiers out at the goal. The shot-down helicopter has tests but no shots.
- **Not changed:** the Seats row counts a squad that is still boarding. Boarding is a timed stand-still beside every kind of carrier.
- **Found:** soldiers under a hovering helicopter were drawn over its fuselage. That is a renderer depth bug for any low aircraft over infantry, sent to a renderer fix.

## Slice 11b
- **The simulation says which steps unload:** `goal_unloads` on the own unit and `queue_unloads` beside `queue`, one flag per step. A step unloads when the order right after its move is an Unload; the rule lives only in the sim (`Unit::unloads_after`). The other option, turning `queue` into step records, would have changed every reader of `queue`.
- **Layout:** the own row gains `goalUnloads` (56 fields), and each queue step gains `unload`. `unloadPoints.ts` and its whole path are deleted.
- **The unload mark is pixel-identical:** compared against a fresh run of the pre-11b build. Slice 11's saved evidence didn't match pixel for pixel even on the untouched move line.
- **Captures re-recorded once** (`f8f276f0b`) for the hold, refusal and unload-step layout. Both scenes decode to the same observations as before, apart from the new fields.

## Closeout review
- **A squad being collected boards only when it is near and still;** a farther squad gets a Board order and walks over. Before, Collect pulled every listed squad onto the hull.
- **Boarding soldiers stay standing on the ground.** Only squads riding inside follow the carrier (`Hold::riding`). Before, boarding soldiers floated up to a hovering helicopter's height.
- **The referee's survival test counts every living armed unit, riders included;** only units on the field hold the zone. Before, an attacker whose infantry all rode lost at once.
- **A destroyed carrier spares squads still getting in,** as a collapsing building already did.
- **Seats are counted in one place** (`carriage::seats`, `seats_used`, `fits`), used by orders, the simulation and the AI. A squad still getting in keeps its seats reserved.
- **Each squad gets its own bail-out rolls,** in buildings too. This changed the market-town collapse again, so the reel was re-recorded again; every event before tick 2093 is identical.
- **A helicopter's settled hover is the height it actually holds** over props and canopy, so boarding over a lamp or forest no longer waits forever.
- **Hold state and digest moved into `sim::hold`,** with one timer. Every hold starts its timer on the same tick. Exit-spot searches ignore soldiers riding other carriers. No digest changed.
- **The browser speaks per-unit refusals** from `CommandAck.refused`: "no room to unload" is now heard, and a refused squad isn't in the accepted answer. A refusal the ack already spoke isn't repeated from the frame.
- **One callout per refused unit;** `inBuilding(u)` replaces in-building checks that also matched riders.
- **The panel specimens and the transport scene pick carriers from data,** not by unit id or label.
