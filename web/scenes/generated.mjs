// A battle the player starts from the main menu on a generated map: the
// menu's type, size and seed are the map the preparation worker makes, a
// loading screen covers the wait, the battle on it runs in the village's
// battle view, and its buildings (massing), trees and roads are drawn where
// the static map says they are, with fog over what blue does not see. The
// camera flown through its main town never enters a building. The menu's
// saved battlefield (a generated map of the catalogue) starts the same way.
//
// `CAMERA_MAP=metro:large:1` flies the camera through that map's main town
// instead, and reports what clearance costs there. `STARTUP_MAP=mixed:large:1`
// only starts that map from the menu and reports how long it took.
import { writeFile } from "node:fs/promises";
import { readFileSync } from "node:fs";
import { advance, lab, obs, openMenu, presented } from "./_lab.mjs";
import { decode, pixel } from "./_png.mjs";
import { flyTown } from "./_cameraClearance.mjs";

const game = JSON.parse(readFileSync(new URL("../../fixtures/game.json", import.meta.url)));
const TICK_HZ = game.tick_hz;
const MAP = { type: "mixed", size: "small", seed: "1" };
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

/** What the preparation worker made, as the scene reads it: the report, with
 *  the generated map's request and identity and the objective's centre. */
async function preparedBattle(page) {
  const report = await lab(page, () => window.__lab.route.prepared());
  return {
    ...report,
    map: report.request.map_source.request,
    generation: report.identity.generation,
    town: report.objective.center,
  };
}

/** Start a battle as a player does: open the main menu, choose the map's
 *  type and size, type its seed, and deploy. Returns what the menu and the
 *  loading screen showed. */
async function deployFromMenu(ctx, page, map, shots = false) {
  await page.goto(new URL("/", ctx.url).href);
  await page.getByTestId("menu-deploy").waitFor();
  if (shots) await writeFile(ctx.evidencePath("menu-1920x1080.png"), await page.screenshot());
  await page.getByTestId(`menu-map-${map.type}`).click();
  await page.getByTestId(`menu-size-${map.size}`).click();
  await page.getByTestId("menu-new-seed").click();
  const drawn = await page.getByTestId("menu-seed").inputValue();
  await page.getByTestId("menu-seed").fill(map.seed);
  const menu = {
    drawn,
    href: await page.getByTestId("menu-deploy").getAttribute("href"),
    checked: await page.locator('.menu [role="radio"][aria-checked="true"]').allTextContents(),
  };
  if (shots)
    await writeFile(ctx.evidencePath("menu-chosen-1920x1080.png"), await page.screenshot());
  await page.getByTestId("menu-deploy").click();
  await page.getByTestId("loading").waitFor();
  const loading = await page.evaluate(() => ({
    subject: document.querySelector("[data-testid=loading-subject]")?.textContent,
    stage: document.querySelector("[data-testid=loading-stage]")?.textContent,
    cancel: document.querySelector("[data-testid=loading-cancel]")?.getAttribute("href"),
  }));
  if (shots) await writeFile(ctx.evidencePath("loading-1920x1080.png"), await page.screenshot());
  return { menu, loading };
}

/** `STARTUP_MAP`: one start from the menu, timed from the press of Deploy
 *  (the battle page's navigation) to each loading stage. */
async function startupOf(ctx, spec) {
  const [type, size, seed] = spec.split(":");
  const page = await ctx.newPage({ viewport: { width: 1920, height: 1080 } });
  await deployFromMenu(ctx, page, { type, size, seed });
  await playable(page, 600000);
  const startup = await lab(page, () => window.__lab.route.startup());
  const report = await preparedBattle(page);
  console.log(
    `METRIC startup ${type} ${size} seed ${seed}: prepared ${startup.prepared.toFixed(0)} ms (map ${report.timings.map.toFixed(0)}, encounter ${report.timings.encounter.toFixed(0)}), world ${startup.world.toFixed(0)} ms, first frame ${startup.renderer.toFixed(0)} ms, playable ${startup.playable.toFixed(0)} ms after Deploy; ${report.counts.buildings} buildings (development build)`,
  );
  await writeFile(ctx.evidencePath(`startup-${type}-${size}-${seed}.png`), await page.screenshot());
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
  const generated = await preparedBattle(page);
  const flown = await cameraKeepsOut(ctx, page, generated.town, { reps: 200 });
  await ctx.writeEvidence(`camera-${type}-${size}-${seed}.json`, {
    map: generated.map,
    counts: generated.counts,
    ...flown,
  });
}

