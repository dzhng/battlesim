// Camera clearance: every scripted trajectory round the lab's buildings keeps
// the eye and its near plane out of them, by the policy's own order (a lift,
// a slide, then pushback), without jarring moves; the viewport's own camera
// does the same when it rides them; and an idle camera recovers when the side
// sees a building fall. Evidence: the two eye paths from outside, and matched
// frames of the pose asked for (raw) beside the pose drawn.
import { writeFile } from "node:fs/promises";
import { lab } from "./_lab.mjs";
import { decode } from "./_png.mjs";
import { eyeOf, gap } from "./_cameraClearance.mjs";

/** Cuts each trajectory is allowed: one past each tower it crosses (too tall
 *  to go over), one per framing the placement trajectory cuts to. */
const CUTS = { "tower-pass": 1, "tower-orbit": 1, placement: 5 };
/** The fastest the drawn eye may move beyond the eye asked for, m/s, outside
 *  a cut: the lab's flights measure 25. */
const EXTRA_SPEED_MAX = 30;
/** What holds the eye off its pose in each trajectory, at least. */
const HOLDS = {
  wall: ["lift"],
  "tower-pass": ["slide", "pushback"],
  "tower-zoom": ["pushback"],
  corner: ["pushback"],
  courtyard: ["lift"],
  compound: ["lift"],
};
/** The trajectories whose drawn eye path is counted in pixels. */
const PATHS_WATCHED = ["wall", "courtyard", "compound"];
const HIDE_PANEL = "[data-testid=camera-panel] { display: none !important; }";

const shot = async (ctx, page, name) => {
  await lab(page, () => window.__lab.frame());
  const png = await page.screenshot();
  await writeFile(ctx.evidencePath(name), png);
  return png;
};

