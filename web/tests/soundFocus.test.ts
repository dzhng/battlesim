// @vitest-environment node
// How far the camera hears a sound: it listens at its eye, and it is a
// directional mic, so what it looks at sounds nearer (louder and clearer)
// than the same sound off screen or behind it, though still farther than
// one beside it; a battle on the horizon is muffled, and the side's own
// shots never fall under the floor.
import { beforeAll, expect, test } from "vitest";
import { presentationSettings } from "@web/presentationSettings";
import { loadPresentation } from "@web/battle/sim/presentation";
import { cameraListener } from "@packages/battle-audio/src/battleAudio";
import type { Listener } from "@packages/battle-audio/src/soundFrame";
import { AUDIO, DT, STILL, firefight, heard, setup, shot } from "./support/soundScene";

beforeAll(() => loadPresentation());

/** The camera `distance` metres out at `pitch`, looking at `target` (the
 *  fixture's rig), heard as the battle hears it. */
function cameraAt(target: number[], distance: number, pitch?: number): Listener {
  const cam = presentationSettings.camera;
  pitch ??= cam.pitch_curve[cam.pitch_curve.length - 1][1];
  const rig = { target: [target[0], target[1], 0], distance, pitch, yaw: -1.57 };
  return cameraListener(rig as unknown as Parameters<typeof cameraListener>[0]);
}

/** One blue rifleman's shots at `at`, heard from `listener`: the positional
 *  transients started. */
function heardFrom(listener: Listener, at: number[]) {
  const { sink, frame } = setup();
  for (let tick = 1; tick <= 10; tick++) {
    frame.note({ ...heard(firefight(tick, [{ key: 2, at }])), audible: [] });
    sink.time = tick * DT;
    frame.update(tick * DT, tick * DT, STILL, listener);
  }
  return sink.started.filter((v) => v.position !== null && !v.loop);
}

/** A typical RTS camera height: most players never zoom in from here. */
const RTS_DISTANCE_M = 500;

test("a shot the camera looks at sounds at RTS height nearly as it does zoomed in", () => {
  const at = [200, 300, 1];
  const report = shot("rifle").shot;
  const close = presentationSettings.camera.default.distance;
  const [zoomedIn, zoomedOut] = [close, RTS_DISTANCE_M].map((d) =>
    heardFrom(cameraAt(at, d), at).filter((v) => v.sound === report.near),
  );
  expect(zoomedIn.length).toBeGreaterThan(0);
  expect(zoomedOut.length).toBe(zoomedIn.length);
  zoomedOut.forEach((v, i) => {
    // As loud, and only a little duller.
    expect(v.gain).toBeGreaterThan(zoomedIn[i].gain * 0.9);
    expect(v.lowpass).toBeGreaterThan(AUDIO.air.near_hz / 2);
  });
});

test("the camera is a directional mic: what it looks at is louder and clearer than the same shot behind it", () => {
  const at = [200, 300, 1];
  const seen = cameraAt(at, RTS_DISTANCE_M);
  const behind = {
    position: seen.position,
    forward: seen.forward.map((v) => -v) as typeof seen.forward,
    look: seen.look!.map((v) => -v) as typeof seen.forward,
  };
  const loudest = (l: Listener) => {
    const v = heardFrom(l, at);
    return v.reduce((a, b) => (b.gain > a.gain ? b : a));
  };
  const [front, back] = [loudest(seen), loudest(behind)];
  expect(front.gain).toBeGreaterThan(back.gain * 1.5);
  expect(front.lowpass).toBeGreaterThan(back.lowpass);
  // Still farther than a shot beside the camera.
  const beside = loudest({ ...behind, position: [at[0], at[1] - 5, 6] });
  expect(front.lowpass).toBeLessThan(beside.lowpass);
});

test("looking at a battle on the horizon from high up, every shot is muffled", () => {
  // About the view of a town 2 km off, looked at 9° below level from 320 m up.
  const at = [200, 300, 1];
  const report = shot("rifle").shot;
  const heard = heardFrom(cameraAt(at, 2000, 0.16), at).filter((v) => v.sound === report.far);
  expect(heard.length).toBeGreaterThan(0);
  for (const v of heard) {
    expect(v.lowpass).toBeLessThan(AUDIO.air.near_hz / 4);
    expect(v.gain).toBeLessThan(report.gain / 2);
    expect(v.gain).toBeGreaterThanOrEqual(AUDIO.distance.weapons.floor * report.gain - 1e-9);
  }
});

test("a shot far across the map sounds far: muffled, wet, softened, never under the floor", () => {
  const at = [200, 300, 1];
  const report = shot("rifle").shot;
  const far = heardFrom(cameraAt([at[0], at[1] - 800], RTS_DISTANCE_M), at).filter(
    (v) => v.sound === report.far,
  );
  const close = heardFrom(cameraAt(at, RTS_DISTANCE_M), at).filter((v) => v.sound === report.near);
  expect(far.length).toBeGreaterThan(0);
  expect(close.length).toBeGreaterThan(0);
  for (const v of far) {
    expect(v.gain).toBeGreaterThanOrEqual(AUDIO.distance.weapons.floor * report.gain - 1e-9);
    expect(v.gain).toBeLessThan(close[0].gain);
    expect(v.lowpass).toBeLessThan(Math.min(...close.map((c) => c.lowpass)));
    expect(v.wet).toBeGreaterThan(close[0].wet);
    expect(v.attack).toBeGreaterThan(0);
  }
});
