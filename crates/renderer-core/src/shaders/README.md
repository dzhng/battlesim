# Shader composition

WGSL modules own the rendering rules. The [composition index](pipeline_sources.txt)
selects ordered named blocks for each pipeline; the core build writes expanded
sources only into Cargo's output directory. The typed Rust catalog supplies
pipeline state. Neither platform carries shader text or descriptor policy.

One algorithm has one authored body, even when its callers bind differently
named uniforms. A block reference may supply identifier aliases in brackets,
separated by semicolons: `block[from=to;other=target]`. Substitution replaces
whole identifier tokens once, so aliases can rename colliding locals without
changing expressions, operation order or unrelated names. Every alias must name
a token present in its block; a replacement never feeds another alias.

A binding declaration shared across groups uses `@group(GROUP)` in its body.
Its reference supplies the group as `block@N`, or `block@N[from=to]` when it
also needs aliases; `N` is a nonnegative `u32`. The build substitutes the group
attribute without changing binding numbers, access modes or identifiers.
Declarations that differ only by group share one block. Different layouts'
binding numbers, access modes or resource types remain separate declarations,
named for a layout that uses them; another layout may reuse the same
declaration when its binding agrees. Aliases do not select a different algorithm.

The build refuses duplicate block ids, blocks no pipeline references, and
aliases whose source token is absent. A block containing the identifier
`GROUP` requires `@N`; a block without it refuses `@N`. Other identifiers that
contain those letters, such as workgroup-size constants, are ordinary tokens.

A built-in shader's source starts with the presentation's
[typed constant table](../../../presentation/src/shader_constants.rs). Each
value belongs to the CPU concept that bounds, allocates or clocks it; the
prelude reads the whole table; generated TypeScript exports the browser's chosen
subset. Integer constants remain integers so workgroup sizes and array lengths
share the CPU's owner.

GPU-only tuning stays as named constants beside its composed function. Keep
the original literal spelling and inferred type: an abstract WGSL constant
must not acquire an f32 rounding step before the original expression did.

## Concept ownership

Add a helper to its concept's owner and name the dependency in the composition
index. Ownership follows the rule or resource a block serves, even when another
concept also consumes it. Pure arithmetic can stay with a domain when it encodes
that domain's policy; `math` holds only functions without bound resources.

| Concept | Module | Ownership principle |
| --- | --- | --- |
| Camera and projection | `camera` | View records, rays, wind access and clip-space reconstruction. |
| Sight and visibility | `fog` | Sight records, horizon maps, visibility queries, culling scratch and color/fog outputs. |
| Screen fog | `fog_masks` | Mask views, display treatment, distance transforms and rim composition. |
| Surface and atmospheric lighting | `lighting` | Shared light records, PBR shading and aerial perspective. |
| Sky and environment lookup | `sky` | Sky generation, environment filtering, lookup coordinates and environment/material samplers. |
| Directional shadows | `shadow` | Cascade records, receiver resources and cascade blending. |
| Ground configuration | `ground_params` | Ground appearance records and their uniform binding. |
| Ground lookup | `ground_surface` | Height pages, surface-field decoding, plot queries and cubic sampling. Height adapters stay here even when their callers are fog or grass. |
| Ground appearance | `ground_material` | Field color, forest floor, verges, banks and material-class visualization. |
| Road surfacing | `ground_roads` | Paving, shoulders, markings, joints and road variation. Separate from field/forest appearance to keep each module within the source budget. |
| Water | `water` | Water appearance, normals, wakes, bed sampling and guarded scene mips. |
| Grass | `grass` | Clumps, blade construction and placement resources, including prop exclusion. |
| Ground scars | `scars` | Scar cache sampling, relief, material changes and view-dependent scar drawing. |
| Scenery | `scenery` | Placement, wind sway, leaf clumps and foliage surface shading. |
| Paint and effects | `effects` | Ground-paint sampling and treatment, sprite lighting and overlay glow. |
| Display processing | `post` | Grading, bloom, blur and sampled color inputs shared with other pipelines. |
| Resource-free arithmetic | `math` | Noise, geometric predicates, interpolation and color conversion without global resources. |
| Model geometry | `models_geometry` | Articulation, card transforms and x-ray coverage resources. |
| Model materials | `models_material` | Authored material sampling, cutout coverage and room appearance. |
| Effect and paint stages | `passes_effects` | Entry points and their stage IO. |
| Visibility and world fragments | `passes_fog` | Fog compute and world-material entry points and their fragment inputs. |
| Model stages | `passes_models` | Model entry points and their stage IO. |
| Scenery stages | `passes_scenery` | Scenery and grass vertex entry points and their stage IO. |
| Reused stages | `passes_shared` | Full-screen and shared mesh entry points and their reusable stage IO. |
| Terrain and sprite stages | `passes_terrain` | Sprite, background and water-processing entry points and their stage IO. |

Stage IO follows its entry-point family; reusable stage IO stays in
`passes_shared`. Identical contracts share a struct, preserving every field
name, location and interpolation attribute. The color/fog output belongs to
`fog`, since it defines the visibility contract shared by world materials.
Algorithm records without stage attributes stay with their concepts.

## Validation

The core's reflection tests validate assembled WGSL against canonical
presentation layouts, while real GPU
scenes judge its output and lifetime. The pipeline registry's unit test also
checks that every bound global has a matching group, binding and resource kind
in that pipeline's layouts. Layouts may expose entries a particular shader
doesn't use. WGSL distinguishes comparison samplers and texture scalar kinds,
dimensions and multisampling; filtering support remains a layout/device choice.
