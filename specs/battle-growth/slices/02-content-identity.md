# 02 Content identity

**Question:** does the catalog's content hash, not a name, reach every place
that will decide reuse?

**Depends on:** 00.

## Contract

Names route; content identifies. The same name with new content is different
art. The same content under two names is the same art.

## Seam

- `ContentId`: the runtime catalog's bundle hash, with the skeleton's hash for
  skinned art. Generated art (the box proxies) is hashed once, where it is
  made.
- TypeScript: `InstalledAppearance`
  ([loader.ts](../../../packages/scene-assets/src/loader.ts)) and `ModelAsset`
  ([modelAssets.ts](../../../web/src/battle/render/modelAssets.ts)) carry it.
- Rust: `add_bundle`
  ([appearance.rs](../../../crates/renderer-browser/src/appearance.rs)) takes
  it, and [`Library`](../../../crates/renderer-core/src/bundle.rs) maps a name
  to its content and the decoded bundle. A bundle whose content the library
  already holds is not decoded again.
- The native client takes the identity from the runtime catalog it already
  reads.

**Invariant:** no reuse decision is keyed by a name or a row number.

## Verification

Write first, in `crates/renderer-core/tests/asset_generations.rs`:

1. The same name with new content is a miss and replaces the old art.
2. Identical content under a second name is a hit: nothing is decoded.

`tests/bundle_memory.rs` still holds: a draft cannot restore stale sources.

## What you can run or see

Nothing changes on screen.

## Delegated

The identity type's layout, and whether it reuses an identity type from
[`crates/contract`](../../../crates/contract/README.md).

## Stays green

The standing gates in the [README](../README.md).

## Feedback that would change this slice

If some art reaches Rust with no catalog hash (a tooling path that builds
bundles in the page), hash it once where it is made; do not fall back to its
name.
