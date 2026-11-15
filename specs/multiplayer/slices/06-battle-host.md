# 06 Battle host

**Stage:** Foundation. **Needs:** 02, 03, 04, 05. **Carries:** decisions 13,
56, 57, P2, P8; landmines 3 and 4.

## Contract

One battle on its own thread that never waits for a player: orders in, each
side's records out. It knows nothing of HTTP.

## Seam

New `apps/game-server/src/battleHost/`:

```ts
interface BattleSpec { setup: BattleSetup }            // wire.md
interface BattleClock { now(): number; schedule(run: () => void, delayMs: number): void }
type BattleRun = "thread" | { clock: BattleClock; generator?: MapGenerator };
interface BattleHost {
  ready: Promise<{ layout: string; tickHz: number; mapIdentity: string }>;
  attach(port: SeatPort): void;                         // slice 05
  setController(player: string, kind: ControllerKind): void;
  onEnd(handle: (end: { kind: "finished" } | { kind: "crashed"; message: string }) => void): void;
  replay(): Promise<string>;                            // the recorded orders (decision 56)
  dispose(): void;
}
function createBattleHost(spec: BattleSpec, run: BattleRun, data: BattleData): BattleHost;
```

- The thread's entry follows `web/src/battle/prepare/worker.ts`: it runs
  `prepare()` (`web/src/battle/prepare/prepare.ts`) and then hosts
  `createAuthority` as `web/src/battle/sim/workerAuthority.ts` does. Its
  clock is a deadline loop on `performance.now()`.
- The host starts the battle at once (P8), lists both sides, returns every
  publication buffer the moment it has copied the record into that side's
  ring, and never sends `pause` or `hidden`.
- Each side has one `recordSource` (slice 05). A port's `onReader` attaches
  to its side's source, which owns the ring and the resume rule; the source's
  request for a keyframe is that side's resync, at most once a second per
  side. A publication that opens a new epoch is flagged as the keyframe.
- A port's seat is stamped from the port: the host gives the authority the
  port's side and player with each order, whatever the batch claims. The
  authority's `command` request therefore names its seat when a host serves
  more than one (see "For slice 06" in [slice 02](02-per-side-publisher.md)).
- The host changes a seat's controller only when told to, through
  `setController`. It reports a port going away and times nothing: deciding
  that a player is gone belongs to the presence module (slice 15).
- `"inline"` runs the authority on the caller's thread with a clock the test
  drives, as `web/tests/authority.test.ts` does.
- The simulation file and game data are read from disk, following
  `web/src/maps/node.ts` and `web/src/battle/catalog/node.ts`.

## Run or see

New root script `bun run server:soak`: a generated 5v5 of AIs on a worker
thread, printing ticks, late ticks and memory once a second.

## Verification

`web/tests/multiplayer/battleHost.test.ts`, inline, over `loopback()`:

- Two ports on two sides each issue orders. The final digest equals the same
  orders given to an authority directly.
- Red's port never receives one of Blue's records.
- A batch for a unit the seat does not own is refused.
- A port that stops reading does not slow the tick, and resumes with a
  snapshot.
- A snapshot sent mid-stream after a resync is accepted by the page's reader
  and by every other reader on that side. (The reader accepts a newer epoch,
  `crates/sim/src/publication/reader.rs:298`; this test proves it end to
  end.)
- A seat set to `active_ai` mid-battle is ordered by its AI, and setting it
  back to `human` returns it to its player, whose next order is applied.
- A trap in the simulation reports `crashed`; every port gets `end`. A
  crashed battle is lost: nothing restores one, and the alpha accepts that.
  One known trap is a corpse sort that panics on a bad position
  (`crates/sim/src/battle.rs:4139`).
- The battle's recorded orders, replayed, end on the same digest.

## Must stay green

`web/tests/authority.test.ts`; `serverGraph.test.ts`, now covering the
server's entries.

## Delegated

Messages between the main thread and the worker; how the soak picks its map.

## As built

Three files under `apps/game-server/src/battleHost/`:

- **`runtime.ts`** runs on the battle's thread in either mode. It prepares
  the battle (`prepareBattle`), hosts the authority with one
  earliest-deadline timer, and turns each publication into its stream
  record's payload. The payload is the copy, so the authority has its buffer
  back before anything crosses to another thread. It also decides the
  keyframe flag, from the record: a publication whose ground epoch is later
  than its side's last.
- **`worker.ts`** is the thread's entry: the runtime with the parent port as
  its host.
- **`index.ts`** is the host: a `recordSource` per side, the ports, and the
  end. It stamps each order with the port's seat, defers a keyframe request
  that comes within a second of the last, and settles every waiting and
  later request when the battle is over.

What differs from the plan above, and why:

- **The run mode carries the test's clock** (`{ clock }`), where the plan had
  the word `"inline"`. A test also passes a map generator that makes each
  map once.
- **`replay()`** was added: the tests need the recorded orders, and decision
  56 makes replay export the server's.
- **Every `command` request names its seat** (`seat: { side, player? }`). The
  page fills it from its own options; the host from the port. The authority
  keeps no seat, so there is one way an order is stamped. `init.player` is
  gone.
- **A new request, `{ type: "resync"; side }`,** asks for a side's snapshot.
- **The authority's starting pool is three buffers per listed side,** so two
  sides have the headroom one side had.
- **`mapIdentity` is the generated map's hash.** A battle on any other kind
  of map fails to start.
- **`attach` throws for a seat the battle does not have,** and
  `setController` for a player it does not have. `not_seated` is the
  transport's answer (slice 07), which is the only code that meets a
  stranger.
- **Thread mode runs from the bundle.** The host looks for `worker.mjs` beside
  its own bundled file, and the server's data takes the repository root as
  an argument. `bun run server:soak [seconds]` bundles for Node and runs
  under Node.
- **The crash test uses an error, not a trap.** It gives the simulation a
  controller kind it refuses mid-battle, which travels the same path a trap
  does from the authority outward. No cheap way to cause a real trap was
  found.

The first soak on this Mac, a 5v5 of AIs for 30 seconds under Node: 30 ticks
every second, 2 ticks of 885 over the 33 ms budget, 95th percentile step
9 ms, 15 ticks shed at start-up and none after, 420 MB resident.

