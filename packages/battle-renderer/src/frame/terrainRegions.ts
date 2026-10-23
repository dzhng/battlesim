import type { tgpu, TgpuRenderPass } from "typegpu";
import type { FrameTargets } from "./targets";
import type { ScarRegion } from "./scarTexture";

/** Submit each visible scar region before the shared atlas is rewritten. */
export function drawTerrainRegions(
  root: ReturnType<typeof tgpu.initFromDevice>,
  targets: FrameTargets,
  regions: readonly ScarRegion[],
  scissor: (
    region: ScarRegion,
    width: number,
    height: number,
  ) => readonly [number, number, number, number] | null,
  prepare: (region: ScarRegion) => void,
  draw: (pass: TgpuRenderPass) => void,
) {
  const colorView = targets.hdrMsaa.createView();
  const litView = targets.lit.createView();
  let first = true;
  for (const region of regions) {
    const rect = scissor(region, targets.width, targets.height);
    if (!rect) continue;
    prepare(region);
    const batch = root["~unstable"].createCommandEncoder({ label: "ground-region" });
    const groundPass = batch.beginRenderPass({
      label: "ground-region",
      colorAttachments: [
        { view: colorView, resolveTarget: litView, loadOp: "load", storeOp: "store" },
        {
          view: targets.fogMaskMsaa.createView(),
          resolveTarget: targets.fogMask.createView(),
          loadOp: first ? "clear" : "load",
          storeOp: "store",
          clearValue: [0, 0, 0, 0],
        },
      ],
      depthStencilAttachment: {
        view: targets.depth.createView(),
        depthLoadOp: "load",
        depthStoreOp: "store",
      },
    });
    groundPass.setScissorRect(...rect);
    draw(groundPass);
    groundPass.end();
    batch.submit();
    first = false;
  }
  return first;
}
