// @vitest-environment node
// The battle's light (battle-look slice 13): `presentation.light` in the
// fixture is the one owner of the sun, sky, haze, grade, bloom and cascades.
import { expect, test } from "vitest";
import village from "@fixtures/village.json";
import {
  postSettings,
  sunDirection,
  validateLight,
  type LightPresentation,
} from "@packages/battle-renderer/src/light/sceneLight.ts";
import { skyModelParams } from "@packages/battle-renderer/src/light/skyParameters.ts";
import { photorealEnvironment } from "@packages/battle-renderer/src/light/physicalEnvironment.ts";
import { aerialWgsl } from "@packages/battle-renderer/src/shaders/aerial.ts";
import { aerialParams } from "@packages/battle-renderer/src/light/aerialParameters.ts";
import { createTypegpuPost } from "@packages/battle-renderer/src/world/post.ts";
import { cascadeBlendWeight } from "@packages/battle-renderer/src/light/cascadePolicy.ts";
import { cascadeFrameData } from "@packages/battle-renderer/src/shadowData.ts";
import { mapBox, receiverRange } from "@packages/battle-renderer/src/frame/receiverRange.ts";
import { MeshBuilder } from "@packages/battle-renderer/src/mesh.ts";
import {
  viewMatrix,
  screenRay,
  type Camera3DParams,
} from "@packages/renderer-core/src/camera3d.ts";
import type { Mat4 } from "@packages/renderer-core/src/mat4.ts";
import { vec3, type Mat4 as MathMat4, type Vec3 } from "math";

const LIGHT = village.presentation.light as unknown as LightPresentation;
const withLight = (edit: (l: LightPresentation) => void): LightPresentation => {
  const copy = structuredClone(LIGHT);
  edit(copy);
  return copy;
};

const [MAP_W, MAP_H] = village.map.size;
/** The village's ground as one flat slab (its 20 m ridge is inside the box's
 *  standing headroom). */
const MAP = mapBox(
  new MeshBuilder().box(MAP_W / 2, MAP_H / 2, 0, MAP_W / 2, MAP_H / 2, 0, [0, 0, 0, 1]).build(),
);

const camera = (over: Partial<Camera3DParams>): Camera3DParams => ({
  target: [800, 800, 0],
  distance: 65,
  pitch: 0.85,
  yaw: -1.57,
  fovY: 0.8,
  aspect: 16 / 9,
  near: 1,
  ...over,
});

test("the village's light is valid, and a broken one is refused by name", () => {
  expect(validateLight(LIGHT)).toBe(LIGHT);
  expect(() => validateLight(withLight((l) => (l.exposure = Number.NaN)))).toThrow(/exposure/);
  expect(() => validateLight(withLight((l) => (l.sun_elevation = -0.1)))).toThrow(/sun_elevation/);
  expect(() => validateLight(withLight((l) => (l.grade.shadow_tint = [1, Infinity, 1])))).toThrow(
    /grade\.shadow_tint/,
  );
  expect(() => validateLight(withLight((l) => (l.cascades.count = 3)))).toThrow(/cascades\.count/);
});

test("the sky, the environment light, the haze and the cascades share one sun", () => {
  const sun = sunDirection(LIGHT);
  expect(Math.hypot(...sun)).toBeCloseTo(1, 12);
  expect(sun[2]).toBeCloseTo(Math.sin(LIGHT.sun_elevation), 12);
  expect(skyModelParams(LIGHT).sunDirection).toEqual(sun);
  expect(photorealEnvironment(LIGHT).sunDirection).toEqual(sun);
  // Each cascade light looks down the sun: its view's forward row is -sun.
  const frame = cascadeFrameData(LIGHT.cascades, camera({}), sun, [30, 400]);
  for (const cascade of frame.cascades) {
    const v = cascade.view;
    [v[2], v[6], v[10]].forEach((c, i) => expect(c).toBeCloseTo(sun[i], 6));
  }
  // The haze's sunward tint is compiled from the same vector.
  const literal = (x: number) => `${x.toExponential(16)}f`;
  expect(aerialWgsl(LIGHT)).toContain(`vec3f(${sun.map(literal).join(",")})`);
});

const _inCascade_clip = vec3.create();
/** A world point through a cascade's light: inside its clip square, within
 *  its reverse-Z depth range. */
function inCascade(m: Mat4, p: Vec3): boolean {
  const [u, v, depth] = vec3.transformMat4(_inCascade_clip, p, m as unknown as MathMat4);
  return Math.abs(u) <= 1 && Math.abs(v) <= 1 && depth >= 0 && depth <= 1;
}

/** Every map point on screen, with its view depth. */
function visibleGround(cam: Camera3DParams) {
  const view = viewMatrix(cam);
  const points: { p: [number, number, number]; depth: number }[] = [];
  const n = 24;
  for (let iy = 0; iy <= n; iy++)
    for (let ix = 0; ix <= n; ix++) {
      const { origin, dir } = screenRay(cam, (ix / n) * 2 - 1, (iy / n) * 2 - 1);
      if (dir[2] >= 0) continue;
      const t = -origin[2] / dir[2];
      const p: [number, number, number] = [origin[0] + dir[0] * t, origin[1] + dir[1] * t, 0];
      if (p[0] < 0 || p[1] < 0 || p[0] > MAP_W || p[1] > MAP_H) continue;
      const depth = -(view[2] * p[0] + view[6] * p[1] + view[10] * p[2] + view[14]);
      points.push({ p, depth });
    }
  return points;
}