export async function run(ctx) {
  const page = await ctx.newPage({ viewport: { width: 1920, height: 1080 } });
  await ctx.openLab(page);
  await page.waitForFunction(() => window.__lab.route?.boxes);

  const setup = await lab(page, () => ({
    boxes: window.__lab.route.boxes(),
    counts: window.__lab.route.counts(),
    placed: window.__lab.stats().buildings.buildings,
    trajectories: window.__lab.route.trajectories(),
    clearance: window.__lab.route.clearance(),
  }));
  const { boxes, clearance, trajectories } = setup;
  ctx.check(
    "every building of the lab is drawn from its template's rows, and each of their parts is a camera obstacle",
    boxes.length === 11 &&
      setup.counts.buildings > 0 &&
      setup.placed === setup.counts.buildings &&
      trajectories.length === 8,
    JSON.stringify({ boxes: boxes.length, counts: setup.counts, placed: setup.placed, clearance }),
  );

  // Every trajectory, flown frame by frame through the viewport's own rig
  // numbers and obstacles.
  const flights = {};
  const rows = [];
  for (const { id } of trajectories) {
    const frames = await lab(page, (id) => window.__lab.route.flight(id), id);
    flights[id] = frames;
    const drawn = frames.map((f) => gap(eyeOf(f.drawn), boxes));
    const asked = frames.filter((f) => gap(f.askedEye, boxes) < clearance.envelope);
    const cuts = frames.filter((f) => f.cut).length;
    let extra = 0;
    for (let k = 1; k < frames.length; k++) {
      if (frames[k].cut) continue;
      const step = Math.hypot(...frames[k].eye.map((v, i) => v - frames[k - 1].eye[i]));
      const askedStep = Math.hypot(
        ...frames[k].askedEye.map((v, i) => v - frames[k - 1].askedEye[i]),
      );
      extra = Math.max(extra, (step - askedStep) * 60);
    }
    const holds = [...new Set(frames.map((f) => f.hold))].filter((h) => h !== "none");
    rows.push({
      id,
      frames: frames.length,
      askedInside: asked.length,
      minGap: Math.min(...drawn),
      cuts,
      extra,
      holds,
      blocked: frames.filter((f) => f.blocked).length,
      moved: frames.filter(
        (f) =>
          Math.hypot(...f.drawn.target.map((v, i) => v - f.asked.target[i])) > 1e-9 ||
          Math.min(...eyeOf(f.drawn).map((v, i) => Math.abs(v - f.eye[i]))) > 1e-6,
      ).length,
    });
  }
  await ctx.writeEvidence("flights.json", rows);
  for (const r of rows)
    console.log(
      `METRIC camera ${r.id}: ${r.frames} frames, ${r.askedInside} asked inside a building, drawn never nearer than ${r.minGap.toFixed(2)} m, ${r.cuts} cuts, at most ${r.extra.toFixed(1)} m/s over the eye asked for, holds ${r.holds.join("+") || "none"}`,
    );
  ctx.check(
    "every trajectory asks for an eye inside a building, and none is drawn there: the near plane's envelope stays out of every box",
    rows.every(
      (r) =>
        r.askedInside > 10 && r.minGap >= clearance.envelope && r.blocked === 0 && r.moved === 0,
    ),
    rows.map((r) => `${r.id} ${r.askedInside} asked inside, min ${r.minGap.toFixed(2)}`).join("; "),
  );
  ctx.check(
    "nearby poses come before pushback: a lift over what is low, a slide round what is narrow, pushback otherwise",
    Object.entries(HOLDS).every(([id, holds]) => {
      const seen = rows.find((r) => r.id === id).holds;
      return holds.every((h) => seen.includes(h));
    }) && rows.find((r) => r.id === "wall").holds.join() === "lift",
    rows.map((r) => `${r.id}: ${r.holds.join("+")}`).join("; "),
  );
  ctx.check(
    "the drawn eye cuts only past a tower too tall to go over and at a scripted cut, and otherwise never outruns the eye asked for by more than a bound",
    rows.every((r) => r.cuts === (CUTS[r.id] ?? 0)) &&
      rows.every((r) => r.id === "placement" || r.extra <= EXTRA_SPEED_MAX),
    rows.map((r) => `${r.id}: ${r.cuts} cuts, +${r.extra.toFixed(1)} m/s`).join("; "),
  );

  // From outside: both eye paths are drawn.
  await page.addStyleTag({ content: HIDE_PANEL });
  const colours = {};
  for (const { id } of trajectories) {
    await lab(page, (id) => window.__lab.route.watch(id, 4.2), id);
    await shot(ctx, page, `watch-${id}-1920x1080.png`);
    // Where the eye goes over a building its path is long; where a tower
    // holds it, a few pixels.
    if (!PATHS_WATCHED.includes(id)) continue;
    await lab(page, () => window.__lab.setFrameView("overlays-on-black"));
    const overlay = decode(await page.screenshot());
    await lab(page, () => window.__lab.setFrameView("final"));
    let [amber, red, cyan] = [0, 0, 0];
    for (let i = 0; i < overlay.data.length; i += 4) {
      const [r, g, b] = [overlay.data[i], overlay.data[i + 1], overlay.data[i + 2]];
      if (r > 150 && g > 80 && g < 0.85 * r && b < 0.5 * g) amber++;
      else if (r > 150 && g < 0.4 * r && b < 0.4 * r) red++;
      else if (b > 150 && g > 150 && r < 0.6 * b) cyan++;
    }
    colours[id] = { amber, red, cyan };
  }
  ctx.check(
    "watched from outside, the eye path asked for (amber, red inside a building's clearance) and the eye path drawn (cyan) are both on screen",
    Object.values(colours).every((c) => c.amber > 500 && c.red > 100 && c.cyan > 2000),
    JSON.stringify(colours),
  );

  // Ridden: the viewport's own camera flies each in real time, placed and
  // cleared frame by frame.
  const ridden = {};
  for (const id of ["wall", "tower-pass", "courtyard"]) {
    const { seconds } = trajectories.find((t) => t.id === id);
    const frames = await lab(
      page,
      async ([id, seconds]) => {
        window.__lab.route.ride(id);
        const frames = [];
        for (const start = performance.now(); performance.now() - start < seconds * 1000; ) {
          await new Promise(requestAnimationFrame);
          const c = window.__lab.clearance();
          frames.push({
            camera: window.__lab.camera(),
            target: c.asked.target,
            hold: c.hold,
            blocked: c.blocked,
          });
        }
        return frames;
      },
      [id, seconds],
    );
    const eyes = frames.map((f) => eyeOf(f.camera));
    ridden[id] = {
      frames: frames.length,
      minGap: Math.min(...eyes.map((eye) => gap(eye, boxes))),
      top: Math.max(...eyes.map((eye) => eye[2])),
      moved: frames.filter(
        (f) => f.blocked || f.camera.target.some((v, i) => Math.abs(v - f.target[i]) > 1e-9),
      ).length,
      holds: [...new Set(frames.map((f) => f.hold))],
    };
  }
  ctx.check(
    "ridden, the viewport's camera keeps its near plane out of every building, looks where it was asked, and goes over the wall and the courtyard block",
    Object.values(ridden).every(
      (r) => r.frames > 100 && r.minGap >= clearance.envelope && r.moved === 0,
    ) &&
      ridden.wall.holds.includes("lift") &&
      ridden.wall.top > 18.5 + clearance.envelope &&
      ridden.courtyard.top > 18.5 + clearance.envelope &&
      ridden["tower-pass"].holds.includes("slide"),
    JSON.stringify(ridden),
  );

  // Matched frames: the pose asked for, drawn raw (the harness's framing,
  // as before clearance), beside the pose the viewport draws for it.
  const pairs = [
    ["wall", 3.7],
    ["tower-pass", 4.0],
    ["courtyard", 4.6],
    ["corner", 6.0],
    ["placement", 2.0],
  ];
  const paired = [];
  for (const [id, s] of pairs) {
    // Ridden in real time up to the moment, so the drawn pose is the one a
    // flight reaches, then stopped there.
    const { asked, drawn } = await lab(
      page,
      async ([id, s]) => {
        window.__lab.route.ride(id);
        while (window.__lab.route.seconds() < s) await new Promise(requestAnimationFrame);
        window.__lab.route.hold();
        await new Promise(requestAnimationFrame);
        return { asked: window.__lab.clearance().asked, drawn: window.__lab.camera() };
      },
      [id, s],
    );
    await shot(ctx, page, `drawn-${id}-1920x1080.png`);
    await lab(page, (c) => window.__lab.setCamera(c), asked);
    await shot(ctx, page, `asked-raw-${id}-1920x1080.png`);
    paired.push({ id, s, asked: gap(eyeOf(asked), boxes), drawn: gap(eyeOf(drawn), boxes) });
  }
  ctx.check(
    "each matched pair is a pose asked for inside a building's clearance and the pose drawn clear of it",
    paired.every((p) => p.asked < clearance.envelope && p.drawn >= clearance.envelope),
    JSON.stringify(paired),
  );

  // Idle recovery: the player's camera placed inside the tower is drawn
  // clear of it; blue sees the tower fall; with no input the camera comes to
  // rest on the pose asked for.
  const inside = flights.placement[Math.round(2 * 60)].asked;
  await lab(page, async () => {
    window.__lab.route.watch("placement", 0);
    // The lab cuts the free camera to its watching place first.
    await window.__lab.frame();
  });
  const held = await lab(
    page,
    async (pose) => {
      window.__lab.placeCamera(pose);
      await window.__lab.frame();
      return { camera: window.__lab.camera(), clearance: window.__lab.clearance() };
    },
    { target: [inside.target[0], inside.target[1]], ...inside },
  );
  await shot(ctx, page, "tower-standing-1920x1080.png");
  await lab(page, () => window.__lab.route.setFallen(true));
  const recovering = [];
  for (let k = 0; k < 8; k++) {
    await page.waitForTimeout(60);
    recovering.push(await lab(page, () => window.__lab.camera().distance));
  }
  await page.waitForFunction(() => window.__lab.clearance().settled, undefined, { timeout: 5000 });
  const rest = await lab(page, () => ({
    camera: window.__lab.camera(),
    clearance: window.__lab.clearance(),
  }));
  await shot(ctx, page, "tower-seen-fallen-1920x1080.png");
  const same = (a, b) =>
    ["distance", "pitch", "yaw"].every((k) => Math.abs(a[k] - b[k]) < 1e-9) &&
    a.target.every((v, i) => Math.abs(v - b.target[i]) < 1e-9);
  ctx.check(
    "an idle camera held off a tower recovers by itself, in steps, once the side has seen the tower fall",
    held.clearance.hold !== "none" &&
      !same(held.camera, held.clearance.asked) &&
      gap(eyeOf(held.camera), boxes) >= clearance.envelope &&
      new Set(recovering.map((d) => d.toFixed(3))).size >= 3 &&
      rest.clearance.hold === "none" &&
      same(rest.camera, rest.clearance.asked),
    JSON.stringify({
      held: held.clearance.hold,
      heldGap: gap(eyeOf(held.camera), boxes),
      recovering,
      rest: rest.clearance.hold,
    }),
  );
  await lab(page, () => window.__lab.route.setFallen(false));

  // `CAMERA_FILM=1`: the wall and the tower pass as the viewport's camera
  // rides them, five frames a second, each frame reached by a ride from the
  // trajectory's start.
  if (process.env.CAMERA_FILM === "1")
    for (const [id, from, to] of [
      ["wall", 2.2, 5.2],
      ["tower-pass", 2.2, 5.4],
    ])
      for (let s = from; s <= to + 1e-9; s += 0.2) {
        await lab(
          page,
          async ([id, s]) => {
            window.__lab.route.ride(id);
            while (window.__lab.route.seconds() < s) await new Promise(requestAnimationFrame);
            window.__lab.route.hold();
          },
          [id, s],
        );
        await shot(ctx, page, `film-${id}-${s.toFixed(1)}.png`);
      }

  // What the resolver costs on these trajectories.
  const cost = await lab(page, () => {
    const poses = [];
    for (const { id, seconds } of window.__lab.route.trajectories())
      for (const f of window.__lab.route.flight(id).slice(0, seconds * 60))
        poses.push({
          target: [f.asked.target[0], f.asked.target[1]],
          distance: f.asked.distance,
          yaw: f.asked.yaw,
          pitch: f.asked.pitch,
        });
    const { msPerFrame, boxTestsPerFrame } = window.__lab.flyClearance(poses, 1 / 60, 20);
    return { frames: poses.length, msPerFrame, boxTestsPerFrame };
  });
  console.log(
    `METRIC camera resolver: ${(cost.msPerFrame * 1000).toFixed(1)} µs and ${cost.boxTestsPerFrame.toFixed(1)} box tests a frame over ${cost.frames} lab frames (development build)`,
  );
  ctx.check(
    "the resolver's cost is a small share of a frame",
    cost.msPerFrame < 0.25,
    `${(cost.msPerFrame * 1000).toFixed(1)} µs a frame`,
  );
  await ctx.writeEvidence("meta.json", {
    browser: ctx.browser,
    adapter: await page.evaluate(() => window.__lab.adapter),
    viewport: [1920, 1080],
    clearance,
    rows,
    ridden,
    paired,
    cost,
  });
}
