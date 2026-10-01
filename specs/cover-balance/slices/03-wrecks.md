# 03 — Name wrecks by their actual category

## Contract and seam

Rename catalog identifiers and all their consumers atomically:

| Existing | Replacement |
| --- | --- |
| `tank_wreck` | `heavy_wreck` |
| `supply_wreck` | `medium_wreck` |
| `jeep_wreck` | `light_wreck` |

The entries remain ordinary `PropType`/`PropBody` rows and runtime `Prop` objects. A live hull still derives cover from weight and names its initial wreck. Wreck props retain explicit cover and existing generic destruction transitions. Keep the cover-consistency catalog validator and tests.

## Work

Search exact identifiers across authored catalogs, scenarios, tests, code and documentation. Rename semantic prop IDs/references and generated resolved catalog output. Do not rename source art files merely because their tank/jeep model names resemble old prop IDs; asset names describe appearance, not cover category. Keep provenance hashes intact unless bytes actually change.

No new body field, runtime inheritance state, special wreck type, optional-cover default, compatibility alias or persistence migration is planned. Preserve heavy 400 HP → medium at 1.2 m height, medium 250 HP → light at 0.8 m, light 150 HP → removed. Preserve footprints, armor, collision flags, appearance binding and knowledge of replacements. The renderer may select/scale models by box as it already does; no new progressive destruction artwork is requested.

Document the runtime validation finding from the README: validation runs at catalog load, tests exercise it too, and the exported WASM resolver accepts supplied documents. Questioning redundancy is not approval to delete the check. Record supported input-path findings; runtime removal is a future separate decision.

## Acceptance

Invoke write-tests before any behavioral implementation. Use existing catalog, cover and destruction tests to show live vehicles and initial wrecks have matching tiers, and further degradation changes to the intended lower tier. Regenerate `fixtures/catalog.json` using the repo's catalog workflow. Search for stale semantic IDs and explain intentional retained asset filenames.

This slice must not change balance or rendered geometry. Catalog order/identifier changes can legitimately change digests: compare semantic state and replay determinism rather than requiring pre-rename raw digest equality. No village battle report is needed solely for renaming; the balance pass owns that evidence.

If a rendered capture is used to verify the rename, compare-screenshots against the pre-rename wreck crop, show with preview-shots, and run screenshot-critique last before accepting it. A naming cleanup must not become an art overhaul.

Internal helper organization is delegated only where necessary for the rename. The data model and destruction semantics are fixed. Update the README and run implementation closeout checks after all slices.
