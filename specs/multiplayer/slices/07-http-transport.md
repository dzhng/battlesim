# 07 HTTP transport

**Stage:** Foundation. **Needs:** 05, 06. **Carries:** decisions 9 to 11, 55,
56, 60, P5, P6, P22.

## Contract

The first real transport. Above the existing `SimConnection` seam
(`web/src/battle/sim/client.ts:79`) the page's battle code does not change.
A WebSocket later replaces the two files this slice adds and nothing else.

## Seam

- **`apps/game-server/src/http/`** (new): the only server code that imports
  `node:http`. It keeps one `SeatPort` per seat, calls its reader handler
  for each stream request, and serves the endpoints in
  [wire.md](../wire.md) that exist so far: the battle stream, `up` and
  `preview`. It sets the headers wire.md lists on every response, answers
  the cross-origin preflight, watches `writableLength` and closes a socket
  whose queue passes a bound, and treats a response's `close` as the reader
  going away. It never logs or echoes a request header.
- **`apps/game-server/src/main.ts`** (new): starts the server. Root script
  `bun run server` bundles it with `bun build` and runs it under Node (P2).
- **`web/src/battle/remote/httpLink.ts`** (new):
  `httpLink(address, battle, browserId): SeatLink`, reading with
  `response.body.getReader()` and never pausing the read.
- **`web/src/battle/remote/connection.ts`** (new):
  `remoteConnection(link: SeatLink): SimConnection`. It keeps one batch in
  flight, retries a batch whose reply is late with the same `seq`, sends only
  the newest preview, and turns records into the existing `SimReply`
  messages.
- **`web/src/battle/sim/client.ts`:** an online client registers no
  visibility listener and offers no pause, speed, replay export, side switch
  or controller change. The remote connection cannot express them, and the
  server has no endpoint for them.

## Run or see

`bun run server`, then `curl` a battle stream and watch records arrive.

## Verification

`web/tests/multiplayer/http.test.ts` on an ephemeral localhost port:

- A stream delivers records; a forced cut resumes by id with nothing lost or
  repeated.
- A batch retried with the same `seq` is applied once.
- A batch from a browser id with no seat is refused `not_seated`.
- Orders past the rate are refused `rate` and never reach the battle.
- Previews past about five a second are answered `dropped`.
- Every response carries `no-store`, and a preflight is answered.
- With a spy on the logger, no request header value is ever written.

The remote connection is tested over `loopback()`, without HTTP.

## Must stay green

Single-player battles: the local connection is untouched.

## Accepted rough edge

A move or purchase ghost trails the cursor by a round trip, and a purchase
cannot be confirmed until its preview's answer matches the cursor, so a
purchase waits that long too (decision 55). Do not work around it here; the
fallback is the deferred "previews in the browser" plan.

## Delegated

The router's shape; the queue bound; the retry timeout.

## As built

- **Server:** `apps/game-server/src/http/` serves `GET /battles/:id/stream`,
  `POST /battles/:id/up` and `POST /battles/:id/preview`, and the preflight.
  `battleRoute(host, seatOf, ordersPerSecond)` is one battle's seats:
  `seatOf(browserId)` is asked on every request, so whoever owns seating
  (the room service, later) can move a seat while the battle runs. Each seat
  has one port, one order allowance and one remembered batch, shared by all
  its sockets.
- **Page:** `httpLink` is one connection per `records(after)` call;
  `remoteConnection` turns a link into the page's existing channel.
- **`bun run server`** bundles for Node and serves one battle named `dev`
  on localhost, with its two browser ids taken from `BATTLE_BLUE_ID` and
  `BATTLE_RED_ID`. This is scaffolding until rooms create battles.

What the plan did not say, and what was decided:

- **A page learns the battle, then lets the stream go until it starts.** On
  connecting it reads the layout, rate and current tick and closes the
  connection. When the page starts it opens the stream afresh and begins on
  a snapshot taken then. Nothing of the battle is read or held while the
  page loads, however long that takes. The first cut buffered up to four
  seconds of battle and failed the page past that.
- **A batch's number counts up from the page's clock,** so a reloaded page's
  batches are later than its old ones. The first cut counted from 1, and the
  server would have refused every batch from a reloaded page as stale.
- **The stream's `open` record says the battle's status.** A page that joins
  a running battle missed the `running` record.
- **A transport refusal reaches the page as its own reply**
  (`request_refused`), which rejects that order or preview. It is not
  dressed up as an acknowledgement from the simulation.
- **HTTP statuses:** 400 for a body or cursor that is not well formed, 403
  with `not_seated` for a stranger, 404 for an unknown battle, 405 for the
  wrong method, 409 for a batch older than the seat's last or sent while
  one is in flight, 410 once the battle cannot answer.
- **Constants,** each beside the code that enforces it: a socket's queue
  may hold 4 MiB before it is closed; a request body 64 KiB; one preview
  per 200 ms per seat; orders at the rules' own rate (30 a second),
  refused a whole batch at a time; a batch is retried after 2 s without a
  reply; a preview is given up after 2 s.

Not stress-tested: a socket queue passing its bound, and a preview that is
never answered.

A page still cannot play through this: slice 08 wires the page's battle to
`remoteConnection`. The online client's options still ask for a scenario
and seed it does not use; slice 08 owns that shape.

