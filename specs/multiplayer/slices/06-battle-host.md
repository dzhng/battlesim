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
interface BattleHost {
  ready: Promise<{ layout: string; tickHz: number; mapIdentity: string }>;
  attach(port: SeatPort): void;                         // slice 05
  setController(player: string, kind: ControllerKind): void;
  onEnd(handle: (end: { kind: "finished" } | { kind: "crashed"; message: string }) => void): void;
  dispose(): void;
}
function createBattleHost(spec: BattleSpec, run: "thread" | "inline", data: SetupData): BattleHost;
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
