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

## As built (2026-11-15)

- **`apps/game-server/src/rooms.ts`** holds the rooms and feeds every action
  to `roomReduce`. It restates no room rule. Each room, and the room list,
  is one stream that keeps only its current state: every open gets the
  whole state, and every change is sent to every reader. A reader's
  `RoomView` is made for that reader as it is delivered.
- **`apps/game-server/src/presence.ts`** is the one owner of who is
  connected. It counts a browser's room, list and battle streams together,
  and says when a browser has had none open for the grace.
- **Routes** in `http/rooms.ts`: `GET /rooms`, `POST /rooms`,
  `GET /rooms/:code` (opening it joins, as wire.md says) and
  `POST /rooms/:code/act`. A refusal on opening a room stream is an ordinary
  reply: the stream's headers are written with its first record, not before.

Decided here where the plan was silent:

- **Numbers,** each a named constant where it is enforced: a browser is gone
  after 5 seconds with no stream open; a room with no human connected is
  deleted after 60 seconds; five wrong codes from one address refuse that
  address for 10 seconds from the first.
- **The battle cap** is the smaller of cores minus one and memory at 2 GB a
  battle, computed once at start-up (11 on this machine). A room holds a
  slot from creation to deletion, private and started rooms included.
- **A client's address** is the socket's, unless start-up names a trusted
  header (`CLIENT_ADDRESS_HEADER`, set by the deploy files); then it is that
  header's value, and a request without it is refused. No host's header was
  guessed.
- **A callsign is released** once its browser has been gone for the grace
  and no room remembers it. This closes the gap slice 09 left.
- **Extra refusals:** `callsign` for creating or joining without a
  successful `/hello`; `address` when the trusted header is configured and
  missing. Throttled is HTTP 429; other refusals 409.
- **Each new room opens on a map of its own** (added at integration).
  `createRoom` now takes the map's seed from its caller; before, every room
  opened on the same map.
- **The seam for slice 18** is the moment the service accepts a transition
  into `starting`: that is where a battle is prepared, and `recordBattle`
  then publishes the tickets through the same path as any other change.
  Nothing was built there.

The two-terminal check passed: two `curl` readers on one room both printed
the guest's ready and an added seat, each as its own view.

