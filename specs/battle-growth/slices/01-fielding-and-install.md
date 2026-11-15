# 01 Fielding and install

**Question:** does adding one type read and install only that type's files?

**Depends on:** 00.

## What happens today

Every change to the fielded kinds runs
[`fieldUnits`](../../../web/src/battle/sim/fielding.ts). It asks
[`Fielding::contents`](../../../crates/presentation/src/fielded.rs) for every
drawn bundle, every wreck and every skeleton, inflates all of them
([loader.ts `contents`](../../../packages/scene-assets/src/loader.ts), no
cache), joins them into one buffer, copies that into WASM and parses every
container again. The art library then appears to install twice
(`withUnits`, then `withAppearances`, in
[gameAppearances.ts](../../../apps/battle-lab/src/gameAppearances.ts)), which
would explain the report's generation counter moving by two per new type and
two full rebuilds per growth.

## Confirm first

- That the library installs twice per new type: count installs through the
  loader's injected fetcher.
- Whether catalog admission (`Asset::from_bundle` in
  [lab.rs](../../../crates/presentation/src/presentation/lab.rs)) and fielding
  derive the same facts from a bundle.

## Contract

- A change to the fielded kinds reads only contents fielding has not resolved.
- One library install per change.

## Seam

- Fielding keeps the facts it derives, by content hash, across calls.
  `contents()` names only what it lacks.
- **Decision rule for the owner:** if the two derivations are the same facts,
  one owner derives them and both consumers read it; the second derivation is
  deleted. If they differ, the facts stay with fielding.
- **Fixed:** no inflated bytes are kept in JS. The native client's one-shot
  read (the `files` feature) behaves as before.

## Verification

Write first:

1. [`crates/presentation/tests/fielded.rs`](../../../crates/presentation/tests/fielded.rs):
   resolving kinds {a}, then {a, b}, asks its reader only for b's contents and
   yields the same sections as a cold resolve of {a, b}.
2. A loader test through the injected fetcher: one kinds change makes one
   install.

Then `battle-growth`: contents inflated at growth equals the new type's, and
installs equals one.

## What you can run or see

In `battle-growth` and in play, the long task attributed to the gzip module
just before `battle staged` is gone.

## Delegated

Where the kept facts live inside fielding, and how the binding exposes them.

## Stays green

`crates/presentation` tests (fielded, catalog, showcase), the armory and unit
showcase pages, which field units through the same call.

## Feedback that would change this slice

If the install counter shows one install per type, the doubled rebuild has
another cause: find it here before moving on, because slice 08's gate asserts
one admission per growth.
