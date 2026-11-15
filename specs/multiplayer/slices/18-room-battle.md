# 18 Room battle

**Stage:** Rooms. **Needs:** 08, 15, 17. **Carries:** decisions 35, 37, 38, 39,
P7, P8, P14, P23.

## Contract

A room becomes a battle, survives a player dropping, and ends cleanly.

- **Start:** the host starts; the server makes the battle from the room's
  seats (open ones dropped, the rest closed up), and every member's
  `RoomView` turns to the battle phase carrying that member's ticket. The
  room leaves the list. Nobody can join it now.
- **Drop:** when the presence module says a player is gone, their seat is
  run by its AI, with the army it has.
- **Return:** the same browser id opening the room again gets its own seat
  back, at any point while the battle runs. Nobody else can take it.
- **End:** the result shows on each page from its final observation; everyone
  returns to the menu; the room and its battle are deleted (P14). A room
  whose humans have all been gone for a minute is deleted with its battle.

## Seam

- `apps/game-server/src/rooms.ts` calls `createBattleHost` with the setup
  from slice 04's functions, and maps each member to their battle player.
- The page's room driver follows the phase into the online battle route
  (slice 08) and back to the menu at the end.
- The existing preparation phase covers loading: it runs 60 seconds and ends
  early once every human is ready. A controller change does not touch a
  seat's ready flag; confirm that a dropped player's AI-run seat does not
  hold the phase open.
- **Delete the scaffolding:** `POST /dev/battle`, the `/online/dev` page and
  the server's dev flag go in this slice. Slice 08's two-seat scene now
  creates a room through the API.

This slice changes nothing under `crates/`. A room battle uses only what the
simulation already does: a seat's controller changing between a human and an
AI, which already replays exactly.

## Run or see

Two browsers: create, join, ready, start, fight. Close one tab; its units
keep fighting under the AI. Reopen the link; the seat is back. Play to the
end; both see the result and return to the menu.

## Verification

`web/tests/multiplayer/roomBattle.test.ts`, at the service seam:

- A started room has left the list and refuses a joiner.
- An open seat is not a player in the battle.
- After the grace period a dropped player's units receive orders from the AI.
- The dropped player returns to their own seat; another player cannot.
- The battle's replay, with its controller changes, ends on the same digest.
- After the end, the room and the battle are gone and their slot is free.

Then the two-browser scene from slice 08, now through a room.

## Must stay green

Every existing digest. Nothing under `crates/` changed.

## Delegated

The result screen's wording for an online match.

## Feedback that would change this

P14: if the user wants a rematch, the end returns everyone to the room's
lobby instead of the menu.
