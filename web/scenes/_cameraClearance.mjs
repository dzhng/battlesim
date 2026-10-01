// What the camera scenes share: the geometry that judges a drawn eye against
// the buildings drawn (apart from the resolver's own index), and scripted
// camera moves through a town flown by the viewport's own camera.
import { writeFile } from "node:fs/promises";
import { lab } from "./_lab.mjs";

/** The distance from `p` to the nearest of `boxes` (0 inside one). */
export function gap(p, boxes) {
  let nearest = Infinity;
  for (const b of boxes) {
    const [cos, sin] = [Math.cos(b.yaw), Math.sin(b.yaw)];
    const [dx, dy] = [p[0] - b.center[0], p[1] - b.center[1]];
    nearest = Math.min(
      nearest,
      Math.hypot(
        Math.max(0, Math.abs(dx * cos + dy * sin) - b.half[0]),
        Math.max(0, Math.abs(dy * cos - dx * sin) - b.half[1]),
        Math.max(0, b.baseZ - p[2], p[2] - (b.baseZ + 2 * b.half[2])),
      ),
    );
  }
  return nearest;
}

/** The eye of an orbit pose. */
export const eyeOf = (c) => [
  c.target[0] + c.distance * Math.cos(c.pitch) * Math.cos(c.yaw),
  c.target[1] + c.distance * Math.cos(c.pitch) * Math.sin(c.yaw),
  c.target[2] + c.distance * Math.sin(c.pitch),
];

/** The sphere round the eye that holds the near plane's rectangle. */
export const nearEnvelope = (c, aspect) =>
  c.near * Math.sqrt(1 + Math.tan(c.fovY / 2) ** 2 * (1 + aspect ** 2));

/**
 * Two scripted moves through a town, 60 poses a second: a street-level
 * dolly straight across it, and a spiral that turns round the building
 * `box` while zooming from above the roofs down to the closest zoom.
 */
export function townMoves(town, box) {
  const dolly = Array.from({ length: 481 }, (_, k) => ({
    target: [town[0] - 160 + (320 * k) / 480, town[1]],
    distance: 30,
    pitch: 0.3,
    yaw: -Math.PI / 2,
  }));
  const spiral = Array.from({ length: 481 }, (_, k) => {
    const t = k / 480;
    return {
      target: [box.center[0], box.center[1]],
      distance: 140 * (25 / 140) ** t,
      pitch: 0.85 + (0.22 - 0.85) * t,
      yaw: -Math.PI / 2 + 2 * Math.PI * t,
    };
  });
  return { dolly, spiral };
}

/**
 * `poses[from..to]` (60 a second) flown in real time by the viewport's own
 * camera: what it drew each animation frame, and beside it the pose it had
 * been asked for. The camera is left still on the last pose drawn.
 */
export const flyLive = (page, poses, from = 0, to = poses.length - 1) =>
  lab(
    page,
    async ([poses, from, to]) => {
      const frames = [];
      const start = performance.now();
      window.__lab.placeCamera(poses[from]);
      for (;;) {
        // The viewport's own frame runs first: it draws the pose placed last.
        await new Promise(requestAnimationFrame);
        const c = window.__lab.clearance();
        frames.push({
          camera: window.__lab.camera(),
          target: c.asked.target,
          hold: c.hold,
          blocked: c.blocked,
        });
        const k = from + Math.floor(((performance.now() - start) / 1000) * 60);
        if (k > to) break;
        window.__lab.placeCamera(poses[k]);
      }
      window.__lab.setCamera(window.__lab.camera());
      return frames;
    },
    [poses, from, to],
  );

/** How long before a matched frame its flight starts, in poses: time enough
 *  for the camera to be where a whole flight would have it. */
const RUN_UP = 150;

/**
 * A film of `poses` flown live: the frame drawn at every `step`th pose from
 * `from` to `to`, each reached by its own run-up, saved as
 * `film-<name>-<pose>.png`. Motion is judged from these in order.
 */
export async function film(ctx, page, name, poses, from, to, step) {
  for (let k = Math.max(0, from); k <= Math.min(to, poses.length - 1); k += step) {
    await flyLive(page, poses, Math.max(0, k - RUN_UP), k);
    await lab(page, () => window.__lab.frame());
    await writeFile(
      ctx.evidencePath(`film-${name}-${String(k).padStart(3, "0")}.png`),
      await page.screenshot(),
    );
  }
}

/**
 * Fly the two town moves with the viewport's own camera over `boxes` (the
 * buildings drawn), in real time, and judge each drawn eye against every box.
 * Saves `shots` matched frames a move of the pose asked for (raw: the camera
 * as it was before clearance) and the pose drawn, and returns what was
 * measured. The battle must be paused; the camera is left as it was.
 */
