# 21b — Textured appearances: clear the "toy-like" gate

**Status:** done (2026-09-26; gate verdicts and escalations in [`choices.md`](../choices.md#slice-21b)). **Depends on:** 21, 22 (both merged with the gate open); lands after 23 and 24 so the models layer is settled. **Lane:** asset.

## Contract

The user's decision (2026-09-25): models get a real modelling budget, and the unprimed critique is a pass/fail gate. Slices 21 and 22 shipped the rigs, clips, parts and parity, but the gate failed for the soldiers, tank, truck, wrecks and small props. The houses passed. In every round the critic's top asks were surface detail: printed camouflage, fabric weave and folds, a distinct look for cloth, nylon, rubber and metal, chips, mud and markings. Bundles carry only vertex colour, spaced about 0.28 m apart on vehicles, so none of that can be drawn. This slice adds baked textures to the one appearance pipeline and re-surfaces the failing art until the gate passes.

## API seam

- **Bundle format version 3** (`packages/scene-assets`): per material, optional baked textures.
  - albedo (sRGB);
  - a tangent-space normal map;
  - ORM (occlusion, roughness, metalness);
  - the side-tint mask, moved from vertex or material to a texture channel where the art needs it.

  Textures are content-addressed inside the bundle like other views, use a GPU-ready compressed format where WebGPU on this Mac supports it (else RGBA8 with mips), and are shared across LOD tiers. The validator gains texture findings: size, missing mips, a tangent basis present when a normal map is.
- **Bake.** The Blender scripts in `packages/scene-assets/blender/` bake textures deterministically from procedural materials: camouflage print, weave, wear and grime, markings. Every source stays `project-owned` or CC0 with a manifest entry.
- **Renderer.** The models layer, including the pose-kernel skinned path, articulated vehicles and static scenery, samples the textures through one material function with a tangent frame, within the bind-group and storage limits. Impostor bakes include them. Slice 19's trees and slice 18's grass may adopt them, but don't have to.
- **Workbench:** texture preview, and a toggle per channel.
- **Memory:** record texture MiB per appearance and in total. Keep the battle's texture growth within a budget this slice sets and records.

## Verification

- Format round-trip and determinism tests: the same source gives the same hash. There is a golden failure for each new texture finding.
- **Gate:** an unprimed screenshot-critique per appearance (rifle, recon, AT, tank, truck, both wrecks, wall, crate, bridge deck), each at its close view and battle views. It must not call any of them toy-like. Iterate the art until it passes, or escalate one specific, named remaining item with evidence.
- compare-screenshots against the named WARNO and Broken Arrow crops, with better edge energy and material contrast than the slice 21 and 22 sheets.
- The benchmark short run, plus a 100-a-side stress run, recording frame cost and texture MiB.

## Decision budget

- **Delegated:** texture resolutions per tier, the compression format, and the bake recipes.
- Anything else this slice has to invent is a spec gap. Record it in [`choices.md`](../choices.md) and resolve it here; don't improvise.

## Must stay green

Replay and digest parity. Spike 03 parity rows. Every existing scene and test. `bun run check` and `bun run verify` at closeout.
