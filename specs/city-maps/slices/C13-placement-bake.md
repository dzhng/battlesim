# C13: offline template source export

**Depends on:** C00, C11/C12; S2/S5/G0 verdicts. **Kind:** slice.

## Question
Can one legal source recipe export shared modules, physical descriptor data and placement rows independently of a map?

## Contract it unlocks
Pinned headless Blender runs offline on `TemplateRecipe` and exports shared module sources, C00 descriptors and local placement rows. Recipe identity includes graph/script, patches, Blender version, inputs and source materials. Read instances before realization; no per-building GLB, per-map bake or runtime Blender.

Start with one accepted regional source. C16–C19 add missing-category recipes independently; their complete coverage is C54's release gate. For developer workbench checkpoints, generate clearly labelled asymmetric massing rows from frozen descriptors through this same source format. These are marked `prototype`; they neither prove source quality nor satisfy release or friend-playtest art acceptance. Remove the temporary massing source when C54 proves all selectable cells have release art.

Descriptor geometry and facade bays follow C00. A recipe that changes dimensions publishes a new physical catalogue identity; a material/LOD/placement change only changes appearance inputs. Exported source files are Git LFS, like every appearance source. Blender is an explicit source-generation stage and never runs inside deterministic asset bake/check.

## API seam
Offline source recipe → shared modules + physical descriptor + local placement rows. C32 owns deterministic packing, fit validation and runtime appearance resolution. These exports feed the existing asset-source lifecycle; no historical hash-directory retention subsystem is added.

## What the human can run or see
One recipe's export log, local-frame fit overlay and cold/warm recipe-cache report. The generated map does not participate in export.

## Verification
- Two exports from frozen inputs agree; warm cache reuses the source result.
- Modules remain shared, descriptors preserve units/bays/floors and placements fit them.
- Prototype rows carry their status; missing regional/category source is explicit.
- Compare template assembly/fit against matched S2 source renders; materials and final class coverage are out of scope. Run compare-screenshots, unprimed screenshot-critique last and preview-shots non-blocking.

## Delegated to the implementer
Export taps and cache grouping within G0's byte/fit limits. Source eligibility, catalogue dimensions and runtime graph evaluation are not delegated.

## Must stay green
Instancing, physical/art fit and no per-map Blender dependency.

## Feedback that would change this slice
An unsupported legal join or bay pattern changes the source recipe before selection consumes it.
