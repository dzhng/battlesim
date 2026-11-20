# The wire contract

Everything a page and the game servers say to each other. The shapes are
types in [src/wire.ts](src/wire.ts) and are not repeated here; this file
holds what a type cannot say: the endpoints, the framing, and the rules a
request and its answer keep. Where the two disagree, the code is right and
this file is wrong: fix it in the same change.

There are two servers. The **lobby** answers the menu: names, rooms,
Frontline. A **battle server** runs one battle and answers only that
battle's players and its lobby. A page learns a battle server's address from
its ticket.

## Every request and response

- **`X-Player`** is the browser id: 128 random bits in hex, and the player's
  credential. A player route finds the seat from it and never reads a side,
  player or seat that a body claims.
- **`X-Battle-Key`** guards the control surface a lobby uses on its battle
  server. A lobby makes one key for each battle; no player is ever sent it.
- **Neither credential appears anywhere else a player or a reader of logs
  could see it:** not in a URL, a reply to another player, a room or list
  view, a ticket, a briefing, an error or a log line. Other players see a
  callsign. A lobby does tell its battle server which browser holds which
  seat, in a control request.
- **Every response** forbids caching and allows any origin:
  - `Cache-Control: no-store, no-transform`
  - `Access-Control-Allow-Origin: *`
  - `Access-Control-Allow-Headers: X-Player, Content-Type`
  - `Access-Control-Allow-Methods: GET, POST, OPTIONS`
  - `Access-Control-Max-Age: 7200`
- **`OPTIONS` on any path answers 204.**
- **Bodies are JSON** (`Content-Type: application/json`), except a stream,
  which is `application/octet-stream` with `X-Accel-Buffering: no`.
- **A refusal is `{ "refused": "<reason>" }`.** Some refusals of an order
  batch also carry its `seq`.
- **A repeat is not refused.** A member reopening their room's stream is
  still in their seat, including after the battle has started; a second
  `join` returns the same reservation; a second `deploy` returns the same
  ticket; leaving a room you are not in is fine. A refusal is for a request
  that cannot be honoured. A room that no longer exists is one of those: its
  last member's second `leave` is refused `unknown`.

### Status codes

A page reads the reason, not the number. The number matters only as the
line between an answer and a failure: a refused stream must be non-2xx,
because that is how a page tells a refusal from a stream; and a non-2xx
answer with no readable refusal in it is a reply that did not arrive.

| Status | When                                                                                                                         |
| ------ | ---------------------------------------------------------------------------------------------------------------------------- |
| 200    | An answer. Also a Frontline rule saying no, and an order batch refused by its seat's allowance or by the simulation          |
| 400    | `invalid`: a body or cursor that cannot be read. `address`: the lobby's trusted address header is missing                    |
| 403    | `invalid`: no usable `X-Player`. `not_seated`. `forbidden`: a wrong battle key                                               |
| 404    | `unknown`: no such route, or a battle this server is not running                                                             |
| 405    | `method`                                                                                                                     |
| 409    | A rule said no: `/hello`, room create, room stream, room act; `sequence`; `started` and `not_started` on the control surface |
| 410    | `unavailable`: the battle has ended, or anything unexpected. Exception text never leaves the server                          |

## The lobby's endpoints

| Method and path                  | Body                  | Answer                                                                                                                       |
| -------------------------------- | --------------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| `GET /build`                     |                       | `BuildIdentity` only. No credential, and no player is registered                                                             |
| `GET /deployment`                |                       | The hosted server's source commit for deployment checks. No credential or player is registered                               |
| `POST /hello`                    | `{ build, callsign }` | `{ callsign }` (with `#2` on a clash), or refused `build`, `callsign` or `invalid`                                           |
| `GET /rooms`                     |                       | state stream of `RoomList`                                                                                                   |
| `POST /rooms`                    | `{ visibility }`      | `{ code }`, or refused `full`, `game_running` or `callsign`                                                                  |
| `GET /rooms/:code`               |                       | state stream of `RoomView`. Opening it joins the room. Refused `unknown`, `started`, `no_open_seat`, `callsign` or `removed` |
| `POST /rooms/:code/act`          | `RoomAction`          | `{ ok: true }`, or refused with the rule that said no                                                                        |
| `POST /frontline/join`           |                       | `Briefing`, or refused `full` (no battle slot), `unavailable` (no battlefield could be made), `callsign` or `game_running`   |
| `POST /frontline/:battle/deploy` |                       | `BattleTicket`, or refused `expired`                                                                                         |
| `POST /frontline/:battle/leave`  |                       | `{ ok: true }`. The seat goes back to its AI                                                                                 |
| `POST /battles/:id/replay`       | `{}`                  | `ReplayAnswer`, or refused `unknown`, `not_seated`, or `unavailable` (the battle was lost with its machine)                  |

