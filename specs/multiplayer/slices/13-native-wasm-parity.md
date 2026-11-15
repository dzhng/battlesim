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
