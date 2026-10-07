// @vitest-environment node
// Cast lights at their GPU seam: the lights the effects offer, packed into
// the uniform every world material reads. Every light that reaches into the
// view while they fit; over the world's cap, the ones the camera sees
// strongest; a full candidate list drops and counts the rest.
import { expect, test } from "vitest";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { createDetailView, setDetailView } from "@packages/battle-renderer/src/frame/detailView";
import {
  CAST_LIGHTS_BYTES,
  CAST_LIGHTS_MAX,
  createCastLightList,
  offerCastLight,
  packCastLights,
} from "@packages/battle-renderer/src/light/castLights";

const WHITE = [1, 1, 1];
/** The street test map's default framing: 65 m from the origin, looking north. */
const CAMERA: Camera3DParams = {
  target: [0, 0, 0],
  distance: 65,
  pitch: 0.85,
  yaw: -Math.PI / 2,
  fovY: 0.8,
  aspect: 16 / 9,
  near: 1,
};
const SIDES = setDetailView(createDetailView(), CAMERA, 1080).sides;
const pack = (out: Float32Array, list: ReturnType<typeof createCastLightList>) =>
  packCastLights(out, list, SIDES, CAMERA.target, CAMERA.distance);
const image = () => new Float32Array(CAST_LIGHTS_BYTES / 4);
/** The packed lights' positions (x) and reach, from the uniform image. */
const packed = (out: Float32Array) =>
  Array.from({ length: out[0] }, (_, k) => ({
    x: out[4 + k * 8],
    radius: 1 / Math.sqrt(out[4 + k * 8 + 3]),
    rgb: Array.from(out.subarray(4 + k * 8 + 4, 4 + k * 8 + 7)),
  }));

test("every light in view is packed while they fit, with the list's wrap", () => {
  const list = createCastLightList();
  list.wrap = 0.5;
  offerCastLight(list, 1, 2, 3, 4, [1, 0.5, 0.25], 8, "flash:rifle");
  offerCastLight(list, 9, 0, 0, 2, WHITE, 1, "round:rifle");
  // Dark, or reaching nothing: not a light.
  offerCastLight(list, 9, 0, 0, 2, WHITE, 0, "round:rifle");
  offerCastLight(list, 9, 0, 0, 0, WHITE, 5, "round:rifle");
  const out = image();
  expect(pack(out, list)).toBe(2);
  expect(out[1]).toBe(0.5);
  const [a, b] = packed(out);
  expect(a.x).toBe(1);
  expect(a.radius).toBeCloseTo(4, 5);
  expect(a.rgb).toEqual([8, 4, 2]);
  expect(b.x).toBe(9);
});

test("a light whose reach never enters the view costs the world nothing", () => {
  const list = createCastLightList();
  // Off to the east, well past the view's side: its pool can't be seen.
  offerCastLight(list, 400, 0, 1, 16, WHITE, 40, "flash:tank_ap");
  // Its centre past the side too, but its pool reaches in.
  offerCastLight(list, 80, 0, 1, 60, WHITE, 40, "blast");
  const out = image();
  expect(pack(out, list)).toBe(1);
  expect(packed(out)[0].x).toBe(80);
});

test("over the cap, the lights kept are those the camera sees strongest", () => {
  const list = createCastLightList();
  // A crowd of faint tracers round the camera's focus.
  for (let i = 0; i < CAST_LIGHTS_MAX + 20; i++)
    offerCastLight(list, i * 0.1, 0, 1, 2, WHITE, 1, "round:rifle");
  // A tank's flash in the middle of them.
  offerCastLight(list, 3, 0, 2, 16, WHITE, 40, "flash:tank_ap");
  const out = image();
  expect(pack(out, list)).toBe(CAST_LIGHTS_MAX);
  const kept = packed(out);
  expect(kept.some((l) => l.x === 3 && l.radius > 15)).toBe(true);
  // No light twice.
  expect(new Set(kept.map((l) => `${l.x}:${l.radius}`)).size).toBe(CAST_LIGHTS_MAX);
});

test("a full candidate list drops what does not fit and counts it", () => {
  const list = createCastLightList(3);
  for (let i = 0; i < 5; i++) offerCastLight(list, i, 0, 0, 1, WHITE, 1, "round:rifle");
  expect(list.count).toBe(3);
  expect(list.dropped).toBe(2);
});
