import {
  SunShadow,
  shadowVisibility,
  sunShadowEntries,
  sunShadowSampleBodyWgsl,
  type TypegpuSunShadow,
} from "./shadow";
import { Camera, typegpuCameraLayout } from "./camera";
import type { NativeShadowMode } from "../shadowData";
import { typegpuTextureBytes } from "./textureUpload";
import { tgpu, d, std } from "typegpu";
import type { LightPresentation } from "../light/sceneLight";
import { photorealEnvironment } from "../light/physicalEnvironment";
import { skyModelParams } from "../light/skyParameters";
import { createTypegpuSky } from "./sky";
import { createTypegpuPmrem } from "./pmrem";
import { samplePmrem } from "./pmremSampling";
import { standardPbrWgsl } from "../shaders/standardPbr";
import { aerialWgsl } from "../shaders/aerial";
import { equirectUvWgsl } from "../shaders/physicalSky";
import { DFG_LUT_DATA, DFG_LUT_SIZE } from "../shaders/dfgLut";
import { environmentFunctions, type WorldSurfaceDiagnostic } from "../shaders/environment";
import {
  CAST_ALBEDO_FLOOR,
  CAST_ALBEDO_GREY,
  CAST_FALLOFF,
  CAST_LIGHTS_MAX,
  castLightsBytes,
  CastLights,
} from "../light/castLights";

export const Environment = d.struct({
  worldToView: d.mat4x4f,
  observer: d.vec4f,
  sunDirection: d.vec4f,
  sunRadiance: d.vec4f,
  /** The PMREM's top mip, the shadow floor (`shadow_floor`), unused. */
  settings: d.vec4f,
  /** Linear rgb multiplier on the sky's environment light (the shadow fill). */
  fill: d.vec4f,
});
const environmentEntries = {
  data: { uniform: Environment, visibility: ["vertex", "fragment"] },
  /** The frame's cast lights (`light/castLights.ts`): flashes, motors, bursts, fires. */
  lights: { uniform: CastLights, visibility: ["fragment"] },
  sky: { texture: d.texture2d(), visibility: ["fragment"] },
  pmrem: { texture: d.texture2d(), visibility: ["fragment"] },
  dfg: { texture: d.texture2d(), visibility: ["fragment"] },
  linear: { sampler: "filtering", visibility: ["fragment"] },
} satisfies Parameters<typeof tgpu.bindGroupLayout>[0];
export const environmentLayout = tgpu.bindGroupLayout(environmentEntries);
const shadowEnvironmentLayout = tgpu.bindGroupLayout({
  ...environmentEntries,
  ...sunShadowEntries,
});
const casterEnvironmentLayout = tgpu.bindGroupLayout({
  data: { uniform: Environment, visibility: ["vertex"] },
});
const standardPbr = tgpu
  .fn(
    [
      d.vec3f,
      d.vec3f,
      d.f32,
      d.f32,
      d.f32,
      d.f32,
      d.vec3f,
      d.vec3f,
      d.vec3f,
      d.vec3f,
      d.f32,
      d.vec3f,
      d.texture2d(),
      d.sampler(),
      d.f32,
      d.texture2d(),
      d.sampler(),
    ],
    d.vec3f,
  )(standardPbrWgsl)
  .$uses({ samplePmrem });
const equirectUv = tgpu.fn([d.vec3f], d.vec2f)(equirectUvWgsl);

/** The receiver's sun-shadow entry point for one mode: the inherited sampling
 * body (see shadow.ts for that boundary) bound to THIS owner's typed resources
 * — the environment block, the world camera and the cascade depth array. High
 * reads the shared view row and near plane, so a receiver blends its cascades
 * against the same admitted frame the fits came from. */
export function typegpuSunShadowSample(mode: NativeShadowMode) {
  const inherited = tgpu
    .fn(
      [
        Environment,
        Camera,
        SunShadow,
        d.textureDepth2dArray(),
        d.comparisonSampler(),
        d.vec3f,
        d.vec3f,
        d.vec2f,
      ],
      d.f32,
    )(sunShadowSampleBodyWgsl(mode))
    .$uses({ Environment, Camera, SunShadow, shadowVisibility });
  return tgpu.fn(
    [d.vec3f, d.vec3f, d.vec2f],
    d.f32,
  )((world, normal, pixel) => {
    "use gpu";
    return inherited(
      shadowEnvironmentLayout.$.data,
      typegpuCameraLayout.$.cam,
      shadowEnvironmentLayout.$.sun,
      shadowEnvironmentLayout.$.sunDepth,
      shadowEnvironmentLayout.$.sunCompare,
      world,
      normal,
      pixel,
    );
  });
}
/** Shadows off: receivers keep the same entry point and read nothing. */
const unshadowed = tgpu.fn(
  [d.vec3f, d.vec3f, d.vec2f],
  d.f32,
)("(world:vec3f,normal:vec3f,pixel:vec2f)->f32{return 1.0;}");

