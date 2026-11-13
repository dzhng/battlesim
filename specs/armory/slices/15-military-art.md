# 15 · Military art: one prop per gate [held: L1]

Status: done 2026-11-13, commits f41e1fb1b (15a, 15b), 3196df08b (split), 140dfef0e (15c), 573bf60be (15d), 10facb466 (15e), 7118e5c24 (15f).

Map rows: Q2, L2, L3. Depends on 14.

## Contract

Each military row from slice 14 gets procedural art and leaves `systems_only`:
1. a Blender source;
2. an export;
3. an `assets/catalog.json` entry;
4. a bake into `assets/runtime/`.

**One prop per gate:** each sub-slice below has one visual variable and its own
verdict. Nothing new is added to the asset pipeline: there is no new
workbench, and the existing `/labs/workbench` inspects one prop at a time (P6).

Run `make-3d-models` and `renderer` before starting. Gather each prop's
references into `assets/references/`, with `references.json` recording the
source URL, the licence or provenance, and whether the image is a photo or
generated. Never judge a prop from a generated image alone.

## Seam

- **Sources:** concept-named scripts under `packages/scene-assets/blender/`.
  Courts are the precedent: `blender/courts.py`, `build_sources.sh`,
  `scenery.ts`, then `assets/catalog.json`.
  - Axes and units: Z up, +X forward, metres, origin on the ground.
- **The footprint must match.** `catalogFootprints.test.ts` requires each art
  box to equal a box the simulation places.
- **Bake and check.** Run `bun run --cwd web asset bake`, then `asset check`.
  Never hand-edit runtime output.
- **Dropping `systems_only`** on a row re-blesses `fixtures/catalog.json`
  (precedent `48f64a64d`).

## Sub-slices

| # | Prop | The one visual variable | Crop | References to gather |
|---|---|---|---|---|
| 15a | `hangar` | standing silhouette and corrugation | full footprint, front quarter, ground line | Quonset front quarter and frame joints |
| 15b | `hangar_ruin` | destroyed-state readability | the ruin's edges and rubble, which still give cover | collapsed hangar or warehouse frames |
| 15c | `revetment` | crest and bay silhouette at vehicle scale | crest, opening and a vehicle beside it | earth revetment, from the air and the side |
| 15d | `hesco` | modular seams and fill | one module beside a soldier for scale | HESCO stack, side view |
| 15e | `tent`, `flagpole` | silhouette and how the flag attaches | pole from top to base, flag, tent and shadow | field tent row; yard flagpole |
| 15f | `supply_net` | how the net drapes over supplies | the net's edges, the stack contact, the clear standing space beside it | cargo net over stacked supplies |

**Each sub-slice runs in this order:**
1. The source exports twice, byte for byte.
2. The Blender lint passes: not floating, grounded, no intersections.
3. Inspect it alone in `/labs/workbench`.
4. `compare-screenshots` against its references.
5. `screenshot-critique` last.
6. Bake.
7. Hold the non-blocking `preview-shots` checkpoint: open the shots, wait about
   5 minutes, decide on the evidence, record it in `decisions.md`, close the
   shots and proceed.

## The human runs or sees

`/labs/workbench`, showing each military prop alone, standing and, for the
hangar, destroyed.

## Verification

Per sub-slice: the order above, plus a passing `asset check` and the
`catalogFootprints` tests.

After the last sub-slice:
- read the bake's load line against the 768 MiB tripwire (S5);
- the texture-layer count stays under 2048 (`schema.ts:779`).

## Delegated to the implementer

- Mesh decomposition and material slot names.
- Material recipes, including the fill texture and segment counts.
- Whether to batch sub-slices into one bake. Each still keeps its own verdict.
- The idle motion of cloth on tents and flags, in presentation only.

## Must stay green

- `asset check` and `asset validate`.
- The city and forest scenes.
- Every Armory test.

## Feedback that would change it

A prop that looks wrong goes back to its own sub-slice. A prop that reads as
cover or concealment its body doesn't have goes back to slice 14's row first,
then the art.
