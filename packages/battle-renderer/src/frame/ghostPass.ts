// The placement ghosts (`ModelLayer.drawGhosts`): where the UI's units will
// stand, translucent over the fog-treated world, reading its depth and
// writing none. `battleFrame.ts` encodes them after the fog mask pass.
import { tgpu, type TgpuCommandEncoder } from "typegpu";
import { modelAttribs, modelVertex, type ModelLayer } from "../models/modelLayer";
import { createGhostFragment } from "../models/surfaceFragments";
import { battleWorldDepth } from "../worldDepth";
import type { EnvironmentFrame } from "./environmentFrame";
import type { CameraGroup } from "./geometry";
import { FRAME_MSAA, HDR_FORMAT, type FrameTargets } from "./targets";

type Root = ReturnType<typeof tgpu.initFromDevice>;

/** The ghosts' pipeline and pass, lit by `environment`. */
export function createGhostPass(root: Root, environment: EnvironmentFrame, models: ModelLayer) {
  const modelGhost = root
    .createRenderPipeline({
      attribs: modelAttribs,
      vertex: modelVertex,
      primitive: { topology: "triangle-list", cullMode: "none" },
      fragment: createGhostFragment(environment),
      targets: {
        format: HDR_FORMAT,
        blend: {
          color: { srcFactor: "src-alpha", dstFactor: "one-minus-src-alpha" },
          alpha: { srcFactor: "one", dstFactor: "one-minus-src-alpha" },
        },
      },
      depthStencil: battleWorldDepth("read"),
      multisample: { count: FRAME_MSAA },
    })
    .$name("modelGhost");
  return {
    pipeline: modelGhost,
    /** Placement meshes over the fog-treated world, without changing its mask or depth. */
    encode(encoder: TgpuCommandEncoder, targets: FrameTargets, cameraGroup: CameraGroup) {
      if (!models.hasGhostMeshes) return;
      const pass = encoder.beginRenderPass({
        label: "placement-ghosts",
        colorAttachments: [
          {
            view: targets.hdrMsaa.createView(),
            resolveTarget: targets.hdr.createView(),
            loadOp: "load",
            storeOp: "discard",
          },
        ],
        depthStencilAttachment: { view: targets.depth.createView(), depthReadOnly: true },
      });
      models.drawGhosts(
        modelGhost.with(pass).with(cameraGroup).with(environment.group) as unknown as Parameters<
          ModelLayer["drawGhosts"]
        >[0],
      );
      pass.end();
    },
  };
}