/** TypeGPU resource ownership; exposed GPU views are borrowed by component bindings. */
export async function createTypegpuEnvironment(
  device: GPUDevice,
  light: LightPresentation,
  diagnostic?: WorldSurfaceDiagnostic,
  backgroundSamples: 1 | 4 = 1,
  shadow?: TypegpuSunShadow,
  aerial = true,
) {
  const root = tgpu.initFromDevice({ device }),
    owned: { destroy(): void }[] = [];
  let disposed = false;
  const dispose = () => {
    if (disposed) return;
    disposed = true;
    for (const r of owned) r.destroy();
    root.destroy();
  };
  try {
    const sky = await createTypegpuSky(device, skyModelParams(light), backgroundSamples);
    owned.push({ destroy: sky.dispose });
    const pmrem = await createTypegpuPmrem(device, sky.lut);
    owned.push({ destroy: pmrem.dispose });
    const dfg = root
      .createTexture({ size: [DFG_LUT_SIZE, DFG_LUT_SIZE], format: "rg16float" })
      .$usage("sampled");
    owned.push(dfg);
    dfg.write(typegpuTextureBytes(DFG_LUT_DATA));
    const data = root.createBuffer(Environment).$usage("uniform");
    owned.push(data);
    // No light until the frame's effects say so: a zero header.
    const lights = root.createBuffer(CastLights).$usage("uniform");
    owned.push(lights);
    const linear = root.createSampler({ minFilter: "linear", magFilter: "linear" });
    const resources = {
      data,
      lights,
      sky: sky.lut.createView(),
      pmrem: pmrem.texture.createView(),
      dfg: dfg.createView(),
      linear,
    };
    const layout = shadow ? shadowEnvironmentLayout : environmentLayout;
    const group = shadow
      ? root.createBindGroup(shadowEnvironmentLayout, {
          ...resources,
          sun: shadow.state,
          sunDepth: shadow.receiverView,
          sunCompare: shadow.comparison,
        })
      : root.createBindGroup(environmentLayout, resources);
    const casterGroup = root.createBindGroup(casterEnvironmentLayout, { data });
    const sampleSunShadow = shadow ? typegpuSunShadowSample(shadow.mode) : unshadowed;
    const spec = photorealEnvironment(light),
      functions = environmentFunctions(diagnostic, aerial);
    const applyAerial = tgpu
      .fn(
        [d.vec4f, d.vec3f, d.vec3f, d.vec3f, d.texture2d(), d.sampler()],
        d.vec4f,
      )(aerialWgsl(light))
      .$uses({ equirectUv });
    const fromView = tgpu.fn([d.vec3f], d.f32)(functions.geometryRoughnessFromView);
    const shadeAlgorithm = tgpu
      .fn(
        [
          d.vec3f,
          d.vec3f,
          d.f32,
          d.f32,
          d.f32,
          d.f32,
          d.vec3f,
          d.vec3f,
          d.f32,
          d.vec3f,
          d.vec3f,
          d.vec3f,
          d.vec3f,
          d.vec3f,
          d.f32,
          d.texture2d(),
          d.texture2d(),
          d.texture2d(),
          d.sampler(),
        ],
        d.vec4f,
      )(functions.shadeEnvironment)
      .$uses({ standardPbr, applyAerial });
    /** The frame's cast lights on a surface of albedo `base` at `position`
     *  facing `normal`: each one's falloff inside its radius, its facing
     *  wrapped by the header's wrap, as Lambert on the albedo greyed by
     *  `CAST_ALBEDO_GREY` and held above `CAST_ALBEDO_FLOOR` (`light/castLights.ts`). */
    const castLight = tgpu.fn(
      [d.vec3f, d.vec3f, d.vec3f],
      d.vec3f,
    )((base, normal, position) => {
      "use gpu";
      const count = d.u32(layout.$.lights.header.x);
      const wrap = layout.$.lights.header.y;
      const n = std.normalize(normal);
      let sum = d.vec3f(0);
      for (let i = d.u32(0); i < count; i++) {
        const toward = std.sub(layout.$.lights.lights[i].position.xyz, position);
        const d2 = std.dot(toward, toward);
        const x2 = d2 * layout.$.lights.lights[i].position.w;
        if (x2 < 1) {
          const falloff = ((1 - x2) * (1 - x2)) / (1 + CAST_FALLOFF * x2);
          const facing = std.dot(n, toward) * std.inverseSqrt(std.max(d2, 1e-4));
          const wrapped = std.clamp((facing + wrap) / (1 + wrap), 0, 1);
          sum = std.add(sum, std.mul(layout.$.lights.lights[i].color.xyz, falloff * wrapped));
        }
      }
      const grey = std.dot(base, d.vec3f(0.2126, 0.7152, 0.0722));
      const albedo = std.max(
        std.mix(base, d.vec3f(grey), CAST_ALBEDO_GREY),
        d.vec3f(CAST_ALBEDO_FLOOR),
      );
      return std.mul(albedo, std.mul(sum, 1 / Math.PI));
    });
    const shade = tgpu.fn(
      [d.vec3f, d.vec3f, d.f32, d.f32, d.f32, d.f32, d.vec3f, d.vec3f, d.f32, d.vec3f],
      d.vec4f,
    )((base, emissive, roughness, geomRoughness, metal, ao, normal, position, shadow, eye) => {
      "use gpu";
      // A sun shadow keeps the light's shadow floor (`settings.y`) of the sun.
      const sun = std.mix(layout.$.data.settings.y, 1, shadow);
      return shadeAlgorithm(
        base,
        std.add(emissive, castLight(base, normal, position)),
        roughness,
        geomRoughness,
        metal,
        ao,
        normal,
        position,
        sun,
        eye,
        layout.$.data.observer.xyz,
        layout.$.data.sunDirection.xyz,
        layout.$.data.sunRadiance.xyz,
        layout.$.data.fill.xyz,
        layout.$.data.settings.x,
        layout.$.sky,
        layout.$.pmrem,
        layout.$.dfg,
        layout.$.linear,
      );
    });
    const unlitAlgorithm = tgpu
      .fn(
        [
          d.vec3f,
          d.vec3f,
          d.vec3f,
          d.vec3f,
          d.vec3f,
          d.vec3f,
          d.vec3f,
          d.f32,
          d.texture2d(),
          d.texture2d(),
          d.sampler(),
        ],
        d.vec4f,
      )(/* wgsl */ `(colour:vec3f,worldPosition:vec3f,eye:vec3f,observer:vec3f,sunDirection:vec3f,sunRadiance:vec3f,environmentIntensity:vec3f,maxMip:f32,sky:texture_2d<f32>,pmrem:texture_2d<f32>,linear:sampler)->vec4f {
        // What a white matte surface facing the sky returns: the sun's share
        // and the sky's, as standardPbr's diffuse terms have them.
        let skyLight=samplePmrem(pmrem,linear,vec3f(0.0,0.0,1.0),1.0,maxMip)*environmentIntensity;
        let white=sunRadiance*max(sunDirection.z,0.0)*0.3183098861837907+skyLight;
        return ${aerial ? "applyAerial(vec4f(colour*white,1),worldPosition,eye,observer,sky,linear)" : "vec4f(colour*white,1)"};
      }`)
      .$uses({ samplePmrem, applyAerial });
    /** A finished picture's place in the frame: a surface that takes no
     *  light of its own (a room behind a window, lit when its picture was
     *  made). `colour` is shown at the scene's exposure, as bright as a matte
     *  surface of that albedo lying in the open under this sun and sky, and
     *  behind the air between it and the eye; no sun shadow, no cast light,
     *  nothing of its own facing. */
    const unlit = tgpu.fn(
      [d.vec3f, d.vec3f, d.vec3f],
      d.vec4f,
    )((colour, position, eye) => {
      "use gpu";
      return unlitAlgorithm(
        colour,
        position,
        eye,
        layout.$.data.observer.xyz,
        layout.$.data.sunDirection.xyz,
        layout.$.data.sunRadiance.xyz,
        layout.$.data.fill.xyz,
        layout.$.data.settings.x,
        layout.$.sky,
        layout.$.pmrem,
        layout.$.linear,
      );
    });
    return {
      group,
      layout,
      /** The environment uniform and PMREM as raw resources, for raw passes
       *  that shade with the same light (the effect pass). */
      raw: { uniform: root.unwrap(data), pmrem: pmrem.texture, lights: root.unwrap(lights) },
      casterLayout: casterEnvironmentLayout,
      casterGroup,
      shadows: Boolean(shadow),
      shadowMode: shadow?.mode ?? null,
      sampleSunShadow,
      shade,
      unlit,
      geometryRoughnessFromView: fromView,
      sky,
      pmrem,
      /** The one sun direction every material shades with. */
      sunDirection: spec.sunDirection as readonly [number, number, number],
      /** This frame's cast lights, a packed `CastLights` image
       *  (`packCastLights`): only the header and the lights in use are sent. */
      setCastLights(image: Float32Array<ArrayBuffer>, count: number) {
        if (disposed) return;
        const bytes = castLightsBytes(Math.min(CAST_LIGHTS_MAX, count));
        device.queue.writeBuffer(root.unwrap(lights), 0, image.buffer, image.byteOffset, bytes);
      },
      setView(worldToView: ArrayLike<number>, observer: readonly [number, number, number]) {
        if (disposed) throw Error("TypeGPU environment disposed");
        if (worldToView.length !== 16) throw Error("Expected camera view matrix");
        data.write({
          worldToView: Array.from(worldToView),
          observer: d.vec4f(...observer, 0),
          sunDirection: d.vec4f(...spec.sunDirection, 0),
          sunRadiance: d.vec4f(
            ...(spec.sunColor.map((c) => c * spec.sunIntensity) as [number, number, number]),
            0,
          ),
          settings: d.vec4f(pmrem.maxMip, spec.shadowFloor, 0, 0),
          fill: d.vec4f(...spec.fill, 0),
        });
      },
      dispose,
    };
  } catch (error) {
    dispose();
    throw error;
  }
}
export type TypegpuEnvironment = Awaited<ReturnType<typeof createTypegpuEnvironment>>;
