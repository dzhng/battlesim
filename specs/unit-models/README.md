# Disabled unit model production

This implementation pass adds authored 3D model coverage for roster cards that remain disabled until their mechanics are implemented. The card catalog remains the authority for availability: adding model metadata or source art must not admit a unit into `fixtures/catalog.json` or make it deployable.

## Current contract

- Every disabled roster card has one model-production record identifying its faction, role family, source/model status, and the reason its runtime card remains disabled.
- Completed ground models use the scene-assets source/bake/validation owner and are bound only when the resolved unit type has physical mechanics and mount fit.
- Air, helicopter, drone, artillery, transport, and air-defense models may be prepared as disabled art records, but they remain unbound placeholders until their simulation contracts exist.
- Shared infantry rigs may reuse geometry; faction and role skins remain distinct in the source record.
- Vehicle deaths use the existing presentation contract for every vehicle hull: a watched vehicle keeps its last live model while it coasts to the wreck, then jolts as it cooks off. Wrecks with authored hull/turret pieces throw the turret; whole-body wrecks (including wheeled and support vehicles) use the same jolt without inventing a turret.

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
`assets/catalog.json` and cannot be selected or deployed. Challenger 3, BRM-3K,
and EBRC Jaguar also have authored family models beside their disabled records.
The Armata family and Type 15 have dedicated authored models as well.

The vehicle death presentation is already covered by `apps/battle-lab/src/cookOffs.ts`
and `packages/battle-renderer/src/effects/cookOff.ts`; the focused cook-off suite
passes for whole-body and turret-throwing wrecks, including vehicles killed while
moving. New model work must preserve the wreck states and fit contract so every
vehicle continues to use that presentation path.

The next art pass should replace the remaining generic disabled silhouettes with
family-specific meshes as the corresponding mechanics contracts are designed.
Do not bind these records to the runtime catalog until a unit type, mount
layout, animation contract, and the relevant mechanics exist.

The archived roster model-production rationale remains at [`specs/done/unit-roster/model-production.md`](../done/unit-roster/model-production.md). The source and validation owner is [`packages/scene-assets`](../../packages/scene-assets/README.md).
