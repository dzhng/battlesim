// @vitest-environment node
import { expect, test } from "vitest";
import {
  BATCH_WORDS,
  CAMERA_WORDS,
  FrameHeader,
  NONE,
  frameViews,
} from "@packages/battle-renderer/src/records.gen";
import { copyPresentationFrame } from "../src/battle/render/publication";

test("a copied presentation remains usable after its WASM memory grows and its next frame rewrites the streams", () => {
  const memory = new WebAssembly.Memory({ initial: 1 });
  const header = new Uint32Array(memory.buffer, 0, 28);
  header[FrameHeader.tick] = 14;
  header[FrameHeader.batches] = 256;
  header[FrameHeader.batches + 1] = 1;
  header[FrameHeader.camera] = 512;
  header[FrameHeader.camera + 1] = 1;
  const batch = new Uint32Array(memory.buffer, 256, BATCH_WORDS);
  batch.set([7, 1, 0, NONE]);
  const camera = new Float32Array(memory.buffer, 512, CAMERA_WORDS);
  camera[0] = 1.25;
  const captured = copyPresentationFrame(frameViews(memory.buffer, 0));
  memory.grow(1);
  new Uint32Array(memory.buffer, 256, BATCH_WORDS).fill(0);
  new Float32Array(memory.buffer, 512, CAMERA_WORDS).fill(-9);
  expect(captured.header[FrameHeader.tick]).toBe(14);
  expect(Array.from(captured.batches.u32.subarray(0, 4))).toEqual([7, 1, 0, NONE]);
  expect(captured.batches.count).toBe(1);
  expect(captured.camera.f32[0]).toBe(1.25);
  expect(captured.batches.f32.buffer).toBe(captured.batches.u32.buffer);
});
