# 01: A bundle may carry a far mesh

## What this unlocks

A static prop's source can name a fifth, coarsest mesh, and the bake
validates, encodes and ships it. No shipped appearance uses it yet, so every
runtime file stays byte-identical.

## Seam

All in `packages/scene-assets`, plus the Blender helpers.

- **Naming.** The far mesh is exactly the source's `_LOD4` meshes. An
  unsuffixed mesh joins tiers 0 to 3 as today and never the far mesh, so no
  existing source gains one by accident.
- **Where it is allowed.** Static bundles of scenery kinds whose footprint is
  `prop` (`src/scenery.ts`). Skinned and articulated bundles, kit modules,
  trees, dressing and grass refuse `_LOD4` with a finding.
- **Shape.** `src/build.ts` (`tierParts`) builds four tiers, or five when the
  source names `_LOD4`. `src/codec.ts` (`assertShape`) accepts four or five for
  a static state and exactly four everywhere else. `FORMAT_VERSION` does not
  move.
- **Rules** (`src/validate.ts`, findings declared in `src/schema.ts`):
  - the far mesh has at least one triangle and strictly fewer than tier 3;
  - every state of an appearance carries it, or none does;
  - a kind that budgets its tiers (`SceneryRule.tier_triangles`) and carries a
    far mesh gives the budget a fifth entry, and the far mesh stays under it.
    A kind without a budget needs none.
- **Constants.** `TIER_COUNT` stays 4 and keeps meaning the tiers every bundle
  has. One new named constant in `schema.ts` is the far mesh's index. It is
  the pair of `FAR` in `crates/presentation/src/view.rs`
  ([slice 02](02-far-selection.md)), cross-referenced in both comments the way
  `TIER_COUNT` and `TIERS` are.
- **Blender.** `blender/parts.py` gains one constant for the far tier. Its
  default `TIERS` stays the four, so a part reaches the far mesh only by naming
  it (`lods=(…, FAR)`).
- **Review sheet.** The asset sheet and the model workbench list a state's
  far mesh beside its four tiers when it has one.

## What the user can see

A synthetic far-carrying prop on the asset review sheet with five tiers, and
the validator's findings for the refused cases.

## Tests, written first

With synthetic sources and budgets the test defines, in the existing bake,
validate and transport tests under `web/tests/sceneAssets/`:

1. A static prop source naming `_LOD4` bakes to a five-mesh state whose fifth
   mesh holds only those meshes, and survives encode and decode.
2. The same source without `_LOD4` bakes to the bytes it bakes to today.
3. A skinned source, a kit module and a tree naming `_LOD4` are each refused.
4. A far mesh with no triangles, or with as many as tier 3, is refused.
5. An appearance with a far mesh in one state and not another is refused.
6. A budgeted kind's far mesh over its fifth entry is refused; a budgeted kind
   with a far mesh and only four entries is refused.

Then the real check: bake the whole catalog and confirm no runtime file's
content hash changed (`bun run --cwd web asset bake`, then `asset check`).

## Delegated

- Finding names and wording.
- The new constant's name.
- How the review sheet lays out a fifth tier.

## Must stay green

The scene-assets tests; `asset check` on the unchanged catalog; every runtime
file's hash.

## Feedback that would change this slice

- "Every kind should declare whether it carries one": add a required field on
  the scenery rule and bind it both ways. It was left out because most kinds
  have no rule of their own, and a far mesh that is authored but missing is
  already visible in the census.
