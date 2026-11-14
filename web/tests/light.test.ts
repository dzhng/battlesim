// @vitest-environment node
// The battle's light: `presentation.light` in the
// fixture is the one owner of the sun, sky, haze, grade, bloom and cascades.
// What the sky, the haze, the sun, post and the cascades make of it is
// resolved and tested in the presentation crate (`crates/presentation/tests/
// light.rs` and `shadow.rs`); here, the page's binding hands the renderer the
// words defined by the shared Rust record layouts.
import {
  lightForPreset,
  lightPreset,
  lightPresetLabels,
  lightPresets,
  presentationSettings,
} from "@web/presentationSettings";
import presentation from "../../fixtures/presentation.json";
import { expect, test } from "vitest";
import { type LightPresentation } from "@packages/battle-renderer/src/light/sceneLight.ts";
import { loadPresentation, ScenePresentation } from "@web/battle/sim/presentation";
import {
  SHADOW_CASCADES,
  CAMERA_WORDS,
  SUN_SHADOW_WORDS,
  CameraAt,
} from "@packages/battle-renderer/src/records.gen.ts";

import { check_presentation, light_records } from "@wasm/renderer_browser";
await loadPresentation();
const checkLight = (light: LightPresentation) => {
  light_records(JSON.stringify(light));
  return light;
};

const LIGHT = presentationSettings.light as unknown as LightPresentation;
const withLight = (edit: (l: LightPresentation) => void): LightPresentation => {
  const copy = structuredClone(LIGHT);
  edit(copy);
  return copy;
};

test("the street test map's light is valid, and a broken one is refused by name", () => {
  expect(checkLight(LIGHT)).toBe(LIGHT);
  expect(() => checkLight(withLight((l) => (l.exposure = Number.NaN)))).toThrow(/exposure/);
  expect(() => checkLight(withLight((l) => (l.sun_elevation = -0.1)))).toThrow(/sun_elevation/);
  expect(() => checkLight(withLight((l) => (l.grade.shadow_tint = [1, Infinity, 1])))).toThrow(
    /grade\.shadow_tint/,
  );
  expect(() => checkLight(withLight((l) => Object.assign(l.cascades, { count: 3 })))).toThrow(
    /cascades\.count/,
  );
  expect(() => checkLight(withLight((l) => (l.shadow_floor = 0.8)))).toThrow(/shadow_floor/);
  expect(() => checkLight(withLight((l) => (l.shadow_floor = -0.1)))).toThrow(/shadow_floor/);
  expect(() => checkLight(withLight((l) => (l.glow.wrap = -0.1)))).toThrow(/glow\.wrap/);
  expect(() => checkLight(withLight((l) => (l.vegetation_tint = [1, 1, 4.1])))).toThrow(
    /vegetation_tint/,
  );
  expect(() => checkLight(withLight((l) => (l.vegetation_lit_keep = 1.1)))).toThrow(
    /vegetation_lit_keep/,
  );
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
    const slab = new Float32Array(60);
    [0, 0, 1600, 0, 1600, 1600, 0, 0, 1600, 1600, 0, 1600].forEach(
      (v, i) => (slab[(i >> 1) * 10 + (i & 1)] = v),
    );
    presentation.setShadow(LIGHT, slab);
    const frame = presentation.frame(0, camera, ui, 0);
    expect(frame.sun_shadow.f32.byteLength).toBe(SUN_SHADOW_WORDS * 4);
    const active = frame.casters.count;
    expect(active).toBe(SHADOW_CASCADES);
    const casters = frame.casters.f32;
    const receiver = frame.sun_shadow.f32;
    const size = CAMERA_WORDS;
    expect(casters).toHaveLength(active * size);
    for (let i = 0; i < active; i++) {
      const c = casters.subarray(i * size, (i + 1) * size);
      // A cascade's viewport is its map, square, and its light's near is a metre.
      expect([c[CameraAt.width], c[CameraAt.height]]).toEqual([
        LIGHT.cascades.map_size,
        LIGHT.cascades.map_size,
      ]);
      expect(c[CameraAt.znear]).toBe(1);
      // The receiver's record holds the same light.
      const record = receiver.subarray(i * 24, i * 24 + 16);
      const viewProj = CameraAt.view_proj;
      expect([...c.subarray(viewProj, viewProj + 16)]).toEqual([...record]);
    }
  } finally {
    presentation.free();
  }
});

test("a cloud field past full is refused by name", () => {
  expect(() =>
    checkLight(
      withLight((l) => {
        l.sky.clouds.coverage = 1.5;
      }),
    ),
  ).toThrow(/sky\.clouds\.coverage/);
});

test("the grade's switch is a boolean the validation accepts either way", () => {
  expect(checkLight(withLight((l) => (l.grade.enabled = false)))).toBeTruthy();
  expect(checkLight(withLight((l) => (l.grade.enabled = true)))).toBeTruthy();
});

test("the fixture admits a day default and a selectable golden_hour look", () => {
  expect(lightPreset).toBe("day");
  expect(lightPresetLabels.day).toBe("Daylight");
  expect(lightPresetLabels.golden_hour).toBe("Golden hour");
  expect(LIGHT.sun_elevation).toBe(0.7);
  expect(LIGHT.sun_azimuth).toBe(-0.35);
  expect(lightPresets.day.sun_elevation).toBe(0.7);
  const golden = lightForPreset("golden_hour");
  expect(golden.sun_azimuth).toBe(LIGHT.sun_azimuth);
  expect(golden.sun_elevation).toBe(0.16);
  expect(golden.exposure).toBe(1.78);
  expect(golden.sky.fill).toEqual([0.62, 0.7, 0.84]);
  expect(golden.sky.turbidity).toBe(2.8);
  expect(golden.sky.saturation).toBe(1.32);
  expect(golden.grade.highlight_tint).toEqual([1.0, 1.04, 0.98]);
  expect(golden.grade.split_tone).toBe(0.5);
  expect(golden.glow.wrap).toBe(0.8);
  expect(golden.glow.rim).toBe(0.45);
  expect(golden.bloom.threshold).toBe(1.12);
  expect(golden.haze.sun_strength).toBe(0.5);
  expect(LIGHT.glow.wrap).toBeLessThan(golden.glow.wrap);
  expect(LIGHT.glow.rim).toBeLessThan(golden.glow.rim);
  expect(LIGHT.vegetation_tint).toEqual([1.0, 1.0, 1.0]);
  expect(LIGHT.vegetation_lit_keep).toBe(0);
  expect(golden.vegetation_tint).toEqual([0.9, 1.3, 1.08]);
  expect(golden.vegetation_lit_keep).toBe(0.7);
  expect(() => lightForPreset("dusk")).toThrow(/unknown light preset/);
  expect(checkLight(golden)).toBe(golden);
});

test("selecting golden hour as the document default is a one-key change", () => {
  const admitted = JSON.parse(
    check_presentation(JSON.stringify({ ...presentation, light_preset: "golden_hour" })),
  ) as {
    light: LightPresentation;
    light_preset: string;
    light_presets: { day: LightPresentation };
  };
  expect(admitted.light_preset).toBe("golden_hour");
  expect(admitted.light.sun_elevation).toBe(0.16);
  expect(admitted.light_presets.day.sun_elevation).toBe(0.7);
});
