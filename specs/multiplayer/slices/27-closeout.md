# 27 Closeout

**Stage:** Frontline. **Needs:** every other slice.

## Contract

The plan is finished, proven once in full, documented where each concept
lives, and archived.

## Steps

1. Pull what the full run needs from large-file storage first: the audio,
   the roster and infantry sources, the scene baselines and the city.
2. **The one full run.** Start it in the background with a visible sign of
   progress, and leave that checkout alone until it ends. Fix what it finds,
   each unrelated failure in its own commit.
3. Sweep for leftovers with
   [refactor-change](../../../.agents/skills/refactor-change/SKILL.md): the
   dev battle scaffolding (slice 18 should have removed it), any second copy
   of a seat or lobby model, any page-side sequence counter, any server code
   that names the host outside `apps/game-server/deploy/`.
4. Run [review](../../../.agents/skills/review/SKILL.md).
5. Docs, with [write-docs](../../../.agents/skills/write-docs/SKILL.md):
   `development.md` gains where the server, the wire contract, rooms and
   Frontline live, and the two seat events if they are deliberate rules. The
   root README says how to run the server beside the site. Each new folder
   has a README written for the next agent.
6. Delete the probe apps `mp-probe`, `mp-soak` (the host soak of
   2026-11-15) and `addr-probe` (the address header check of 2026-11-16) from the `armata` workspace, with the user's word, and the
   scratch under `throwaway/mp-probe/` and `throwaway/multiplayer/`.
7. **Ask the user** before promoting and deploying Frontline, as in slice 19.
8. Archive the plan with
   [close-spec](../../../.agents/skills/close-spec/SKILL.md): move it to
   `specs/done/` and rewrite it from a ladder into the rationale that should
   outlive the build.

## Verification

- The full run is green, and its output is linked from this file.
- Every open item in the README is closed or named in decisions.md with what
  it now means.
- Every row of decisions.md part 3 marked ★ has the user's answer beside it,
  or a note that they were shown it and let it stand.

## Results

Not run yet.

Notes for the run (2026-11-16):

- Staging's merge of that day brought buying a squad aboard a carrier. The
  online purchase preview dropped the carrier; it now carries it (wire,
  server, battle host), with tests.
- The three result-screen pictures (`ui/report-*`) are re-approved: the
  only difference from the approved ones was the headings, "YOUR SIDE" and
  "ENEMY". The main menu's picture is re-approved with the Multiplayer
  tile, whose art the user approved on 2026-11-16.

## Results (2026-11-16)

The one full run, on the branch with staging merged (`1ac47e628`) and the
fixes that followed it.

- **Format, lint, typecheck, the WebAssembly build:** pass.
- **The Rust workspace, run to the end:** 165 test groups pass. One
  failed, the digest pins in `seat_replay`, which matched staging's own
  values and are retired (decisions, part 4).
- **The web suite:** 1,856 of 1,857 pass. One Frontline test timed out
  inside the whole suite and passes alone; it has its neighbours' hang
  watchdog now.
- **The scene suite:** 1,669 checks pass. Four pictures are re-approved
  (above). Two failures are staging's own, shown by running the same
  scenes on a plain staging checkout the same day, and are left to the
  sessions that own that work: `allies` (your tanks do not reach your
  ally's force) and `frame`'s `river` picture (3 pixels).

What the run found that the multiplayer tests could not: a release-order
test that runs the real promotion script; a stale expectation of the
authority's opening request; a site build that threw where git cannot be
run; a function name the math-owner check reads as a matrix helper; a
scene helper that imported a module the scene runner cannot load, which
stopped the whole scene suite. All fixed.

Not rerun after those fixes: the whole of each suite. The affected tests,
the lint and the scene suite were.

## The sweep's list, from an independent read on 2026-11-16

Fixed the same day: a lobby nobody is left in lingered for a minute with
no host, shown in the public list, and held its maker's one game; the
battle server's stop route had no caller; a test still named the dev
battle; comments that told history.

Left for the sweep (step 3), most valuable first:

1. **The lobby's in-process battle path outlived the move to a battle
   server for each battle.** The remote host's `attach` and the raw
   `/control/seats/*` routes, the slot's `id` and `seatOf`,
   `online.battle()`, the services' `createBattle` option and the
   capacity-only form of the online state are reached by tests alone.
   Tests should reach seats the way players do, and `BattleTicket.address`
   becomes required.
2. **The publication payload's layout** (tick, digest, step time, words)
   is encoded once and decoded by hand in three places. One pair of
   functions beside the record stream.
3. **The browser id's format and its header's name** have no owner; the
   control header has one.
4. **Returning to Multiplayer with a notice** is spelled out six times
   with bare state keys; "leave Frontline on the way out" twice.
5. **The seat model keeps its position-based twin** (`SkirmishSeat`,
   `skirmishPlayers`) for the shareable link's parser.
6. **The room service turns the room model's sentences into wire codes**
   by comparing strings.
7. **Names:** `roomFailure` words Frontline's refusals too;
   `OnlineSimClient` is what the offline input code takes; two types are
   both called `Seat`; `http/seats.ts` owns `HttpRefusal`; `battles.ts`
   beside `battle.ts`.
8. **Smaller:** which lighting values are valid is decided twice; a random
   seed and a clamped random index are each written twice; page wording
   has drifted between files; `hello`'s error sentences are never shown,
   so a server that is down reads as a wrong build; `keepBuiltMap` and
   `OnlineMatchResult`'s optional return are test seams; the battle
   process loads the simulation on its main thread for a field only tests
   read; `Machine.control` equals `Machine.address` everywhere.

Known and not fixed: two tabs of one browser in one seat. The older tab's
orders are refused for their sequence number and retried for ever without
a word to the player.

