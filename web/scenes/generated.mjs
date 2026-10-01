// A battle on a generated map: the map the URL asks for is the one the
// preparation worker made, a loading screen covers the wait, the battle on it
// runs, and its buildings (massing), trees and roads are drawn where the
// static map says they are, with fog over what blue does not see. The camera
// flown through its main town never enters a building.
//
// `CAMERA_MAP=metro:large:1` flies the camera through that map's main town
// instead, and reports what clearance costs there.
import { writeFile } from "node:fs/promises";
import { readFileSync } from "node:fs";
import { advance, lab, obs, presented } from "./_lab.mjs";
import { decode, pixel } from "./_png.mjs";
import { flyTown } from "./_cameraClearance.mjs";

const village = JSON.parse(readFileSync(new URL("../../fixtures/village.json", import.meta.url)));
const TICK_HZ = village.tick_hz;
const MAP = { type: "mixed", size: "small", seed: "1" };
const QUERY = `?type=${MAP.type}&size=${MAP.size}&seed=${MAP.seed}`;
const HIDE_HUD = "[data-testid=battle-panel], .ro-layer { display: none !important; }";
/** The ground mask's two values: a pixel that is mostly ground, and one that is not. */
const isGround = ([r]) => r > 200;
const isBody = ([r]) => r < 55;

/** The far ground's mean green-over-blue at the overview (0-255): open
 *  fields through the stretched haze measure about 45, a whited-out map 23. */
const OVERVIEW_WARMTH_MIN = 34;
const delta = (a, b) => a.reduce((s, v, i) => s + Math.abs(v - b[i]), 0);

/** Wait until the page's battle is playable: the loading screen has lifted
 *  over a running battle. */
export async function playable(page, timeout = 120000) {
  await page.waitForFunction(
    () =>
      document.querySelector("[data-testid=error]") ||
      window.__lab?.error ||
      (window.__lab?.ready &&
        window.__lab.route?.tick?.() > 3 &&
        !document.querySelector("[data-testid=loading]")),
    undefined,
    { timeout },
  );
  const error = await page.evaluate(
    () => document.querySelector("[data-testid=error]")?.textContent ?? window.__lab?.error,
  );
  if (error) throw new Error(`lab failed: ${error}`);
}

/** The frame in `view` ("final", "ground-mask", "fog-mask"), as a decoded
 *  PNG; `clear` draws it without fog (the masks are fog's own). */
async function frame(ctx, page, view, file, clear = false) {
  await lab(page, (on) => window.__lab.suppressFog(on), clear);
  await lab(page, (v) => window.__lab.setFrameView(v), view);
  const shot = await page.screenshot();
  await writeFile(ctx.evidencePath(file), shot);
  await lab(page, () => window.__lab.setFrameView("final"));
  await lab(page, () => window.__lab.suppressFog(false));
  return decode(shot);
}

/** Straight down, so nothing standing hides the ground beside it. */
const TOP_DOWN = Math.PI / 2 - 0.03;
/** The camera on world point `at`, `distance` metres out, at the tactical
 *  pitch unless told. */
const look = (page, at, distance, pitch = 0.85) =>
  lab(
    page,
    ({ at, distance, pitch }) =>
      window.__lab.setCamera({
        ...window.__lab.camera(),
        target: [at[0], at[1], window.__lab.route.surfaceZ(at[0], at[1])],
        distance,
        pitch,
      }),
    { at, distance, pitch },
  );

const project = (page, p) => lab(page, (q) => window.__lab.projectToCss(q[0], q[1], q[2]), p);

/** Fly the camera through the main town of the paused battle on `page`, and
 *  check that no eye drawn, by the viewport's camera or by its rig over the
 *  whole of each move, comes inside a building. */
