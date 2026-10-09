// The impostor bake: an appearance in one pose (its far pose, or a corpse)
// rendered by our own renderer (decision "Impostors are baked by our own
// renderer in the model workbench") from a ring of yaws at a few battle
// pitches, into an atlas of unlit, untinted albedo with coverage, a matching
// atlas of model-space normals, and one of the side-tint mask, so the battle
// can relight and tint far soldiers as cards (`impostorCards.ts`). The
// workbench bakes for its sheets; the battle frame bakes its cards at install.
//
// Every cell shares one orthographic frame fitted to the appearance's
// bounding sphere, so a cell's pixels are the same size in metres from every
// side. Cells are rendered at a supersampled size and box-filtered on the
// CPU, which keeps the edge antialiasing deterministic: same inputs, same
// bytes, same hash.

import { tgpu } from "typegpu";
import { mat4, vec3, type Mat4, type Vec3 } from "math";
import { orthographicReverseZ } from "@packages/renderer-core/src/camera3d";
import {
  CAMERA_UNIFORM_FLOATS,
  packCameraUniform,
} from "@packages/renderer-core/src/cameraUniform";
import { GPU_DEPTH_CLEAR, GPU_DEPTH_FORMAT } from "@packages/renderer-core/src/depthContract";
import type { Bounds } from "@packages/scene-assets/src/schema";
import { Camera, typegpuCameraLayout } from "../world/camera";
import { battleWorldDepth } from "../worldDepth";
import type { EnvironmentFrame } from "../frame/environmentFrame";
import type { GpuRegistry } from "../frame/registry";
import { createModelFragments, modelAttribs, modelVertex, type ModelLayer } from "./modelLayer";
import type { ModelInstance, ModelPose } from "./modelInstances";

type Root = ReturnType<typeof tgpu.initFromDevice>;

export interface ImpostorSpec {
  /** Views round the model, the first looking at its front (+X). */
  yaws: number;
  /** Camera pitches above the horizon, radians, one atlas row each. */
  pitches: readonly number[];
  /** Cell size in atlas pixels. */
  cell: number;
  /** Render scale before the box filter. */
  supersample: number;
}

/** Eight headings at the battle camera's far pitch (0.85 rad, the pitch
 *  curve's plateau) and a lower mid-zoom pitch. */
export const IMPOSTOR_SPEC: ImpostorSpec = {
  yaws: 8,
  pitches: [0.85, 0.5],
  cell: 128,
  supersample: 2,
};

export interface ImpostorView {
  column: number;
  row: number;
  yaw: number;
  pitch: number;
  eye: Vec3;
  viewProj: Mat4;
}

export interface ImpostorFrame {
  center: Vec3;
  radius: number;
  views: ImpostorView[];
}

/** The views of one atlas: where each cell's camera stands and what it sees. */
export function impostorViews(spec: ImpostorSpec, bounds: Bounds): ImpostorFrame {
  const center = vec3.lerp(vec3.create(), bounds.min, bounds.max, 0.5);
  const radius = Math.max(0.01, vec3.distance(bounds.min, bounds.max) / 2);
  const distance = radius * 4;
  const views: ImpostorView[] = [];
  const view = mat4.create();
  const projection = orthographicReverseZ(
    mat4.create(),
    -radius,
    radius,
    radius,
    -radius,
    distance - radius,
    distance + radius,
  );
  spec.pitches.forEach((pitch, row) => {
    for (let column = 0; column < spec.yaws; column++) {
      const yaw = (2 * Math.PI * column) / spec.yaws;
      const toward: Vec3 = [
        Math.cos(pitch) * Math.cos(yaw),
        Math.cos(pitch) * Math.sin(yaw),
        Math.sin(pitch),
      ];
      const eye = vec3.scaleAndAdd(vec3.create(), center, toward, distance);
      mat4.lookAt(view, eye, center, [0, 0, 1]);
      views.push({
        column,
        row,
        yaw,
        pitch,
        eye,
        viewProj: mat4.multiply(mat4.create(), projection, view),
      });
    }
  });
  return { center, radius, views };
}