export async function flyTown(ctx, page, town, boxes, { shots = 4, reps = 20 } = {}) {
  const box = boxes.reduce((a, b) =>
    Math.hypot(a.center[0] - town[0], a.center[1] - town[1]) <=
    Math.hypot(b.center[0] - town[0], b.center[1] - town[1])
      ? a
      : b,
  );
  const before = await lab(page, () => window.__lab.camera());
  const aspect = await lab(page, () => {
    const canvas = document.querySelector("canvas");
    return canvas.width / canvas.height;
  });
  const envelope = nearEnvelope(before, aspect);
  await lab(page, () => window.__lab.suppressFog(true));
  const moves = townMoves(town, box);
  const out = { envelope, boxes: boxes.length, moves: {} };
  for (const [name, poses] of Object.entries(moves)) {
    // The whole move, by the viewport's rig and obstacles, drawing nothing:
    // every pose's drawn eye, and what resolving it cost.
    const flown = await lab(
      page,
      ([poses, reps]) => {
        const { frames, msPerFrame, boxTestsPerFrame } = window.__lab.flyClearance(
          poses,
          1 / 60,
          reps,
        );
        return {
          frames: frames.map((f) => ({
            camera: f.camera,
            hold: f.hold,
            cut: f.cut,
            blocked: f.blocked,
          })),
          msPerFrame,
          boxTestsPerFrame,
        };
      },
      [poses, reps],
    );
    const asked = await lab(
      page,
      (poses) =>
        poses.map((p) => [
          p.target[0] + p.distance * Math.cos(p.pitch) * Math.cos(p.yaw),
          p.target[1] + p.distance * Math.cos(p.pitch) * Math.sin(p.yaw),
          window.__lab.route.surfaceZ(p.target[0], p.target[1]) + p.distance * Math.sin(p.pitch),
        ]),
      poses,
    );
    const inside = asked.map((eye) => gap(eye, boxes) < envelope);
    const flownGaps = flown.frames.map((f) => gap(eyeOf(f.camera), boxes));
    // The same move with the viewport's camera itself, in real time.
    const live = await flyLive(page, poses);
    const liveGaps = live.map((f) => gap(eyeOf(f.camera), boxes));
    const drawnGap = Math.min(...liveGaps);
    const nearest = live[liveGaps.indexOf(drawnGap)];
    const moved = live.filter(
      (f) => f.blocked || f.camera.target.some((v, i) => Math.abs(v - f.target[i]) > 1e-9),
    ).length;
    // Matched frames, spread over the poses that ask for an eye inside a
    // building: the pose drawn, then the pose asked for, raw (where the
    // camera stood before clearance).
    const marks = inside.map((is, k) => (is ? k : -1)).filter((k) => k >= 0);
    const pick = new Set(
      Array.from(
        { length: shots },
        (_, n) => marks[Math.floor(((n + 0.5) * marks.length) / shots)],
      ),
    );
    for (const k of pick) {
      await flyLive(page, poses, Math.max(0, k - RUN_UP), k);
      await lab(page, () => window.__lab.frame());
      await writeFile(
        ctx.evidencePath(`camera-${name}-${String(k).padStart(3, "0")}-drawn.png`),
        await page.screenshot(),
      );
      await lab(
        page,
        async (pose) => {
          const [x, y] = pose.target;
          window.__lab.setCamera({
            ...window.__lab.camera(),
            ...pose,
            target: [x, y, window.__lab.route.surfaceZ(x, y)],
          });
          await window.__lab.frame();
        },
        poses[k],
      );
      await writeFile(
        ctx.evidencePath(`camera-${name}-${String(k).padStart(3, "0")}-asked-raw.png`),
        await page.screenshot(),
      );
    }
    out.moves[name] = {
      poses: poses.length,
      askedInside: inside.filter(Boolean).length,
      flownGap: Math.min(...flownGaps),
      liveFrames: live.length,
      drawnGap,
      /** The live frame that came nearest a building. */
      nearest: { frame: liveGaps.indexOf(drawnGap), hold: nearest.hold, camera: nearest.camera },
      moved,
      blocked: flown.frames.filter((f) => f.blocked).length,
      cuts: flown.frames.filter((f) => f.cut).length,
      holds: [...new Set(flown.frames.map((f) => f.hold))],
      usPerFrame: flown.msPerFrame * 1000,
      boxTestsPerFrame: flown.boxTestsPerFrame,
      shots: [...pick],
    };
    // `CAMERA_FILM=1`: the stretch round the first building met, five frames a second.
    if (process.env.CAMERA_FILM === "1")
      await film(ctx, page, name, poses, marks[0] - 96, marks[0] + 144, 12);
  }
  await lab(page, (camera) => window.__lab.setCamera(camera), before);
  await lab(page, () => window.__lab.suppressFog(false));
  return out;
}
