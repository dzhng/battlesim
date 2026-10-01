# Cover balance and wreck clarity

## Next Agent Prompt

Status: planned, not implemented. Updated 2026-09-30.

**Only do this after the main session is done with everything else already in progress, including its validation and closeout. Do not interrupt or expand that pass for this spec.** Earlier side-conversation messages are superseded by the decisions here.

The user reports explicitly telling the main session to stop. This document is a queued handoff, not an instruction to resume stopped work. Preserve that stop. When the user resumes the main session, finish its earlier work before starting this spec. If any earlier cover messages already caused edits, audit those edits against this complete final plan rather than assuming the earlier requests remain valid.

When the current pass is complete, read the repo README and AGENTS.md, then reconcile this plan against the completed code. Start with [01: establish the measurement](slices/01-measurement.md), then [02: tune protection](slices/02-protection.md), then [03: clarify wreck names](slices/03-wrecks.md). Reuse existing helpers, fixtures and tests. Do not implement architecture proposals that this conversation rejected. Update this handoff and the checklist before ending each pass.

- [ ] Establish paired cover measurements and baseline evidence: [01](slices/01-measurement.md).
- [ ] Tune light, medium, heavy and building protection: [02](slices/02-protection.md).
- [ ] Rename wreck categories without changing their behavior: [03](slices/03-wrecks.md).
- [ ] Complete relevant battle reports, closeout checks and documentation.

No implementation blocker is known. Medium's exact target and acceptance tolerances below are recommended implementation starting points, not independently confirmed numeric user requirements. Runtime-validator removal and improved wreck destruction art are not authorized changes in this plan.

## Scope and intent

This spec captures the **side conversation about cover**, not the inherited main-session requests about panels, camera movement, supply timings, range reduction or projectile lifetime. Those belong to the existing main pass. Its resulting weapon ranges and projectile behavior are frozen inputs for the cover experiment; do not retune them to make cover pass.

The user wants meaningful protection, preferably produced by visible projectile scatter and fewer hits rather than reducing every hit's damage. Damage reduction is an explicitly acceptable fallback if scatter proves too difficult to balance. The user also wants simple object definitions, clear wreck names, and no unnecessary new fields or specialized wreck logic.

## Final decisions

| Subject | Decision |
| --- | --- |
| Light cover | Approximately 15% effective protection. |
| Medium cover | Between light and heavy; use approximately 30% as the proposed starting point. |
| Heavy cover | Approximately 50% effective protection. |
| Buildings / garrisons | Approximately 70% effective protection, inspired by WARNO's documented building value. Keep the existing building shelter mechanic. |
| Ruins | Heavy cover, approximately 50%. The earlier WARNO-inspired 25% request is superseded. |
| Trees | Medium cover. The temporary request to make them heavy is superseded. Preserve nearby directional trunk cover and forest concealment; do not add blanket forest-area cover. |
| Protection mechanism | Prefer existing launch scatter. Do not add an invisible extra miss roll. Successful direct hits retain their damage unless a documented fallback is necessary. |
| Live vehicle cover | Continue deriving it from the existing weight class. No new hull cover field. |
| Initial vehicle wreck | Continue spawning an ordinary prop with its catalog-defined cover. Preserve consistency with the live vehicle. No inheritance state or new wreck-specific object type. |
| Prop cover | Keep the existing `cover_tier`; do not delete it or change omitted values to weight-derived defaults. |
| Wreck identifiers | Rename `tank_wreck` to `heavy_wreck`, `supply_wreck` to `medium_wreck`, and `jeep_wreck` to `light_wreck`. These are categories of remains, not a vehicle morphing into another vehicle. |
| Further wreck destruction | Preserve existing HP, replacement heights, blocking and removal behavior. No new degradation mechanic or destruction artwork requested. |

Targets describe reduced incoming harm through fewer hits under representative controlled fire. They are not literal scatter multipliers, per-hit damage multipliers, guarantees at every distance, or claims that all explosive weapons receive identical protection.

Current authoring classifications: crates, fences, rubble, light wrecks and configured craters are light; sandbags, dragon's teeth, trunks and medium wrecks are medium; walls, ruins and heavy wrecks are heavy. Recheck completed main-pass fixtures before tuning. Exterior building-body cover and garrison shelter are distinct contexts: do not apply both to the same shot.

## Why retain the current object shape?

`contract::catalog::Hull` defines live vehicles. Its `weight_class` supplies live cover and its `wreck` names the prop left on destruction. `PropType` and `PropBody` define all props; runtime wrecks use the same `world::Prop` as scenery. Wreck entries already have ordinary durability, appearance and `destroyed` transitions. There is no need for a specialized wreck struct.

The small repeated cover value is explicit and checked. Both walls and rubble are immovable, yet their cover differs, so weight cannot replace generic prop cover. Preserve the existing separation: weight governs pushing, prop cover governs protection, live vehicles use their established weight rule. Do not add shared-body abstractions, optional-cover defaulting rules, runtime origin fields, compatibility aliases, or vehicle-to-wreck inheritance merely to remove a checked authored value.

The catalog validator currently rejects a vehicle whose referenced initial wreck has a different cover tier from its weight-derived live cover. This does not prohibit later degradation. A tank wreck currently has 400 HP and becomes medium remains at 1.2 m overall height; medium remains have 250 HP and become light remains at 0.8 m; light remains have 150 HP and disappear. The footprint is retained. All use the generic wreck appearance binding, with the renderer selecting and scaling an asset by the resulting box. This is an abrupt replacement/scaling mechanism, not authored progressive tank destruction; these observations are code inspection, not a visual-quality verification.

## Runtime validation: findings and disposition

