# 28 A battle in its own process

**Stage:** Hosting. **Needs:** 26. **Carries:** decisions 68, 8, 13, P5, P15.

## Why

The temporary host cannot hold a battle's tick rate (slice 01, O1), and a
sandbox can. The user chose on 2026-11-15 to keep the rooms and Frontline
service where it is and give every battle a machine of its own (decision
68). This slice makes that split on one computer, where it can be tested;
slice 29 puts the battle's half on a sandbox.

## Contract

- **The lobby never steps a battle.** The lobby is the server that answers
  `/hello`, `/rooms` and `/frontline`. Each battle runs in a **battle
  server**: a second entry point of the same bundle that hosts one battle,
  serves that battle's `stream`, `up` and `preview` to its players, and
  answers the lobby on a control surface.
- **A ticket says where its battle lives.** `BattleTicket` gains
  `address?: string`, present when the battle's three routes are not at the
  lobby's address. The page uses it for the battle's link and nothing else.
- **The lobby asks a provider for a machine,** and knows nothing about what
  kind it is:

  ```ts
  interface BattleMachines {
    capacity: number;                       // battles at once
    start(key: string): Promise<Machine>;   // rejects when none can be had
  }
  interface Machine {
    address: string;   // for players
    control: string;   // for the lobby; may be the same
    stop(): void;
  }
  ```

  Two providers here: a child process on this computer, which is what the
  local server uses, and one inside the test's own process. The sandbox
  provider is slice 29.
- **Rooms and Frontline still hold a `BattleHost`.** A second
  implementation of that interface speaks the control surface. Neither
  service learns which kind it holds.
- **The control surface** is the lobby's alone. Each battle has a key the
  lobby makes (128 random bits), sent in a header. It is a credential like a
  browser id: never in a URL, a reply, an error or a log line. Over it the
  lobby starts the battle from a `BattleSetup` and gets back what `ready`
  gives today (or the refusal of a map that was not admitted), says who is
  seated where, sets a controller, seats a human, ends a seat, asks for the
  referee's view and the replay, and stops the battle. Back comes one
  resumable record stream of what the lobby must hear: a player gave an
  order, a player's stream opened or closed, the battle ended and how.
- **A player is seated only when the lobby says so.** The battle server
  takes its seating from the lobby and refuses everyone else `not_seated`,
  as today.
- **Presence keeps its one owner,** the lobby's module. A battle stream
  still counts as being there: the battle server reports it.
- **A battle server that dies, or cannot be reached, is a crashed battle,**
  through the path a crash takes today. A battle server whose lobby has
  gone stops by itself, so nothing is left running.

## Seam

- `apps/game-server/src/`: the battle server's entry point and control
  routes; the machines interface and its two providers; the remote
  `BattleHost`. `bundle.mjs` builds the second entry.
- `apps/game-server/src/http/index.ts` loses the battle routes and the
  unused battle lookup that is its first argument: the battle server owns
  those routes now. One owner; delete the old path.
- `packages/multiplayer/src/wire.ts` and [wire.md](../wire.md): the
  ticket's `address`.
- The page's battle route: the link's address.

**Slice variable:** where a battle runs. Out of scope: Bento, sandboxes,
WebSockets, any change to the simulation or to what a player sees.

## Run or see

`bun run server`, two browsers, a room battle. The computer shows one more
process for each battle, and it goes when the battle ends.

## Verification

1. The tests that say what a `BattleHost` does run against both
   implementations.
2. Tests of how it fails: a control request without the key is refused; a
   browser id the lobby has not seated is refused by the battle server; a
   battle server killed in the middle of a room battle ends the room as a
   failed battle and frees its place; a battle server whose lobby stops,
   stops; a player whose battle stream drops is still held for the grace
   and returns to their seat.
3. Rooms and Frontline through the split: the `online-battle`,
   `room-lobby` and `frontline` scenes, with the local server running each
   battle as a child process.
4. Every existing digest and replay is unchanged: this slice moves a
   battle, it does not change one.

## Must stay green

The whole multiplayer domain and the three scenes above.

## Delegated

The control surface's encoding; ports; how the child is started and
stopped; names.

## As built (2026-11-15)

The lobby answers hello, rooms and Frontline. A battle server answers only its
own battle's player routes and its authenticated control surface. Local startup
always allocates one child process per battle; that process uses the unchanged
worker-backed authority. The in-process provider uses the same HTTP server and
control protocol with the caller's test authority.

