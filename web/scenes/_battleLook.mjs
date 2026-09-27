// Battle-look slice 27 (lane 27b): the village as a game.
//
// - `battle`: the whole-battle frames the composed look is judged on. Blue is
//   played by `scout-suppress-flank` (`/battle/village?script=`), the battle
//   stepped to BATTLE_TICK (default 11400: 6:20, blue's line in contact),
//   and each named frame posed from the battle's own state: blue's front,
//   the known wreck, the densest known craters. 1920×1080, DPR 1, fixed seed.
// - `woods`: a squad sent into the west wood is drawn through the canopy as
//   an x-ray, and a squad in the open is not.
// - `cleanup`: repeated reset and remounts (a new seed) leave nothing behind:
//   GPU allocations, devices, workers, audio contexts, listeners, effects,
//   corpses and sound voices.
//
// Rerun: `VILLAGE_TOURS=battle bun run --cwd web scene -- village` (also
// `woods`, `cleanup`); `BATTLE_TICK=<tick>` moves the battle frames.
import { readFile } from "node:fs/promises";
import { lab, obs, advance, snapshot } from "./_lab.mjs";
import { decode } from "./_png.mjs";
import { trackPageResources, pageResources } from "./_leaks.mjs";

const village = JSON.parse(
  await readFile(new URL("../../fixtures/village.json", import.meta.url), "utf8"),
);
const CAMERA = village.presentation.camera;
const BATTLE_TICK = Number(process.env.BATTLE_TICK ?? 11400);
const VIEWPORT = { width: 1920, height: 1080 };
const COMBAT = new Set(["rifle", "recon", "at", "tank", "jeep"]);
const HIDE_READOUTS = ".ro-unit, .ro-goal { display: none !important; }";

const inRect = (p, [x, y, w, h]) => p[0] > x && p[0] < x + w && p[1] > y && p[1] < y + h;
const dist = (a, b) => Math.hypot(a[0] - b[0], a[1] - b[1]);

/** The camera curve's pitch at `distance` (the controller's own rule). */
function curvePitch(distance) {
  const curve = CAMERA.pitch_curve;
  if (distance <= curve[0][0]) return curve[0][1];
  for (let k = 1; k < curve.length; k++)
    if (distance <= curve[k][0]) {
      const [[d0, p0], [d1, p1]] = [curve[k - 1], curve[k]];
      return p0 + ((p1 - p0) * (distance - d0)) / (d1 - d0);
    }
  return curve.at(-1)[1];
}

const pose = (page, at, distance, pitch = curvePitch(distance), yaw = CAMERA.default.yaw) =>
  lab(
    page,
    (c) =>
      window.__lab.setCamera({
        ...window.__lab.camera(),
        distance: c.distance,
        pitch: c.pitch,
        yaw: c.yaw,
        target: [c.at[0], c.at[1], window.__lab.route.surfaceZ(c.at[0], c.at[1])],
      }),
    { at, distance, pitch, yaw },
  );

async function overlayOnly(ctx, page, name) {
  await lab(page, () => window.__lab.setFrameView("overlays-on-black"));
  const png = decode(await snapshot(ctx, page, name));
  await lab(page, () => window.__lab.setFrameView("final"));
  return png;
}

