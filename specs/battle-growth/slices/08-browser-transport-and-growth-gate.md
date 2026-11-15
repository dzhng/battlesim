# 08 Browser transport and the growth gate

**Question:** does a real growth, end to end, touch only the new type?

**Depends on:** 01, 03, 04, 05, 07, and 06 if it ran.

## What happens today

- [`ModelAsset.bundle`](../../../web/src/battle/render/modelAssets.ts) is a
  getter that re-encodes the bundle on every read, and texture bytes are
  re-encoded on every source change.
- [`prepareAppearances`](../../../web/src/battle/sim/presentationModels.ts)
  copies every asset into WASM.
- An admission is requested whenever the installed-art object changes
  identity ([scene.ts](../../../web/src/battle/render/scene.ts)), and every
  request passes a new array
  ([sceneAppearances.ts](../../../web/src/battle/render/sceneAppearances.ts)),
  so any install counts as "everything changed".
- A growth may also request twice when the new art and the staged battle
  arrive on different frames (inferred, not exercised). The gate's
  one-admission assertion covers it.

## Contract

At growth the browser encodes and copies only the bundles Rust lacks, and it
requests an admission only when the set of drawn names and their content
changed.

## Seam

- `ModelAsset` holds its content identity and encodes on request; encoding
  happens only for content Rust reports missing. The re-encoding getter is
  deleted.
- `prepareAppearances` declares every wanted asset and sends the missing ones.
- `SceneAppearances.request` and the scene's "art changed" test compare the
  set of (name, content), not object identity.
- One candidate at a time stays. No cancellation.
- A battle staged again while a build is in flight, wanting the same art,
  leaves that build committable. Measured after slice 01: the reference
  battle's first growth stages twice four seconds apart with the same 145
  appearances, and the first build is thrown away after its bake (109 s to
  commit against 65–70 s). Confirm first what the second staging changes.

## Verification

Write first:

1. A test with a fake renderer: an unchanged set makes no request; one new
   asset makes one admission that sends one bundle.
2. The gate, in
   [`battle-growth`](../../../web/scenes/battle-growth.mjs), red at the start
   of this slice: for the growth it plays, bundles sent, contents inflated and
   the Rust work report all equal the delta of staged appearance names, with
   exactly one install and one admission. A loose long-frame tripwire sits
   beside it: no frame between `battle staged` and `art committed` longer than
   about a quarter of what slice 00 measured at the scene's profile.

The existing assertion, no scene rebuild, stays.

## Closing measurements

Record each in the [ledger](../decisions.md):

- The wait for a one-type arrival at high graphics. This decides the preload's
  depth (slice 09).
- A multi-type arrival in the reference battle. A visible stutter fires the
  reslice trigger in the README.
- Staging to commit in the reference battle.

## What you can run or see

Play the reference battle. The diagnostics report shows no long frame at
`battle staged`.

## Delegated

The tripwire's exact value and whether the fixture is marked serial to keep
its timing honest.

## Stays green

Every lab route that uses `SceneAppearances`, the workbench's re-bake (same
name, new content), the visual suite.

## Feedback that would change this slice

A multi-type stutter, or a spotted type seen waiting behind a build, fires a
reslice trigger in the [README](../README.md); neither is patched in here.
