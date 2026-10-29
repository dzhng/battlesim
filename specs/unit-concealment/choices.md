# Choices ledger

Decisions made where the spec was silent or wrong, with verdicts. Final consolidation happens at closeout.

## Slice 00

- **The clippy guard covers library code, not native-only tests and examples.** The slice said test code follows the rule too. That would mean rewriting 176 calls in test and example crates, which only ever run natively, so their values never meet WebAssembly. `crates/clippy.toml` lists the methods. Each of contract, sim, mapgen and presentation sets `disallowed_methods = "allow"` in its `Cargo.toml` `[lints.clippy]` and `#![deny(clippy::disallowed_methods)]` in its `lib.rs`. So the library, including its `#[cfg(test)]` modules, is guarded, and no test file changes. Per-file allow attributes were tried first, but the size ratchet refuses growth in test files already over budget. *Verdict: sound; the rule protects what runs in both builds.*
- **`powi` is not disallowed.** It isn't transcendental, and `contract/src/map.rs` squares with it. *Verdict: sound.*
- **The presentation crate had four production violations** of its own rule (`scars/cache.rs` `log2`, `sound/bank.rs` `log10`), plus test-module calls. All are converted, and the guard now covers it too. *Verdict: sound.*
- **No digest, parity record, menu reel or map moved.** The whole Rust suite passed except one pre-existing failure, which is fixed separately (see below). *Verdict: evidence.*

## Slice 01

- **The rules type declares the three non-simulation sections it still carries; it doesn't strip them.** `UncheckedRules` admits `presentation` (owned and parsed by presentation, so the simulation doesn't inspect it), `controls` (typed and strict, owned by the browser's `reverseZone.ts`) and `seed` (the battle lab's default). The alternative was stripping them in each reader: native `fixtures::game()`/`admit()` and the browser's `compose.ts`. That is more places to keep in step. *Verdict: sound.*
- **`status`, top-level `units` and `physics.blast_falloff` were deleted.** They were prose annotations nothing read; strict admission refused them. *Verdict: sound.*
- **Two callers had embedded `map` inside the rules** (`settlement_tests.rs`, `examples/ground_resources.rs`). They now pass the map separately. *Verdict: sound; strictness found a real misuse.*
- **The forest rule types moved to `crates/contract/src/scenario/forest.rs`** unchanged, so the over-budget `scenario.rs` shrinks. *Verdict: sound; slice 04 edits them there.*
- **`mechanics_validate`'s tests now run with the workspace suite** (`[[example]] test = true`). Before, `cargo test --workspace` built them but never ran them, including the existing admission tests. *Verdict: sound; a latent gap.*
- **The accuracy ceiling has one owner, `FlightConfig::check_layer`.** `check_rules` calls it for every target layer of every row. The duplicate nonnegative-spread check went, because contract admission already refuses a negative layer spread. *Verdict: sound.*
- **The editor's range-to-spread coupling is deleted, not ported per layer.** Each layer's range and spread now edit independently, in milliradians. Porting the coupling to three layers would have added code to a tool the user tunes by hand, and the old behaviour (keep landing metres when the range changes) is one multiplication the user can make. Re-enabling a layer restores its admitted values instead of placeholder defaults. The server's intent check now lets a later child edit (a layer's range) supersede that field inside an earlier parent replacement (enabling the layer). *Verdict: sound; revisit if tuning by landing metres is missed.*
- **The air admission tests moved to `crates/contract/tests/air_admission.rs`**, by concept, so the oversized `tests/catalog.rs` shrinks. *Verdict: sound.*
- **Codex removed three doc comments in `sim/src/weapons.rs` to stay under the ratchet; I restored shortened versions.** The file ends exactly at its HEAD size. *Verdict: corrected.*

## Slice 03

- **Shared sight bands live on the existing profiles, with child overrides only where a unit's former reach differed.** Every unit doesn't get a copy of each leaf. Each ground profile authors `low_air_m` equal to its `ground_m` (the old fallback) and `high_air_m` from the old body table (600 on foot and for aircraft, 300 for ground vehicles). Each former radar emitter carries its old `detection_m` as `high_air_m`. *Verdict: sound; the resolved-reach dump is byte-identical for the game, test and capture sets.*
- **Test patches that made a unit an emitter no longer raise its high-air sight.** `radar_emitter` is only the signature now. The radar tests that relied on emitting to see high air author `high_air_m` themselves, and the band test merged into `each_bands_sight_is_the_units_own_and_zero_is_blind`. *Verdict: sound; that coupling was the thing being deleted.*
- **The browser test's fake unit factory moved to `web/tests/sceneAssets/unitType.ts`**, so that `synthetic.ts` shrinks under the ratchet. *Verdict: sound.*

