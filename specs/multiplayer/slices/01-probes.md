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

**O1: run on 2026-11-15, and it misses.** `bun run server:soak 600` (a
generated 5v5 of ten AIs on a battle thread, real time, ten minutes) on the
temporary host: 47% of ticks ran over the 33 ms budget, the 95th percentile
step was 95 ms, and the battle shed 30% of its ticks. From the fifth minute
it ran at about half speed. The host is an 8-core Xeon Platinum 8259CL at
2.5 GHz. Raw numbers for everything below are in
[host-comparison-2026-11-15.txt](../assets/host-comparison-2026-11-15.txt).

**The same soak misses on fast hardware too, by less.** On this Mac (an M6)
and on a Ryzen 9 9950X it ran 18% of ticks over budget and shed about 5%,
all of it from the fifth minute on, when the armies are large. So the host
is one limit and the simulation's step with ten AIs is another: no host
measured here holds 30 ticks a second through a full 5v5 of AIs.

**A second host was measured the same day: a Bento sandbox,** at the user's
suggestion (a sandbox for each match). It is a rented Linux machine, billed
by the second, on a Ryzen 9 9950X in France.

| | Bento VM | Sandbox, 2 vCPU | Sandbox, 4 vCPU | Sandbox, 8 vCPU | Mac |
|---|---|---|---|---|---|
| Soak: ticks over budget | 47% | 18% | 19% | 30% | 18% |
| Soak: ticks shed | 30% | 4% | 6% | 18% | 5% |
| 200-unit fight flat out: median tick | 22.8 ms | 11.0 ms | 11.8 ms | 14.5 ms | 10.2 ms |
| 200-unit fight paced: late ticks | 24% | 7% | 16% | 18% | |
| An order's round trip from California | 95 ms private, 300 ms public | 182 ms | | | |
| A stream is cut after | 300 s | not within 400 s | | | |
| Price | the workspace's | $0.018 an hour | $0.036 | $0.072 | |

- **Speed:** a sandbox core is about twice the VM's and level with the Mac.
  More cores buy nothing for one battle, which is one thread; the three
  sandboxes landed on different machines and the largest was the slowest,
  so expect a spread of about 30% between machines.
- **Native is no faster than WebAssembly there either.** The native
  endurance report on the sandbox ran a median tick of 6.5 ms in its first
  five minutes and 13.2 ms in its second; the Mac ran 2.6 and 6.9. Decision
  13 stands on this host too.
- **Starting one takes about nine seconds:** from the create call to the
  real game server answering `/hello` through HTTPS, with a 3 MB bundle
  uploaded on the way. A match-hour on the smallest size costs about two
  cents.
- **The route:** an HTTPS address per sandbox and port. The stream arrived
  unbuffered, one record a chunk, and was not cut in 400 seconds. Every
  machine was in France, with no choice of region seen: 182 ms an order
  from California is distance, and a player in Europe would be close.
  Measured through the token-protected address; a page on another origin
  cannot send that token, so real play needs the port made public.
- **Memory grew through the soak** on every machine: about 1.1 GB at ten
  minutes on the sandboxes, rising about 75 MB a minute from the fourth.
  Whether it levels off was not measured. The smallest sandbox has 4 GB.
- **Limits not tested:** how many sandboxes may start a minute and exist at
  once under the workspace's plan; a start from a saved disk (the provider
  says seconds, and Bento does not admit it yet); a player in Europe.

- **WebSockets work through a sandbox's address,** which the VM refuses.
  One held for seven minutes was not cut. A message's round trip from
  California was 173 ms against 177 ms for an HTTP request at the same
  moment, and frames pushed 30 times a second arrived evenly (95th
  percentile gap 34 ms). The gain is a few milliseconds: the wait is
  distance. A socket would simplify the transport, not speed it up.

**What this changed:** the user chose a sandbox for each battle (decision
68, slices 28 and 29).
