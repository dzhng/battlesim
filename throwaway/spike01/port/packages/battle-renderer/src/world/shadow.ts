import type { Camera3DParams } from "../../../renderer-core/src/camera3d";
import { tgpu, d, type TgpuCommandEncoder, type TgpuRenderPass } from "typegpu";
import {
  SUN_SHADOW_BLOCK_FLOATS,
  shadowPcfWgsl,
  shadowVisibilityWgsl,
  sunShadowSampleWgsl,
} from "../shaders/shadow";
import { nativeGpuScope } from "../gpuScope";
import { BATTLE_DEPTH_ATTACHMENT } from "../worldDepth";
import { Camera, typegpuCameraLayout } from "./camera";
import {
  NativeShadowFrame,
  type NativeShadowData,
  type NativeShadowMode,
  type ShadowTuning,
} from "../shadowData";
import { CSM_CASCADES } from "../../../game-renderer/src/battle/shadowPolicy";
import type { CivsimEnvironment } from "../../../game-renderer/src/environment/environment";

/** One cascade's receiver record, as a typed schema: the shared 96-byte layout
 * of `SunCascade` expressed once for TypeGPU instead of a WGSL struct string. */
export const SunCascade = d.struct({
  matrix: d.mat4x4f,
  bias: d.vec4f,
  interval: d.vec4f,
});
/** The world's fixed receiver block — every shadow receiver's only uniform, in
 * both modes. The record count is the cascade policy's, so the typed schema and
 * the shared WGSL block describe the same 208 bytes. */
export const SunShadow = d.struct({
  cascades: d.arrayOf(SunCascade, CSM_CASCADES),
  control: d.vec4f,
});

/** The receiver binding shape, owned once. Both the world environment group and
 * the lab shadow control spread these entries, so the depth dimension and the
 * block type cannot drift apart between them. */
export const sunShadowEntries = {
  sun: { uniform: SunShadow, visibility: ["fragment"] },
  sunDepth: { texture: d.textureDepth2dArray(), visibility: ["fragment"] },
  sunCompare: { sampler: "comparison", visibility: ["fragment"] },
} satisfies Parameters<typeof tgpu.bindGroupLayout>[0];

export const sunSamplingLayout = tgpu.bindGroupLayout(sunShadowEntries);

const shadowPcf = tgpu.fn(
  [d.textureDepth2dArray(), d.comparisonSampler(), d.i32, d.vec2f, d.f32, d.vec2f, d.f32],
  d.f32,
)(shadowPcfWgsl());
export const shadowVisibility = tgpu
  .fn(
    [
      d.textureDepth2dArray(),
      d.comparisonSampler(),
      d.i32,
      d.mat4x4f,
      d.vec4f,
      d.vec3f,
      d.vec3f,
      d.vec2f,
    ],
    d.f32,
  )(shadowVisibilityWgsl())
  .$uses({ shadowPcf });

/** INHERITED-WGSL BOUNDARY. This pass types the shadow RESOURCES, not the
 * shading math: the PCF taps, the reverse-Z coordinate/bias contract and the
 * cascade fade all remain the shared owner's WGSL text.
 *
 * That owner writes `sampleSunShadow` against the world's module-scope shadow
 * bindings (`sunShadow`, `sunDepth`, `sunCompare`, `environment`, `cam`), which
 * TypeGPU instead owns inside typed bind group layouts. So the renderer
 * re-heads the SAME function with those five names as parameters and passes the
 * typed resources in; the body — every line of the inherited math — is the
 * shared text verbatim. Only the signature is synthesized, and the guard below
 * fails loudly rather than silently forking if the shared head ever moves. */
const SHARED_SAMPLE_HEAD = "fn sampleSunShadow(world:vec3f,normal:vec3f,pixel:vec2f)->f32 {";
export function sunShadowSampleBodyWgsl(mode: NativeShadowMode): string {
  const shared = sunShadowSampleWgsl(mode);
  if (!shared.startsWith(SHARED_SAMPLE_HEAD))
    throw Error("Shared sun-shadow sampler no longer has the head the renderer re-heads");
  return `(environment:Environment,cam:Camera,sunShadow:SunShadow,sunDepth:texture_depth_2d_array,sunCompare:sampler_comparison,world:vec3f,normal:vec3f,pixel:vec2f)->f32 {${shared.slice(SHARED_SAMPLE_HEAD.length)}`;
}

/** Camera-fitted directional depth, typed. Owns the depth ARRAY, one caster
 * camera per cascade and the one receiver block; the world owns caster
 * selection and binds the same pose with each cascade's own camera.
 *
 * Resource shape follows the mode and nothing else: the fitted single map is
 * ONE 1024 layer, High is TWO 2048 layers. Selecting single never allocates a
 * High-sized map. Both bind the same array view to receivers, so the binding
 * shape is fixed while the allocation is not.
 *
 * The device is BORROWED: `tgpu.initFromDevice` does not take ownership, so
 * this owner's `root.destroy()` releases only what it allocated. */
