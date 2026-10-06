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

## Current handoff

The disabled-card registry now has source-authored GLBs for all 86 disabled
cards under `assets/source/roster/disabled/`. These are deliberately static
source placeholders for deferred mechanics families (aircraft, rotorcraft,
artillery/support, drones, and deferred ground cards); they are not entries in
`assets/catalog.json` and cannot be selected or deployed. Challenger 3 also has
an authored family model at `assets/source/roster/challenger/`.

The next art pass should replace the generic disabled silhouettes with
family-specific meshes as the corresponding mechanics contracts are designed.
Do not bind these records to the runtime catalog until a unit type, mount
layout, animation contract, and the relevant mechanics exist.

The archived roster model-production rationale remains at [`specs/done/unit-roster/model-production.md`](../done/unit-roster/model-production.md). The source and validation owner is [`packages/scene-assets`](../../packages/scene-assets/README.md).
