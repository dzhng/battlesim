# Multiplayer research

External practice the plan leans on, gathered 2026-11-14. "Inference" marks
reasoning from the cited source; "not confirmed" means no primary source was
found. The measurements of this repo and of the temporary host are in the
[map](visualizations/unknowns-map.html), not here.

## Reading a long binary response in the browser

- `response.body.getReader()` reads incrementally; BYOB readers are only newly
  baseline (March 2026), so the default reader is the portable choice.
  [MDN: using readable streams](https://developer.mozilla.org/en-US/docs/Web/API/Streams_API/Using_readable_streams),
  [MDN: BYOB reader](https://developer.mozilla.org/en-US/docs/Web/API/ReadableStreamBYOBReader)
- Chunk boundaries are arbitrary: Fetch enqueues "whenever one or more bytes
  are available". Records need a length prefix (inference).
  [Fetch](https://fetch.spec.whatwg.org/)
- A slow reader suspends the fetch, and HTTP/2 flow control "operates between
  the endpoints of a single hop", so backpressure reaches the proxy and the app
  only through the proxy's socket.
  [RFC 9113](https://www.rfc-editor.org/rfc/rfc9113.html)
- Hidden tabs: animation frames stop, timers run once a second and then once a
  minute, and a frozen page runs no fetch callbacks. The throttling exemption
  covers WebSockets and WebRTC, not fetch streams.
  [Page Visibility](https://developer.mozilla.org/en-US/docs/Web/API/Page_Visibility_API),
  [Chrome timer throttling](https://developer.chrome.com/blog/timer-throttling-in-chrome-88),
  [Page Lifecycle](https://developer.chrome.com/docs/web-platform/page-lifecycle-api)
- Safari: no confirmed WebKit bug, but anecdotal reports of a paused reader
  leaving `read()` pending forever. Read continuously and drop stale data in
  application code.
- Intermediaries buffer compressed bodies; `X-Accel-Buffering: no` and
  `Cache-Control: no-store, no-transform` are the conventional guards.
  [nginx proxy module](https://nginx.org/en/docs/http/ngx_http_proxy_module.html),
  [MDN: Cache-Control](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Cache-Control)
- Without HTTP/2 a browser holds six connections per origin; with it, streams
  default to 100. [MDN: EventSource](https://developer.mozilla.org/en-US/docs/Web/API/EventSource)
- Streaming a request body is not a usable upstream channel: it needs HTTP/2
  end to end, Safari does not allow it with `fetch`, and proxies may buffer it.
  [Chrome: fetch streaming requests](https://developer.chrome.com/docs/capabilities/web-apis/fetch-streaming-requests)

## Resuming a delta stream

- Server-Sent Events: each event carries an id and the reconnect sends
  `Last-Event-ID`. [HTML: server-sent events](https://html.spec.whatwg.org/multipage/server-sent-events.html)
- Quake 3 keeps "the 32 last gamestate sent" per client and deltas against "the
  last valid acknowledged snapshot"; with none usable it sends a full update.
  [Fabien Sanglard](https://fabiensanglard.net/quake3/network.php)
- Source sends full snapshots "only … when a game starts or a client suffers
  from heavy packet loss for a couple of seconds".
  [Valve: Source Multiplayer Networking](https://developer.valvesoftware.com/wiki/Source_Multiplayer_Networking)
- Fiedler: "the sender only encodes snapshots relative to baselines that the
  other side has received"; with a too-old base, "drop back to absolute
  encoding". [Snapshot compression](https://gafferongames.com/post/snapshot_compression/),
  [State synchronization](https://gafferongames.com/post/state_synchronization/)
- Trade-off (inference): a shared stream per team is encoded once and resumes
  by replaying a bounded ring of records by id, or by a keyframe. A baseline per
  client resumes anyone cheaply but costs an encode and a history per client.
  On an ordered, reliable stream the only acknowledgement needed is the last
  record id, sent at reconnect.

## Timing on the client

- Source renders 100 ms behind at about 20 snapshots a second; Fiedler suggests
  about 150 ms at 30 a second.
  [Snapshot interpolation](https://gafferongames.com/post/snapshot_interpolation/)
- On underrun, hold: extrapolation "starts to break down" once objects collide.
- An HTTP stream never loses a record; it stalls and then delivers a burst, so
  the buffer is sized for jitter (inference).
- Supreme Commander runs commands at "200–300ms latency", masked because "the
  UI response … is immediate" and "there is frequently an audio response".
  [Forrest Smith: synchronous RTS engines](https://www.forrestthewoods.com/blog/synchronous_rts_engines_and_a_tale_of_desyncs/)
- Age of Empires: "250 milliseconds of command latency was not even noticed"
  (confirmed through search results only).
- Planetary Annihilation is server-authoritative and ticks at 10 a second.
  [Chrono cam](https://www.forrestthewoods.com/blog/tech_of_planetary_annihilation_chrono_cam/)
- This system's budget (arithmetic): a request of 100–300 ms, up to 33 ms for
  the tick, the stream's delay and a 100–150 ms buffer: roughly 250–500 ms from
  click to seeing it happen.

## The Node server

- A compiled `WebAssembly.Module` can be posted to worker threads, and buffers
  in a transfer list move without copying.
  [worker_threads](https://nodejs.org/api/worker_threads.html)
- A trap throws `WebAssembly.RuntimeError`; uncaught, that worker is terminated
  and its `'error'` event fires. Without a listener the process exits.
  [events](https://nodejs.org/api/events.html)
- A worker's `resourceLimits` "only affect the JS engine, and no external data,
  including no `ArrayBuffer`s", so they do not cap WebAssembly memory
  (inference). The ceiling is 4 GB. [V8: 4GB wasm memory](https://v8.dev/blog/4gb-wasm-memory)
- Timers have a 1 ms floor and "no guarantees about the exact timing". Use a
  deadline loop on `performance.now()` (inference).
  [timers](https://nodejs.org/api/timers.html)
- `res.write()` returns false once data is queued in user memory, and
  `writableLength` reports the queue. Ignoring it took the guide's example from
  88 MB to 1.5 GB. [Backpressure](https://nodejs.org/en/learn/modules/backpressuring-in-streams)
- The response's `'close'` event reports a client that went away.
  [http](https://nodejs.org/api/http.html)
- WebAssembly SIMD has been on by default since Node 16.4.
  [V8: SIMD](https://v8.dev/features/simd)

## Orders over HTTP

- Idempotency keys: "a unique value generated by the client which the resource
  uses to recognize subsequent retries".
  [IETF draft](https://datatracker.ietf.org/doc/html/draft-ietf-httpapi-idempotency-key-header),
  [Stripe](https://docs.stripe.com/api/idempotent_requests)
- "Streams are largely independent of each other" (RFC 9113), so separate
  requests can arrive reordered. One request in flight, carrying the queued
  orders with a client sequence number, restores order (inference).
- `keepalive` lets a request outlive the page, to 64 KiB. MDN calls `pagehide`
  unreliable; the server still needs a timeout.
  [RequestInit](https://developer.mozilla.org/en-US/docs/Web/API/RequestInit)

## A server with no accounts

- A token needs "at least 64 bits of entropy"; 128 is advised.
  [OWASP session management](https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html)
- With `Access-Control-Allow-Origin: *` the browser sends no ambient
  credentials, so a token in a header leaves nothing for a forged request to
  ride (inference). A custom header forces a preflight, cacheable for up to two
  hours in Chromium. [MDN: CORS](https://developer.mozilla.org/en-US/docs/Web/HTTP/Guides/CORS)
- Rate limits "must only use IP addresses added by a trusted proxy".
  [MDN: X-Forwarded-For](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/X-Forwarded-For)
- A WORD-1234 code over 2,048 words is about 24 bits: guessable in bulk, so
  join attempts are throttled (arithmetic).
- Under `Cross-Origin-Embedder-Policy: require-corp`, "requests made in `cors`
  mode won't be blocked … but must still be permitted by CORS".
  [MDN: COEP](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Cross-Origin-Embedder-Policy)

## Joining and leaving a running match

- Rocket League's casual mode holds a leaver's slot with a bot and stops
  backfilling humans once the goal gap reaches three (paraphrased from search
  results).
- Team Fortress 2 stopped balancing teams in the last 60 seconds of a round and
  dropped volunteer balancing; players "voiced disapproval at being switched to
  the losing side". [Team balance](https://wiki.teamfortress.com/wiki/Team_balance)
- League of Legends offers a remake after a player is idle for 55 seconds
  early in a match. [Surrendering](https://wiki.leagueoflegends.com/en-us/Surrendering)