A stream takes `?after=<id>` to resume. A room stream opened with `after` by
someone who is no longer a member is refused `removed`: a resume keeps a
membership and never makes one.

Replay export is the lobby's, so it answers before and after the battle's own
server is gone. While the battle runs, a seated player gets its full accepted
command history, including the opponent's orders outside their fog; export
does not broaden the side's ordinary publication. Once it is over, anyone who
held one of its seats gets it from the lobby's archive. The browser adds its
own human camera history to the downloaded battle; cameras are not sent to the
server.

## A battle server's endpoints

| Method and path                      | Body           | Answer                                                |
| ------------------------------------ | -------------- | ----------------------------------------------------- |
| `GET /battles/:id/stream?after=<id>` |                | battle stream for the caller's side                   |
| `POST /battles/:id/up`               | `UpBatch`      | `UpReply`                                             |
| `POST /battles/:id/preview`          | `PreviewQuery` | `PreviewAnswer`, or refused `invalid` or `not_seated` |

### The control surface

Only the battle's lobby calls these, with `X-Battle-Key`. The routes and
their bodies are in `apps/game-server/src/http/control.ts`, and the lobby's
side in `apps/game-server/src/battleHost/remote.ts`; nothing else speaks
this half.

- **`POST /control/start`** takes `{ id, candidates: BattleSetup[] }` and
  prepares the candidates side by side. The first in the list that can be
  prepared is admitted, and success says which (a zero-based `candidate`)
  with the layout, tick rate and map hash. No
  admitted candidate is `unavailable`; a second start is `started`. Tickets
  and briefings carry only the admitted setup.
- **Which browser holds which seat** is a request the lobby may make before
  the start. **A seat's controller, seating a human in a running battle,
  ending one seat's readers, the referee's view and the replay** are
  refused `not_started` until then. A finished battle still answers its
  replay: the lobby asks for it once more, to archive it, before it stops the
  machine.
- **`GET /control/events`** is a record stream of who is connected and who
  has given an order, and the battle's end. After a gap its snapshot
  reconciles both. It never contains a browser id or the key.
- **The lease.** The lobby pings once a second. A battle server that hears
  nothing from its lobby for about twelve seconds stops, and a lobby that
  cannot reach a battle server for as long treats the battle as crashed.
  While a battle is being prepared the start request holds the lease, for
  two minutes at most. The timings are in
  `apps/game-server/src/http/controlProtocol.ts`.

## Record framing

One framing for every stream. Little-endian, a 12-byte header so that `f32`
payloads stay aligned, then the payload padded to four bytes.

```
u32 payloadBytes | u32 id | u8 kind | u8 flags | u16 0 | payload
```

| Kind          | Payload                 | Meaning                                                                                                                 |
| ------------- | ----------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| `open`        | JSON `StreamOpen`       | First record of every response                                                                                          |
| `publication` | binary                  | One tick of one side: `u32 tick`, 16 ASCII bytes of digest, `f32 stepMs`, then the publication's `f32` words            |
| `status`      | JSON `{ status, slow }` | The battle's status changed (`running`, `finished`, `failed`), or it began or stopped running behind its clock (`slow`) |
| `state`       | JSON                    | The whole current `RoomList` or `RoomView`; on the control stream, one event or its snapshot                            |
| `end`         | JSON `{ reason }`       | The server's last word on this stream                                                                                   |

`flags` bit 0 marks a keyframe: a record a reader can start from. A full
publication is one, and so is every `state` record of a room or the list.

Ids count up from 1 for each stream source (one battle side, one room, the
room list). `open` and `end` are not numbered: their id is 0. Chunk
boundaries mean nothing: a decoder must accept any split.

`StreamOpen` says whether the stream `continued` or restarts at a
`keyframe`. A battle stream's also carries the layout, the tick rate, and
whether the battle is running behind its clock, for a reader who arrives
late.

