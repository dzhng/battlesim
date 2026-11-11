# 16: integration re-export and bake (coordinator)

## Contract

Every tracked family, and every sibling a shared change touched, is
re-exported once through slices 08–15:
- the receipts are fresh;
- the runtime is baked on the merged tree;
- `bun run check` is green;
- the milestone is pushed.

The family lanes branch from this commit.

## Steps

1. Merge the experiment, jet and survey lanes that have finished, following the
   merge rules in `../parallel.md`.
2. **Re-export** every tracked script, live and `--wreck`, plus `s300`, in the
   background. Also re-export every wheeled script that calls `bolted_panel`
   or `armour_tiles`, because slice 11's bevel floor reaches them (C19).
   Today that's `acv`, `boxer`, `btr`, `caesar`, `centauro`, `fennek`,
   `iris_t_slm`, `jaguar`, `lav`, `patriot`, `s300`, `s350`, `stryker`,
   `stryker_support`, `truck_chassis`, `vbci` and `zbl08`. Grep again at the
   time.
3. **Lint:** `--roster` must be clean against the baseline. Changed frames may
   leave fit edits for their family slices only if `asset validate` passes.
4. **Bake:** `bun run --cwd web asset -- bake`, then
   `bun run --cwd web asset -- check`. Read the catalog-load line.
   - Bevels raise the load and the crew removal lowers it.
   - If it exceeds the tripwire, raise `CATALOG_LOAD_MAX_BYTES`
     (`packages/scene-assets/src/schema.ts:772-773`) to the measured load plus
     room, as decision Q3 allows. Record the new value.
5. **Checks:** `bun run check`, then `cargo test` for the digest-bearing suites
   (re-run after slice 15).
6. **Montage:** one closeup montage of all 37 in `assets/after-shared/`, shown
   with `preview-shots`. The checkpoint never blocks.
7. **Push milestone.**

## Verification

As in the steps. The visual check here is the montage only, since every
variable was judged in its own slice.

## Delegated

The batch order.
