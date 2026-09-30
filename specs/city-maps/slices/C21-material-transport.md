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