// The village's cameras: ground level, the Defilade default, the village at
// 160 m, WARNO's 420 m, the route overview and the strategic camera.
const CAMERAS: Record<string, Partial<Camera3DParams>> = {
  ground: { target: [170, 800, 0], distance: 25, pitch: 0.22 },
  default: { target: [170, 800, 0], distance: 65 },
  village: { target: [990, 790, 0], distance: 160, yaw: -1.2 },
  warno: { target: [700, 800, 0], distance: 420, yaw: -1.2 },
  route: { target: [800, 800, 0], distance: 1150 },
  strategic: { target: [170, 800, 0], distance: 2000 },
};

test.each(Object.entries(CAMERAS))(
  "every map point the %s camera sees within the reach is inside a cascade that shades it",
  (_, over) => {
    const cam = camera(over);
    const range = receiverRange(cam, MAP);
    const frame = cascadeFrameData(LIGHT.cascades, cam, sunDirection(LIGHT), range);
    expect(frame.cascades).toHaveLength(LIGHT.cascades.count);
    // Splits start where the map starts, not at the camera's near plane.
    expect(frame.splitNear).toBeCloseTo(range[0], 6);
    expect(frame.cappedFar).toBeLessThanOrEqual(LIGHT.cascades.max_far_m);
    const last = frame.cascades.length - 1;
    const points = visibleGround(cam).filter((g) => g.depth <= frame.cappedFar);
    expect(points.length).toBeGreaterThan(20);
    for (const { p, depth } of points) {
      // The receiver shader's normalisation: a fraction of the receiver range.
      const linear = Math.max(0, (depth - frame.splitNear) / (frame.cappedFar - frame.splitNear));
      let weight = 0;
      for (const c of frame.cascades) {
        const w = cascadeBlendWeight(
          linear,
          [c.index === 0 ? 0 : frame.breaks[c.index - 1], frame.breaks[c.index]],
          { first: c.index === 0, last: c.index === last },
        );
        if (w === 0) continue;
        weight += w;
        expect(inCascade(c.viewProjection, p), `${p} at ${depth} m in cascade ${c.index}`).toBe(
          true,
        );
      }
      // Full weight everywhere short of the last cascade's fade to its reach.
      if (linear < frame.breaks[last - 1]) expect(weight).toBeCloseTo(1, 6);
      else expect(weight).toBeGreaterThan(0);
    }
  },
);

test("the strategic camera spends its cascades on the map, 1.6 km out and beyond", () => {
  const cam = camera(CAMERAS.strategic);
  const range = receiverRange(cam, MAP);
  const frame = cascadeFrameData(LIGHT.cascades, cam, sunDirection(LIGHT), range);
  // The map's whole visible depth, not the kilometre and a half of air above it.
  const depths = visibleGround(cam).map((g) => g.depth);
  expect(Math.min(...depths)).toBeGreaterThan(1400);
  expect(frame.splitNear).toBeGreaterThan(0.95 * Math.min(...depths));
  expect(frame.cappedFar).toBeGreaterThanOrEqual(
    Math.min(Math.max(...depths), LIGHT.cascades.max_far_m),
  );
  // At about two metres a texel or finer, and never past the reach.
  for (const c of frame.cascades) expect(c.worldUnitsPerTexel).toBeLessThan(2.5);
  const beyond = cascadeFrameData(LIGHT.cascades, cam, sunDirection(LIGHT), [1600, 5000]);
  expect(beyond.cappedFar).toBe(LIGHT.cascades.max_far_m);
});

test("every light the validator accepts gives finite, non-negative HDR light", () => {
  for (const sun_elevation of [0.05, 0.3, LIGHT.sun_elevation, 1.2, Math.PI / 2])
    for (const turbidity of [1, LIGHT.sky.turbidity, 6, 12]) {
      const light = validateLight(
        withLight((l) => {
          l.sun_elevation = sun_elevation;
          l.sky.turbidity = turbidity;
        }),
      );
      const env = photorealEnvironment(light);
      const radiance = env.sunColor.map((c) => c * env.sunIntensity);
      const haze = aerialParams(light);
      for (const v of [...radiance, ...env.fill, ...haze.extinction, haze.visibilityKm]) {
        expect(Number.isFinite(v), `elevation ${sun_elevation}, turbidity ${turbidity}`).toBe(true);
        expect(v).toBeGreaterThanOrEqual(0);
      }
      expect(aerialWgsl(light)).not.toMatch(/NaN|Infinity/);
    }
});

test("post encodes sRGB itself, so it refuses an sRGB canvas that would encode twice", async () => {
  const device = null as unknown as GPUDevice;
  const input = null as unknown as GPUTextureView;
  const post = postSettings(LIGHT);
  await expect(createTypegpuPost(device, input, 256, 256, "bgra8unorm-srgb", post)).rejects.toThrow(
    /sRGB/,
  );
});