export interface ImpostorAtlas {
  appearance: string;
  spec: ImpostorSpec;
  width: number;
  height: number;
  center: Vec3;
  radius: number;
  /** RGBA8, display-encoded albedo, alpha = coverage. */
  albedo: Uint8Array;
  /** RGBA8, model-space normal mapped to 0..1, alpha = coverage. */
  normal: Uint8Array;
  /** RGBA8, red = the side-tint mask (how much of a side's tint the surface
   *  takes), alpha = coverage. Albedo is baked untinted. */
  mask: Uint8Array;
  /** sha256 of the metadata and every atlas. */
  hash: string;
}

/** Average `factor`² texels into one, weighting colour by coverage. */
export function boxFilter(
  source: Uint8Array,
  width: number,
  height: number,
  factor: number,
): Uint8Array {
  const w = width / factor;
  const h = height / factor;
  const out = new Uint8Array(w * h * 4);
  const n = factor * factor;
  for (let y = 0; y < h; y++)
    for (let x = 0; x < w; x++) {
      let r = 0;
      let g = 0;
      let b = 0;
      let a = 0;
      for (let dy = 0; dy < factor; dy++)
        for (let dx = 0; dx < factor; dx++) {
          const i = ((y * factor + dy) * width + x * factor + dx) * 4;
          const alpha = source[i + 3];
          r += source[i] * alpha;
          g += source[i + 1] * alpha;
          b += source[i + 2] * alpha;
          a += alpha;
        }
      const o = (y * w + x) * 4;
      if (a > 0) {
        out[o] = Math.round(r / a);
        out[o + 1] = Math.round(g / a);
        out[o + 2] = Math.round(b / a);
      }
      out[o + 3] = Math.round(a / n);
    }
  return out;
}

const ATLAS_FORMAT = "rgba8unorm" as const;

