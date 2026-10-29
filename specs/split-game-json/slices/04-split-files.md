# 04 Three files, same shape

**Contract:** the combined fixture becomes three files, each with one reader,
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
  about 30 places typed as the `rules.json` import take the narrow type they
  read.
- **Editors:** the mechanics editor writes and hot-reloads only `rules.json`
  (allow-list, hot-reload skip, frozen startup object). The map workbench
  hashes only `rules.json` (`map-workbench/server.ts`). The desktop
  repository sentinel (`main_desktop.rs:116-124`) names `rules.json`.
- **Scope:** every reader and document names its independent fixture; the
  retired filename survives only in archived history and this spec's map/audit.

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

**Done (2026-10-29):** rules, presentation and audio now have independent
fixtures and browser owners (`shippedRules`, `presentationSettings`, `audioMix`).
Input distances live in `web/src/battle/input/settings.ts`; the lobby default
lives in `apps/battle-lab/src/lobby/defaults.ts`. Fielding takes presentation
and authored weapon inheritance separately, preserving the behaviour slice 10
will change. No sections were reshaped or values retuned.

All 77 entries in the supplied `before/SHA256SUMS` match `after-04/` exactly;
all six native plates also match byte for byte. Native replay used the parent
checkout's recording, which produced those supplied controls, with identical
runtime catalog contents. Required browser checks (79 tests), supplemental
browser checks (68 tests), native counts/scenery/identity, contract admission,
the presentation suite and simulation mechanics, typecheck, Clippy, lint and
the size ratchet passed. The native exact-numbers text check caught float-token
normalization in the first rewrite; lossless field deletion fixed it, and both
native and browser checks passed. The unchanged long emitter-throughput report
(`flight_load::report_200_unit_emitter_load`) was also skipped. Frozen generation/config
records passed; the full Rust run was narrowed after about 13 minutes in
unchanged map-generation stress sweeps (unfinished `street_props`, plus
`street_warts`, `swamps`, `town_junctions`, and `towns` skipped).

The one-shot scratch script `throwaway/split-proof/rewrite-split-files.mjs`
removes only the three application fields from the exact-numbers record,
preserving every retained numeric token, and
reports the workbench's changed input receipts/revision. Battle rows and stored
rules/config identities were not re-blessed. Developer snapshot revisions
naturally change with their input files; the simulation source fingerprint
changes as the standing gate permits. Strict section readers remain slice 05.
