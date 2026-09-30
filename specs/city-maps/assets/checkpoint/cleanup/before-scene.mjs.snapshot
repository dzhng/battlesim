// The village as a game.
//
// On `/battle/village/watch` (scene `village-watch`: blue played by
// `scout-suppress-flank`):
// - `battle`: the whole-battle frames the composed look is judged on, the
//   battle stepped to BATTLE_TICK (default 9900, 5:30) and on to the first
//   tick of a fight, and each named frame
//   posed from the battle's own state: blue's
//   front, any known wreck, the densest known craters. 1920×1080, DPR 1,
//   fixed seed; each also HUD-free.
// - `edge`: fog runs on past the map edge as inside, and a dim white border marks
//   the playable area.
// On `/battle/village` (scene `village`, the player's controls):
// - `woods`: a squad sent into the west wood is drawn through the canopy as
//   an x-ray, and a squad in the open is not.
// - `cleanup`: repeated reset and remounts (a new variant) leave nothing behind:
//   GPU allocations, devices, workers, audio contexts, listeners, effects,
//   corpses and sound voices.
//
// Rerun: `WATCH_TOURS=battle bun run --cwd web scene -- village-watch`
// (`BATTLE_TICK=<tick>` moves the frames); `VILLAGE_TOURS=woods,cleanup bun
// run --cwd web scene -- village`.
import {
  lab,
  obs,
  advance,
  until,
  snapshot,
  restart,
  chooseVariant,
  openBattle,
  aim,
  groundCss,
} from "./_lab.mjs";
import { anyNear, decode, mostChanged } from "./_png.mjs";
import { trackPageResources, pageResources } from "./_leaks.mjs";
import { paintOnly } from "./_overlays.mjs";
import { hasRole, village, curvePitch } from "./_units.mjs";

const CAMERA = village.presentation.camera;
const BATTLE_TICK = Number(process.env.BATTLE_TICK ?? 9900);
const VIEWPORT = { width: 1920, height: 1080 };
// The callout layer: each unit's panel and the leader line joining it to
// the unit. A leader is DOM, in the HUD's cyan, so it survives every canvas
// frame view and would read as x-ray where it ends on a soldier.
const HIDE_READOUTS = ".ro-layer { display: none !important; }";

const inRect = (p, [x, y, w, h]) => p[0] > x && p[0] < x + w && p[1] > y && p[1] < y + h;
const dist = (a, b) => Math.hypot(a[0] - b[0], a[1] - b[1]);

const pose = (page, at, distance, pitch = curvePitch(distance), yaw = CAMERA.default.yaw) =>
  aim(page, at, { distance, pitch, yaw });

async function overlayOnly(ctx, page, name) {
  await lab(page, () => window.__lab.setFrameView("overlays-on-black"));
  const png = decode(await snapshot(ctx, page, name));
  await lab(page, () => window.__lab.setFrameView("final"));
  return png;
}

