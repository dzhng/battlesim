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
