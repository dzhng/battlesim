# C21: material transport

**Depends on:** G0. **Kind:** slice.

## Question
Can materials say opaque, cutout, blended or interior without overloading alpha?

## Contract it unlocks
`scene-assets` `Material` gains explicit coverage (`opaque | cutout{cutoff} | blended`) plus optional interior metadata. `scene.ts` keeps `alphaMode` (today it drops it, `:370-395`). The codec, validation and loader cut over together with a strict format bump; every bundle is rebaked. Albedo alpha stays wear and ORM alpha stays tint mask.

## API seam
`packages/scene-assets/src/{schema.ts,scene.ts,build.ts}`, the bundle loader.

## What the human can run or see
A material round-trip diagnostic.

## Verification
- Round-trip tests.
- Unsupported combinations fail.
- Every existing bundle rebakes byte-stable.

## Delegated to the implementer
The packed representation. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Three texture slots; ≤1K; emissive zero; no old-format reader.

## Feedback that would change this slice
Lost tint/wear/coverage channels reopen material transport parity; source texture repainting cannot hide transport loss.

## Outcome

Yes. A material now states its coverage (`opaque`, `cutout` with a cutoff, `blended`) and, optionally, the interior atlas sheet a room surface shows. The principle is in the [scene-assets readme](../../../packages/scene-assets/README.md) ("Coverage and rooms"), the types in `schema.ts`, the rules in `material.ts`, and the decisions in [choices](../choices.md#buildings-lane).

- **No alpha is overloaded.** The coverage value is the base colour's alpha times the normal texture's alpha, the two alphas nothing used. Albedo alpha is still the wear threshold, ORM alpha the tint mask, vertex-colour alpha how worn.
- **The source says it in standard glTF** (`alphaMode`, `alphaCutoff`, and `interior` in the material's extras), and the Blender material helpers write all three from two arguments, `coverage` and `interior`. No model uses them yet.
- **Bundle format 4, and only 4 is read.** A format 3 bundle, and a format 4 bundle whose material states no coverage, fail the whole load.
- **Refused by name:** `material.coverage` (an alpha mode or a cutoff that cannot be read, a cutoff outside 0..1), `material.coverage_source` (a cutout or blended material whose coverage never crosses its threshold), `material.wear` (blended and worn), `material.interior` (a room that is not opaque, has textures or wear of its own, is on a skinned or articulated body, or names a sheet the atlas lacks).
- **Nothing drawn changed.** All 300 materials of the 37 shipped appearances are `opaque`. Every one of the 39 rebaked bundles, with its coverage stripped and encoded as format 3, hashes to the bundle it replaced, so the rebake added the coverage statement and nothing else; the rows the renderer builds from them are the same. A second bake wrote the same 41 files.
- **On screen:** the `workbench` scene passes on the rebaked runtime. Its frame of a shipped bundle installed through the loader (`bundle-grass_meadow.png`) and its ten frames and sheets of bundles baked in the page are pixel-identical to the same scene run on the commit before. The other frames differ only in the panel's "validated in N ms" text and a few dozen edge pixels, as two runs of the same commit do. `village` was not run.
- **The diagnostic** is `asset validate <glb>`, which prints each material as it would be baked.

For C24 to C26: the renderer does not read `coverage` or `interior` yet. A recipe's coverage image is `textures.Baked(coverage=…)`. `parts.export(worn=…)` is per file, so a blended material that samples a recipe must be exported unworn.
