// @vitest-environment node
import { audioMix } from "@web/audioMix";
import { afterEach, beforeAll, expect, test, vi } from "vitest";
import { loadPresentation } from "@web/battle/sim/presentation";
import type { AudioMix } from "@packages/battle-audio/src/audioMix";
import type { SoundCatalog } from "@packages/battle-audio/src/catalog";
import type { SoundBank } from "@packages/battle-audio/src/soundBank";
import { WebAudioSink } from "@packages/battle-audio/src/webAudioSink";
import { sectorPan } from "@packages/battle-audio/src/soundFrame";
import { cameraListener } from "@packages/battle-audio/src/battleAudio";
import { eyePosition, type Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { vec3 } from "math";

beforeAll(() => loadPresentation());
afterEach(() => vi.unstubAllGlobals());

test("a cue's stereo pan follows the camera: east is right when looking north", () => {
  const north = [0, 1, 0];
  expect(sectorPan(0, north)).toBeCloseTo(1);
  expect(sectorPan(4, north)).toBeCloseTo(-1);
  expect(sectorPan(2, north)).toBeCloseTo(0);
  // Turn to look east: north is now on the left.
  expect(sectorPan(2, [1, 0, 0])).toBeCloseTo(-1);
});

test("the listener stands at the camera's eye and looks where it looks", () => {
  for (const camera of [
    { target: [100, 200, 0] as [number, number, number], distance: 600, pitch: 0.85, yaw: 0 },
    { target: [0, 0, 0] as [number, number, number], distance: 2000, pitch: 0.16, yaw: 1 },
  ]) {
    const eye = eyePosition(vec3.create(), camera as unknown as Camera3DParams);
    const { position, look } = cameraListener(camera);
    for (const i of [0, 1, 2]) expect(position[i]).toBeCloseTo(eye[i], 6);
    // It looks where the camera looks: from the eye toward its target.
    const to = [0, 1, 2].map((i) => camera.target[i] - eye[i]);
    const length = Math.hypot(...to);
    for (const i of [0, 1, 2]) expect(look![i]).toBeCloseTo(to[i] / length, 6);
  }
});

test("cameraListener forward vector depends on yaw", () => {
  const camera1 = {
    target: [0, 0, 0] as [number, number, number],
    distance: 100,
    pitch: -Math.PI / 4,
    yaw: 0,
  };
  const camera2 = {
    target: [0, 0, 0] as [number, number, number],
    distance: 100,
    pitch: -Math.PI / 4,
    yaw: Math.PI / 2,
  };
  const forward1 = cameraListener(camera1).forward;
  const forward2 = cameraListener(camera2).forward;
  expect(Math.hypot(forward1[0], forward1[1])).toBeGreaterThan(0);
  expect(Math.hypot(forward2[0], forward2[1])).toBeGreaterThan(0);
  expect(forward1[0]).not.toBeCloseTo(forward2[0]);
});

test("the browser's master limiter and clip are the presentation's limiter", () => {
  const made: Record<string, unknown[]> = {};
  for (const name of ["DynamicsCompressorNode", "WaveShaperNode", "GainNode", "ConvolverNode"])
    vi.stubGlobal(
      name,
      class {
        constructor(_: unknown, options: unknown) {
          (made[name] ??= []).push(options);
        }
        connect(node: unknown) {
          return node;
        }
      },
    );
  const context = {
    sampleRate: 8000,
    destination: {},
    createBuffer: () => ({ copyToChannel() {} }),
  } as unknown as BaseAudioContext;
  const limiter = { threshold_db: -9, attack_s: 0.004, release_s: 0.3, ratio: 8, clip_db: -3 };
  const presentation = {
    ...(audioMix as unknown as AudioMix),
    limiter,
  };
  new WebAudioSink(context, presentation, {} as SoundCatalog, {} as SoundBank);

  expect(made.DynamicsCompressorNode).toEqual([
    { threshold: -9, knee: 0, ratio: 8, attack: 0.004, release: 0.3 },
  ]);
  const [{ curve }] = made.WaveShaperNode as { curve: Float32Array }[];
  const ceiling = 10 ** (-3 / 20);
  expect(Math.max(...curve)).toBeCloseTo(ceiling, 6);
  expect(Math.min(...curve)).toBeCloseTo(-ceiling, 6);
  // Below the ceiling the signal passes unchanged.
  const quiet = curve.findIndex((y) => y > -0.5);
  expect(curve[quiet]).toBeCloseTo((2 * quiet) / (curve.length - 1) - 1, 6);
});
