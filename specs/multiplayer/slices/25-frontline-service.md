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
