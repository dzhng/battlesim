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