The cover-match check runs in the catalog resolver at load time, not each frame. Tests also exercise that resolver. Tests validate their inputs, not every later caller's inputs.

`crates/game-wasm/src/lib.rs::resolve_catalog(documents_json)` is an exported entry point accepting supplied JSON documents and invokes `contract::catalog::resolve`. Rust scenario construction also consumes catalog documents; tests include custom inputs. An external API exists, but whether untested catalogs are a supported player-facing product feature has not been established. Do not claim all runtime data is guaranteed to be the tested bundle.

Keep the current check for this pass. The user questioned its necessity but did not explicitly authorize removing it. A later proposal may remove **only this consistency check** from runtime if the supported loading contract guarantees tested catalogs, preserving the catalog invariant in tests. Do not remove general parsing, unknown-reference, inheritance or schema validation as collateral cleanup. No new validator is needed here.

## Existing regression and measurement limits

`crates/sim/tests/damage.rs::cover_lowers_losses_to_the_same_fire_over_many_seeds` uses a mixed wall/crate scene. Its `covered < open * 0.85` assertion is a minimum regression guard, not proof that heavy cover is only 15% effective. Not all soldiers receive the same tier. The revised durable-target fixture prevents early squad death from saturating the comparison; keep this rationale if that test remains useful.

Current authored scatter factors at inspection were light 1.4, medium 1.8, heavy 2.4, and building 3.0. Building fragment probability separately used 0.35. These are historical measurements, not desired final constants. Main-session changes may alter this baseline.

Two effects must be reported separately: the additional protection from scatter under identical geometry, and total protection from a real obstacle including physical interception. A wall that stops a shot may protect far more than a nominal tier target. Do not weaken collision or penetration to force a damage ratio.

## Ownership and contracts

- `fixtures/village.json` owns balance numbers. Catalog rows own unit/prop properties and wreck transitions.
- Existing cover and shelter readers own scatter selection; existing flight/collision owns physical interception; existing blast exposure owns explosive protection. Each physical effect has one owner.
- The catalog resolver remains the single resolution path. Generated `fixtures/catalog.json` follows catalog edits.
- No new command, observation, publication or body fields are planned. Wreck identifier names change at catalog, scenario and observation consumers together.
- No backward compatibility or data migration is required for the proposed internal identifier rename. If a supported persisted/external catalog contract is discovered, record it before broadening scope; do not silently create aliases.

## Verification and review

Each implementation slice invokes `write-tests` before behavior changes, uses the narrowest meaningful runner, and preserves deterministic replay. Naming-only changes compare behavior after accounting for changed identifiers; do not falsely promise raw digest identity when IDs or catalog ordering change.

Run the quick village report while iterating a rule change and the full report once at balance closeout. Capture a baseline before tuning using the report's save/compare support; main's baseline may differ from the completed main pass, so label it accurately. Follow AGENTS.md for isolated Rust target directories and shared dependencies. Run `bun run check` and `bun run verify` once at implementation closeout, not during spec authoring.

For any visual evidence produced during implementation, invoke game-ui/renderer skills where applicable; use compare-screenshots against the completed main-pass baseline, preview-shots to show the result, and **screenshot-critique as the last check before accepting each shot**. Review the cover/firefight crop and projectile impact pattern, not unrelated scenery polish. User review is non-blocking for reversible visual tuning; record the evidence and decision. Preserve feature-owned evidence under this folder.

## Research and limits

The user chose WARNO as a balance reference, not a demand to copy its implementation. The [reproduced WARNO manual](https://steamcommunity.com/sharedfiles/filedetails/?id=2727549821) documents forest 50%, buildings 70% and ruins 25% damage reduction. These are documented guide values, not a verification of the latest game patch. Our final ruins target deliberately differs, and our medium trunk cover is not WARNO's whole-forest rule.

Earlier comparisons: [CoH3 rifle statistics](https://coh3stats.com/explorer/races/american/units/riflemen_us) show representative light accuracy 0.5 and damage 1, heavy accuracy 0.5 and damage 0.5; approximately 50%/75% expected direct-fire reduction under those multipliers, with weapon exceptions. A [firsthand Broken Arrow infantry test](https://www.reddit.com/r/BrokenArrowTheGame/comments/1lzyfqy/guide_to_detailed_infantry_mechanics/) reported approximately 68% protection in heavy cover; this is an empirical community result, not an official universal constant. Neither comparison is an additional implementation requirement. Replicate the requested effective outcomes in our own controlled fixture before translating them into tuned scatter factors.

## Superseded proposals

Do not implement: ruins at 25%; heavy trees; an explicit new live-hull cover field; deleting prop `cover_tier`; deriving live cover from the referenced wreck; inheriting cover into a wreck instance; changing omission to weight-derived prop cover; generic shared-body refactoring; automatic runtime-validator removal. The final choice is the current simple model with clearer wreck category names.

This plan was authored without agents or implementation edits because the side conversation prohibits sub-agents and the user assigned implementation to the main session. Its architecture review follows refactor-clean: reuse existing owners and avoid adding a second cover or object system.

## Coverage of earlier messages to the main session

This plan replaces all previous side-conversation handoffs: the initial scatter-first request with 15% light / proposed 30% medium / 50% heavy targets, the separate 70% building / temporary 25% ruins request, promotion of trees to heavy, correction back to medium, correction of ruins to heavy at 50%, and the request for wrecks to follow live vehicle cover. Subsequent discussion chose to retain the current checked catalog model instead of adding inheritance machinery; matching the initial wreck to live cover remains the invariant. Explosive exposure, several distances, separate rifle/HMG measurements and the insufficiency of the old mixed-coverage 15% floor remain included. No earlier side-conversation instruction needs a separate implementation pass.