/** The whole-battle frames (slice 27's visual acceptance). */
export async function battleTour(ctx) {
  const page = await ctx.newPage({ viewport: VIEWPORT });
  await ctx.openLab(page, `${ctx.url}?script=scout-suppress-flank`);
  await page.waitForFunction(() => window.__lab.route?.tick() > 3, undefined, { timeout: 30000 });
  await page.waitForFunction(() => window.__lab.stats?.().grass.enabled, undefined, {
    timeout: 30000,
  });
  await lab(page, () => window.__lab.route.pause());
  await advance(page, BATTLE_TICK - (await lab(page, () => window.__lab.route.tick())));
  const o = await obs(page);
  const fighters = o.own.filter((u) => COMBAT.has(u.kind));
  ctx.check(
    `the battle stands at tick ${BATTLE_TICK} with blue fighting`,
    o.tick === BATTLE_TICK && fighters.length > 0,
    JSON.stringify({ tick: o.tick, fighters: fighters.length }),
  );
  if (!fighters.length) return page.close();
  const zone = village.encounter.success_zone_center;
  const front = fighters.reduce((a, b) =>
    dist(a.position, zone) <= dist(b.position, zone) ? a : b,
  );
  const line = [
    fighters.reduce((n, u) => n + u.position[0], 0) / fighters.length,
    fighters.reduce((n, u) => n + u.position[1], 0) / fighters.length,
  ];
  const battle = [(line[0] + zone[0]) / 2, (line[1] + zone[1]) / 2];
  const wreck = o.knownProps.find((p) => p.kind.endsWith("_wreck"));
  const craters = await lab(page, () => window.__lab.route.craters());
  const facing = Math.atan2(zone[1] - front.position[1], zone[0] - front.position[0]);
  const frames = {
    strategic: [battle, 1100],
    line: [line, 240],
    default: [front.position, CAMERA.default.distance],
    ground: [front.position, CAMERA.zoom_min, curvePitch(CAMERA.zoom_min), facing - Math.PI / 2],
    ...(wreck && { wreck: [wreck.center, CAMERA.default.distance] }),
    ...(craters?.densest && {
      craters: [craters.densest, CAMERA.default.distance],
      "craters-low": [craters.densest, 45, 0.3],
    }),
  };
  const shot = {};
  for (const [name, [at, distance, pitch, yaw]] of Object.entries(frames)) {
    await pose(page, at, distance, pitch, yaw);
    shot[name] = await snapshot(ctx, page, `battle-${name}-1920x1080.png`);
  }
  // The same frames without the panel and readouts: what the references show.
  await page.addStyleTag({
    content: `${HIDE_READOUTS} [data-testid=battle-panel] { display: none !important; }`,
  });
  for (const [name, [at, distance, pitch, yaw]] of Object.entries(frames)) {
    await pose(page, at, distance, pitch, yaw);
    await snapshot(ctx, page, `battle-${name}-clean-1920x1080.png`);
  }
  await ctx.writeEvidence("battle-frames.json", {
    tick: BATTLE_TICK,
    seed: village.seed,
    script: "scout-suppress-flank",
    frames,
    craters,
    wreck: wreck ?? null,
    rounds: o.projectiles.length,
    corpses: o.corpses.length,
  });
  ctx.check(
    "the battle frames show fire, a wreck and craters",
    o.projectiles.length > 0 && !!wreck && (craters?.cells ?? 0) > 0,
    JSON.stringify({ rounds: o.projectiles.length, wreck: wreck?.kind, craters }),
  );
  await page.close();
  return shot;
}

/** The squad's soldiers whose torso is in view, as page points. */
async function torsos(page, members) {
  const out = [];
  for (const m of members) {
    const p = await lab(page, (q) => window.__lab.projectToCss(q[0], q[1], q[2] + 1), m);
    if (p && p[0] > 440 && p[1] > 8 && p[0] < VIEWPORT.width - 8 && p[1] < VIEWPORT.height - 8)
      out.push(p);
  }
  return out;
}

/** Whether the x-ray's pale blue lies within 3 px of `p` (overlay-only view). */
function xrayAt(png, p) {
  for (let dy = -3; dy <= 3; dy++)
    for (let dx = -3; dx <= 3; dx++) {
      const i = ((Math.round(p[1]) + dy) * png.width + Math.round(p[0]) + dx) * 4;
      const [r, g, b] = [png.data[i], png.data[i + 1], png.data[i + 2]];
      if (b > 60 && b > r + 25 && g > r + 10) return true;
    }
  return false;
}

/** Slice 27: the unit-in-woods cue. A squad walks into the west wood; under
 *  the canopy its soldiers are drawn as an x-ray, and a squad in the open is
 *  drawn plainly. */
