# 07 · Labs, recordings, docs and closeout

**Unlocks:** the rule is shown, recorded and documented; the plan closes.

## Labs and scenes

- Garrison lab (`apps/battle-lab/src/routes/garrison.tsx`,
  `web/scenes/garrison.mjs`): two squads share the corner shop, and the scouts
  arrive to "no room" and stay outside.
- Transport controls fixture (`web/scenes/transport.mjs`): two squads board one
  carrier, and a third walks over and is refused on arrival. The existing
  "too big for this carrier" check at the command stays.
- Rewrite the garrison and transport descriptions in
  `apps/battle-lab/src/fixtures.json`, the tutorial line
  (`apps/battle-lab/src/Tutorial.tsx`, "one squad per building"), and the
  comment in `routes/garrison.tsx`.

## Recordings

- Re-record the browser capture files once, in the background, as the named
  publication layout change from slices 01 and 05.
- The menu reel is expected unchanged: confirm its event-log tests, don't
  bless.
- Re-record and inspect only the contested scenes.

## Docs

- `garrison.rs` module doc and the `command.rs` doc comments.
- A superseded note at `specs/done/battle-foundation/contracts.md` (the one
  squad per building line), pointing here.
- Reword "one squad per group" in `specs/separate-farm-buildings/README.md`,
  and "one squad to a building" in `crates/contract/src/encounter.rs` and
  `crates/sim/src/encounter/mod.rs` as a posting choice.

## Verification

- The garrison, transport, panels, UI and cursor browser scenes, once.
- Lab shots: `compare-screenshots` before against after, an unprimed
  `screenshot-critique` as the last check, and `preview-shots` (non-blocking).
- The full check ladder and the balance report, once.
- Then run `close-spec`.
