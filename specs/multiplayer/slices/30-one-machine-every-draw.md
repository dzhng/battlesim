# 30 One machine tries every draw

**Stage:** Hosting. **Needs:** 29. **Carries:** decision 68; slice 25's map
retry.

## Why

Pressing Play on an empty server took 57 seconds on the deployed lobby
(slice 29). A new Frontline game draws up to the game's limit of maps
before one is admitted, and since slice 28 every refused draw costs a whole
machine: rent it, send it the battle server, start it, be refused, give it
back. The user asked on 2026-11-16 for this wait to be fixed before the
site points at the lobby.

## Contract

- **A battle is asked for with its candidates, and one machine tries them
  in order.** `createBattle` takes the list of `BattleSetup`s a game may
  be made from. `ready` resolves with what it resolves with today plus the
  candidate that was admitted, and rejects only when none was. A room
  passes one candidate; Frontline passes all its draws at once.
- **The control surface's start carries the list** and answers with the
  admitted one. A battle server refuses a second start as it does today.
- **`unavailable` still means no battlefield could be made,** and `full`
  still means no machine could be had. Neither changes on the wire.
- **The battle server's archive travels compressed.** It is 10 MB as it
  is, nearly all of it the simulation, and is sent for every battle.

## Seam

`apps/game-server/src/battleHost/` (both implementations and the worker's
preparation), `http/control.ts`, `frontline.ts`, `rooms.ts`, the deploy
step's archive and the provider's fixed start command.

**Slice variable:** how many machines a new game costs. Out of scope: how
long one map takes to prepare; keeping a game warm when nobody is playing.

## Run or see

Play on an empty local server: one battle process is started for the game,
however many maps it draws.

## Verification

1. The battle host contract tests, against both implementations: a list
   whose first candidates are refused becomes ready on the first that is
   admitted and says which; a list with none admitted rejects.
2. Frontline's service tests keep their meaning: another map is drawn when
   one is refused, and `unavailable` is answered when all are. Add: the
   draws of one game start one machine.
3. The provider tests with a compressed archive.
4. On the deployed lobby: Play on an empty server, timed, beside slice
   29's 57 seconds.

## Must stay green

The multiplayer domain; the `frontline` and `online-battle` scenes.

## Delegated

Whether the admitted candidate is named by index or sent back whole; the
archive's compression.

## As built (2026-11-16)

A service asks `createBattle(candidates: BattleSetup[], id)` for one host.
`BattleSpec` carries `candidates` to either host and the worker; the shared
runtime tries preparation in order and starts one authority on the first
admitted world. `BattleHost.ready` adds `candidate`, the zero-based index,
to its existing layout, tick rate and map identity. Exhaustion rejects readiness
and settles waiting operations and readers. A room supplies its one setup.

The authenticated `POST /control/start` takes `{ id, candidates }` and returns
`{ layout, tickHz, mapIdentity, candidate }`. A second start is still refused.
The selected setup owns the server's raw seat routes and both hosts' seat
validation. No credentials enter the start reply.

Frontline draws the candidates and their initial placements together before
creating one host. The admitted index selects the setup, placement view and
initial commander used by briefing, roster, tickets and lifecycle accounting.
`mapAttempts` still comes from the game's admission data. Each attempt draws
its map and then its initial seat in the same order as before. The whole batch
is drawn up front, including the suffix that is not prepared after an earlier
candidate wins. Allocation failure answers `full`; exhausted preparation
answers `unavailable` and releases the game. Machine stop belongs
to the provider; the remote host no longer waits for a control-stop round trip
before rejecting readiness.

Release preparation writes `battle.tar.gz` with `tar -czf`. The provider uploads
that archive and uses the fixed command
`tar -xzf battle.tar.gz && exec node battle.mjs`. The key remains solely in the
startup environment and control headers. The local stand-in executes the same
command against a compressed archive.

This pass is local only. Verification, deliberate failing checks, Play timings
and remaining limits are recorded in `throwaway/multiplayer/slice30-report.md`.
The deployed comparison against slice 29's 57 seconds remains an integration
measurement; no hosting service was contacted by this pass.

Local verification covered all 308 multiplayer-domain cases across the full
run and targeted reruns of two migrated control-protocol fixtures. Host
contracts, real processes, compressed provider/release tests, typecheck and
changed-file linters pass. Three empty-server HTTP Play requests each started
one battle process and left none after leave. No simulation or parity fixture
changed.

The `frontline` and `online-battle` scenes were **NOT RUN**: their command was
queued behind another session's shared GPU lock. At the user's instruction,
only this pass's queued command was cancelled; the parent will run both scenes
at integration. No approved picture was changed or re-approved.
