// A battle the player starts from the main menu on a generated map: the
// menu's type, size and seed are the map the preparation worker makes, a
// loading screen covers the wait, the battle on it runs in the battle view,
// and its buildings (their templates' rows), trees and roads
// are drawn where the static map says they are, with fog over what blue does
// not see. The camera flown through its main town never enters a building.
//
// `CAMERA_MAP=metro:large:1` flies the camera through that map's main town
// instead, and reports what clearance costs there. `STARTUP_MAP=mixed:large:1`
// only starts that map from the menu and reports how long it took.
import { startupResources } from "./_startupResources.mjs";
import { writeFile } from "node:fs/promises";
import { readFileSync } from "node:fs";
import { advance, buildingsSettled, lab, obs, openMenu, openMenuPage } from "./_lab.mjs";
import { decode, pixel } from "./_png.mjs";
import { flyTown } from "./_cameraClearance.mjs";
import { bareClassMask, classAt } from "./_groundStations.mjs";

const MAP = { type: "mixed", size: "medium", seed: "1" };
const HIDE_HUD = "[data-testid=battle-panel], .ro-layer { display: none !important; }";
/** The ground mask's two values: a pixel that is mostly ground, and one that is not. */
const isGround = ([r]) => r > 200;
const isBody = ([r]) => r < 55;

/** How much of the near fields' green-over-blue the far fields keep at the
 *  overview, through the stretched haze. A whited-out far band keeps about
 *  half. */
const OVERVIEW_WARMTH_SHARE = 0.75;
/** Fewer trees than this are missing from the overview: a street's tree at
 *  the map's rim is a chunk of its own, which the frame's edge can cut off. */
const TREES_OUT_MOST = 5;
const delta = (a, b) => a.reduce((s, v, i) => s + Math.abs(v - b[i]), 0);

/** Wait until the page's battle is playable: the loading screen has lifted
 *  over a running battle. */
export async function playable(page, timeout = 120000, minimumTick = 4) {
  await page.waitForFunction(
    (minimumTick) =>
      document.querySelector("[data-testid=error]") ||
      window.__lab?.error ||
      (window.__lab?.ready &&
        (window.__lab.route?.tick?.() ?? -1) >= minimumTick &&
        !document.querySelector("[data-testid=loading]")),
    minimumTick,
    { timeout },
  );
  const error = await page.evaluate(
    () => document.querySelector("[data-testid=error]")?.textContent ?? window.__lab?.error,
  );
  if (error) {
    const details = page.getByRole("button", { name: "Details", exact: true });
    if (await details.count()) await details.click();
    throw new Error(`lab failed: ${await page.locator("body").innerText()}`);
  }
}

/** What the preparation worker made, as the scene reads it: the report, with
 *  the generated map's request and identity and its main town's centre. */
async function preparedBattle(page) {
  const report = await lab(page, () => window.__lab.route.prepared());
  return {
    ...report,
    map: report.request.map_source.request,
    generation: report.identity.generation,
    town: report.town,
  };
}

/** Start a battle as a player does: open the main menu, choose the map's
 *  type and size, and deploy. The menu shows no seed; the scene pins one by
 *  the menu's address so the map is the same every run. Returns what the
 *  menu and the loading screen showed. */
