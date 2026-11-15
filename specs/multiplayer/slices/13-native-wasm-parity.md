# 13 Native and WebAssembly parity

**Stage:** Foundation, in parallel. **Needs:** nothing. **Burns:** O2.
Slice 21 waits for it.

## Contract

Know where the native and WebAssembly builds first disagree, and settle it.

The same endurance battle ends on `63ca37e9b78dcd8a` natively (Mac and Linux)
and `421fb87634122caa` in WebAssembly (Mac and Linux). After one simulated
minute, before any shooting, both are `217810a3a7c10c42`. Each build agrees
with itself across machines.

This does not block rooms: the server and every page run the same
WebAssembly file. It does contradict the repo's parity tests for longer
fights, and a battle recorded on the server may not replay in a native tool.

## Seam

A probe that steps both runners over the endurance battle and compares
digests tick by tick, then narrows the first differing tick to a subsystem.
Scratch output goes in `throwaway/multiplayer/`.

## Run or see

The first differing tick, the subsystem, and the cause.

## Verification

One of two endings, both acceptable:

- **Fixed:** the two builds end on one digest, and a parity test covers a
  battle long enough to include fighting.
- **Named:** the cause is written in [decisions.md](../decisions.md) part 4
  as a decision, with what it means for replaying a server battle natively.

## What the result changes

If the divergence sits in code slices 21 to 23 change (objectives, unit
ownership, the economy), fix it before those slices. Otherwise it may be
named and left.

## Delegated

How the probe steps the WebAssembly runner.

## Result (2026-11-15): the two builds agree

The disagreement did not reproduce, at commit `cad00e8a0`, on the endurance
battle (200 units, one seed, an hour of fighting):

| Tick | Simulated time | Native | WebAssembly |
|---|---|---|---|
| 1,800 | 1 minute | `217810a3a7c10c42` | `217810a3a7c10c42` |
| 18,000 | 10 minutes | `63ca37e9b78dcd8a` | `63ca37e9b78dcd8a` |
| 27,000 | 15 minutes, with losses | `535237c44d3c3c34` | `535237c44d3c3c34` |
| 108,000 | 60 minutes, 73,259 rounds fired | `284b8769ce57c980` | `284b8769ce57c980` |

The two builds were stepped over the same scenario bytes. Digests taken
every 900 ticks agree at all 31 checkpoints through tick 27,000
([the list](../assets/parity-endurance-checkpoints.txt)), and the two
hour-long runs end on one digest.

What the walk saw: its "native" digest, `63ca37e9b78dcd8a`, is this battle
at ten minutes. Its WebAssembly digest, `421fb87634122caa`, appears at no
checkpoint of either build now. The likeliest explanation is that the walk
compared two runs of different length or a stale WebAssembly file; that
cannot be checked, because the walk's files are gone.

What this does and does not show:

- A battle recorded on the server replays in a native tool, as far as this
  battle exercises the simulation: movement, sight, fire, damage, an hour
  long.
- The endurance battle has no skirmish in it: no purchases, objectives,
  score or AI players. Parity there rests on the existing paired records
  (`fixtures/parity/skirmish-*.json`), which are short.
- Nothing was added to the tree. A standing test that runs both builds for
  an hour would cost about 25 minutes a build; the comparison is two
  commands when it is wanted again (`endurance_report 60` natively, and the
  same scenario stepped through the WebAssembly binding).

Slice 21 no longer waits on this.

