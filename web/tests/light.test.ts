// @vitest-environment node
// The battle's light: `presentation.light` in the
// fixture is the one owner of the sun, sky, haze, grade, bloom and cascades.
// What the sky, the haze, the sun, post and the cascades make of it is
// resolved and tested in the presentation crate (`crates/presentation/tests/
// light.rs` and `shadow.rs`); here, the page's binding hands the renderer the
// words its TypeGPU structs read.
import { expect, test } from "vitest";
import game from "@fixtures/game.json";
import {
  validateLight,
  type LightPresentation,
} from "@packages/battle-renderer/src/light/sceneLight.ts";
import { loadPresentation, resolveLight, ScenePresentation } from "@web/battle/sim/presentation";
import { createTypegpuPost } from "@packages/battle-renderer/src/world/post.ts";
import { Camera, SunShadow } from "@packages/battle-renderer/src/records.gen.ts";
import { d } from "typegpu";

await loadPresentation();

const LIGHT = game.presentation.light as unknown as LightPresentation;
const withLight = (edit: (l: LightPresentation) => void): LightPresentation => {
  const copy = structuredClone(LIGHT);
  edit(copy);
  return copy;
};

test("the street test map's light is valid, and a broken one is refused by name", () => {
  expect(validateLight(LIGHT)).toBe(LIGHT);
  expect(() => validateLight(withLight((l) => (l.exposure = Number.NaN)))).toThrow(/exposure/);
  expect(() => validateLight(withLight((l) => (l.sun_elevation = -0.1)))).toThrow(/sun_elevation/);
  expect(() => validateLight(withLight((l) => (l.grade.shadow_tint = [1, Infinity, 1])))).toThrow(
    /grade\.shadow_tint/,
  );
  expect(() => validateLight(withLight((l) => (l.cascades.count = 3)))).toThrow(/cascades\.count/);
  expect(() => validateLight(withLight((l) => (l.shadow_floor = 0.8)))).toThrow(/shadow_floor/);
  expect(() => validateLight(withLight((l) => (l.shadow_floor = -0.1)))).toThrow(/shadow_floor/);
});

test("the sky and the environment light share one sun", async () => {
  // The records the sky and the environment upload carry it.
  const light = await resolveLight(LIGHT);
  const sun = [...light.sun.subarray(0, 3)];
  expect(Math.hypot(...sun)).toBeCloseTo(1, 6);
  expect(sun[2]).toBeCloseTo(Math.sin(LIGHT.sun_elevation), 6);
  [...light.sky.subarray(0, 3)].forEach((c, i) => expect(c).toBeCloseTo(sun[i], 6));
});

test("a frame's cascades are the structs the renderer binds: a receiver block and a camera per cascade", () => {
  const presentation = new ScenePresentation();
  try {
    // The default camera over the street map: eye, target, up, lens, viewport.
    const camera = {
      eye: [170, 758, 50],
      target: [170, 800, 0],
      up: [0, 0.75, 0.66],
      fovY: 0.8,
      near: 1,
      width: 1920,
      height: 1080,
    };
    const ui = { selected: [], xray: false, ghosts: [] };
    // Before the light, a frame fits none.
    expect(presentation.frame(0, camera, ui, 0).sun_shadow.count).toBe(0);
    // The street map's ground: one flat slab, 1600 m square.
    const slab = new Float32Array(40);
    [0, 0, 1600, 0, 1600, 1600, 0, 1600].forEach((v, i) => (slab[(i >> 1) * 10 + (i & 1)] = v));
    presentation.setShadow(LIGHT, slab);
    const frame = presentation.frame(0, camera, ui, 0);
    expect(frame.sun_shadow.f32.byteLength).toBe(d.sizeOf(SunShadow));
    const active = frame.casters.count;
    expect(active).toBe(LIGHT.cascades.count);
    const casters = frame.casters.f32;
    const receiver = frame.sun_shadow.f32;
    const size = d.sizeOf(Camera) / 4;
    expect(casters).toHaveLength(active * size);
    type CameraValue = d.Infer<typeof Camera>;
    const at = (field: (c: CameraValue) => unknown) => d.memoryLayoutOf(Camera, field).offset / 4;
    for (let i = 0; i < active; i++) {
      const c = casters.subarray(i * size, (i + 1) * size);
      // A cascade's viewport is its map, square, and its light's near is a metre.
      expect([c[at((x) => x.width)], c[at((x) => x.height)]]).toEqual([
        LIGHT.cascades.map_size,
        LIGHT.cascades.map_size,
      ]);
      expect(c[at((x) => x.znear)]).toBe(1);
      // The receiver's record holds the same light.
      const record = receiver.subarray(i * 24, i * 24 + 16);
      const viewProj = at((x) => x.viewProj);
      expect([...c.subarray(viewProj, viewProj + 16)]).toEqual([...record]);
    }
  } finally {
    presentation.free();
  }
});

test("post encodes sRGB itself, so it refuses an sRGB canvas that would encode twice", async () => {
  const device = null as unknown as GPUDevice;
  const input = null as unknown as GPUTextureView;
  const post = await resolveLight(LIGHT);
  await expect(createTypegpuPost(device, input, 256, 256, "bgra8unorm-srgb", post)).rejects.toThrow(
    /sRGB/,
  );
});

test("a cloud field past full is refused by name", () => {
  expect(() =>
    validateLight(
      withLight((l) => {
        l.sky.clouds.coverage = 1.5;
      }),
    ),
  ).toThrow(/sky\.clouds\.coverage/);
});
