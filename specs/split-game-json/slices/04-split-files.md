# 04 Three files, same shape

**Contract:** `fixtures/game.json` becomes three files, each with one reader,
and no value or behaviour changes:
- `fixtures/rules.json`: exactly what contract `Rules` reads;
- `fixtures/presentation.json`: today's `presentation` minus `audio`;
- `fixtures/audio.json`: today's `presentation.audio`.

`controls` leaves the rules for the browser's input settings, read where input
is handled. `seed` becomes the lobby default in one owner; see the README's
correction 8 for which literals move.

**Seam:**
- **Rust:** `sim::fixtures::rules()` replaces `game()`. Presentation and
  native loaders take `presentation.json` and `audio.json` explicitly.
  `UncheckedRules`' skip list goes, because there is nothing left to skip.
- **TypeScript:** one validating owner module per file, the
  `shippedSounds.ts`/`gameBiome.ts` pattern. `SessionCatalog.rules` is built
  from `rules.json` alone (`web/src/battle/catalog/compose.ts:57,80`). The
  about 30 places typed as the `game.json` import take the narrow type they
  read.
- **Editors:** the mechanics editor writes and hot-reloads only `rules.json`
  (allow-list, hot-reload skip, frozen startup object). The map workbench
  hashes only `rules.json` (`map-workbench/server.ts`). The desktop
  repository sentinel (`main_desktop.rs:116-124`) names `rules.json`.
- **Scope:** `git grep game.json` finds about 146 files. Every reference
  goes, docs included.

**Run / see:** start a battle; open `/mechanics`, `/map-workbench` and
`/sound-workbench`. Nothing looks different.

**Verify:**
- 01's probe: identical bytes. Native plates identical.
- Battle digests, `rules_hash` and `config_hash` unchanged.
- Web tests: `mechanicsServer`, `mechanicsStartup`, `mapWorkbenchServer`,
  `soundFrame`, `exactNumbers`.
- New: an edit to `presentation.json` doesn't change a map-workbench revision
  hash.

**Moves:**
- map-workbench revision hashes, once, because the bytes it hashes change;
- the text of `fixtures/parity/exact-numbers.json`, which embeds the raw game.

Rewrite both with one script, `throwaway/split-proof/rewrite-split-files.mjs`.
Nothing else moves.

**Depends on:** [01](01-proof-controls.md).

**Delegated:**
- TS module names;
- the input-settings file name;
- where the lobby seed owner lives.

**Would change this slice:** a reader that genuinely needs both rules and
presentation. It takes both explicitly; never re-merge the files.