`BattleMachines` and `Machine` match the interface above. Capacity comes from
the provider; the child provider uses `battleCapacity()`. The key is 16 random
bytes, passed through the child's environment and `X-Battle-Key` headers.
Startup IPC carries only the listening address. Child diagnostics redact the
key and seated browser credentials before the provider forwards them.

The private control surface is owned by `apps/game-server/src/http/control.ts`:

| Method and route                                                                                             | Request / response                                                                                                                                                                                                           |
| ------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `POST /control/start`                                                                                        | `{ id, setup }`; returns layout, tick rate and admitted map hash. Failed preparation is a refusal, with no ticket. Only one start is accepted.                                                                               |
| `POST /control/seating`                                                                                      | Complete `[browser id, seat]` entries; acknowledges only `ok`.                                                                                                                                                               |
| `POST /control/controller`                                                                                   | `{ player, kind }`; acknowledges enqueueing the controller change.                                                                                                                                                           |
| `POST /control/human`                                                                                        | `{ player, opening }`; acknowledges the existing atomic hand-over, reset and human controller after publication.                                                                                                             |
| `POST /control/end-seat`                                                                                     | `{ player, reason }`; ends just that seat's readers (`idle` or `leave`).                                                                                                                                                     |
| `POST /control/referee`                                                                                      | Existing referee view, without credentials.                                                                                                                                                                                  |
| `POST /control/replay`                                                                                       | Existing replay JSON as a JSON string.                                                                                                                                                                                       |
| `POST /control/ping`                                                                                         | Renews the lobby lease; acknowledges `ok`.                                                                                                                                                                                   |
| `POST /control/stop`                                                                                         | Acknowledges `ok`, then disposes the host and listener.                                                                                                                                                                      |
| `GET /control/events?after=<id>`                                                                             | Existing record framing, ring and resume rule. `state` records describe order activity, opened/closed player connections and battle end. A gap gets a keyframe containing current connections and per-player order counters. |
| `GET /control/seats/:player/stream`, `POST /control/seats/:player/up`, `POST /control/seats/:player/preview` | The authenticated raw seat-port transport used by the remote `BattleHost.attach` contract. It uses the same request parsing and stream framing as player routes. Browser allowances remain on player routes.                 |

All control routes require the key, including the raw seat ports. No credential
appears in their replies or events. Connection ids are correlation ids, not
credentials. Start failures use the shared generic `unavailable` refusal; the
server logs a redacted diagnostic and Frontline retains its bounded map retry.

Rooms and Frontline still hold `BattleHost`. `setController` and `endSeat` now
return promises: crossing the control boundary needs an acknowledged enqueue or
reader termination. Seat-ending reasons are the lifecycle vocabulary `idle` and `leave`.
Both host implementations retain synchronous unknown-player
validation where it existed. The lobby's hosting owner, separate from the host
interface, publishes credentials/seating and translates connection events into
calls to the existing presence module. Seating is acknowledged before a ticket
is published. Neither service branches on host implementation.

`BattleTicket.address` is present for the separate battle listener, and absent
for a host at the lobby origin. The page changes only the address passed to its
battle link. The lobby's battle router and the old injected lookup argument are
deleted; its HTTP constructor is named `createLobbyHttp`. Browser request validation, seat allowances, HTTP IO, record history,
presence grace, AI takeover, idle timing and replay remain with their owners.

A lost control surface or battle process follows `onEnd(crashed)`: its room
ends `battle_failed` and frees its place, or its Frontline game is removed.
Running battles require a renewable lease. Preparation holds a bounded lease
through its live start request, so a slow map does not look like a lost lobby.
A broken start request stops preparation. Locally, parent disconnect also stops
the child. Disposal stops the listener and worker, with bounded termination of
a child that does not exit.

Verification and implementation decisions are recorded in the ignored
`throwaway/multiplayer/slice28-report.md` and part 4 of the decisions ledger.

The requested multiplayer domain passed all 282 tests, including the contract
suite against both hosts. The final process-failure suite passed all 13 tests.
Typecheck, changed-file linters and the size ratchet pass. The local-process
`online-battle`, `room-lobby` and `frontline` scenes passed without changing any
approval. The manual HTTP room-battle probe recorded one additional child while
running and none after the existing empty-room end. No simulation or fixture
changed.
