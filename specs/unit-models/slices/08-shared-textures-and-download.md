# 08 Shared textures and a download limit

**Unlocks:** detailed models without a multi-gigabyte page load, and a test
that catches growth before a player does.

## What is wrong (measured 2026-10-06)

- `assets/runtime/` holds 215 raw bundles, 1.34 GB. Every appearance not
  fetched on request (`packages/scene-assets/src/schema.ts:116`: all unit and
  most scenery bundles) loads with the catalog on every page
  (`loader.ts:130-134`, `appResources.ts` `warm()`), uncompressed.
- Unit bundles carry 765 MB of texture bytes but only 51 distinct textures
  (17.8 MB): each bundle embeds its own copy (`texture.ts`); only the GPU
  shares them by address (`modelTextures.ts:20-29`).
- The only byte gates cover the template library (`loader.ts:117`), map
  downloads (`gzip.ts` `downloadBytes`) and court pieces
  (`downloadBudget.test.ts`). Nothing counts the catalog load.
- Texture arrays are sized to the largest texture installed
  (`modelTextures.ts:58`), and the device requests no `maxTextureArrayLayers`
  (`device.ts:40`), so the cap is WebGPU's default 256; normal+ORM already
  holds 234 distinct layers. Over the cap, the page is refused.

## Contract

- **A texture is its own content-addressed runtime file**, named in the
  runtime catalog; a bundle references textures by address and never embeds
  them. The loader fetches each texture once, however many bundles use it.
- **Unit and scenery bundles travel as gzip** like kits (`gzip.ts`), with
  their original content hashes kept as art identity.
- **Catalog-load bytes have a limit**, a constant in `schema.ts` beside
  `MAP_DOWNLOAD_MAX_BYTES`: what every page downloads before its first frame
  (catalog bundles, their textures, skeleton clips, the template library). A
  test fails above it; `asset check` prints the total. Set the limit from the
  measured total after this slice with room for Part B's growth, recorded in
  choices.md.
- **The device requests the adapter's `maxTextureArrayLayers`**, and
  `asset check` counts distinct texture layers per array against the lower of
  that and a recorded floor for the target machine (the Mac mini), so a new
  recipe can't silently push a page over.
- **Recipe texture size stays a per-recipe choice** (`textures.py` `SIZE` is
  256 for all today); raising one is a budget decision recorded in choices.md,
  never a global change.

## Work

1. Tests first: the catalog-load byte test and the layer-count check, red at
   today's 1.34 GB / unknown layer limit.
2. Codec and bake: textures out of bundles, content-addressed; loader fetches
   by address; renderer unchanged (it already keys by address).
3. Gzip transport for unit and scenery bundles; native readers (icons, fit)
   read the original bytes through the same `gzip.ts` owner.
4. Request the adapter limit at device admission (`renderer-core` device).

## Verify

The two tests; `scene-assets` and `battle-renderer` unit tests; one browser
scene that loads a battle (the narrowest that draws units and kits). Bundle
art hashes unchanged: only transport moved.

## Stays green

Every bundle's original content hash; map hashes; replay identity.
