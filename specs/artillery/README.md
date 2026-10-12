# Artillery units

Status: implementation complete; closeout is blocked by the browser scene startup gate. Last updated 2026-10-12.

## Next Agent Prompt

The implementation is landed in commits `0fbd2638` through `e2a503e4`, and `origin/main` is already an ancestor of this branch. Focused native, catalog, publication, typecheck and command-deck gates are green. The artillery scene reaches the existing ballistics lab but times out during lab startup in this checkout, so the required visual critique/compare gate is not honestly green and this plan remains active. Do not weaken the scene timeout or archive until that environment can complete the production route. The full formatter gate also reports four untouched pre-existing files; the touched HUD stylesheet is formatted.

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
- **One command vocabulary:** `Order::Attack`, `Order::AttackMove`, `TargetRef::{Identified, Contact, Ground}` and `SetEngagement` remain the only order shapes.
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
- [ ] Regenerate catalog/replay/parity evidence, run visual review and full checks, and record the named behavior cutover — [slice 05](slices/05-closeout.md)

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

Use the narrowest check per slice. Native tests use `CARGO_TARGET_DIR=throwaway/target`; browser evidence goes in ignored `throwaway/evidence/`; GPU scenes serialize with the repository's `throwaway/gpu.lock`. Any slice that produces a shot must run [screenshot-critique](../../.agents/skills/screenshot-critique/SKILL.md) last. Slice 04 also compares the changed command-bar treatment against its approved pre-change shot with [compare-screenshots](../../.agents/skills/compare-screenshots/SKILL.md), then opens captures with [preview-shots](../../.agents/skills/preview-shots/SKILL.md). Human visual review is non-blocking: give the user about five minutes; if silent, decide from the evidence, record the rationale here or in the slice choices, close Preview and continue.

