# 07 HTTP transport

**Stage:** Foundation. **Needs:** 05, 06. **Carries:** decisions 9 to 11, 55,
56, 60, P5, P6, P22.

## Contract

The first real transport. Above the existing `SimConnection` seam
(`web/src/battle/sim/client.ts:79`) the page's battle code does not change.
A WebSocket later replaces the two files this slice adds and nothing else.

## Seam

- **`apps/game-server/src/http/`** (new): the only server code that imports
  `node:http`. It turns requests into `SeatPort`s and serves the endpoints in
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
