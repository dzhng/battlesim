# 08 — Normal upload/play route and embedded assets

## Contract unlocked

The normal player runtime accepts an uploaded package, starts the ordinary BattleView/session, exposes one human slot, permits human→AI takeover, and plays bundled user audio through the existing audio owner.

## Seam

Extend the normal battle route/preparation identity with an explicit uploaded-bundle path. Keep worker/session lifecycle and publication feeds intact. The route must never reinterpret hidden state or resolve a package through fixtures. The session catalog consumes built-in content hashes plus the bundle's user asset handles.

Do not reuse `replayFile.tsx`'s IndexedDB `last-replay` behavior for scenarios. File input and Download are the only exchange path.

## Runnable artifact

Upload the sample package from slice 07 into the normal runtime route, start a battle, issue a command from the human slot, switch that slot to active AI, and hear one bundled audio clip caused by the simulation.

## Verification

- normal route upload and loading/refusal stages;
- one-human control and takeover;
- observation/publication remains side-scoped;
- no fixture/network lookup for resolved scenario inputs;
- audio hash/type validation and object URL disposal;
- route departure cancels worker work and releases assets.

## Visual gate

Judge only upload-to-battle transition and controller status/readout legibility. Run `game-ui`, `compare-screenshots` against any changed loading/battle target, unprimed `screenshot-critique`, and `preview-shots` non-blocking review.

