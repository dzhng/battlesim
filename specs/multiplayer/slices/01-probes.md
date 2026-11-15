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

Not run yet.