/** The whole-battle frames (the village's visual acceptance). */
export async function battleTour(ctx) {
  const page = await openBattle(ctx, { viewport: VIEWPORT, tick: BATTLE_TICK, grass: true });
  // Find live fighting rather than requiring a particular casualty: a
  // successful flank can keep every vehicle alive. Dedicated destruction
  // scenes own the mandatory wreck checks; here we frame any wreck present.
  const fightersOf = (o) => o.own.filter((u) => !hasRole(u.kind, "logistics"));
  const wreckOf = (o) => o.knownProps.find((p) => p.kind.endsWith("_wreck"));
  const moment = (o) => o.projectiles.length > 0 && fightersOf(o).length > 0;
  let o = await obs(page);
  if (!moment(o)) o = (await until(page, moment, 30 * 240, 30)) ?? (await obs(page));
  const fighters = fightersOf(o);
  const wreck = wreckOf(o);
  ctx.check(
    `from tick ${BATTLE_TICK}, the battle reaches fire with blue fighting`,
    moment(o),
    JSON.stringify({ tick: o.tick, rounds: o.projectiles.length, fighters: fighters.length }),
  );
  if (!moment(o)) return page.close();
  const zone = village.encounter.success_zone_center;
  const front = fighters.reduce((a, b) =>
    dist(a.position, zone) <= dist(b.position, zone) ? a : b,
  );
  const line = [
    fighters.reduce((n, u) => n + u.position[0], 0) / fighters.length,
    fighters.reduce((n, u) => n + u.position[1], 0) / fighters.length,
  ];
  const battle = [(line[0] + zone[0]) / 2, (line[1] + zone[1]) / 2];
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
  // The HUD goes from here on: the x-ray probe reads only what the canvas
  // draws, and the clean frames below show the world alone.
  await page.addStyleTag({
    content: `${HIDE_READOUTS} [data-testid=battle-panel] { display: none !important; }`,
  });
  // The x-ray draws only what the world hides: soldiers in the open (here
  // the squad nearest the zone, bodies touching the ground) show none of it.
  const squads = fighters.filter((u) => u.members.length > 0);
  const squad = squads.reduce((a, b) => (dist(a.position, zone) <= dist(b.position, zone) ? a : b));
  await pose(page, squad.position, CAMERA.default.distance);
  const bare = await overlayOnly(ctx, page, "battle-squad-overlay.png");
  const bodies = await torsos(
    page,
    squad.members.map((m) => [m[0], m[1], m[2] - 0.8]),
  );
  const flecked = bodies.filter((p) => xrayAt(bare, p)).length;
  ctx.check(
    "soldiers in plain view carry no x-ray, prone ones included",
    bodies.length > 0 && flecked === 0,
    JSON.stringify({ soldiers: bodies.length, flecked }),
  );
  // The same frames without the HUD (panel, readouts and every overlay: the
  // frame's world view, graded): what the references show.
  await lab(page, () => window.__lab.setFrameView("world"));
  for (const [name, [at, distance, pitch, yaw]] of Object.entries(frames)) {
    await pose(page, at, distance, pitch, yaw);
    await snapshot(ctx, page, `battle-${name}-clean-1920x1080.png`);
  }
  await lab(page, () => window.__lab.setFrameView("final"));
  await ctx.writeEvidence("battle-frames.json", {
    tick: o.tick,
    seed: village.seed,
    script: "scout-suppress-flank",
    frames,
    craters,
    wreck,
    rounds: o.projectiles.length,
    corpses: o.corpses.length,
  });
  ctx.check(
    "the battle frames show craters",
    (craters?.cells ?? 0) > 0,
    JSON.stringify({ wreck: wreck?.kind ?? null, craters }),
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
const xrayAt = (png, p) => anyNear(png, p, 3, ([r, g, b]) => b > 60 && b > r + 25 && g > r + 10);

/** The unit-in-woods cue. A squad walks into the west wood; under
 *  the canopy its soldiers are drawn as an x-ray, and a squad in the open is
 *  drawn plainly. */
export async function woodsTour(ctx) {
  const page = await openBattle(ctx, { viewport: VIEWPORT, tick: 60 });
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

  // Round the south, where the wood itself screens the walk from the
  // village, then queued into the wood from its south-west corner. The
  // corner leg keeps the wood between the squad and the village. Since
  // rounds slowed and firing reports shrank, a squad cutting across the
  // open south of the wood is pinned there by the village's fire and falls.
  // It walks holding fire: in the battle as it plays since the tank rounds
  // sped up, a walker firing at will draws the red rifles' return fire at
  // the wood's edge.
  const approach = [wood[0] - 300, wood[1] + wood[3] + 110];
  const corner = [wood[0] - 60, wood[1] + wood[3] + 110];
  await lab(
    page,
    (c) => {
      window.__lab.route.command({
        kind: "set_engagement",
        units: [c.id],
        policy: "return_fire_only",
      });
      window.__lab.route.command({
        kind: "move",
        units: [c.id],
        gesture: 2701,
        goal: c.approach,
        route: "fastest",
      });
      window.__lab.route.command(
        { kind: "move", units: [c.id], gesture: 2703, goal: c.corner, route: "fastest" },
        true,
      );
      window.__lab.route.command(
        { kind: "move", units: [c.id], gesture: 2702, goal: c.goal, route: "fastest" },
        true,
      );
    },
    { id: walker.id, goal, approach, corner },
  );
  let inside = [];
  for (let t = 0; t < 15000 && inside.length < 6; t += 300) {
    await advance(page, 300);
    o = await obs(page);
    const u = o.own.find((u) => u.id === walker.id);
    inside = u ? u.members.filter((m) => inRect(m, wood)) : [];
  }
  const u = o.own.find((x) => x.id === walker.id);
  if (!u || inside.length < 6) {
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
  // No soldier in the wood is lost: each is either drawn as his model
  // (the frame changes at him with models off) or drawn by the x-ray.
  const points = await torsos(page, inside);
  await lab(page, () => window.__lab.setFrameView("world"));
  const withModels = decode(await snapshot(ctx, page, "woods-models-on.png"));
  await lab(page, () => window.__lab.suppressModels(true));
  const without = decode(await snapshot(ctx, page, "woods-models-off.png"));
  await lab(page, () => window.__lab.suppressModels(false));
  await lab(page, () => window.__lab.setFrameView("final"));
  const seen = (p) => mostChanged(withModels, without, p, 4) > 40;
  const modelled = points.filter(seen).length;
  const xrayed = points.filter((p) => xrayAt(png, p)).length;
  const lost = points.filter((p) => !seen(p) && !xrayAt(png, p)).length;
  ctx.check(
    "in the wood, no soldier is lost: each is drawn plainly or through the canopy (x-ray)",
    points.length >= 6 && lost === 0 && xrayed >= 1,
    JSON.stringify({ tick: o.tick, inWood: points.length, modelled, xrayed, lost }),
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

/** Reset and remount, again and again, leave nothing behind. */
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
    await restart(page);
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
    await settle();
  };
  // The battle runs a few real-time ticks before it is paused: step every
  // fresh battle to the same tick, so each cycle fights the same battle.
  const FRESH_TICK = 60;
  const settle = async () =>
    advance(page, FRESH_TICK - (await lab(page, () => window.__lab.route.tick())));

  await settle();
  const quiet = await census(page);
  await fight();
  const fought = await census(page);
  const cycles = [];
  for (let k = 0; k < 3; k++) {
    await reset();
    const fresh = await census(page);
    await fight();
    cycles.push({ fresh, fought: await census(page) });
  }
  await ctx.writeEvidence("cleanup-reset.json", { quiet, fought, cycles });
  ctx.check(
    "reset returns the battle's effects, corpses and sound voices to a first start's",
    fought.effects > 0 &&
      fought.corpses > 0 &&
      cycles.every(
        // Voices follow real-time presentation: the countryside bed, ± a
        // sound or two at the moment of the census.
        (c) =>
          c.fresh.effects === quiet.effects &&
          c.fresh.corpses === 0 &&
          Math.abs(c.fresh.voices - quiet.voices) <= 2,
      ),
    JSON.stringify({ quiet, fought, fresh: cycles.map((c) => c.fresh) }),
  );
  // After the first cycle (buffers grown to the fight's size), every reset
  // and every fight holds the same allocations and live resources.
  ctx.check(
    "three resets and fights hold GPU allocations, workers, audio and listeners steady",
    cycles
      .slice(1)
      .every((c) => same(c.fresh, cycles[0].fresh) && same(c.fought, cycles[0].fought)) &&
      cycles.every((c) => c.fresh.page.devices === 1 && c.fresh.page.workers === 1),
    JSON.stringify(
      cycles.map((c) => ({ fresh: c.fresh.gpu, fought: c.fought.gpu, page: c.fresh.page })),
    ),
  );

  // A new variant remounts the whole battle view: a new device, worker and
  // sound, the old ones released.
  const remounts = [];
  const [crossfire, ordinary] = ["Prepared crossfire", "Ordinary ambush"];
  for (const variant of [crossfire, ordinary, crossfire, ordinary]) {
    await chooseVariant(page, variant);
    await lab(page, () => window.__lab.route.pause());
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

/** The pixel at page point `p`: [r, g, b]. */
const pixelAt = (png, p) => {
  const i = (Math.round(p[1]) * png.width + Math.round(p[0])) * 4;
  return [png.data[i], png.data[i + 1], png.data[i + 2]];
};

/** Fog runs on past the playable area, computed as inside; a red
 *  border marks the area. Blue's start is near the map's west edge. */
export async function edgeTour(ctx) {
  const page = await openBattle(ctx, { viewport: VIEWPORT });
  await page.addStyleTag({
    content: `${HIDE_READOUTS} [data-testid=battle-panel] { display: none !important; }`,
  });
  const o = await obs(page);
  const westmost = Math.min(...o.own.map((u) => u.position[0]));
  // Ground inside the map, 40 m past the west edge (within every eye's
  // range of the nearest unit), and 3 km past it (beyond any eye).
  const y = o.own.find((u) => u.position[0] === westmost).position[1];
  const points = { inside: [westmost + 30, y], past: [-40, y], far: [-3000, y] };
  const seen = {};
  await lab(page, () => window.__lab.setFrameView("fog-mask"));
  for (const [name, p] of Object.entries(points)) {
    await pose(page, p, 300, 0.85);
    const png = decode(await snapshot(ctx, page, `edge-${name}-mask-1920x1080.png`));
    const css = await lab(page, (q) => window.__lab.projectToCss(q[0], q[1], 0), p);
    seen[name] = pixelAt(png, css);
  }
  await lab(page, () => window.__lab.setFrameView("final"));
  const white = (c) => c.every((v) => v > 200);
  ctx.check(
    "ground past the map's edge within an eye's range is seen, and far past it unseen",
    white(seen.inside) && white(seen.past) && !white(seen.far) && seen.far[0] < 128,
    JSON.stringify({ westmost, points, seen }),
  );
  // The strategic view over the west edge, for the eye.
  await pose(page, [60, y], 900);
  await snapshot(ctx, page, "edge-strategic-1920x1080.png");

  // The border lies along the edge: dim white ink near every sample down
  // the west edge in view (neutral: its blue rises with its red, which no
  // grass or soil does). It is painted on the ground, so it is read as the
  // paint's rise over the ground there.
  await pose(page, [60, y], 300, 0.85);
  const ink = await paintOnly(ctx, page, "edge-border");
  const width = (await lab(page, () => window.__lab.camera())).distance;
  let samples = 0;
  const missed = [];
  for (let dy = -60; dy <= 60; dy += 10) {
    const css = await groundCss(page, [0.3, y + dy]);
    if (!css || css[0] < 4 || css[1] < 4 || css[0] > 1916 || css[1] > 1076) continue;
    samples++;
    let best = [0, 0, 0];
    for (let oy = -3; oy <= 3; oy++)
      for (let ox = -3; ox <= 3; ox++) {
        const c = pixelAt(ink, [css[0] + ox, css[1] + oy]);
        if (Math.min(c[0], c[2]) > Math.min(best[0], best[2])) best = c;
      }
    const [r, g, b] = best;
    if (!(r > 20 && b > 20)) missed.push({ dy, css, rgb: [r, g, b] });
  }
  ctx.check(
    "a dim white border is drawn along the playable area's edge",
    samples >= 8 && missed.length === 0,
    JSON.stringify({ samples, missed: missed.slice(0, 4), distance: width }),
  );
  await page.close();
}