export function createImpostorBaker(
  root: Root,
  registry: GpuRegistry,
  models: ModelLayer,
  environment: EnvironmentFrame,
) {
  const device = root.device;
  const fragments = createModelFragments(environment);
  const pipeline = root.createRenderPipeline({
    attribs: modelAttribs,
    vertex: modelVertex,
    fragment: fragments.impostor,
    targets: {
      albedo: { format: ATLAS_FORMAT },
      normal: { format: ATLAS_FORMAT },
      mask: { format: ATLAS_FORMAT },
    },
    primitive: { topology: "triangle-list", cullMode: "none" },
    depthStencil: battleWorldDepth("read-write"),
  });

  return {
    /** Bake `appearance` in `pose` (its far pose) into an atlas. */
    async bake(
      appearance: string,
      pose: ModelPose,
      bounds: Bounds,
      spec: ImpostorSpec = IMPOSTOR_SPEC,
    ): Promise<ImpostorAtlas> {
      await pipeline.initAsync();
      const frame = impostorViews(spec, bounds);
      const px = spec.cell * spec.supersample;
      const width = spec.yaws * px;
      const height = spec.pitches.length * px;
      const scope = registry.scope();
      try {
        const usage = 0x10 | 0x01; // RENDER_ATTACHMENT | COPY_SRC
        const albedo = scope.texture({
          label: "impostor-albedo",
          size: [width, height],
          format: ATLAS_FORMAT,
          usage,
        });
        const normal = scope.texture({
          label: "impostor-normal",
          size: [width, height],
          format: ATLAS_FORMAT,
          usage,
        });
        const mask = scope.texture({
          label: "impostor-mask",
          size: [width, height],
          format: ATLAS_FORMAT,
          usage,
        });
        const depth = scope.texture({
          label: "impostor-depth",
          size: [width, height],
          format: GPU_DEPTH_FORMAT,
          usage: 0x10,
        });
        const cameras = frame.views.map((view) => {
          const data = packCameraUniform(
            new Float32Array(CAMERA_UNIFORM_FLOATS),
            view.viewProj,
            mat4.invert(mat4.create(), view.viewProj) ?? mat4.create(),
            view.eye,
            0,
            0,
            0,
            0,
          );
          const buffer = scope.own(root.createBuffer(Camera).$usage("uniform"));
          buffer.write(data.buffer);
          return root.createBindGroup(typegpuCameraLayout, { cam: buffer });
        });
        // Pose the one model at the origin, facing +X; the frame packs its own
        // models again at its next prepare.
        const instance: ModelInstance = { appearance, x: 0, y: 0, z: 0, yaw: 0, pose, tier: 0 };
        models.packExact([instance]);
        const encoder = device.createCommandEncoder({ label: "impostor-bake" });
        models.encodePose(encoder);
        const pass = encoder.beginRenderPass({
          label: "impostor-atlas",
          colorAttachments: [
            {
              view: albedo.createView(),
              loadOp: "clear",
              storeOp: "store",
              clearValue: [0, 0, 0, 0],
            },
            {
              view: normal.createView(),
              loadOp: "clear",
              storeOp: "store",
              clearValue: [0, 0, 0, 0],
            },
            {
              view: mask.createView(),
              loadOp: "clear",
              storeOp: "store",
              clearValue: [0, 0, 0, 0],
            },
          ],
          depthStencilAttachment: {
            view: depth.createView(),
            depthClearValue: GPU_DEPTH_CLEAR,
            depthLoadOp: "clear",
            depthStoreOp: "discard",
          },
        });
        frame.views.forEach((view, i) => {
          pass.setViewport(view.column * px, view.row * px, px, px, 0, 1);
          pass.setScissorRect(view.column * px, view.row * px, px, px);
          // A card carries a body's opaque surfaces: no unit has another kind.
          models.draw(
            pipeline.with(pass).with(cameras[i]) as unknown as Parameters<ModelLayer["draw"]>[0],
            "opaque",
          );
        });
        pass.end();
        const rowBytes = width * 4;
        const reads = [albedo, normal, mask].map((texture) => {
          const read = scope.buffer({
            label: "impostor-read",
            size: rowBytes * height,
            usage: 0x01 | 0x08,
          });
          encoder.copyTextureToBuffer({ texture }, { buffer: read, bytesPerRow: rowBytes }, [
            width,
            height,
          ]);
          return read;
        });
        device.queue.submit([encoder.finish()]);
        const [albedoBytes, normalBytes, maskBytes] = await Promise.all(
          reads.map(async (read) => {
            await read.mapAsync(1);
            const bytes = new Uint8Array(read.getMappedRange().slice(0));
            read.unmap();
            return boxFilter(bytes, width, height, spec.supersample);
          }),
        );
        const meta = new TextEncoder().encode(
          JSON.stringify({ appearance, spec, center: frame.center, radius: frame.radius }),
        );
        const hashed = new Uint8Array(
          meta.length + albedoBytes.length + normalBytes.length + maskBytes.length,
        );
        hashed.set(meta, 0);
        hashed.set(albedoBytes, meta.length);
        hashed.set(normalBytes, meta.length + albedoBytes.length);
        hashed.set(maskBytes, meta.length + albedoBytes.length + normalBytes.length);
        const digest = new Uint8Array(await crypto.subtle.digest("SHA-256", hashed));
        return {
          appearance,
          spec,
          width: width / spec.supersample,
          height: height / spec.supersample,
          center: frame.center,
          radius: frame.radius,
          albedo: albedoBytes,
          normal: normalBytes,
          mask: maskBytes,
          hash: Array.from(digest, (b) => b.toString(16).padStart(2, "0")).join(""),
        };
      } finally {
        scope.release();
      }
    },
  };
}
