# 06 · Move each struct and function to its concept's module

**Contract.** Each shared struct and function lives in the module named for its
concept, and `shaders/README.md` maps concept to module. This slice only moves
blocks: no renames, no merged copies, no change to the order inside a pipeline.

## Target modules

`shared_bindings.wgsl` goes: nothing owns "shared". `passes_*` keep entry
points and their own input/output structs.

| Module | Owns |
|---|---|
| `camera` | `Camera`, `Wind`, the camera binding, clip and projection helpers |
| `fog` | fog params, layer, eye and box records and bindings; unpacking; seen-by tests; the fog output struct |
| `fog_masks` | `FogLook`, mask views, fog composition, distance rows and columns |
| `lighting` | `Environment`, `Sun`, `Aerial`, cast lights, surface light, aerial perspective |
| `sky` | `SkyLight`, sky/prefiltered-environment/BRDF lookup bindings and helpers |
| `shadow` | `SunCascade`, `SunShadow`, sun depth bindings, the cascade blend |
| `ground_params` | `TerrainParams`, `RoadLook`, `ShoreTuning` |
| `ground_surface` | height pages and the height lookup, surface field, plots |
| `ground_material` | the ground rules now in `models_material.wgsl:85-160`, forest floor and verges |
| `water` | `WaterLook`, `WaveRun`, `WaveSplat`, water normals |
| `grass` | `GrassParams`, clumps |
| `scars` (was `terrain_scars`) | `ScarParams` and scar drawing |
| `scenery` | sway, placement, leaf clumps |
| `effects` | paint and `GroundPaintStyle`, effect sprites |
| `post` | `Bloom`, `Grade`, blur kernels |
| `math` | pure functions only |

WGSL counts against the repo's file-size budget (8,000 tokens; ratchet-files),
which is why the ground code splits three ways.

## Verify

- The assembled-shader dump is byte-identical to base. No plates needed.
- `bun .agents/skills/ratchet-files/scripts/check-file-size.mjs` passes.

**Delegated:** a module name, or which of two listed modules owns a borderline
helper, when the table doesn't settle it; record the call in `shaders/README.md`.
**Stays green:** every assembled shader, byte for byte.
