# Slice 02 — Renderer quality inputs and atomic rebuild

## Contract unlocked

Changing the effective profile while a battle is paused builds a complete replacement renderer generation, keeps the old generation alive until the replacement is ready, then swaps atomically and disposes the old generation.

## API seam

Thread one resolved profile from `BattleView`/`MenuBackdrop` into `LabViewport` and the existing `createBattleFrame`/presentation owners. Adapt model, building, tree/dressing, shadow, and grass cost at their existing owners. Add the grass multiplier at the grass population/window seam; preserve plot/verge/road/forest ownership and deterministic clump identity. Keep bloom, cast lights, and authored mood unchanged. Do not mutate fixture or biome JSON.

Expose a rebuild transaction with generation/cancellation protection. A failed replacement leaves the current frame intact when one exists; a required startup failure is surfaced to the fatal boundary. The simulation client stays paused for the whole transaction.

## Verification

Write unit tests for atomic swap, failed build, cancellation, disposal, and bounded GPU allocation accounting. Run a browser battle scene that changes profile while paused and proves camera/feed continuity, no duplicate RAF, and unchanged simulation digest/replay. Run grass growth/mix scenes at all tiers; inspect field lower-third coverage and plot/verge/road transitions.

For captures, run `compare-screenshots` against High where the candidate changes detail, then `screenshot-critique` last, and show the shots with `preview-shots`.

## Delegated freedom

Exact internal generation-token names and adapter plumbing are delegated. No freedom to alter simulation, fog authority, mood passes, CSS size, or the render-scale contract.
