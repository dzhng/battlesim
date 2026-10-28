// What the wave field does with each frame's packet (`Presentation.waves`):
// a new epoch or a battle gone clears it, even in a frame with no step, so
// one side's waves never linger for the next; and every texel the camera
// moves reaches the field, whether or not the frame steps it.
import { expect, test } from "vitest";
import { wavePlan } from "@packages/battle-renderer/src/frame/waveField";

const packet = (steps: number, epoch: number, sx: number, sy = 0) =>
  Float32Array.of(steps, epoch, sx, sy);

test("a new epoch clears the field, even in a frame with no step", () => {
  const plan = wavePlan(packet(0, 8, 0), 7);
  expect(plan.clear).toBe(true);
  expect(plan.epoch).toBe(8);
});

test("a battle gone clears the field once", () => {
  const gone = wavePlan(new Float32Array(0), 7);
  expect(gone.clear).toBe(true);
  expect(gone.runs).toEqual([]);
  expect(wavePlan(new Float32Array(0), gone.epoch).clear).toBe(false);
});

test("every texel the camera moves reaches the field, stepped or not", () => {
  let epoch: number | null = 3;
  let moved = 0;
  // Panning 8 texels a frame at twice the step rate: a step every other frame.
  for (const steps of [0, 1, 0, 1, 0, 0]) {
    const plan = wavePlan(packet(steps, 3, 8), epoch);
    epoch = plan.epoch;
    expect(plan.clear).toBe(false);
    moved += plan.runs.reduce((sum, r) => sum + r.shift[0], 0);
  }
  expect(moved).toBe(48);
});

test("only a frame's first step carries its shift, and a still run never steps", () => {
  const plan = wavePlan(packet(3, 3, 5, -2), 3);
  expect(plan.runs.map((r) => [r.shift, r.still])).toEqual([
    [[5, -2], false],
    [[0, 0], false],
    [[0, 0], false],
  ]);
  expect(wavePlan(packet(0, 3, 5, -2), 3).runs).toEqual([{ shift: [5, -2], step: 0, still: true }]);
});