export async function run(ctx) {
  if (process.env.CAMERA_MAP) return cameraOnMap(ctx, process.env.CAMERA_MAP);
  if (process.env.STARTUP_MAP) return startupOf(ctx, process.env.STARTUP_MAP);
  const page = await ctx.newPage({ viewport: { width: 1920, height: 1080 } });
  const warnings = [];
  page.on("console", (m) => {
    if (m.type() === "warning" && /webgpu|validation|gpu\w*error/i.test(m.text()))
      warnings.push(m.text().slice(0, 200));
  });

  // The player's way in: the main menu's type, size and seed, then Deploy.
  const { menu, loading } = await deployFromMenu(ctx, page, MAP, true);
  ctx.check(
    "the menu draws a new seed when asked, and deploys the type, size and seed it shows",
    /^\d+$/.test(menu.drawn) &&
      menu.href === `/battle?type=${MAP.type}&size=${MAP.size}&seed=${MAP.seed}` &&
      menu.checked.join() === `${MAP.type},${MAP.size}`,
    JSON.stringify(menu),
  );
  ctx.check(
    "a loading screen names the map and the stage while it is prepared, and can be cancelled back to the menu's choice",
    loading.subject === "MIXED · SMALL · SEED 1" &&
      !!loading.stage &&
      loading.cancel === `/?type=${MAP.type}&size=${MAP.size}&seed=${MAP.seed}`,
    JSON.stringify(loading),
  );

  await playable(page);
  const generated = await preparedBattle(page);
  const startup = await lab(page, () => window.__lab.route.startup());
  ctx.check(
    "the battle runs on the map the menu asked for",
    generated.map.type === MAP.type &&
      generated.map.size === MAP.size &&
      generated.map.seed === MAP.seed &&
      generated.generation.seed === MAP.seed &&
      generated.size.join() === "6000,6000",
    JSON.stringify({ map: generated.map, identity: generated.generation, size: generated.size }),
  );
  console.log(
    `METRIC generated ${MAP.type} ${MAP.size}: prepared ${startup.prepared.toFixed(0)} ms, world ${startup.world.toFixed(0)} ms, renderer ${startup.renderer.toFixed(0)} ms, playable ${startup.playable.toFixed(0)} ms after Deploy (development build)`,
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
      // The nearest building of a coloured category: a grey one's roof
      // takes the sun's warmth, and its channels' order with it.
      const coloured = boxes.filter((b) => Math.max(...b.tint) - Math.min(...b.tint) > 0.15);
      const box = coloured.reduce((a, b) =>
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
    generated.town,
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
  const camera = await cameraKeepsOut(ctx, page, generated.town);

  // A tree: the trunk nearest the town, its crown over the trunk.
  const [trunk] = await lab(
    page,
    (town) => window.__lab.route.propsNear("trunk", town[0], town[1], 1),
    generated.town,
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
  await look(page, generated.town, 400);
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
    { id: jeep.id, goal: generated.town },
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
  const toTown = (p) => Math.hypot(p[0] - generated.town[0], p[1] - generated.town[1]);
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
  await look(page, generated.town, 400);
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

  // The battle saved and watched: the file holds the request that made the
  // battle, and the viewer prepares it again and reaches the same digest.
  const saved = await lab(page, () => ({
    tick: window.__lab.route.tick(),
    digest: window.__lab.route.digest(),
  }));
  const file = await lab(page, () => window.__lab.route.exportReplay());
  await page.goto(`${ctx.url}?replay=saved`);
  await playable(page);
  await lab(page, () => window.__lab.route.pause());
  await page.waitForFunction(() => window.__lab.route.status().status === "paused");
  await advance(page, saved.tick - (await lab(page, () => window.__lab.route.tick())));
  const replayed = await lab(page, () => ({
    tick: window.__lab.route.tick(),
    digest: window.__lab.route.digest(),
  }));
  const watched = await preparedBattle(page);
  ctx.check(
    "a saved battle holds its preparation request, and its replay prepares the same map and reaches the same digest",
    JSON.stringify(file.request) === JSON.stringify(generated.request) &&
      watched.generation.map_hash === generated.generation.map_hash &&
      !!saved.digest &&
      replayed.tick === saved.tick &&
      replayed.digest === saved.digest,
    JSON.stringify({ saved, replayed }),
  );
  // The same commands pinned to another build's generator: refused, never
  // replayed on whatever this build would make of the seed.
  const stale = structuredClone(file);
  stale.request.map_source.request.generator_version = "layout-0";
  await openMenu(page);
  await page.getByTestId("replay-file").setInputFiles({
    name: "stale.json",
    mimeType: "application/json",
    buffer: Buffer.from(JSON.stringify(stale)),
  });
  await page.getByTestId("loading").getByTestId("error").waitFor();
  const staleMessage = await page.getByTestId("loading").getByTestId("error").textContent();
  ctx.check(
    "a replay saved by another build's generator is refused, saying so",
    /saved by another version of the game/.test(staleMessage),
    staleMessage,
  );

  // A request for no such map says so, and starts no battle.
  const refused = await ctx.newPage({ allowErrors: true });
  await refused.goto(`${ctx.url}?type=metro&size=tiny`);
  await refused.getByTestId("error").waitFor();
  const message = await refused.getByTestId("error").textContent();
  await refused.getByRole("button", { name: "Details" }).click();
  const details = await refused.getByTestId("error-details").textContent();
  ctx.check(
    "a request for no such map is refused, with the parameter at fault in its details and no battle",
    /This link does not name a battle/.test(message) &&
      /size must be one of small, medium, large/.test(details) &&
      !(await refused.evaluate(() => window.__lab?.ready ?? false)),
    `${message} | ${details}`,
  );
  await writeFile(ctx.evidencePath("refused-1280x800.png"), await refused.screenshot());

  // The saved battlefield, started from the menu beside the village: the
  // catalogue's generated map under the identity its sources pin, with its
  // saved encounter, in the same battle view.
  const town = await ctx.newPage({ viewport: { width: 1920, height: 1080 } });
  await town.goto(new URL("/", ctx.url).href);
  const entry = town.getByRole("link", { name: "Play Market Town" });
  await entry.waitFor();
  await entry.hover();
  await writeFile(ctx.evidencePath("menu-saved-1920x1080.png"), await town.screenshot());
  const href = await entry.getAttribute("href");
  await entry.click();
  await town.getByTestId("loading").waitFor();
  const subject = await town.getByTestId("loading-subject").textContent();
  await playable(town);
  const report = await lab(town, () => window.__lab.route.prepared());
  const start = await lab(town, () => window.__lab.route.startup());
  ctx.check(
    "the menu's saved battlefield plays the catalogue's map with its saved encounter",
    href === "/battle?map=market-town&recipe=assault" &&
      /MARKET TOWN/.test(subject) &&
      report.request.map_source.kind === "catalogue" &&
      report.identity.kind === "generated" &&
      report.planned === null &&
      report.objective !== null &&
      report.counts.buildings > 1000,
    JSON.stringify({ href, subject, identity: report.identity, counts: report.counts }),
  );
  console.log(
    `METRIC saved map market-town: prepared ${start.prepared.toFixed(0)} ms (map ${report.timings.map.toFixed(0)}, encounter ${report.timings.encounter.toFixed(0)}), playable ${start.playable.toFixed(0)} ms after the menu's link; ${report.counts.buildings} buildings (development build)`,
  );
  await writeFile(ctx.evidencePath("saved-battle-1920x1080.png"), await town.screenshot());
}