async function deployFromMenu(ctx, page, map, shots = false, beforeDeploy = async () => {}) {
  await page.goto(new URL("/", ctx.url).href);
  await openMenuPage(page, "Skirmish");
  const drawn = await page.getByTestId("menu-deploy").getAttribute("href");
  await page.goto(new URL(`/?seed=${map.seed}`, ctx.url).href);
  await page.getByTestId("menu-deploy").waitFor();
  if (shots) await writeFile(ctx.evidencePath("menu-1920x1080.png"), await page.screenshot());
  await page.getByTestId(`menu-map-${map.type}`).click();
  await page.getByTestId(`menu-size-${map.size}`).click();
  const menu = {
    drawn,
    seedShown: await page
      .locator(".menu")
      .innerText()
      .then((t) => /seed/i.test(t)),
    href: await page.getByTestId("menu-deploy").getAttribute("href"),
    checked: await page
      .locator(
        '[data-testid^="menu-map-"][aria-checked="true"], [data-testid^="menu-size-"][aria-checked="true"]',
      )
      .allTextContents(),
    region: await page
      .getByRole("radiogroup", { name: "region", exact: true })
      .getByRole("radio", { checked: true })
      .textContent(),
  };
  if (shots)
    await writeFile(ctx.evidencePath("menu-chosen-1920x1080.png"), await page.screenshot());
  await beforeDeploy();
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
  for (const cache of ["cold", "warm"]) {
    let measuring, resources;
    try {
      await deployFromMenu(ctx, page, { type, size, seed }, true, async () => {
        measuring = await startupResources(page);
      });
      await playable(page, 180000, 0);
      resources = await measuring.finish();
      const startup = await lab(page, () => window.__lab.route.startup());
      const report = await preparedBattle(page);
      const mainWasmBytes = await page.evaluate(async () => {
        const moduleURL = performance
          .getEntriesByType("resource")
          .find((entry) => entry.name.split("?")[0].endsWith("/src/battle/sim/module.ts"))?.name;
        if (!moduleURL) throw new Error("The page's simulation module was not observed");
        const { loadSimModule } = await import(moduleURL);
        return (await loadSimModule()).memory.buffer.byteLength;
      });
      console.log(
        `METRIC startup ${type} ${size} seed ${seed} ${cache}: ${JSON.stringify({
          startup,
          timings: report.timings,
          worldBuildMs: report.worldBuildMs,
          wasmBytes: report.wasmBytes,
          mainWasmBytes,
          buildings: report.counts.buildings,
          resources,
        })}`,
      );
      ctx.check(
        `${type} ${size} ${cache} startup is playable in under 30 seconds`,
        startup.playable < 30000,
        `${startup.playable.toFixed(0)} ms`,
      );
      await ctx.writeEvidence(`startup-${type}-${size}-${seed}-${cache}.json`, {
        cache,
        startup,
        report,
        resources,
        mainWasmBytes,
      });
      await writeFile(
        ctx.evidencePath(`startup-${type}-${size}-${seed}-${cache}.png`),
        await page.screenshot(),
      );
    } finally {
      if (measuring && resources === undefined) await measuring.finish();
    }
  }
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

export async function roadAlignment(ctx, page, start) {
  // A road: under blue's entry, which stands on the road it arrives by
  // (`start`, facing along it), against the field off to its side.
  const mid = start.at;
  const side = [-Math.sin(start.yaw) * 25, Math.cos(start.yaw) * 25];
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
  const roadMask = await bareClassMask(ctx, page, "road-classes.png");
  const classes = [roadPx, fieldPx].map((p) => classAt(roadMask, ...p.map(Math.round)));
  ctx.check(
    "a road is drawn where the map has one: paving under blue's entry, ground outside its edge",
    surfaces[0] === "road" &&
      surfaces[1] !== "road" &&
      classes[0]?.roadSd < 0 &&
      classes[1]?.roadSd > 0,
    JSON.stringify({ surfaces, classes, roadPx, fieldPx }),
  );
  await ctx.writeEvidence("road-alignment.json", {
    surfaces,
    classes,
    roadPx,
    fieldPx,
    colourDiagnostic: {
      road,
      beside,
      distance: delta(road, beside),
      historicalTargetMet: road[0] > road[1] && road[1] > road[2] && delta(road, beside) > 30,
    },
  });
  return { mid };
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
  const boxes = await lab(page, () => window.__lab.route.buildings().flatMap((b) => b.parts));
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
  await page.goto(`${ctx.url}?type=${type}&size=${size}&seed=${seed}&faction=us`);
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
  if (process.env.PLAY_REFUSAL) return playRefusal(ctx);
  if (process.env.PLAY_DEADLINE) return playRefusal(ctx, true);
  await playRefusal(ctx);
  await playRefusal(ctx, true);
  const page = await ctx.newPage({ viewport: { width: 1920, height: 1080 } });
  const warnings = [];
  page.on("console", (m) => {
    if (m.type() === "warning" && /webgpu|validation|gpu\w*error/i.test(m.text()))
      warnings.push(m.text().slice(0, 200));
  });

  // The player's way in: the main menu's type and size, then Deploy.
  const { menu, loading } = await deployFromMenu(ctx, page, MAP, true);
  const asked = new URL(menu.href, ctx.url);
  ctx.check(
    "ordinary Play is unpinned, and an explicit seeded menu address deploys its exact chosen map",
    menu.drawn === "/battle?play=1&type=mixed&size=small&profile=skirmish&faction=us" &&
      !menu.seedShown &&
      asked.pathname === "/battle" &&
      !asked.searchParams.has("play") &&
      ["type", "size", "seed", "faction"].map((k) => asked.searchParams.get(k)).join() ===
        `${MAP.type},${MAP.size},${MAP.seed},us` &&
      menu.checked.join() === `${MAP.type},${MAP.size}` &&
      menu.region === "random",
    JSON.stringify(menu),
  );
  ctx.check(
    "a loading screen names the map and the stage while it is prepared, and can be cancelled back to the menu's choice",
    loading.subject === "MIXED · MEDIUM" &&
      !!loading.stage &&
      new URL(loading.cancel, ctx.url).pathname === "/" &&
      ["type", "size", "seed"]
        .map((k) => new URL(loading.cancel, ctx.url).searchParams.get(k))
        .join() === `${MAP.type},${MAP.size},${MAP.seed}`,
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
      generated.map.profile === "skirmish" &&
      generated.generation.seed === MAP.seed &&
      // A skirmish's medium playable area (`GenerationRequest::extent_m`).
      generated.size.join() === "2400,2400",
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

  // Everything on the map is drawn: every building a reference drawn from
  // its template's rows in the frame's static chunks, a tree every tree
  // body (a forest's trunk, a street's tree), and every other body on the
  // map its own models (or the kit's stand-in), whether it draws with the
  // static world (a boulder, a pitch: nothing changes them) or among the
  // structures the side's knowledge redraws.
  const counts = await lab(page, () => {
    const stats = window.__lab.stats();
    const drawn = window.__lab.route.buildings();
    return {
      buildings: stats.buildings.buildings,
      coarse: stats.buildings.coarse,
      references: drawn.length,
      parts: drawn.reduce((n, b) => n + b.parts.length, 0),
      trees: stats.scenery.forest.placed,
      trunks: window.__lab.route.propsNear("trunk", 0, 0, Infinity).length,
      streetTrees: window.__lab.route.propsNear("street_tree", 0, 0, Infinity).length,
      structures: stats.structures,
      bodies: window.__lab.route.structureBodies(),
    };
  });
  ctx.check(
    "every building is drawn from its template's rows, every trunk and street tree a tree and every body of street furniture a model, and no building is a model",
    counts.buildings === generated.counts.buildings &&
      counts.references === generated.counts.buildings &&
      counts.parts === generated.counts.parts &&
      // The whole map can draw at the coarsest tier: a row or more a building.
      counts.coarse >= counts.buildings &&
      // A skirmish medium map's towns: hundreds of buildings.
      counts.buildings > 500 &&
      counts.trees === counts.trunks + counts.streetTrees &&
      counts.trunks > 1000 &&
      counts.streetTrees > 0 &&
      // Every body but a building's part is drawn by a model of its own; a
      // body of repeated modules (a fence, a railing) by several.
      counts.bodies.unmodelled === 0 &&
      counts.bodies.parts === generated.counts.parts &&
      counts.bodies.sideModels === counts.structures &&
      counts.bodies.bodies - counts.bodies.parts + counts.streetTrees === generated.counts.props &&
      generated.counts.props > 100,
    JSON.stringify({ ...counts, map: generated.counts }),
  );

  await page.addStyleTag({ content: HIDE_HUD });

  // A building: the one nearest the town's centre, and open ground beside it.
  const building = await lab(
    page,
    (town) => {
      const buildings = window.__lab.route.buildings();
      const boxes = buildings.flatMap((b) => b.parts);
      const inside = (b, x, y) => {
        const [dx, dy] = [x - b.center[0], y - b.center[1]];
        const [c, s] = [Math.cos(b.yaw), Math.sin(b.yaw)];
        return (
          Math.abs(dx * c + dy * s) <= b.half[0] + 1 && Math.abs(-dx * s + dy * c) <= b.half[1] + 1
        );
      };
      const from = (b) => Math.hypot(b.frame[0] - town[0], b.frame[1] - town[1]);
      const nearest = buildings.reduce((a, b) => (from(a) <= from(b) ? a : b));
      const box = nearest.parts[0];
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
      return { box, ground, template: nearest.template };
    },
    generated.town,
  );
  await look(page, building.box.center, 90, TOP_DOWN);
  await buildingsSettled(page);
  const near = await lab(page, () => window.__lab.stats().buildings);
  const roofAt = [...building.box.center, building.box.baseZ + 2 * building.box.half[2]];
  const groundAt = [...building.ground, 0];
  const [roofPx, groundPx] = [await project(page, roofAt), await project(page, groundAt)];
  const buildingMask = await frame(ctx, page, "ground-mask", "building-ground-mask.png");
  // Without fog for the colours: the town is unseen from blue's start.
  const buildingShot = await frame(ctx, page, "final", "building-1920x1080.png", true);
  const roof = pixel(buildingShot, ...roofPx);
  ctx.check(
    "a building is drawn from its template where the map puts it: not ground at its roof, ground beside it, its chunk expanded at a fine tier",
    isBody(pixel(buildingMask, ...roofPx)) &&
      isGround(pixel(buildingMask, ...groundPx)) &&
      delta(roof, pixel(buildingShot, ...groundPx)) > 30 &&
      near.residentChunks > 0 &&
      near.tiers[0] > 0 &&
      near.pool.used <= near.pool.capacity,
    JSON.stringify({
      template: building.template,
      box: building.box,
      resident: near.residentChunks,
      tiers: near.tiers,
      pool: near.pool,
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

  await roadAlignment(ctx, page, generated.start);
  // Fog: in preparation blue has no unit on the map, so it sees none of the
  // town and knows no enemy.
  const o = await obs(page);
  await look(page, generated.town, 400);
  const townMask = await frame(ctx, page, "fog-mask", "town-fog-mask.png");
  await writeFile(ctx.evidencePath("town-fogged-1920x1080.png"), await page.screenshot());
  const townPx = await project(page, [...building.ground, 0]);
  ctx.check(
    "fog hides the town blue cannot see, and no enemy is known",
    o.identified.length === 0 && o.contacts.length === 0 && pixel(townMask, ...townPx)[0] < 55,
    JSON.stringify({
      identified: o.identified.length,
      contacts: o.contacts.length,
      town: pixel(townMask, ...townPx),
    }),
  );

  // The whole map in one view: the overview keeps the ground's contrast
  // (haze stretches with the camera) and every static chunk draws.
  await lab(
    page,
    (size) =>
      window.__lab.setCamera({
        ...window.__lab.camera(),
        target: [size[0] / 2, size[1] / 2, 0],
        distance: size[0] * 1.45,
        pitch: 1.3,
      }),
    generated.size,
  );
  const overview = await frame(ctx, page, "final", "overview-1920x1080.png", true);
  const overviewStats = await lab(page, () => ({
    ...window.__lab.stats().scenery,
    buildings: window.__lab.stats().buildings,
  }));
  // The ground's own colour over a grid of the far third of the map, where
  // haze is thickest: fields are green to straw (more green than blue), and
  // haze washes them toward the sky's blue-grey.
  // The median of each band, so a wood or a town under some of the samples
  // does not stand in for the fields: which part of a map is open differs by
  // seed. The far band is judged against the near one, so the fields' own
  // palette cancels and only what the haze took is left.
  const band = async (from, to) => {
    const values = [];
    for (let i = 1; i < 12; i++)
      for (let j = from; j < to; j++) {
        const px = await project(page, [
          (generated.size[0] * i) / 12,
          (generated.size[1] * j) / 12,
          0,
        ]);
        const [, g, b] = pixel(overview, ...px);
        values.push(g - b);
      }
    values.sort((a, b) => a - b);
    return values[values.length >> 1];
  };
  const warmth = await band(8, 12);
  const nearWarmth = await band(1, 5);
  ctx.check(
    "the whole-map overview draws every building at the coarsest tier with none in the pool, every tree, and its far ground keeps its colour through the haze",
    overviewStats.buildings.tiers[3] === counts.coarse &&
      overviewStats.buildings.residentChunks === 0 &&
      overviewStats.buildings.pool.used === 0 &&
      overviewStats.forest.tiers.slice(0, 3).every((n) => n === 0) &&
      overviewStats.forest.tiers[3] > counts.trees - TREES_OUT_MOST &&
      overviewStats.forest.tiers[3] <= counts.trees &&
      warmth > OVERVIEW_WARMTH_SHARE * nearWarmth,
    JSON.stringify({
      warmth,
      nearWarmth,
      buildings: overviewStats.buildings.tiers,
      resident: overviewStats.buildings.residentChunks,
      forest: overviewStats.forest.tiers,
      standing: overviewStats.forest.placed,
    }),
  );
  await writeFile(ctx.evidencePath("overview-fogged-1920x1080.png"), await page.screenshot());

  // Rebuilding the frame returns every allocation, the buildings' included
  // (after one rebuild, so the per-tier buffers have this view's capacity).
  await look(page, generated.town, 400);
  await lab(page, () => window.__lab.rebuild());
  const baseline = await lab(page, () => window.__lab.allocations());
  for (let i = 0; i < 2; i++) await lab(page, () => window.__lab.rebuild());
  const rebuilt = await lab(page, () => window.__lab.allocations());
  ctx.check(
    "rebuilding the frame returns live GPU buffers and textures, the buildings' included, to baseline",
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

  // The battle saved and watched: exact compiled scenario bytes restore the
  // map, encounter and rules without regenerating from the request.
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
    "a saved battle retains its compiled scenario and reaches the same replay digest",
    JSON.stringify(file.battle.report.request) === JSON.stringify(generated.request) &&
      typeof file.battle.scenario === "string" &&
      watched.generation.map_hash === generated.generation.map_hash &&
      !!saved.digest &&
      replayed.tick === saved.tick &&
      replayed.digest === saved.digest,
    JSON.stringify({ saved, replayed }),
  );
  // The same captured scenario and commands on another engine build are refused.
  const stale = structuredClone(file);
  const commands = JSON.parse(stale.replay);
  commands.engine_build = "another-build";
  stale.replay = JSON.stringify(commands);
  await openMenu(page);
  await page.getByTestId("replay-file").setInputFiles({
    name: "stale.json",
    mimeType: "application/json",
    buffer: Buffer.from(JSON.stringify(stale)),
  });
  await page.getByTestId("error").waitFor();
  const staleMessage = await page.getByTestId("error").textContent();
  ctx.check(
    "a replay saved by another engine build is refused, saying so",
    /build/.test(staleMessage),
    staleMessage,
  );

  // A request for no such map says so, and starts no battle. It names a
  // faction (slice 07 refuses a link without one first), so the size is its
  // one fault.
  const refused = await ctx.newPage({ allowErrors: true });
  await refused.goto(`${ctx.url}?type=metro&size=tiny&faction=us`);
  await refused.getByTestId("error").waitFor();
  const message = await refused.getByTestId("error").textContent();
  await refused.getByRole("button", { name: "Details" }).click();
  const details = await refused.getByTestId("error-details").textContent();
  ctx.check(
    "a request for no such map is refused, with the parameter at fault in its details and no battle",
    /This link does not name a battle/.test(message) &&
      /size must be one of small, medium, large, xl/.test(details) &&
      !(await refused.evaluate(() => window.__lab?.ready ?? false)),
    `${message} | ${details}`,
  );
  await writeFile(ctx.evidencePath("refused-1280x800.png"), await refused.screenshot());
}

/** Exhausted generation remains a refused skirmish, never a fixed map. */
async function playRefusal(ctx, deadline = false) {
  const page = await ctx.newPage();
  const defaults = JSON.parse(
    readFileSync(new URL("../../fixtures/generated-battle.json", import.meta.url)),
  );
  let workers = 0;
  if (deadline)
    await page.route(/\/prepare\/worker\.ts(?:\?|$)/, (route) => {
      ++workers;
      return workers === 1
        ? route.fulfill({
            contentType: "text/javascript",
            headers: {
              "Cross-Origin-Embedder-Policy": "require-corp",
              "Cross-Origin-Opener-Policy": "same-origin",
            },
            body: "self.onmessage = () => {}; setInterval(() => {}, 1000);",
          })
        : route.continue();
    });
  else {
    defaults.limits.max_authored_parts = 1;
    defaults.limits.max_bay_positions = 1;
    await page.route(/\/fixtures\/generated-battle\.json(?:\?|$)/, (route) =>
      route.fulfill({
        contentType: "text/javascript",
        body: `export default ${JSON.stringify(defaults)};`,
      }),
    );
  }
  await page.goto(new URL("/", ctx.url).href);
  await openMenuPage(page, "Skirmish");
  const deploy = page.getByRole("link", { name: "Deploy" });
  ctx.check(
    "ordinary menu Play carries preferences without a promised seed",
    (await deploy.getAttribute("href")) ===
      "/battle?play=1&type=mixed&size=small&profile=skirmish&faction=us",
  );
  await deploy.click();
  await page.getByTestId("error").waitFor({ timeout: 30000 });
  await page.getByRole("button", { name: "Details", exact: true }).click();
  const failure = await page.getByTestId("error").textContent();
  ctx.check(
    "exhausted generation refuses Play without switching to a saved map",
    /The battle could not be prepared/.test(failure) &&
      new URL(page.url()).searchParams.get("play") === "1" &&
      !(await page.evaluate(() => window.__lab?.ready ?? false)),
    `${page.url()} | ${failure}`,
  );
  if (deadline)
    ctx.check("the stalled worker is not replaced by a saved map", workers === 1, workers);
  const name = deadline ? "play-deadline" : "play-refused";
  await writeFile(ctx.evidencePath(`${name}.png`), await page.screenshot());
  await page.close();
}
