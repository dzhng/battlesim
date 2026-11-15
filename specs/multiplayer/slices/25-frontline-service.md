# 25 Frontline service

**Stage:** Frontline. **Needs:** 18, 21 to 24. **Carries:** decisions 6, 18,
21, 27, 61, P20, P21.

## Contract

Frontline over the wire: Play finds or makes a game and reserves a seat;
Deploy takes it; leaving, dropping and idling give it back; the last human
out ends the game.

## Seam

- **`crates/game-wasm`:** `BattleHandle` gains a referee view for the server:
  both scores, the target, and each seat's controller and unit count (P21).
  It also binds slice 22's and 23's events. The authority gains the two
  requests that carry them; only the battle host can send them.
- **`apps/game-server/src/frontline.ts`** (new), with its routes in `http/`:
  keeps a `GameView` per game from the referee view, and runs `place` and
  `seatsToFree` from slice 24.
- **`POST /frontline/join`:** reserves the seat for about a minute and
  answers with the `Briefing`.
- **`POST /frontline/:battle/deploy`:** in one tick, the seat's army is
  handed over, the seat is reset with or without opening credits, and its
  controller becomes human. Answers with the ticket.
- **Leaving, or being gone by the presence module's word:** the seat's controller goes
  back to its AI, which keeps the units and credits. The seat is held for its
  player for about a minute, then is free for the next joiner.
- **Idle:** after about three minutes with no order the same happens, and the
  page is sent back to the menu with a line saying why.
- **End:** the result shows, every page returns to the menu, the game is
  deleted. A game with no human holding or reserving a seat is deleted.

## Run or see

Two browsers and `curl`: Play, Deploy, fight; a second player lands on the
losing side; one leaves and returns inside a minute to their own seat; the
other idles out; the last one leaves and the game is gone.

## Verification

`web/tests/multiplayer/frontlineService.test.ts`, at the service seam:

- A joiner's displaced army is fighting under an AI seat on their side, and
  the joiner has opening credits and no units.
- A player who leaves and returns within the minute has their own seat and
  army back, with no seat event.
- A player who returns later joins afresh, with no second opening grant.
- An idle player's seat is run by its AI and offered to the next joiner.
- A reservation that is not deployed in time lapses, and the seat is offered
  again.
- A game past 90% takes no joiner.
- The game is deleted when its last human has gone, and its slot is free.
- The whole game's replay, with its seat events and controller changes, ends
  on the same digest as the live game.

Read slice 01's O4 figures here: how long a joiner waits for the snapshot at
the target's length. If it is more than the briefing can cover, tell the
user before slice 26.

## Must stay green

Slice 18's room battle tests: a room battle records no seat event.

## Delegated

The referee view's shape; the timings' exact values.

## As built (2026-11-15)

- **`POST /frontline/join`** runs the placement model and answers with a
  `Briefing`: it resumes a seat the caller still holds, reserves a seat in
  a running game, or makes a new game. Refusals: `full` (no battle slot),
  `unavailable` (no battlefield could be made), `callsign` (no `/hello`).
- **`POST /frontline/:battle/deploy`** takes the reserved seat and answers
  with the ticket, or `expired`.
- **`POST /frontline/:battle/leave`** gives the seat back to its AI. The
  plan had no endpoint for leaving on purpose.
- **Seating a human in a running game is one message to the battle's
  thread:** hand the seat's army over, reset the seat (with opening credits
  the first time this player joins this game), and make its controller
  human. Both seat events apply at the same next tick, in that order, and
  only whole ticks are published, so no record shows it half done. A resume
  changes only the controller.
- **`reset_seat` is admitted behind a queued hand-over** of the same seat
  for the same tick; it is still checked for emptiness when it applies.
  That is the one change to slice 23's rules.
- **The referee view** (`Battle::referee`, bound as `referee_view`): tick,
  both scores, the target, phase and result, the opening credits, and for
  each seat its player, side, controller, living units and credits. Only
  the battle host can ask for it or send a seat event; a page's connection
  and the HTTP routes have no way to say either.
- **A seat comes free** when its player leaves, is gone past the one
  presence grace, or gives no order for three minutes. Its AI takes the
  army and wallet. The seat is held for its player for a minute, then
  offered to the next joiner. An idle player's battle stream ends with
  `idle`; other seats' streams carry on.
- **A game ends** when it finishes, crashes, or its last human has left and
  nobody holds a reservation. Rooms and Frontline share one count of battle
  slots, one presence module and one way of serving a battle
  (`apps/game-server/src/online.ts`).

Two defects found at integration and fixed here:

- **Any seated player could end a battle for everyone** by sending one
  order or preview the simulation could not read: the authority treated it
  as a crash. This was true of rooms too. The authority now refuses such a
  request to its sender (`invalid` for a batch of orders, `dropped` for a
  preview) and the battle goes on; a real trap in the simulation still
  ends it. Two of the pass's own tests had pinned the crash.
- **Play said "full" whenever the generator refused the drawn map,** which
  on the largest maps is often (four of five draws in the pass's own
  `curl` run). A new game now draws again, up to the game's existing limit
  of five draws (`fixtures/generated-battle.json`), and answers
  `unavailable` if none is admitted.

**O4, the joiner's wait for a snapshot.** Slice 01 measured the join
snapshot at 2.1 MB near the hour. That is about two seconds at 10 Mbit/s
and about seventeen at 1 Mbit/s, on top of building the map. The stream
opens only after Deploy, so this wait comes after the briefing, not under
it: slice 26 must keep its loading state until the first snapshot arrives.
These are sums from the measured size, not measurements over the internet.

Not built, and open: a new game on the largest maps takes the server about
as long to prepare as a browser (many seconds), and each refused draw adds
to it. A player pressing Play on an empty server waits through that.