export async function woodsTour(ctx) {
  const page = await ctx.newPage({ viewport: VIEWPORT });
  await ctx.openLab(page);
  await page.waitForFunction(() => window.__lab.route?.tick() > 3, undefined, { timeout: 30000 });
  await lab(page, () => window.__lab.route.pause());
  await page.addStyleTag({ content: HIDE_READOUTS });
  const wood = village.map.forests[0].rect;
  const goal = [wood[0] + 40, wood[1] + wood[3] - 40];
  let o = await obs(page);
  const squads = o.own.filter((u) => u.members.length > 0 && u.kind === "rifle");
  // The squad spawned furthest south walks in along the map's south, out of
  // the village's fire; the one spawned furthest north stays in the open.
  const spawns = village.spawn.blue.filter((r) => r[0] === "rifle").map((r) => [r[1], r[2]]);
  const nearest = (p) =>
    squads.reduce((a, b) => (dist(a.position, p) <= dist(b.position, p) ? a : b));
  const walker = nearest(spawns.reduce((a, b) => (a[1] >= b[1] ? a : b)));
  const idle = nearest(spawns.reduce((a, b) => (a[1] <= b[1] ? a : b)));

  // Plain in the open first: the idle squad at the default framing.
  await pose(page, idle.position, CAMERA.default.distance);
  const open = await overlayOnly(ctx, page, "woods-open-overlay.png");
  const plain = await torsos(page, idle.members);
  const plainXray = plain.filter((p) => xrayAt(open, p)).length;
  ctx.check(
    "a squad in the open is drawn without the x-ray",
    plain.length >= 3 && plainXray === 0,
    JSON.stringify({ soldiers: plain.length, xray: plainXray }),
  );

  await lab(
    page,
    (c) =>
      window.__lab.route.command({
        kind: "move",
        units: [c.id],
        gesture: 2701,
        goal: c.goal,
        route: "fastest",
      }),
    { id: walker.id, goal },
  );
  let inside = [];
  for (let t = 0; t < 15000 && inside.length < 3; t += 300) {
    await advance(page, 300);
    o = await obs(page);
    const u = o.own.find((u) => u.id === walker.id);
    inside = u ? u.members.filter((m) => inRect(m, wood)) : [];
  }
  const u = o.own.find((x) => x.id === walker.id);
  if (!u || inside.length < 3) {
    ctx.check(
      "a squad reaches the west wood",
      false,
      JSON.stringify({ tick: o.tick, alive: !!u, inside: inside.length, at: u?.position }),
    );
    return page.close();
  }
  await pose(page, u.position, CAMERA.default.distance);
  await snapshot(ctx, page, "woods-xray-1920x1080.png");
  const png = await overlayOnly(ctx, page, "woods-xray-overlay.png");
  const hidden = await torsos(page, inside);
  const drawn = hidden.filter((p) => xrayAt(png, p)).length;
  ctx.check(
    "under the canopy, the squad's soldiers are drawn through it (x-ray)",
    hidden.length >= 3 && drawn >= Math.ceil(hidden.length / 2),
    JSON.stringify({ tick: o.tick, inWood: hidden.length, xray: drawn }),
  );
  await page.close();
}

/** The frame's GPU allocations, the page's live resources, and what the
 *  battle draws and plays, after a fresh frame. */
async function census(page) {
  await page.evaluate(() => window.__lab.frame());
  // Stopped voices fade out and leave the graph when they end.
  await page.waitForTimeout(1500);
  return {
    gpu: await lab(page, () => window.__lab.allocations()),
    page: await pageResources(page),
    effects: await lab(page, () => window.__lab.route.effects().live),
    models: await lab(page, () => window.__lab.stats().models.instances),
    corpses: await lab(page, () => window.__lab.route.observation()?.corpses.length ?? 0),
    voices: await lab(page, () => window.__lab.route.sound()?.graph ?? 0),
    tick: await lab(page, () => window.__lab.route.tick()),
  };
}

const same = (a, b) =>
  a.gpu.buffers === b.gpu.buffers &&
  a.gpu.bufferBytes === b.gpu.bufferBytes &&
  a.gpu.textures === b.gpu.textures &&
  a.gpu.textureBytes === b.gpu.textureBytes &&
  JSON.stringify(a.page) === JSON.stringify(b.page);

