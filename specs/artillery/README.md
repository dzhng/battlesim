# Artillery units

Status: closeout in progress; implementation and focused gates are complete. Last updated 2026-10-12.

## Next Agent Prompt

Latest main (`003002e0c`) is merged in `f40d776f7`; the artillery implementation is committed in `cf77c20bc`, with the AI replay integration proof in `b85c5a84`. Both WASM modules were rebuilt, the artillery scene passes, runtime artillery fit tests pass after fetching the required LFS objects, and the AI procurement/replay scenario passes. Full `check`/`verify` attempts are recorded below; remaining work is the final review and archival decision. The artillery scene is a proxy flight bench; production roster admission is covered separately by the focused catalog/fit tests.

## Goal

Make every currently planned artillery unit fieldable and useful for both sides. Artillery uses the existing indirect high-arc projectile path, real terrain and prop collision, angular spread, a long range of up to roughly 2.5 km and a large, weapon-specific minimum range. It remains an ordinary weapon: right-click Attack, Attack Move, automatic acquisition and Attack Ground all use the existing command and weapon contracts. A target inside minimum range holds fire and does not make the unit move closer. Existing `FireAtWill` / `ReturnFireOnly` remains authoritative.

The skirmish AI must procure and use artillery, including bounded guesses derived only from its observation frame: identified units, contacts, firing evidence, observed movement and public objectives/routes. It must never read hidden battle state. The existing Attack Ground command receives a contextual tactical emphasis when artillery is selected.

This is a hard cutover. Regenerated catalogs, replays and parity evidence are named behavior changes; no compatibility adapters, old-schema readers or deploy-order migration are planned.

## Settled scope

The 12 rows are:

- US: `us_m109_paladin_m109a7`, `us_m1129_stryker_mortar_120_mm_mortar_carrier`, `us_m142_himars_gmlrs`, `us_m142_himars_prsm`, `us_m270_mlrs_m270a2`
- Europe: `europe_caesar_88`, `europe_mars_ii_multiple_launch_rocket_system`, `europe_pzh_2000_tracked_155_mm_howitzer`
- Eastern: `eastern_2s19_msta_s_2s19m2`, `eastern_9k515_tornado_s_heavy_rocket_artillery`, `eastern_bm_21_grad_bm_21_rocket_artillery`, `eastern_plz_05_chinese_tracked_155_mm_howitzer`

The simulation already owns indirect trajectory selection and collision in `crates/contract/src/ballistics.rs` and `crates/sim/src/flight/solve.rs`; minimum-range holding and normal automatic targeting in `crates/sim/src/weapons.rs`; ordinary `Attack`, `AttackMove`, `TargetRef::Ground` and engagement policy in `crates/contract/src/command.rs` and `crates/sim/src/battle.rs`; and contacts in `crates/contract/src/observation.rs` / `crates/sim/src/knowledge.rs`. The plan extends those owners rather than introducing an artillery subsystem.

## Slice graph

```text
01 ballistic contract and synthetic weapon probe
                │
                ▼
02 physical roster admission and runtime assets
                │
                ├──────────────► 03 observation-bounded AI
                │
                ▼
04 contextual command UI and artillery lab
                │
                ▼
05 hard-cutover closeout and full verification
```

Slice 01 kills the 2.5 km/minimum-range assumption cheaply. Slice 02 makes the planned rows concrete. Slice 03 changes only the doctrine and its observation seam. Slice 04 is the player-facing visual slice. Slice 05 regenerates derived identities and runs the whole system once.

## Global invariants and ownership

- **One projectile owner:** `Trajectory::Indirect`, launch solving, spread and collision stay in the existing flight/weapon owners. No artillery-only flight path.
- **One artillery command vocabulary:** artillery uses the existing `Order::Attack`/`Order::AttackMove`, `TargetRef::{Identified, Contact, Ground}` and `SetEngagement` contracts; it adds no artillery-specific order shape.
- **One catalog authority:** authored fixture documents resolve through `crates/contract/src/catalog.rs`; `fixtures/catalog.json` is derived, never hand-tuned.
- **One AI boundary:** `SkirmishPolicy::decide` consumes `ObservationFrame`, rules and route callbacks. Prediction helpers may retain deterministic evidence-derived memory, never hidden `Battle` state.
- **One UI owner:** `readouts.tsx` owns the command bar; `commandReach.ts` owns eligibility; the session catalog supplies weapon trajectory metadata. Do not create an artillery panel or a second range visualization.
- **No unit-id rules:** artillery behavior comes from weapon/profile/catalog properties. Unit IDs select authored data only.
- **No compatibility sediment:** old planned-card readers, duplicate artillery commands and migration branches are out of scope.