An `end` gives its reason. A battle stream ends `finished`, `crashed` or
`disposed`, and one seat's readers alone can be ended `idle` or `leave`
while the side's stream goes on. A room stream ends `room_deleted`,
`battle_finished`, `battle_failed` or `disposed`; the list ends only
`disposed`. The control stream ends as its battle does.

## Resuming

The reader reconnects with `?after=<last id it handled>`.

- **The id is still held:** `open` says `continued`, and the records after
  that id follow. Nothing is repeated and nothing is skipped.
- **It is not, or there is no `after`:** `open` says `keyframe` and the
  side's publisher is asked to resync. That reader is sent nothing until the
  next keyframe, which is the full snapshot; every reader already on that
  side receives the same snapshot in line. After `keyframe` the reader's
  numbering starts afresh from whatever comes next. A side is resynced at
  most once a second.
- **An id that skips ahead, without `keyframe`,** is the server breaking
  this rule. The reader fails; it does not reconnect. A record it has
  already handled is dropped.
- **An `end` record** is the last: the reader stops and does not reconnect.
  A stream that has already ended answers a late reader with `open` and that
  `end`. A refused stream request also ends the reader for good.
- Until then a reader reconnects at once after a connection that delivered
  something, and backs off while connections deliver nothing.

Battle and control streams keep a bounded backlog to resume from. Room and
list streams keep none: every open is sent the current state.

The server never waits for a reader. A socket whose queue passes a bound is
closed, and that reader resumes like any other.

## Orders

- **A page has one batch in flight.** Orders given meanwhile wait and leave
  together in the next batch, in the order they were given.
- **`seq` counts up from the page's clock,** so a reloaded page is later.
  A batch whose reply does not arrive is sent again with the same `seq`.
  The server keeps each seat's last `seq` and reply, and answers a repeat
  from that record without applying it twice.
- **The battle stamps side, player and sequence.** The page names none of
  them.
- **A refusal settles the batch:** every order in it is refused to the
  player and the batch is never sent again, whether or not the refusal
  names the batch. Only a reply that did not arrive is sent again. The
  reasons a seated page can meet:
  - `sequence`: a later batch from this seat has been seen (another tab of
    the same browser, for example), or another batch is still being applied.
  - `rate`: the batch is beyond what its occupant has left of the
    simulation's allowance for the window.
  - `invalid`: the batch is beyond that whole allowance, or an order in it
    is larger than the server admits, or the simulation cannot read one.
    The size bounds (unit and squad references, and encoded bytes) are in
    `apps/game-server/src/http/input.ts`; an ordinary whole-army selection
    fits.
  - `not_seated`: this browser holds no seat in the battle.
  - `unavailable`: the battle has ended.
- **A batch refused for its size or for the allowance does not reach the
  battle and spends no allowance.** A batch holding an order the simulation
  cannot read is refused whole after its allowance is spent, whatever became
  of its other orders; a page of this build never sends one.
- **The sequence record and the allowances belong to a seat's occupant.**
  Reconnecting the same browser keeps them; seating a different browser
  resets them, even before that browser has sent an order.

## Previews

Newest wins and nothing is retried. The server answers about five a second
for each player and answers the rest `{ dropped: true }`. A page asks no
faster than that, so none of its own are dropped: the interval is
`PREVIEW_INTERVAL_MS` in [src/link.ts](src/link.ts), and the server allows
one early for two that a network delivers closer together than they were
sent.

A preview only draws what the pointer would do. No order waits for one: the
order goes when the player gives it, and the battle admits or refuses it.

## Build identity

A page may play when its `BuildIdentity` equals the lobby's: three hashes,
`sim`, `data` and `wire`. The page sends its own in `/hello`, and the lobby
refuses `build` exactly when one differs from its deployed `build.json`. A
different source commit with equal hashes is admitted.

- `sim` is the simulation module's bytes, as the page actually loaded them.
- `data` is the documents both ends build and field a battle from, including
  JSON embedded in the bundle.
- `wire` is this package's `src/`.

[buildIdentity.ts](buildIdentity.ts) owns which files count and how they are
hashed, for the site's builds and the server's bundle alike;
[src/build.ts](src/build.ts) owns the comparison.

