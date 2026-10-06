# Disabled unit model production

This implementation pass adds authored 3D model coverage for roster cards that remain disabled until their mechanics are implemented. The card catalog remains the authority for availability: adding model metadata or source art must not admit a unit into `fixtures/catalog.json` or make it deployable.

## Current contract

- Every disabled roster card has one model-production record identifying its faction, role family, source/model status, and the reason its runtime card remains disabled.
- Completed ground models use the scene-assets source/bake/validation owner and are bound only when the resolved unit type has physical mechanics and mount fit.
- Air, helicopter, drone, artillery, transport, and air-defense models may be prepared as disabled art records, but they remain unbound placeholders until their simulation contracts exist.
- Shared infantry rigs may reuse geometry; faction and role skins remain distinct in the source record.

## Pass order

1. Inventory and card-level coverage record for all disabled cards.
2. Remaining ground models: BRM-3K, Armata family, Type 15, Challenger 3, Jaguar, Akeron and Javelin.
3. Disabled support/air/drone model families, using one explicit source family per platform and no runtime admission.
4. Asset bake/check and visual review; keep the disabled-card gate green.

The archived roster model-production rationale remains at [`specs/done/unit-roster/model-production.md`](../done/unit-roster/model-production.md). The source and validation owner is [`packages/scene-assets`](../../packages/scene-assets/README.md).