async function cameraKeepsOut(ctx, page, town, options) {
  const boxes = await lab(page, () => window.__lab.route.massing());
  const flown = await flyTown(ctx, page, town, boxes, options);
  const view = await lab(page, () => window.__lab.route.cameraObstacles());
  const moves = Object.entries(flown.moves);
  ctx.check(
    "scripted camera moves through the main town never put the eye inside a building: its near plane stays out of every box drawn",
    view.boxes === boxes.length &&
      moves.every(
        ([, m]) =>
          m.askedInside > 0 &&
          m.liveFrames > 100 &&
          m.flownGap >= flown.envelope &&
          m.drawnGap >= flown.envelope &&
          m.moved === 0 &&
          m.blocked === 0,
      ),
    moves
      .map(
        ([name, m]) =>
          `${name}: ${m.askedInside} of ${m.poses} poses asked for an eye inside a building, drawn never nearer than ${Math.min(m.flownGap, m.drawnGap).toFixed(2)} m (envelope ${flown.envelope.toFixed(2)}; nearest at live frame ${m.nearest.frame} of ${m.liveFrames}, ${m.nearest.hold}), ${m.cuts} cuts, ${m.holds.join("+")}`,
      )
      .join("; "),
  );
  for (const [name, m] of moves)
    console.log(
      `METRIC generated camera ${name}: ${m.usPerFrame.toFixed(1)} µs and ${m.boxTestsPerFrame.toFixed(1)} box tests a frame of ${boxes.length} boxes; the obstacle view built in ${view.buildMs.toFixed(1)} ms (development build)`,
    );
  return { ...flown, view };
}

/** `CAMERA_MAP`: the camera through another map's main town, and its cost. */
async function cameraOnMap(ctx, spec) {
  const [type, size, seed] = spec.split(":");
  const page = await ctx.newPage({ viewport: { width: 1920, height: 1080 } });
  await page.goto(`${ctx.url}?type=${type}&size=${size}&seed=${seed}`);
  await playable(page, 300000);
  await lab(page, () => window.__lab.route.pause());
  await page.waitForFunction(() => window.__lab.route.status().status === "paused");
  await page.addStyleTag({ content: HIDE_HUD });
  const generated = await lab(page, () => window.__lab.route.generated());
  const flown = await cameraKeepsOut(ctx, page, generated.anchors.town, { reps: 200 });
  await ctx.writeEvidence(`camera-${type}-${size}-${seed}.json`, {
    map: generated.map,
    counts: generated.counts,
    ...flown,
  });
}