## Slice 04

- **The neutral data is 2.8 on every hull, 1 on every squad, forest 0.25 and moving 1.0.** The resolved catalog has 117 hulls at 2.8 and 22 squads at 1. Squads author it on their infantry profiles, hulls on their profiles or units. `2.8 × 0.25` reproduces the old vehicle forest value of 0.7 exactly, so no digest moved. *Verdict: evidence.*
- **HIDDEN now runs over the same samples for hulls and squads.** Before, a hull checked one point and a squad its living members. A hull's two samples share one ground point, so its answer doesn't change. *Verdict: sound; one path.*
- **`sensing::validate` folded into `SensorRules::check`**, which also admits `moving_concealment ≥ 1`. The forest rule admits `concealment` in (0, 1]. *Verdict: sound; the spec's single owner.*
- **Split by concept to stay under the ratchet:**
  - `contract::catalog::sensors` (the `Sensors` type and `reach_m`);
  - `contract::scenario::sensors` (`SensorRules` and its check);
  - `sim::units::digest` (unit replay digest);
  - `contract/tests/common/catalog.rs` (shared admission inputs);
  - `mechanics-editor/src/{fieldSchema,mountFields}.ts`.

  *Verdict: sound; each file owns one idea.*
- **Clearing Spotting in the editor writes the neutral 1**, and the display treats its omission as 1. The rejected alternative was a nullable reset. *Verdict: sound.*

## Slice 05

- **The report takes no flags and always writes `throwaway/spotting/report.html`.** Heat shades the fraction of full front sight, not absolute metres, and each group lists its unit names with catalog ids. *Verdict: sound; fast to rerun (well under a second).*
- **Only groups with a squad get a garrisoned row**, because only squads garrison. Codex's first cut gave hull groups one too. *Verdict: corrected in review.*
- **Ground-blind observers stay in.** Fast jets author 1 m of ground sight, so they form their own column. *Verdict: sound; it is the real data.*
- **Browser parity ran once and closed the neutral half:** `exactNumbers`, `skirmishObjectives`, `skirmishPurchases` and `mapLayout` passed 15/15 after a fresh wasm build, with no re-blessing. *Verdict: evidence.*

## Slice 06

- **The values match the README table on all 159 physical units:** 129 hulls (10 at 1.8, 34 at 2.4, 85 at 2.8) and 30 squads (22 at 1, 6 at 0.8, 2 at 0.5). A throwaway script wrote literal values and changed nothing on a second run. *Verdict: evidence.*
- **The test and capture scouts (`test_recon`, and `capture_us_recon` through it) are scouts at 0.8**, like the roster scouts. *Verdict: sound; one rule for every scout.*
- **Frozen skirmish inputs got only the changed scalar fields.** Replacing their whole rulesets would have lost unrelated frozen values. *Verdict: sound.*
- **What moved:**
  - The Market Town menu reel's battle differs from tick 1; its first logged event moves at tick 1907.
  - The street fog publications move from tick 62.
  - Seven generated maps change identity hashes (their physical rules embed the catalog).
  - The simulation parity records' stored digests (ticks 0, 1, 90, 599, 600, 1680 and 120) and the Paris Corner capture battle are unchanged. Their scenes never put a concealed unit inside an observer's reduced range. *Verdict: evidence.*
- **I judged the reel and accepted it.** I compared Market Town shots 1–18 before and after, and Paris shots 1–6 (unchanged battle). Every followed unit is alive and framed at its shot's opening, and the followed tank dies at tick 1249 as before. The unprimed critique's framing findings (an empty field shot, a roof-dominated Paris shot, a blast filling a frame) appear identically before and after. Shot 15 now catches a firing tank's dust burst. *Verdict: accepted; no reel tuning needed.*
- **A starting-value consequence to tune later:** a 500 m MBT with spotting 0.8 identifies a still sniper team in forest at about 37 m (`0.5 × 0.25 = 0.125`, raised to the power 1.25). The spotting report shows every such cell. *Verdict: recorded for the balance pass, not tuned here.*