/** Slice 27: reset and remount, again and again, leave nothing behind. */
export async function cleanupTour(ctx) {
  const page = await ctx.newPage({ viewport: VIEWPORT });
  await trackPageResources(page);
  await ctx.openLab(page);
  await page.waitForFunction(() => window.__lab.route?.tick() > 3, undefined, { timeout: 30000 });
  await lab(page, () => window.__lab.route.pause());
  // A click on open ground: the gesture that starts the battle's sound.
  await page.mouse.click(1300, 900);
  const fight = async () => {
    await lab(page, () => {
      const o = window.__lab.route.observation();
      window.__lab.route.command({
        kind: "attack_move",
        units: o.own.map((u) => u.id),
        gesture: 1,
        goal: [1000, 800],
      });
    });
    await advance(page, 2400);
    await lab(
      page,
      (at) => window.__lab.setCamera({ ...window.__lab.camera(), target: at }),
      [400, 820, 0],
    );
  };
  const reset = async () => {
    await page.getByRole("button", { name: "Reset" }).click();
    await page.waitForFunction(
      () =>
        window.__lab.route.acks().length === 0 &&
        window.__lab.route.tick() < 60 &&
        window.__lab.route.observation() !== null,
      undefined,
      { timeout: 30000 },
    );
    await lab(page, () => window.__lab.route.pause());
    await lab(page, () => window.__lab.reset());
  };

  const quiet = await census(page);
  await fight();
  const fought = await census(page);
  const cycles = [];
  for (let k = 0; k < 4; k++) {
    await reset();
    const fresh = await census(page);
    await fight();
    cycles.push({ fresh, fought: await census(page) });
  }
  await ctx.writeEvidence("cleanup-reset.json", { quiet, fought, cycles });
  ctx.check(
    "reset clears the battle's effects, corpses and sound voices",
    fought.effects > 0 &&
      fought.corpses > 0 &&
      cycles.every(
        (c) => c.fresh.effects === 0 && c.fresh.corpses === 0 && c.fresh.voices === quiet.voices,
      ),
    JSON.stringify({ quiet, fought, fresh: cycles.map((c) => c.fresh) }),
  );
  // After the first cycle (buffers grown to the fight's size), every reset
  // and every fight holds the same allocations and live resources.
  ctx.check(
    "four resets and fights hold GPU allocations, workers, audio and listeners steady",
    cycles
      .slice(1)
      .every((c) => same(c.fresh, cycles[0].fresh) && same(c.fought, cycles[0].fought)) &&
      cycles.every((c) => c.fresh.page.devices === 1 && c.fresh.page.workers === 1),
    JSON.stringify(
      cycles.map((c) => ({ fresh: c.fresh.gpu, fought: c.fought.gpu, page: c.fresh.page })),
    ),
  );

  // A new seed remounts the whole battle view: a new device, worker and
  // sound, the old ones released.
  const remounts = [];
  for (const seed of [7, 8, 7, 8]) {
    await page.getByLabel("Seed").fill(String(seed));
    await page.waitForFunction(
      (s) =>
        new RegExp(`seed ${s} `).test(
          document.querySelector("[data-testid=status]")?.textContent ?? "",
        ) &&
        window.__lab?.ready &&
        window.__lab.route?.tick() > 3,
      seed,
      { timeout: 60000 },
    );
    await lab(page, () => window.__lab.route.pause());
    await page.mouse.click(1300, 900);
    remounts.push(await census(page));
  }
  await ctx.writeEvidence("cleanup-remount.json", remounts);
  ctx.check(
    "remounting the battle four times leaves one device, one worker, at most one audio context, and steady listeners and allocations",
    remounts.every(
      (r) => r.page.devices === 1 && r.page.workers === 1 && r.page.audioContexts <= 1,
    ) &&
      same(remounts[2], remounts[0]) &&
      same(remounts[3], remounts[1]) &&
      remounts[1].page.listeners === remounts[0].page.listeners,
    JSON.stringify(remounts.map((r) => ({ gpu: r.gpu, page: r.page }))),
  );
  await page.close();
}
