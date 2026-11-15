# 15 Room service

**Stage:** Rooms. **Needs:** 07, 09, 14. **Carries:** decisions 4, 19, 21, 23,
38, 48, 49, 62, P11, P13, P25, P26.

## Contract

Rooms over the wire: the room list and each room as state streams, and
actions as requests. Everything is in memory; a restart empties it.

## Seam

New `apps/game-server/src/rooms.ts`, plus its routes in `http/`:

- Holds the `Room`s and runs `roomReduce` for every action, with the acting
  player taken from `X-Player`.
- Serves `GET /rooms`, `POST /rooms`, `GET /rooms/:code` and
  `POST /rooms/:code/act` as [wire.md](../wire.md) defines them. Each state
  stream sends the current state on open and again on every change.
- **`apps/game-server/src/presence.ts`** (new), the one owner of who is
  connected. The transport tells it when a player's stream opens and closes;
  it says when a player has been gone for the grace of about five seconds
  (P7), and for how long. Nothing else in the server times a disconnect.
- A member who is gone has left the room (P13). A room with no human
  connected for about a minute is deleted.
- The battle cap (P11, P26): creating a room takes a battle slot. With none
  free, creation is refused `full`.
- Guesses at private codes are throttled per client address (P25).
- A callsign's clash suffix comes from `/hello` (slice 09).

## Run or see

Two terminals each holding `curl` on one room's stream. A third sends
actions; both streams print the new state.

## Verification

`web/tests/multiplayer/roomService.test.ts`, over `loopback()` where the rule
is the service's and over localhost HTTP once for the wiring:

- Two players in one room both receive each change.
- A started room and a private room are not in the list.
- A player gone for the grace period has left; one who reconnects inside it
  has not.
- A room is deleted a minute after its last human goes.
- With every battle slot taken, creation is refused `full`.
- Repeated wrong codes from one address are refused `throttled`.

## Must stay green

Slice 07's transport tests.

## Delegated

The grace's exact length; the throttle's numbers; the cap's value, set from
slice 01's memory figure.
