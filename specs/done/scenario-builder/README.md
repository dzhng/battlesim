# Scenario builder

The scenario builder is a developer-first way to author a resolved battle setup, download it as a self-contained JSON package, and reopen it through the ordinary battle runtime. It supports authored forces, per-player slots, passive and active controllers, bounded authored scripts, Sandbox and Economy modes, script-defined victory, embedded user media, and explicit replay exchange. A package is a durable file: the application does not hide drafts in browser storage or turn them into repository fixtures.

## Why this shape exists

A scenario is useful only when it can be replayed outside the authoring page. The package therefore carries the resolved map snapshot and the rules/catalog documents that the simulation already owns. Upload admission happens in WASM before the normal worker starts; the browser does not regenerate a map, consult a fixture by name, or reinterpret physical geometry. The same authority and `BattleView` path then serves an uploaded package and an ordinary battle.

The runtime keeps a fixed Blue, Red, and Neutral topology. The side spike exercised the real contract → simulation → publication → replay path and found fixed-two assumptions throughout sighting, movement, economy, publication, and UI consumers; adding a dynamic FFA layer would have created a second architecture rather than a generalization. Neutral remains a first-class passive slot that returns fire and defends itself. Arbitrary side ids and FFA remain a separate architecture project.

Player slots own units even when slots share a side. A local run may select at most one human-capable slot. Other human-capable slots become passive AI, AI-only slots remain active AI, and a live human slot can be taken over by AI. The command envelope carries the slot, so a same-side controller cannot issue an order for another slot's unit. Observation and replay remain simulation-owned: the renderer receives only the selected side's publication, and every replayed digest must match the live run.

The authored-code surface is a contract-owned bounded interpreter. It can inspect only the observation exposed to its side, use deterministic seeded values, and emit ordinary orders, events, and victory signals. It has no ambient clock, I/O, network, DOM, filesystem, or unbounded allocation. User media is embedded and hash-checked; built-in appearance references carry content identity and remain owned by the shared scene-assets loader. Uploaded maps with no scenery request no scenery art, which keeps the normal renderer inside small adapter texture-array limits.

## Principles and invariants

- The simulation is the only authority. UI, AI, presentation, audio, and renderer consume resolved observations and never infer hidden state.
- A package is canonical and self-contained. Its bytes, content identity, map snapshot, rules, slot ownership, scripts, victory source, and embedded assets are admitted together; noncanonical or changed bytes are refused.
- Every authored unit has exactly one slot owner. Slot ownership is checked at command admission, not inferred from side colour in the browser.
- Neutral cannot become a human controller or an aggressive automatic side. It is passive until threatened.
- A replay restores the same accepted commands, controller state, and authored outputs at the same ticks. Matching terminal state is insufficient when an earlier digest differs.
- Scripts observe only their published side, run under deterministic fuel and output limits, and use the same command vocabulary as human input.
- Economy remains the existing binary skirmish owner. Neutral authored forces and unsupported combinations are refused at the bundle boundary.
- Download and upload are explicit persistence boundaries. Campaign promotion, map-geometry editing, and network multiplayer are outside this feature.

## Code pointers

- `crates/contract/src/scenario.rs` and `crates/contract/src/bundle.rs` own scenario and package contracts; `crates/contract/tests/bundle.rs` and `crates/contract/tests/script.rs` pin canonical admission, slot/controller rules, and the script sandbox.
- `crates/game-wasm/src/lib.rs` exposes canonical bundle validation, details, catalog composition, controller normalization, and prepared-world construction; `crates/game-wasm/tests/bundle.rs` checks the browser boundary against native identity.
- `crates/sim/src/battle.rs` and `crates/sim/src/publication.rs` own command ownership, passive/active AI, visibility, victory, and replay identity; the authored interpreter lives in `crates/contract/src/script.rs`. `crates/sim/tests/side_topology.rs` and the simulation parity fixtures exercise those seams, while `web/tests/authority.test.ts` covers the browser authority boundary.
- `web/src/battle/prepare/client.ts`, `web/src/battle/prepare/worker.ts`, and `apps/battle-lab/src/routes/scenarioPlay.tsx` own explicit upload admission and the normal worker connection. `apps/battle-lab/src/useSimSession.ts` deliberately uses the prepared connection only for its one live authority; replay replacement starts a fresh worker.
- `apps/battle-lab/src/scenario-builder/` owns the tactical canvas and in-memory draft. `web/src/battle/scenarioBundle.ts` and the WASM bundle APIs are the only durable serialization boundary. `web/tests/scenarioBuilder.test.ts`, `web/tests/scenarioBundle.test.ts`, and `web/scenes/scenario-builder.mjs` pin authoring, ownership placement, and export.
- `apps/battle-lab/src/gameAppearances.ts`, `apps/battle-lab/src/useBattleSession.ts`, and `packages/scene-assets/src/loader.ts` own session-scoped art requests. `packages/battle-renderer/src/frame/worldPass.ts` owns the aligned x-ray uniform allocation needed by SwiftShader and other strict adapters.
- `web/scenes/scenario-play.mjs` pins the registered upload-to-BattleView path. A local closeout probe also exercised cross-slot `wrong_slot` rejection, AI takeover, side-scoped observation, replay download/upload, and equal live/replay digests; that probe output is transient and is not a second runtime contract.

## Dead ends and deliberate limits

The arbitrary-side/FFA spike was rejected after a Neutral candidate reached fixed-array assumptions and panicked in the battle path. The product keeps the fixed three-slot topology rather than carrying a dynamic-side compatibility layer.

Browser JavaScript evaluation was rejected as the authored-script runtime because it would make native/WASM parity and capability restriction depend on the host environment. A small deterministic interpreter keeps the authority boundary explicit.

Reusing the prepared upload connection for replay replacement left the new session waiting at the world stage. The final contract gives each authority connection one battle lifetime and creates a fresh worker for a replacement replay.

Eagerly installing the full appearance catalog made a small SwiftShader adapter exceed its texture-array layer limit before the battle became playable. Appearance loading now narrows to the fielded scenario art and omits scenery appearance requests for an empty resolved map while preserving the shared loader for ordinary battles.

This feature does not edit map geometry. It stores a resolved snapshot so a future campaign feature can promote selected packages deliberately, and a later map-authoring feature can own editing without changing the package boundary.

## Verification and visual provenance

The closeout command `WEBGPU_BROWSER=local bun run --cwd web scene -- scenario-builder scenario-play` ran both registered browser scenes successfully against the closeout build. The scenario-specific browser tests and focused native contract/WASM/simulation checks pass; unrelated fog-delivery parity expectations remain tied to the broader suite. The generated WASM bindings were rebuilt after the latest main merge, and typecheck then passed. Formatting and lint pass with existing hook warnings only. The repository-wide check cannot build `native-client` in this environment because the system ALSA development package is unavailable. A broader `cargo test --workspace --exclude native-client --all-targets` run also reaches an existing `mapgen` example panic in `skirmish_tournament::tests::corpus_is_versioned_and_repeatable` (`crates/mapgen/examples/skirmish_tournament/runner.rs:87`), outside this feature. The default attached-Windows-Chrome scene path still cancels Playwright's download artifact; the successful closeout uses local Chromium explicitly.

No uploaded/external baseline or approved in-tree PNG defined this feature. The target was the existing holo-tactical HUD contract from the game-ui guidance and battle HUD owners. The scenario scenes exercise route/readability behavior and write transient captures under `throwaway/evidence`; they are not pixel baselines, and no Preview approval is claimed. Durable behavior remains pinned by the code pointers and tests above.