export async function run(ctx) {
  if (process.env.CAMERA_MAP) return cameraOnMap(ctx, process.env.CAMERA_MAP);
  const page = await ctx.newPage({ viewport: { width: 1920, height: 1080 } });
  const warnings = [];
  page.on("console", (m) => {
    if (m.type() === "warning" && /webgpu|validation|gpu\w*error/i.test(m.text()))
      warnings.push(m.text().slice(0, 200));
  });

  // The loading screen names the map and its stage while the worker prepares it.
  await page.goto(`${ctx.url}${QUERY}`);
  await page.getByTestId("loading").waitFor();
  const loading = await page.evaluate(() => ({
    subject: document.querySelector("[data-testid=loading-subject]")?.textContent,
    stage: document.querySelector("[data-testid=loading-stage]")?.textContent,
  }));
  await writeFile(ctx.evidencePath("loading-1920x1080.png"), await page.screenshot());
  ctx.check(
    "a loading screen names the map and the stage while it is prepared",
    loading.subject === "MIXED · SMALL · SEED 1" && !!loading.stage,
    JSON.stringify(loading),
  );

  await playable(page);
  const generated = await lab(page, () => window.__lab.route.generated());
  const startup = await lab(page, () => window.__lab.route.startup());
  ctx.check(
    "the battle runs on the map the URL asked for",
    generated.map.type === MAP.type &&
      generated.map.size === MAP.size &&
      generated.map.seed === MAP.seed &&
      generated.identity.seed === MAP.seed &&
      generated.size.join() === "6000,6000",
    JSON.stringify({ map: generated.map, identity: generated.identity, size: generated.size }),
  );
  console.log(
    `METRIC generated ${MAP.type} ${MAP.size}: prepared ${startup.prepared.toFixed(0)} ms, world ${startup.world.toFixed(0)} ms, renderer ${startup.renderer.toFixed(0)} ms, playable ${startup.playable.toFixed(0)} ms after navigation (development build)`,
  );
  await writeFile(ctx.evidencePath("opening-1920x1080.png"), await page.screenshot());

  // The battle ticks: paused, it advances exactly the ticks asked for.
  await lab(page, () => window.__lab.route.pause());
  await page.waitForFunction(() => window.__lab.route.status().status === "paused");
  const before = await lab(page, () => window.__lab.route.tick());
  await advance(page, 30);
  const after = await lab(page, () => window.__lab.route.tick());
  ctx.check("the battle ticks", after === before + 30, `${before} → ${after}`);

  // Everything on the map is in the frame's static chunks: a box a building
  // part, a tree a trunk.
  const counts = await lab(page, () => {
    const s = window.__lab.stats().scenery;
    return {
      boxes: s.massing.placed,
      trees: s.forest.placed,
      trunks: window.__lab.route.propsNear("trunk", 0, 0, Infinity).length,
      drawnBoxes: window.__lab.route.massing().length,
      structures: window.__lab.stats().structures,
    };
  });
  ctx.check(
    "every building part is a massing box and every trunk a tree, and no building is a model",
    counts.boxes === generated.counts.parts &&
      counts.drawnBoxes === generated.counts.parts &&
      counts.boxes > 1000 &&
      counts.trees === counts.trunks &&
      counts.trees > 1000 &&
      counts.structures === 0,
    JSON.stringify({ ...counts, parts: generated.counts.parts }),
  );

  await page.addStyleTag({ content: HIDE_HUD });

  // A building: the box nearest the town's centre, and open ground beside it.
  const building = await lab(
    page,
    (town) => {
      const boxes = window.__lab.route.massing();
      const inside = (b, x, y) => {
        const [dx, dy] = [x - b.center[0], y - b.center[1]];
        const [c, s] = [Math.cos(b.yaw), Math.sin(b.yaw)];
        return (
          Math.abs(dx * c + dy * s) <= b.half[0] + 1 && Math.abs(-dx * s + dy * c) <= b.half[1] + 1
        );
      };
      const box = boxes.reduce((a, b) =>
        Math.hypot(a.center[0] - town[0], a.center[1] - town[1]) <=
        Math.hypot(b.center[0] - town[0], b.center[1] - town[1])
          ? a
          : b,
      );
      const reach = Math.hypot(box.half[0], box.half[1]) + 6;
      let ground = null;
      for (let k = 0; k < 16 && !ground; k++) {
        const [x, y] = [
          box.center[0] + reach * Math.cos((k * Math.PI) / 8),
          box.center[1] + reach * Math.sin((k * Math.PI) / 8),
        ];
        if (!boxes.some((b) => inside(b, x, y)) && !window.__lab.route.surfaceAt(x, y)?.forest)
          ground = [x, y];
      }
      return { box, ground };
    },
    generated.anchors.town,
  );
  await look(page, building.box.center, 90, TOP_DOWN);
  const roofAt = [...building.box.center, building.box.baseZ + 2 * building.box.half[2]];
  const groundAt = [...building.ground, 0];
  const [roofPx, groundPx] = [await project(page, roofAt), await project(page, groundAt)];
  const buildingMask = await frame(ctx, page, "ground-mask", "building-ground-mask.png");
  // Without fog for the colours: the town is unseen from blue's start.
  const buildingShot = await frame(ctx, page, "final", "building-1920x1080.png", true);
  const roof = pixel(buildingShot, ...roofPx);
  // The roof keeps its tint's order of channels (the box is one flat colour).
  const tint = building.box.tint;
  const order = (c) => [0, 1, 2].sort((i, j) => c[j] - c[i]).join("");
  ctx.check(
    "a building is drawn as a box where the map puts it: not ground at its roof, ground beside it, in its category's tint",
    isBody(pixel(buildingMask, ...roofPx)) &&
      isGround(pixel(buildingMask, ...groundPx)) &&
      order(roof) === order(tint) &&
      delta(roof, pixel(buildingShot, ...groundPx)) > 30,
    JSON.stringify({
      box: building.box,
      roofPx,
      groundPx,
      mask: [pixel(buildingMask, ...roofPx), pixel(buildingMask, ...groundPx)],
      roof,
      ground: pixel(buildingShot, ...groundPx),
    }),
  );

  // The camera, flown through the town.
  const camera = await cameraKeepsOut(ctx, page, generated.anchors.town);

  // A tree: the trunk nearest the town, its crown over the trunk.
  const [trunk] = await lab(
    page,
    (town) => window.__lab.route.propsNear("trunk", town[0], town[1], 1),
    generated.anchors.town,
  );
  await look(page, trunk.center, 45, TOP_DOWN);
  const crownAt = [...trunk.center, trunk.baseZ + 2 * trunk.half[2] * 0.6];
  const crownPx = await project(page, crownAt);
  const treeMask = await frame(ctx, page, "ground-mask", "tree-ground-mask.png");
  const treeShot = await frame(ctx, page, "final", "tree-1920x1080.png", true);
  const crown = pixel(treeShot, ...crownPx);
  ctx.check(
    "a tree stands on its trunk: a green crown over it, not ground",
    isBody(pixel(treeMask, ...crownPx)) && crown[1] > crown[0] && crown[1] > crown[2],
    JSON.stringify({ trunk: trunk.center, crownPx, mask: pixel(treeMask, ...crownPx), crown }),
  );

  // A road: between two units of blue's column, which stands on it, against
  // the field off to its side.
  const column = (await obs(page)).own;
  const [a, b] = [column[2].position, column[3].position];
  const mid = [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2];
  const along = Math.hypot(b[0] - a[0], b[1] - a[1]);
  const side = [(-(b[1] - a[1]) / along) * 25, ((b[0] - a[0]) / along) * 25];
  const field = [mid[0] + side[0], mid[1] + side[1]];
  const surfaces = await lab(
    page,
    ({ mid, field }) => [
      window.__lab.route.surfaceAt(mid[0], mid[1]).kind,
      window.__lab.route.surfaceAt(field[0], field[1]).kind,
    ],
    { mid, field },
  );
  await look(page, mid, 65);
  const [roadPx, fieldPx] = [await project(page, [...mid, 0]), await project(page, [...field, 0])];
  const roadShot = await frame(ctx, page, "final", "road-1920x1080.png");
  const [road, beside] = [pixel(roadShot, ...roadPx), pixel(roadShot, ...fieldPx)];
  ctx.check(
    "a road is drawn where the map has one: bare paving under the column, the field beside it another colour",
    surfaces[0] === "road" &&
      surfaces[1] !== "road" &&
      road[0] > road[1] &&
      road[1] > road[2] &&
      delta(road, beside) > 30,
    JSON.stringify({ surfaces, road, beside, roadPx, fieldPx }),
  );
  // Fog: red stands in the town, unseen, and is neither drawn nor known;
  // blue's own ground is seen.
  const o = await obs(page);
  await look(page, generated.anchors.town, 400);
  const townMask = await frame(ctx, page, "fog-mask", "town-fog-mask.png");
  await writeFile(ctx.evidencePath("town-fogged-1920x1080.png"), await page.screenshot());
  const townPx = await project(page, [...building.ground, 0]);
  await look(page, mid, 65);
  const blueMask = await frame(ctx, page, "fog-mask", "blue-fog-mask.png");
  ctx.check(
    "fog hides the town blue cannot see and none of its defenders are known; blue's own ground is seen",
    o.identified.length === 0 &&
      o.contacts.length === 0 &&
      pixel(townMask, ...townPx)[0] < 55 &&
      pixel(blueMask, ...(await project(page, [...mid, 0])))[0] > 200,
    JSON.stringify({
      identified: o.identified.length,
      contacts: o.contacts.length,
      town: pixel(townMask, ...townPx),
      blue: pixel(blueMask, ...(await project(page, [...mid, 0]))),
    }),
  );

  // An order: the jeep at the head of the column drives for the town along
  // the road it stands on.
  const jeep = column[0];
  const ack = await lab(
    page,
    ({ id, goal }) =>
      window.__lab.route.command({ kind: "move", units: [id], gesture: 1, goal, route: "fastest" }),
    { id: jeep.id, goal: generated.anchors.town },
  );
  const path = [];
  for (let s = 0; s < 12; s++) {
    await advance(page, TICK_HZ);
    const at = (await obs(page)).own.find((u) => u.id === jeep.id).position;
    path.push({
      at: [at[0], at[1]],
      kind: await lab(page, (p) => window.__lab.route.surfaceAt(p[0], p[1])?.kind, at),
    });
  }
  const toTown = (p) =>
    Math.hypot(p[0] - generated.anchors.town[0], p[1] - generated.anchors.town[1]);
  const driven = Math.hypot(
    path.at(-1).at[0] - jeep.position[0],
    path.at(-1).at[1] - jeep.position[1],
  );
  ctx.check(
    "an ordered jeep drives toward the town along the road",
    ack.error === null &&
      driven > 100 &&
      toTown(path.at(-1).at) < toTown(jeep.position) - 100 &&
      path.every((p) => p.kind === "road"),
    JSON.stringify({ ack, driven, path }),
  );
  await presented(page);
  await look(page, path.at(-1).at, 65);
  await lab(page, () => window.__lab.frame());
  await writeFile(ctx.evidencePath("jeep-on-road-1920x1080.png"), await page.screenshot());

  // The whole map in one view: the overview keeps the ground's contrast
  // (haze stretches with the camera) and every static chunk draws.
  await lab(
    page,
    (size) =>
      window.__lab.setCamera({
        ...window.__lab.camera(),
        target: [size[0] / 2, size[1] / 2, 0],
        distance: size[0] * 1.25,
        pitch: 1.3,
      }),
    generated.size,
  );
  const overview = await frame(ctx, page, "final", "overview-1920x1080.png", true);
  const overviewStats = await lab(page, () => window.__lab.stats().scenery);
  // The ground's own colour over a grid of the far third of the map, where
  // haze is thickest: fields are green to straw (more green than blue), and
  // haze washes them toward the sky's blue-grey.
  let warmth = 0;
  let samples = 0;
  for (let i = 1; i < 12; i++)
    for (let j = 8; j < 12; j++) {
      const px = await project(page, [
        (generated.size[0] * i) / 12,
        (generated.size[1] * j) / 12,
        0,
      ]);
      const [, g, b] = pixel(overview, ...px);
      warmth += g - b;
      samples++;
    }
  warmth /= samples;
  ctx.check(
    "the whole-map overview draws every box and tree, and its far ground keeps its colour through the haze",
    overviewStats.massing.tiers[3] === generated.counts.parts &&
      overviewStats.forest.tiers[3] === counts.trees &&
      warmth > OVERVIEW_WARMTH_MIN,
    JSON.stringify({
      warmth,
      massing: overviewStats.massing.tiers,
      forest: overviewStats.forest.tiers,
    }),
  );
  await writeFile(ctx.evidencePath("overview-fogged-1920x1080.png"), await page.screenshot());

  // Rebuilding the frame returns every allocation, the massing's included
  // (after one rebuild, so the per-tier buffers have this view's capacity).
  await look(page, generated.anchors.town, 400);
  await lab(page, () => window.__lab.rebuild());
  const baseline = await lab(page, () => window.__lab.allocations());
  for (let i = 0; i < 2; i++) await lab(page, () => window.__lab.rebuild());
  const rebuilt = await lab(page, () => window.__lab.allocations());
  ctx.check(
    "rebuilding the frame returns live GPU buffers and textures, massing included, to baseline",
    rebuilt.buffers === baseline.buffers &&
      rebuilt.bufferBytes === baseline.bufferBytes &&
      rebuilt.textures === baseline.textures &&
      rebuilt.textureBytes === baseline.textureBytes,
    `baseline ${JSON.stringify(baseline)} after ${JSON.stringify(rebuilt)}`,
  );
  ctx.check(
    "no WebGPU validation warning was logged",
    warnings.length === 0,
    warnings.slice(0, 3).join(" | "),
  );
  await ctx.writeEvidence("meta.json", {
    browser: ctx.browser,
    adapter: await page.evaluate(() => window.__lab.adapter),
    viewport: [1920, 1080],
    dpr: 1,
    generated,
    startup,
    camera,
  });

  // A request the generator cannot serve says so, and starts no battle.
  const refused = await ctx.newPage({ allowErrors: true });
  await refused.goto(`${ctx.url}?type=metro&size=tiny`);
  await refused.getByTestId("error").waitFor();
  const message = await refused.getByTestId("error").textContent();
  ctx.check(
    "a request for no such map is refused by name, with no battle",
    /size must be one of small, medium, large/.test(message) &&
      !(await refused.evaluate(() => window.__lab?.ready ?? false)),
    message,
  );
}
