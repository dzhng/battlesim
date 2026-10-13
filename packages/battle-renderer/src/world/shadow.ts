// Adapted from ~/dev/game battle-renderer/src/world/shadow.ts.
// Local changes: cascade mode only, the map size from `presentation.light.cascades`, and the
// cascades fitted each frame by the presentation crate (`shadow.rs`) over the map's box.
import { vec3 } from "math";
import { eyePosition, type Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { SunShadowFit } from "@wasm/presentation/presentation_wasm.js";
import { tgpu, d, type TgpuCommandEncoder, type TgpuRenderPass } from "typegpu";
import { shadowPcfWgsl, shadowVisibilityWgsl, sunShadowSampleWgsl } from "../shaders/shadow";
import { BATTLE_DEPTH_ATTACHMENT } from "../worldDepth";
import { typegpuCameraLayout } from "./camera";
import { Camera, CAMERA_WORDS, SUN_CASCADES, SunShadow } from "../records.gen";
import type { LightPresentation } from "../light/sceneLight";

/** The receiver binding shape, owned once: the world environment group
 * spreads these entries. */
export const sunShadowEntries = {
  sun: { uniform: SunShadow, visibility: ["fragment"] },
  sunDepth: { texture: d.textureDepth2dArray(), visibility: ["fragment"] },
  sunCompare: { sampler: "comparison", visibility: ["fragment"] },
} satisfies Parameters<typeof tgpu.bindGroupLayout>[0];

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
 * bindings (`sunShadow`, `sunDepth`, `sunCompare`, `environment`), which
 * TypeGPU instead owns inside typed bind group layouts. So the renderer
 * re-heads the SAME function with those four names as parameters and passes the
 * typed resources in; the body — every line of the inherited math — is the
 * shared text verbatim. Only the signature is synthesized, and the guard below
 * fails loudly rather than silently forking if the shared head ever moves. */
const SHARED_SAMPLE_HEAD = "fn sampleSunShadow(world:vec3f,normal:vec3f,pixel:vec2f)->f32 {";
export function sunShadowSampleBodyWgsl(): string {
  const shared = sunShadowSampleWgsl();
  if (!shared.startsWith(SHARED_SAMPLE_HEAD))
    throw Error("Shared sun-shadow sampler no longer has the head the renderer re-heads");
  return `(environment:Environment,sunShadow:SunShadow,sunDepth:texture_depth_2d_array,sunCompare:sampler_comparison,world:vec3f,normal:vec3f,pixel:vec2f)->f32 {${shared.slice(SHARED_SAMPLE_HEAD.length)}`;
}

/** Camera-fitted directional depth, typed. Owns the depth ARRAY (one layer
 * per cascade, `light.cascades.map_size` square), one caster camera per
 * cascade and the one receiver block; the world owns caster selection and
 * binds the same pose with each cascade's own camera.
 *
 * The device is BORROWED: `tgpu.initFromDevice` does not take ownership, so
 * this owner's `root.destroy()` releases only what it allocated. */
export function createTypegpuSunShadow(device: GPUDevice, light: LightPresentation) {
  const root = tgpu.initFromDevice({ device });
  const owned: { destroy(): void }[] = [];
  let disposed = false;
  const fit = new SunShadowFit(JSON.stringify(light));
  const dispose = () => {
    if (disposed) return;
    disposed = true;
    for (const r of owned.reverse()) r.destroy();
    root.destroy();
    fit.free();
  };
  const own = <T extends { destroy(): void }>(r: T) => {
    owned.push(r);
    return r;
  };
  const live = () => {
    if (disposed) throw Error("TypeGPU shadow disposed");
  };
  const layers = SUN_CASCADES;
  const mapSize = light.cascades.map_size;
  try {
    const depth = own(
      root
        .createTexture({
          size: [mapSize, mapSize, layers],
          format: BATTLE_DEPTH_ATTACHMENT.format,
        })
        .$usage("render", "sampled"),
    );
    // Attachments are per-layer; receivers bind the whole array.
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
    // A cold frame publishes a legal empty block, so the receiver never
    // samples uninitialized uniform memory.
    let receiver = fit.receiver();
    state.write(receiver.buffer as ArrayBuffer);
    let active = 0;
    const words = new Float64Array(13);
    const eye = vec3.create();
    return {
      receiverView,
      cameraGroups,
      state,
      comparison,
      /** Fit the cascades for `camera` drawn `width` by `height` pixels,
       *  over the map's `box` (none: empty). */
      update(camera: Camera3DParams, width: number, height: number, box: Float64Array) {
        live();
        const s = Math.sin(camera.pitch);
        words.set(eyePosition(eye, camera));
        words.set(camera.target, 3);
        words.set(
          [-s * Math.cos(camera.yaw), -s * Math.sin(camera.yaw), Math.cos(camera.pitch)],
          6,
        );
        words.set([camera.fovY, camera.near, width, height], 9);
        active = fit.fit(words, box);
        const casters = fit.casters();
        for (let i = 0; i < active; i++)
          cameras[i].write(casters.slice(i * CAMERA_WORDS, (i + 1) * CAMERA_WORDS).buffer);
        receiver = fit.receiver();
        state.write(receiver.buffer as ArrayBuffer);
      },
      /** The fitted cascades, and the receiver range they split: its near
       *  and capped far (m). */
      stats() {
        const control = receiver.subarray(receiver.length - 4);
        return { cascades: active, receiverRange: [control[2], control[0]] as [number, number] };
      },
      /** Encodes one clear-to-0 depth pass per ACTIVE cascade, each pass
       * labelled with its cascade. A cold frame with no fit yet encodes
       * nothing rather than drawing casters against an unfitted box. */
      encode(encoder: TgpuCommandEncoder, draw: (pass: TgpuRenderPass, cascade: number) => void) {
        live();
        for (let cascade = 0; cascade < active; cascade++) {
          const pass = encoder.beginRenderPass({
            label: `typegpu directional shadow ${cascade}`,
            colorAttachments: [],
            depthStencilAttachment: {
              view: layerViews[cascade],
              depthClearValue: BATTLE_DEPTH_ATTACHMENT.clearValue,
              depthLoadOp: BATTLE_DEPTH_ATTACHMENT.loadOp,
              depthStoreOp: BATTLE_DEPTH_ATTACHMENT.storeOp,
            },
          });
          try {
            draw(pass, cascade);
          } finally {
            pass.end();
          }
        }
      },
      dispose,
    };
  } catch (error) {
    dispose();
    throw error;
  }
}
export type TypegpuSunShadow = ReturnType<typeof createTypegpuSunShadow>;
