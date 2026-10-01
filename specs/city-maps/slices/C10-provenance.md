# C10: provenance

**Depends on:** G0. **Kind:** slice.

## Question
Can the manifest record the vendored sources honestly?

## Contract it unlocks
- `reuse-manifest.json` `files[]` entries name their `source_repo`, revision, licence and kept notice (L10). The existing `../game` records are rewritten in place (hard cutover).
- The three `.blend` files go in Git LFS under `packages/scene-assets/blender/vendor/procedural-buildings/`, as source of truth (Q-F′).
- ambientCG sets at ≤1K go under `packs/` with `source_url`.
- A test **refuses** the two unknown-source atlases by hash.

## API seam
`reuse-manifest.json`, `web/tests/reuseManifest.test.ts`, `packages/scene-assets/src/schema.ts`.

## What the human can run or see
`asset check` and the manifest test pass.

## Verification
- Red/green: a file with no licence fails, and either atlas hash fails.
- Art and map-data allow-lists stay separate.

## Delegated to the implementer
Manifest field names; pack cache layout. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Every existing manifest entry.

## Feedback that would change this slice
An unacceptable licence or untraceable source changes the source choice before its art is baked.
