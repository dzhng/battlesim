> Frozen historical evidence. Original identities and rejected arms remain scoped to their source; current completion and user-authorized deferrals are recorded in [completion evidence](../../evidence.md).

# Map acquisition core evidence

This checkpoint proves the pure contract resolver and the compiler's saved-source record. It does not complete C09/C60: no fixture moved, and no native filesystem, browser HTTP/Wasm, asset-tool adapter or catalogue metadata exists yet. Production scenarios still use their existing factories.

## What is proven

The resolver proves typed content identity, catalogue selection and materialization, authored IDs, header validity and the caller's narrow part/bay allowance. The CLI test writes real files and resolves them through the common contract, and the frozen stdout corpus is unchanged. A mutant that ignores the template selection is refused, which shows why one shared physical library needs explicit IDs. No new Wasm/browser parity is claimed.

```sh
cargo test -p contract --test maps
cargo test -p mapgen --test compiler
```

Nonempty shape admission must consume C72's shared owner. C33 still owns complete rules, body, tree, raster and resource preparation. A recorded revision does not authenticate an unavailable historical recipe or art source.

## Producers to cut over

At the baseline revision every route had a scene module and every scene a route; the live route registry is `apps/battle-lab/src/fixtures.json`. Each physical map splits into a saved **source** and per-route **encounter** composition (units, orders, scripts, events):

- **Village** (village, replay, watch, benchmark, fog, lean, ground's village mode): `crates/sim/src/village/mod.rs` composes the fixture map with deployment; `crates/sim/src/village/scripts.rs` discovers buildings for comparison scripts.
- **The labs** (geometry, sensors, movement, weapons, consequences, deployment, ambush, garrison, supply, ground, readouts): one saved physical arena each; `apps/battle-lab/src/scenarios.ts` and the scenario owners supply encounter state.
- **Endurance**: `crates/sim/src/endurance.rs` owns a code-built field and house base. Seeded late wrecks are physical augmentation. Its separate wave/remains RNG streams and the wreck-to-corpse draw continuation must survive extraction.
- **Foundation and workbench**: render-only ground at negative coordinates in `apps/battle-lab/src/routes/foundation.tsx` and `apps/battle-lab/src/workbench/benchWorld.ts`. `MapDefinition` cannot express them exactly; they stay pending under C56 and are not counted as cut over.

Consumers that bypass acquisition today: `apps/battle-lab/src/useStaticWorld.ts` builds a second main-thread query world from the same map (removing that is C33), and asset-fit inputs (`packages/scene-assets/src/authority.ts`, `web/asset.mjs`, `benchWorld.ts`) read fixture-derived geometry. They must receive resolved geometry through the built-Wasm prerequisite, without adding IO or Wasm to scene-assets.

## Open cutover risks

No caller has been cut over. Prepared f64 values, unit IDs, seeded draw order, scripts, replay outcomes and scene checks all need factory-level preservation. A Value/JSON/factory/decoder round trip is a different numeric boundary from raw saved-token decoding, and the earlier publication corpus does not prove every shipping village prop bit. The ground-scene drift seen after C01 came from a missed scripted building-discovery consumer, not this boundary, so it does not show the boundary is harmless.

Raw evidence (receipts, logs, inventory probe): tag `city-maps-evidence-2026-09-30`.