The site's `build.json` also records its source commit, which a deploy
checks against the commit it was asked to ship, and the menu shows a short
commit of its own. Neither takes part in admission. `GET /build` answers
only the three hashes, so a deploy can ask whether a live lobby admits a
site without touching any player's state.

## Tickets

- **A ticket carries the settings, not the prepared battle.** The page
  builds the map itself and refuses to start unless its map's hash is the
  ticket's `mapIdentity`.
- **`BattleSetup` is what both ends feed the one request builder.** Its
  seats are the battle's players, in order.
- **`address` is the battle server's origin.** The page uses it for the
  battle's stream, orders and previews. Presence in a room or the list,
  leaving and deploying go to the lobby.
- **`callsigns` are the public names,** keyed by the battle's player ids.

## Rooms

- **A room's code is its id and its link:** six characters from an alphabet
  with no look-alikes, which is data in [src/room.ts](src/room.ts).
  `normalizeRoomCode` there removes whitespace and dashes and uppercases;
  the page and the server both use it, and links carry the canonical code.
- **A code is not a secret.** Wrong guesses are not counted, and no refusal
  hides whether a room exists.
- **A seat's id is made once and never renumbered.**
- **`canStart`** means every human is ready and each side has a fighter.
  **`starting`** means the host has started and the server is building the
  battle.
- **`RoomSettingsPatch` is exactly what a host may set;** anything else is
  refused. Each value is checked against the game's own lists and applied
  through the lobby's own rules, so a size below the floor is raised to it
  and an unknown value is refused. A map seed must be one the map source's
  `canonicalSeed` accepts, and the room stores its canonical decimal value.
- **The last five `RoomAction` kinds are the host's.**
- **`leave` means two things.** In the lobby it gives the seat up. After
  the start the member keeps their seat and ticket, their army goes to its AI
  at once, and opening the room's stream again returns them to it. Only a
  fresh opening does: a stream resuming with `after` belongs to the page they
  left, and is refused `removed`. A battle every human has left this way
  ends, and its room is deleted.
- **`RoomList` holds only public rooms still in the lobby,** and the two
  Frontline counts: games running and humans seated in them. A reservation
  is not a seated human; the first commander of a new game counts from tick
  zero; a departed seat running on its AI does not count.
- **The list publishes only when what it shows someone changes.** Ready, a
  setting set to what it was, and anything in a private room's lobby send no
  record. A new reader is still sent the current list.
- **`yours` is added for its reader alone:** the started room they hold a
  seat in. It is how a page offers the way back to a battle still running.
  A room starting or a started room going publishes the list, so every
  reader's `yours` follows its battle.

### One game for each device

A device may have one game of its own making: a room it created, or a
Frontline game its Play created. `game_running` says that place is taken.
Joining existing games is unrestricted. The place is held from before the
game is prepared until the game is removed, including after hosting passes
to someone else; every way a game ends, and a failed preparation, gives it
back.

The lobby's `clientAddressHeader` setting names the trusted proxy header
that identifies a device. With it set, a room or Frontline request without
that header is refused `address`. With none set, the device is the browser
id, never the socket's address. The device appears in no view and no reply.

## Frontline

- **`join` reserves a seat for about a minute** and answers a `Briefing`:
  the seat (1 to 4), what the page needs to build the map behind the card,
  and the roster, where a `null` callsign is an AI.
- **`deploy` takes the seat and answers the ticket.** A newcomer displaces
  an AI: the seat's army is handed over, the seat is reset, and the
  controller changes, all at one tick. The first commander of a new game
  displaces nothing. A commander returning inside the hold gets the same
  army back.
- **Opening credits are granted once** to each player in each game.

## A browser that goes quiet

A player is present while any of their streams is open, with a few seconds'
grace. The times are in `apps/game-server/src/presence.ts`,
`apps/game-server/src/rooms.ts` and [src/frontline.ts](src/frontline.ts).

A member is in their room while one of that room's own streams is open: its
state stream, or their seat's stream in its battle. The room list is not one
of them, so a member reading the list has gone quiet in their room.

- **In a room's lobby** the member leaves, and hosting passes on if it was
  theirs. A lobby goes with its last member.
- **In a room's battle** the seat is played by its AI until the browser
  returns. A room with nobody in it for a minute is deleted.
- **In Frontline** the army goes to the seat's AI and the seat is held for a
  minute. A player who gives no order for three minutes has their stream
  ended `idle`.
