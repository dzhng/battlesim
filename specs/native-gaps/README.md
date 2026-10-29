# Native gaps (placeholder)

**Status:** remaining platform gaps are unsliced. The shared renderer resolves
shader-layout naming and actual impostor rendering.
**Updated:** 2026-10-28.

## Next Agent Prompt

You are picking up the known limitations [native rendering parity](../done/native-rendering-parity/README.md) shipped with. The archive's "Known limitations" section names each one and its owner. When the user starts this, walk the list with them: which items matter, in what order, and how each fits the [shared renderer](../done/one-renderer/README.md). Then run `write-spec` and turn this page into a sliced plan. Load `renderer` before any GPU work. Update this section before you end each pass.

Each item is independent, so they slice one by one, roughly cheapest first:

- [x] Name the sky's two bind group layouts.
- [ ] Judge water on a plate. Moved to the [water native follow-up](../water-native/README.md).
- [ ] Load the shout pool in Rust, so unseen enemies' shouts play natively.
- [x] Draw far soldiers as impostor cards on native.
- [ ] Native ground marks, once native has a UI.
- [ ] Native info cards and the high-air screen plane, once native has a UI.
- [ ] Match the browser's music duck and reverb, if the difference is ever audible enough to matter.
- [ ] Verify native on a second platform.
- [ ] Team skirmish on native: the lobby, the allied look in marks and cards, and the per-player report, once native has a UI.

## The gaps

Where an item proposes a move into Rust, it follows the project's rule: Rust decides, renderers draw, one owner per concept.

1. **Shader layouts have shared semantic names.** The [common typed catalog](../../crates/renderer-core/README.md) owns pipeline layouts for both platforms.
2. **Water has never been judged on a plate.** Neither reel map has water. Native draws it through the browser's own shaders, so it should match by construction, but nobody has looked. Film a map with water in `native-parity-capture` and compare.
3. **Unseen enemies' shouts are silent natively.** Rust already picks the take (`presentation::sound::frame`, `Sound::set_shouts`), but the pool of takes is loaded by the browser's `packages/battle-audio/src/voices/shoutPool.ts`, and native loads none. Move the pool's take list into Rust, so it has one owner and native plays the shouts.
4. **The shared renderer bakes and draws actual impostor cards.** Its [model/material path](../../crates/renderer-core/README.md) supplies both platforms. Further view-distance or card-cost optimization belongs to [distant impostors](../distant-impostors/README.md).
5. **Native paints no ground marks.** The UI paints them, and a native replay has no UI. This waits until native gains a UI and plays live; the [shared renderer](../done/one-renderer/README.md) already owns its rendering path.
6. **Native draws no info cards.** Every unit's floating info card (name, weapons, state) and its leader line is browser DOM (`web/src/battle/present/readouts.tsx`, laid out by the pure `readoutLayout.ts`), so a native window shows none. The high-air screen plane depends on them. In the browser, high-air models draw in a second canvas between the ground and low-air cards and the high-air cards, so a jet always sits above the lower cards ([air-war flight](../done/air-war-flight/README.md), 2026-10-15). Native has no cards, so it draws high-air models in its one pass, in the presentation's `HIGH_PASS` batch. When native gains a UI, both move with it:
   - the cards draw in two planes, with the layout rules of `readoutLayout.ts`, best moved into Rust so both clients share one owner;
   - native draws its `HIGH_PASS` batch after the lower cards and before the high-air cards.

   Like item 5, this waits on native having a UI.
7. **Native's music duck and reverb differ.** Native glides the music duck linearly and uses `kira`'s reverb; the browser glides by a time constant and convolves with a synthesized impulse. These are player differences only, not decisions.
8. **Only macOS with Metal is verified.** Windows isn't a requirement, and no native dependency rules it out. Verify on a second platform when one matters.
9. **Performance comparisons require a matched protocol.** Historical browser/native frame gaps are not a standing rendering-policy requirement. The [one-renderer research](../done/one-renderer/research.md) records complete-coverage cohorts, native profiling synchronization and platform surface costs.

10. **Native has no team skirmish.** [Team skirmish](../done/team-skirmish/README.md) is desktop-browser only. Native replays browser parity captures of encounters, and fields each unit kind with one look (`fielded::fielded` passes `factions: None`). It can't replay a skirmish: that needs the battle's players, and each side's faction from them, passed to the shared fielding as the browser does through `presentation-wasm` (a side-keyed `[blue, red]` pair). Allies' models need nothing more, because they come from the same publication. Beyond replay, native lacks everything the browser draws as UI:
   - the order-of-battle lobby and its map preview (the lobby model under `apps/battle-lab/src/lobby/` is plain state, so a native lobby can reuse its rules);
   - the allied colour (`presentation.hud`) and shape cue on ground marks and info cards;
   - the read-only ally panel and own-only selection;
   - the per-player match report.

   Like items 5 and 6, the UI waits on native having one. Native never needs to start a team skirmish to replay one.

## Open questions for the walk

- Does native need to play live (with a HUD) before ground marks, the impostor cost or the audio differences matter?
- Which remaining gaps matter now that the [renderer is shared](../done/one-renderer/README.md)?
- Which second platform, if any?