## Global TODO checklist

- [x] Prove a feasible indirect weapon at long range, with angular scatter and a large minimum range — [slice 01](slices/01-ballistic-contract.md)
- [x] Convert all 12 planned rows into complete physical units, roles, weapons, wrecks and runtime appearances — [slice 02](slices/02-physical-roster.md)
- [x] Add deterministic artillery procurement, normal attacks and observation-bounded guessed salvos — [slice 03](slices/03-skirmish-artillery.md)
- [x] Emphasize Attack Ground contextually and add a focused artillery lab/scene — [slice 04](slices/04-command-ui-and-lab.md)
- [x] Regenerate catalog/replay/parity evidence and record the named behavior cutover — [slice 05](slices/05-closeout.md)
- [ ] Complete repository-wide visual/full-check gates when a compatible GPU and full asset inputs are available — [slice 05](slices/05-closeout.md)

## Known risks and firewalls

- Existing model manifests prove source/reference production, not runtime appearance admission. Slice 02 must run the normal asset bake/load/fit checks.
- A 2.5 km row can fail coupled flight admission even if JSON parsing succeeds. Slice 01 must use the real `FlightConfig`/weapon validation path and stop if no feasible profile exists; it must not lower a requirement silently.
- Ground orders may target any public terrain point, while contacts are side-scoped. AI guesses must be derived from `ObservationFrame` only and must never use exact hidden emitters.
- The range ruler is nominal straight distance. It may show `range_m`, but ballistic admission remains authoritative; changing the ruler's geometry is a separate decision and is not part of this feature.
- Artillery does not get a new deployment rule. Existing `stationary` and per-profile deployment capabilities are data; no universal setup timer is introduced.
- Balance, new map generation, new projectile visuals, new sounds and new faction categories are out of scope unless a slice's evidence proves a required input is missing.

## Decisions ledger

- All 12 planned artillery variants become live: user decision.
- Artillery behaves like a normal weapon, including right-click and Attack Move: user correction superseding the earlier manual-only interpretation.
- Existing angular scatter is retained; miss radius grows with distance: user decision.
- Minimum range is large and authored per weapon; inside it the existing `HoldingFire` behavior remains: user decision and repository test contract.
- Players may blind-fire public terrain; AI predictions are evidence-bounded: user decisions.
- Attack Ground gets contextual emphasis using the existing tactical control: user decision.
- Hold fire already exists and remains unchanged: repository evidence plus user confirmation.
- Hard cutover, no compatibility or migrations: user decision.

## Verification policy

Use the narrowest check per slice, then run the repository-wide checks once at closeout. Native tests use `CARGO_TARGET_DIR=throwaway/target`; browser evidence goes in ignored `throwaway/evidence/`; GPU scenes serialize with the repository's `throwaway/gpu.lock`. The command-bar and generic spread cursor are covered by focused browser tests. The artillery capture has a retained ignored image and independent critique, but there is no approved in-tree visual baseline; the feature remains open for archival until that provenance and the full gates are resolved.

## Closeout evidence

Focused native, catalog, browser and artillery-scene checks pass. The five
publication parity records were regenerated for the artillery catalog cutover;
all 25 native publication tests and 13 browser fog-delivery tests pass. The scene
harness now aborts on the first fatal WebGPU diagnostic, preserves the adapter
limit message, suppresses repeated invalid-resource noise and includes page
state on readiness timeouts; its focused five-test suite passes. The known
SwiftShader parity probe now reports one primary texture-array failure instead
of a page-close cascade. The full `bun run check` was rerun with a local ALSA
development sysroot; its earlier completed run had 859 Rust tests passing and
five publication-pair failures before the parity refresh. A corrected absolute
target-path rerun reached the workspace's long map-generation sweep and was
stopped to avoid repeating unrelated expensive coverage. The earlier lint-only
failure was environmental (`alsa-sys` could not find `alsa.pc`), while
`cargo clippy -p sim --all-targets -- -D warnings` is clean. A full browser
rerun after fetching the required LFS baselines and city inputs removed the
pointer failures but still hit the SwiftShader 256-layer limit against the
complete roster's 477-layer model texture array; it was stopped after 120
reported failures and timeout cascades. The artillery scene itself passes both
assertions. The rerun logs are in
`throwaway/artillery-full-check-rerun2.log` and
`throwaway/artillery-full-verify-rerun.log`; the original attempts remain in
`throwaway/artillery-full-check.log` and `throwaway/artillery-full-verify.log`.

No approved artillery or command-bar baseline was supplied or added to the
repository. The current capture is ignored evidence only (`throwaway/evidence/
artillery/frame-artillery-crest.png`); it is not a close-spec visual reference.
