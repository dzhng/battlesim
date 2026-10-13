# 00b — Fixed Blue/Red/Neutral topology

## Contract unlocked

Materialize the slice-00 verdict as a real fixed three-slot runtime: Blue and Red remain opposing combat sides; Neutral is a first-class passive slot that may defend itself and return fire but never initiates combat. Arbitrary side ids and FFA remain deferred.

## Seam

Update the existing `Side` contract, battle-owned side arrays, movement/knowledge/fog/sighting/audio state, observation/publication layout, WASM boundary, and browser `SideName` mirror coherently. Keep one fixed three-entry owner rather than introducing a dynamic side abstraction or a second compatibility path.

Binary-only encounter/skirmish/economy inputs must refuse Neutral with a named diagnostic until their semantics are explicitly implemented. Sandbox authored units may use Neutral. Controller and command ownership remain per player slot, but side visibility remains the simulation's publication boundary.

## Runnable artifact

The native/WASM probe constructs Blue, Red, and Neutral units, advances ticks, shows each slot's own observation, proves Neutral does not initiate orders, proves a threatened Neutral unit can return fire, and compares replay digests. A browser worker/publication test decodes all three side records.

## Verification

- no fixed-two array panic or out-of-bounds access;
- exact three-slot observation/publication round trip;
- Neutral's hostile-target policy is passive and deterministic;
- binary skirmish/encounter paths refuse unsupported Neutral inputs;
- existing Blue/Red tests remain green;
- native/WASM digests and publication layouts agree.

## Firewalls

Do not widen this into arbitrary N sides, relation matrices, or network multiplayer. Do not add compatibility aliases for old side formats. Do not let the renderer infer Neutral visibility or behavior.

## Human review

No final player-facing visual is required. If the three-side observation report is visualized, review it with `preview-shots` non-blocking; no screenshot approval is needed until the builder UI slices.

