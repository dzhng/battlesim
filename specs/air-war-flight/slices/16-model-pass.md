# 16 — Aircraft model pass (parallel lane)

**Delivers:** D26.

**Order:** one batch per faction (US, Europe, Eastern), working through each family.

## For each family
1. **References.** Fill thin sets with Duet-generated views, following `packages/scene-assets/src/references.ts`: the F-15EX, F-35A/B, Su-30SM2, Tornado IDS and J-20 have thin sets today.
2. **Edit** the Blender script (`packages/scene-assets/blender/roster/`) with `/workbench` open.
3. **Check** with `asset sheet <appearance> --references`.
4. **Judge** with `compare-screenshots` against the references, then `screenshot-critique`.
5. **Accept** into `assets/review/`.

At the end of each batch, make an all-jets contact sheet and open it with `preview-shots`, non-blocking.

## Also
Give the test catalog a `test_jet` model of its own; `test_jet` borrows the F-16 art until then (choices.md).

## Visual gate
- **Variable:** silhouette fidelity. **Crop:** one three-quarter view per family.

## Firewall and delegation
- **Firewall:** no sim reads. Hull sizes must stay within the catalog fit tolerance.
- **Delegated:** part-level modelling, and which reference views to generate.
