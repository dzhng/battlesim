# 01 Host and endurance probes

**Stage:** Foundation. **Needs:** nothing. **Burns:** O1, O3, O4.

Start this first and let it run in the background while slices 02 to 05 are
built. It writes no product code.

## Contract

Three measured facts the later slices read:

- **O1:** the share of late ticks when the temporary host runs a generated
  5v5 of AIs at 30 ticks a second.
- **O3:** how tick time and memory grow over a two-hour 5v5 of AIs.
- **O4:** how many bytes a join snapshot is at each ten-minute mark of that
  run. The code caps a record at 64 MiB.

## Seam

None in the product. The scripts and the probe app from the walk are in
`throwaway/mp-probe/` (`make-scenario.mjs`, `bench.mjs`, `measure.mjs`, and
`app/`). If that folder is gone, rebuild the scripts from the endurance tool
and the skirmish tournament, which already generate a map and play a full AI
match without a browser. The probe app `mp-probe` is registered, private and
idle in the `armata` workspace.

Known from the walk, so you can sanity-check your numbers: the 200-unit
endurance battle ran a median tick of 5.4 ms natively on the Mac, 10.2 ms in
Node on the Mac, and 22.8 ms in Node on the host, with 24% of ticks late
there. Fog took 41% of a tick, weapons 31%, movement 15%. That battle is
200 units all fighting at once; a full 5v5 allows 300 units, and one battle
used 556 MiB after ten minutes.

## Run or see

A table in `throwaway/multiplayer/probes.md`, and its three results copied
into this file under "Results" when the slice closes.

## Verification

- O1 reports ticks over the 33 ms budget and the 95th percentile, not an
  impression.
- O3 and O4 report one row per ten simulated minutes: median tick, 95th
  percentile, memory, snapshot bytes, live units and units ever bought.
- The two-hour run is native and runs on this machine. Give it a progress
  line every ten simulated minutes, and stop it if a ten-minute block takes
  more than an hour of wall time.

## Delegated

Script shape, the report's format, which generated map the 5v5 uses.

## What the results change

- **O1 misses** (the host cannot hold 30 ticks a second): record it. The build
  continues (P27); slice 09 still deploys to this host, and the host swap is a
  later change to one folder.
- **O3:** slice 21 sets Frontline's points target from it. If a battle cannot
  hold its tick rate for an hour, say so in the README's "Deferred" section
  and open "forgetting the dead" as its own plan before slice 25.
- **O4:** slice 25 uses it to bound how long a joiner waits. A snapshot
  heading for the 64 MiB cap inside the target's length is a blocker for
  Frontline, not for rooms.
- **Memory:** slice 15 sets the battle cap from it (P26).

## Results

**O3 and O4: run on 2026-11-14.** Two simulated hours of a generated 5v5 on
an XL map, ten AIs, native build, on a heavily loaded Mac. The full log is
[endurance-two-hour-5v5.log](../assets/endurance-two-hour-5v5.log). The
probe itself is scratch, in `throwaway/multiplayer/probes/`.

| Minutes | Instructions a tick, median | Wall, median | Memory | Join snapshot | Live units | Units ever bought |
|---|---|---|---|---|---|---|
| 10 | 265 M | 15.7 ms | 0.5 GB | 0.7 MB | 183 | 213 |
| 30 | 288 M | 12.8 ms | 1.3 GB | 1.6 MB | 210 | 303 |
| 60 | 319 M | 12.3 ms | 1.8 GB | 2.1 MB | 217 | 371 |
| 90 | 341 M | 13.9 ms | 2.6 GB | 2.5 MB | 205 | 388 |
| 120 | 402 M | 14.7 ms | 3.2 GB | 2.6 MB | 215 | 418 |

- **How the battle was kept going.** A skirmish would end in 30 to 40
  minutes, so the probe fed the battle rules in which no objective can be
  captured and no score reached. The map, the ten AIs and the simulation
  were unchanged. Read the result as the cost of two hours of ten AIs buying
  and fighting; it says nothing about scoring, and the AIs fought over
  objectives they could never take.
- **O3, tick cost: it grows slowly.** About half again more work per tick at
  two hours than at ten minutes, most of it in the first twenty minutes as
  the armies fill out. The armies sat near 200 live units and only 418 were
  ever bought, so this run did not stress the "every unit ever bought stays
  in the loops" landmine hard: a game with humans feeding units into a
  grinder would buy far more. On this Mac the median tick stayed under half
  the 33 ms budget throughout.
- **O3, memory: it grows without levelling off,** about 1.4 GB an hour, to
  3.2 GB at two hours. This is the finding that matters. A 35-minute room
  battle on a large map needs about 1.3 GB, so memory, not cores, caps how
  many battles a host runs (P26): the temporary host's 16 GB holds roughly
  eight room battles, fewer once Frontline games run long. A WebAssembly
  battle cannot pass 4 GB at all, which this rate reaches in under three
  hours. What is growing was not measured; the recorded order log, wrecks
  and corpses are the suspects the walk named.
- **O4, join snapshot: not a problem.** 0.7 MB at ten minutes, 2.6 MB at two
  hours, and flattening as the sides finish exploring the map. The 64 MiB
  cap is far away.

**What this changes.** Frontline's target of about an hour (decision 54)
stands, and now has a second reason: memory. Slice 21 sets the target so a
game ends near the hour. Finding what grows and trimming it is the
"forgetting the dead" plan in the README's deferred list, and it is needed
before any Frontline game is allowed to run much past an hour. Slice 15 sets
the battle cap from memory: about 2 GB a battle.

**O1:** not run yet.