export function createTypegpuSunShadow(
  device: GPUDevice,
  environment: CivsimEnvironment,
  tuning: ShadowTuning,
) {
  const mode: NativeShadowMode = "csm";
  const root = tgpu.initFromDevice({ device });
  const owned: { destroy(): void }[] = [];
  let disposed = false;
  const dispose = () => {
    if (disposed) return;
    disposed = true;
    for (const r of owned.reverse()) r.destroy();
    root.destroy();
  };
  const own = <T extends { destroy(): void }>(r: T) => {
    owned.push(r);
    return r;
  };
  const live = () => {
    if (disposed) throw Error("TypeGPU shadow disposed");
  };
  const layers = CSM_CASCADES;
  const mapSize = tuning.mapSize;
  try {
    const depth = own(
      root
        .createTexture({
          size: [mapSize, mapSize, layers],
          format: BATTLE_DEPTH_ATTACHMENT.format,
        })
        .$usage("render", "sampled"),
    );
    // Attachments are per-layer; receivers bind the whole array in both modes.
    const layerViews = Array.from({ length: layers }, (_, layer) =>
      depth.createView("render", { baseArrayLayer: layer, arrayLayerCount: 1 }),
    );
    const receiverView = depth.createView(d.textureDepth2dArray());
    // One distinct caster camera per active cascade. Sharing a single buffer
    // across passes would let both see the final write queued before submission.
    const cameras = Array.from({ length: layers }, () =>
      own(root.createBuffer(Camera).$usage("uniform")),
    );
    const cameraGroups = cameras.map((cam) => root.createBindGroup(typegpuCameraLayout, { cam }));
    const state = own(root.createBuffer(SunShadow).$usage("uniform"));
    const comparison = root.createComparisonSampler({
      compare: "greater-equal",
      minFilter: "linear",
      magFilter: "linear",
      addressModeU: "clamp-to-edge",
      addressModeV: "clamp-to-edge",
    });
    const samplingGroup = root.createBindGroup(sunSamplingLayout, {
      sun: state,
      sunDepth: receiverView,
      sunCompare: comparison,
    });
    const frameData = new NativeShadowFrame(
      environment,
      mode,
      (data) => {
      // Every distinct caster camera and the shared block are written once
      // their inputs change; a cold frame publishes a legal empty block, so the
      // receiver never samples uninitialized uniform memory.
      for (const cascade of data.cascades) cameras[cascade.index].write(cascade.camera.buffer);
      state.write(data.receiver.buffer);
      },
      tuning,
    );
    return {
      mode,
      mapSize,
      layers,
      depth,
      receiverView,
      cameras,
      cameraGroups,
      state,
      comparison,
      samplingGroup,
      update(camera: Camera3DParams, receiver: readonly [number, number]) {
        live();
        return frameData.update(camera, receiver);
      },
      get data(): NativeShadowData {
        return frameData.data;
      },
      /** Encodes one clear-to-0 depth pass per ACTIVE cascade, each labelled so
       * a timestamp owner can price the cascades separately. A cold frame with
       * no fit yet encodes nothing rather than drawing casters against an
       * unfitted box. */
      encode(encoder: TgpuCommandEncoder, draw: (pass: TgpuRenderPass, cascade: number) => void) {
        live();
        const active = frameData.data.cascades;
        for (const cascade of active) {
          const label = active.length > 1 ? `shadow-cascade-${cascade.index}` : "shadow";
          nativeGpuScope(device, label, () => {
            const pass = encoder.beginRenderPass({
              label: `typegpu directional shadow ${cascade.index}`,
              colorAttachments: [],
              depthStencilAttachment: {
                view: layerViews[cascade.index],
                depthClearValue: BATTLE_DEPTH_ATTACHMENT.clearValue,
                depthLoadOp: BATTLE_DEPTH_ATTACHMENT.loadOp,
                depthStoreOp: BATTLE_DEPTH_ATTACHMENT.storeOp,
              },
            });
            try {
              draw(pass, cascade.index);
            } finally {
              pass.end();
            }
          });
        }
      },
      /** Real textures and buffers, counted as allocated — not as configured. */
      stats() {
        return {
          mode,
          cascades: frameData.data.cascades.length,
          mapSize,
          // First uploaded cascade: mat4 (16 floats), then bias.z is PCF radius.
          radius: frameData.data.receiver[18],
          layers,
          depthBytes: mapSize * mapSize * 4 * layers,
          cameraBuffers: cameras.length,
          receiverBytes: SUN_SHADOW_BLOCK_FLOATS * 4,
        };
      },
      dispose,
    };
  } catch (error) {
    dispose();
    throw error;
  }
}
export type TypegpuSunShadow = ReturnType<typeof createTypegpuSunShadow>;
