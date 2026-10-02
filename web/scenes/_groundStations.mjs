// C62: the ground evidence rig. Every ground slice shoots the same named
// stations, with everything but its own variable held: the route's light, a
// pinned tick with the clock at rest on it, no units, effects, scars, paint
// or fog, 1920 × 1080 at DPR 1. Beside each shot goes the ground's classes
// under the same pixels (`FrameView` "ground-classes"), written by the
// terrain material itself, so a slice measures exactly the band it judges.
//
//   STATIONS=village,river bun run --cwd web scene -- ground
//
// writes every station's shot and mask into throwaway/evidence/ground/ and a
// sheet per map (`river:bend-65+wide-65`: those stations alone, the ones a
// change can move); a later slice imports `openStations` and `shoot`.
import { writeFile } from "node:fs/promises";
import { PNG } from "pngjs";
import { advance, aim, lab, presented } from "./_lab.mjs";
import { decode } from "./_png.mjs";

export const VIEWPORT = { width: 1920, height: 1080 };
/** Looking north, as the labs open. */
const YAW = -Math.PI / 2;
/** The play camera's pitch, and the ground view's. */
const PLAY = 0.85;
const LOW = 0.32;
/** The tick every station is shot at. */
const TICK = 12;
const HIDE_HUD = "[data-testid=battle-panel], .ro-layer, .lab-panel { display: none !important; }";

const at = (target, distance, pitch = PLAY, yaw = YAW) => ({ target, distance, pitch, yaw });

/** A station over the middle of the village's roomiest plot of one kind
 *  (`villagePlots`), so it follows the patchwork when the biome's kinds or
 *  weights change. */
const onPlot =
  (kind, distance, pitch) =>
  ({ plots }) => {
    if (!plots[kind]) throw new Error(`the village has no open ${kind} plot`);
    return at(plots[kind].at, distance, pitch);
  };

/** The kinds of ground that have a station of their own: the wild ones at
 *  the play camera and low, each crop low. */
const WILD = ["meadow", "rough", "prairie"];
const CROPS = ["pasture", "wheat", "barley", "rapeseed", "hay", "stubble", "ploughed"];

/** Each map's route (from the site root) and its named poses. A generated
 *  map's stations stand on what its preparation reports (the objective town,
 *  blue's start), so they follow the generator when its layouts change. */
export const STATION_MAPS = {
  village: {
    route: "/battle/village",
    stations: {
      ...Object.fromEntries(
        WILD.flatMap((kind) => [
          [`${kind}-65`, onPlot(kind, 65)],
          [`${kind}-25`, onPlot(kind, 25, LOW)],
        ]),
      ),
      ...Object.fromEntries(CROPS.map((kind) => [`${kind}-25`, onPlot(kind, 25, LOW)])),
      // A drilled crop's rows, from the play camera.
      "wheat-65": onPlot("wheat", 65),
      // The (420, 420) corner of the north road.
      "bend-25": at([420, 420], 25, LOW),
      "bend-65": at([420, 420], 65),
      "bend-120": at([420, 420], 120),
      "bend-250": at([420, 420], 250),
      // The west wood: its west edge against the fields, and inside it.
      "forest-edge-65": at([700, 960], 65),
      "forest-deep-25": at([790, 960], 25, 0.6),
      // Open fields south-west of the village, and the whole patchwork.
      "field-65": at([420, 1120], 65),
      "field-250": at([420, 1120], 250),
      "patchwork-1100": at([800, 800], 1100, 1.2),
    },
  },
  river: {
    route: "/lab/river",
    stations: {
      // The meander's first bend, and the 30 m stretch.
      "bend-25": at([150, 262], 25, LOW),
      "bend-65": at([150, 262], 65),
      "wide-65": at([330, 250], 65),
      // Along the meander from its first bend, low over the near bank.
      "meander-low-90": at([196, 300], 90, LOW, -0.69),
      // The dirt track, and where it leaves the country road.
      "track-25": at([230, 120], 25, LOW),
      "track-65": at([230, 120], 65),
      "junction-65": at([60, 160], 65),
      // The bridge, and the wood over the far bank.
      "bridge-65": at([60, 240], 65),
      "wood-65": at([500, 277], 65),
    },
  },
  generated: {
    route: "/battle?type=mixed&size=medium&seed=2",
    stations: {
      "overview-2500": ({ size }) => at([size[0] / 2, size[1] / 2], 2500, 1.1),
      "town-250": ({ objective }) => at(objective.center, 250),
      "town-65": ({ objective }) => at(objective.center, 65),
      "country-250": ({ start }) => at(start.at, 250),
      "country-65": ({ start }) => at(start.at, 65),
      "country-25": ({ start }) => at(start.at, 25, LOW),
      // The river's bank, on a map whose layout has a river (seed 2 has).
      "river-250": ({ river }) => at(river, 250),
      "river-65": ({ river }) => at(river, 65),
    },
  },
};

/** What each open page's map reports of itself (a generated map's
 *  preparation report; nothing on a saved map). */
const reports = new WeakMap();

/** The class view's encoding (`terrain/groundClasses.ts`), read from the
 *  page `openStations` opened. */
let encoding = null;

/** A point on the west edge of a generated map's river (it runs from the
 *  north edge to the south), where the water first crosses the map's middle
 *  latitude; undefined on a map with no river. */
const riverBank = (page, size) =>
  lab(
    page,
    ([width, height]) => {
      const wet = (x) => window.__lab.route.surfaceAt(x, height / 2)?.kind === "water";
      for (let x = 0; x < width; x += 4) {
        if (!wet(x)) continue;
        let dry = x - 4;
        for (let step = 2; step > 0.1; step /= 2) if (!wet(dry + step)) dry += step;
        return [dry, height / 2];
      }
      return undefined;
    },
    size,
  );

/** A page on `map`'s route, paused at the stations' tick with the frozen set
 *  on. Shoot it with `shoot`. */
export async function openStations(ctx, map) {
  const page = await ctx.newPage({ viewport: VIEWPORT });
  // A generated map is prepared behind a loading screen first: wait it out.
  await page.goto(new URL(STATION_MAPS[map].route, ctx.url).href);
  await page.waitForFunction(
    () =>
      window.__lab?.error ||
      document.querySelector("[data-testid=error]") ||
      (window.__lab?.ready &&
        window.__lab.route?.tick?.() > 3 &&
        window.__lab.stats().grass.enabled &&
        !document.querySelector("[data-testid=loading]")),
    undefined,
    { timeout: 300000 },
  );
  const error = await page.evaluate(
    () => document.querySelector("[data-testid=error]")?.textContent ?? window.__lab?.error,
  );
  if (error) throw new Error(`lab failed: ${error}`);
  const report = await lab(page, () => window.__lab.route.prepared?.() ?? null);
  reports.set(
    page,
    map === "village"
      ? { plots: await villagePlots(page) }
      : report && { ...report, river: await riverBank(page, report.size) },
  );
  await lab(page, () => window.__lab.route.pause());
  await advance(page, TICK - (await lab(page, () => window.__lab.route.tick())));
  await presented(page);
  await page.addStyleTag({ content: HIDE_HUD });
  encoding = await page.evaluate(
    async (path) => ({ ...(await import(path)) }),
    `/@fs/${new URL("../../packages/battle-renderer/src/terrain/groundClasses.ts", import.meta.url).pathname}`,
  );
  await lab(page, async () => {
    const l = window.__lab;
    await l.suppressModels(true);
    await l.suppressEffects(true);
    await l.suppressCastLights(true);
    await l.suppressScars(true);
    await l.suppressPaint(true);
    await l.suppressFog(true);
    await l.setOverlayGlowStrength(0);
  });
  return page;
}

/** What `openStations` learned of `page`'s map: a generated map's
 *  preparation report, the village's plots by kind (`villagePlots`). */
export const stationReport = (page) => reports.get(page);

/** Where `station` of `map` puts the camera on `page`: the target on the
 *  ground, the distance, pitch and yaw. */
export function stationPose(page, map, station) {
  const placed = STATION_MAPS[map].stations[station];
  if (!placed) throw new Error(`no station ${station} on ${map}`);
  return typeof placed === "function" ? placed(reports.get(page)) : placed;
}

/** One station's frame as a PNG buffer: the final view, or `view`; with
 *  `grass` or `trees` false, without them. */
export async function shoot(
  page,
  map,
  station,
  { view = "final", grass = true, trees = true } = {},
) {
  const pose = stationPose(page, map, station);
  if (!pose.target) throw new Error(`${map} has nothing to stand ${station} on`);
  await aim(page, pose.target, pose);
  await lab(
    page,
    async ({ view, grass, trees }) => {
      const l = window.__lab;
      await l.suppressGrass(!grass);
      await l.suppressTrees(!trees);
      await l.setFrameView(view);
      await l.frame();
    },
    { view, grass, trees },
  );
  const shot = await page.screenshot();
  await lab(page, () => window.__lab.setFrameView("final"));
  return shot;
}

/** The ground's class at a pixel of a "ground-classes" shot: null where the
 *  pixel is not bare ground. Distances are metres outside an edge (negative
 *  inside), exact as far as the ground's own look reads them and holding
 *  their side beyond (`terrain/groundClasses.ts`). */
export function classAt(png, x, y) {
  const i = (y * png.width + x) * 4;
  const [r, g, b] = [png.data[i], png.data[i + 1], png.data[i + 2]];
  if (r === 0 && g === 0 && b === 0) return null;
  const e = encoding;
  return {
    roadSd: (r - e.CLASS_EDGE) / e.CLASS_STEPS_PER_M,
    riverSd: (g - e.CLASS_EDGE) / e.CLASS_STEPS_PER_M,
    forest: e.FOREST_CLASSES[b >> e.CLASS_FOREST_SHIFT],
    plotKind: (b >> e.CLASS_KIND_SHIFT) & e.CLASS_KIND_MAX,
    plotHash: b & e.CLASS_HASH_MASK,
  };
}

/** The mask as a picture a person can read: road red by depth, water blue,
 *  forest green (the verge paler), fields grey by plot. */
function legible(mask) {
  const out = new PNG({ width: mask.width, height: mask.height });
  for (let y = 0; y < mask.height; y++)
    for (let x = 0; x < mask.width; x++) {
      const c = classAt(mask, x, y);
      const o = (y * mask.width + x) * 4;
      let rgb = [0, 0, 0];
      if (c) {
        const plot = 70 + c.plotHash * 28 + c.plotKind * 5;
        rgb = [plot, plot, plot];
        if (c.forest === "inside") rgb = [40, 130, 50];
        else if (c.forest === "verge") rgb = [130, 190, 120];
        if (c.roadSd <= 0) rgb = [255, 120 - Math.max(-15, c.roadSd) * 8, 60];
        else if (c.roadSd < 6) rgb = [200 - c.roadSd * 20, rgb[1], rgb[2]];
        if (c.riverSd <= 0) rgb = [40, 90, 255];
        else if (c.riverSd < 8) rgb = [rgb[0], rgb[1], 250 - c.riverSd * 15];
      }
      out.data[o] = rgb[0];
      out.data[o + 1] = rgb[1];
      out.data[o + 2] = rgb[2];
      out.data[o + 3] = 255;
    }
  return out;
}

/** Each of `rows` (lists of same-size PNGs) side by side, the rows stacked,
 *  every picture shrunk `shrink` times by averaging. */
function sheet(rows, shrink) {
  const [w, h] = [Math.floor(rows[0][0].width / shrink), Math.floor(rows[0][0].height / shrink)];
  const out = new PNG({ width: w * rows[0].length, height: h * rows.length });
  rows.forEach((row, ry) =>
    row.forEach((png, rx) => {
      for (let y = 0; y < h; y++)
        for (let x = 0; x < w; x++) {
          const o = ((ry * h + y) * out.width + rx * w + x) * 4;
          for (let c = 0; c < 3; c++) {
            let sum = 0;
            for (let dy = 0; dy < shrink; dy++)
              for (let dx = 0; dx < shrink; dx++)
                sum += png.data[((y * shrink + dy) * png.width + x * shrink + dx) * 4 + c];
            out.data[o + c] = sum / (shrink * shrink);
          }
          out.data[o + 3] = 255;
        }
    }),
  );
  return out;
}

/** Every station of `map` (or `stations` alone): its shot, its shot without
 *  grass or trees, and its class mask, saved as
 *  `<map>-<station>[-bare|-classes].png`, and one sheet `<map>-stations.png`
 *  with a row a station: the shot, the bare ground, the mask made legible. */
export async function stationSheet(
  ctx,
  map,
  page,
  stations = Object.keys(STATION_MAPS[map].stations),
) {
  const rows = [];
  for (const station of stations) {
    const save = async (suffix, options) => {
      const shot = await shoot(page, map, station, options);
      await writeFile(ctx.evidencePath(`${map}-${station}${suffix}.png`), shot);
      return decode(shot);
    };
    const final = await save("");
    const bare = await save("-bare", { grass: false, trees: false });
    const mask = await save("-classes", { view: "ground-classes" });
    rows.push([final, bare, legible(mask)]);
  }
  await writeFile(ctx.evidencePath(`${map}-stations.png`), PNG.sync.write(sheet(rows, 3)));
}

/** The world point on the ground under each of `pixels`, and the footprint
 *  the terrain's fragment has there (`length(fwidth(world.xy))`). */
export const groundUnder = (page, pixels) =>
  lab(
    page,
    (pixels) => {
      const hit = (x, y) => {
        const ray = window.__lab.rayAt(x, y);
        let p = [ray.origin[0], ray.origin[1]];
        // The ground's height varies slowly: a few steps settle on it.
        for (let k = 0; k < 6; k++) {
          const t = (window.__lab.route.surfaceZ(p[0], p[1]) - ray.origin[2]) / ray.dir[2];
          p = [ray.origin[0] + ray.dir[0] * t, ray.origin[1] + ray.dir[1] * t];
        }
        return p;
      };
      const canvas = document.querySelector("canvas").getBoundingClientRect();
      return pixels.map(([x, y]) => {
        // A pixel's centre, in page coordinates.
        const [cx, cy] = [canvas.left + x + 0.5, canvas.top + y + 0.5];
        const [p, px, py] = [hit(cx, cy), hit(cx + 1, cy), hit(cx, cy + 1)];
        const fx = Math.abs(px[0] - p[0]) + Math.abs(py[0] - p[0]);
        const fy = Math.abs(px[1] - p[1]) + Math.abs(py[1] - p[1]);
        return { xy: p, footprint: Math.hypot(fx, fy) };
      });
    },
    pixels,
  );

/** The village's ground as the simulation exports it, built once in the page
 *  as `window.__villageGround`: its terrain surface (the plots among it), and
 *  `paved` and `forest`, how far inside the paving and the forest a point
 *  lies by the surface field the terrain material reads. */
const villageGround = (page) =>
  page.evaluate(
    async (repo) => {
      if (window.__villageGround) return;
      const file = (p) => `/@fs/${repo}${p}`;
      const [wasm, { villageScenario }, mesh, fields, terrain, biome] = await Promise.all([
        import("/src/wasm/game_wasm.js"),
        import(file("apps/battle-lab/src/savedMaps.tsx")),
        import(file("packages/battle-renderer/src/worldMesh.ts")),
        import(file("packages/battle-renderer/src/terrain/surfaceField.ts")),
        import(file("packages/battle-renderer/src/frame/terrainMaterial.ts")),
        import(file("fixtures/biomes/summer.json")),
      ]);
      await wasm.default();
      const setup = JSON.parse(await villageScenario(wasm, "ordinary"));
      const rules = JSON.stringify(setup.rules);
      const view = new wasm.WorldView(JSON.stringify(setup.map), rules);
      try {
        const surface = mesh.buildWorldLayers(
          mesh.readWorldExports(view),
          JSON.parse(wasm.world_layout(rules)),
          biome.default,
          "surface",
        ).terrain;
        const field = fields.buildSurfaceField(surface.site, terrain.terrainReach(surface));
        window.__villageGround = {
          surface,
          paved: (x, y, footprint) => fields.pavedDistance(field, x, y, footprint),
          forest: (x, y, footprint) => fields.forestDistance(field, x, y, footprint),
        };
      } finally {
        view.free();
      }
    },
    new URL("../../", import.meta.url).pathname,
  );

/** At each point `{ xy, footprint }`, how far inside the paving and the
 *  forest the simulation's export puts it. */
async function villageExport(page, points) {
  await villageGround(page);
  return page.evaluate(
    (points) =>
      points.map(({ xy: [x, y], footprint }) => ({
        paved: window.__villageGround.paved(x, y, footprint),
        forest: window.__villageGround.forest(x, y, footprint),
      })),
    points,
  );
}

/** A plot must keep this far inside the map and from any building to stand
 *  for its kind, and its middle this far from its own edge. */
const PLOT_INSET_M = 60;
const PLOT_ROOM_M = 12;

/** Per plot kind's name, the village's roomiest open plot of that kind
 *  (inside the map, clear of buildings and woods): its middle `at`, and the
 *  unit vector `across` its rows. */
async function villagePlots(page) {
  await villageGround(page);
  return page.evaluate(
    ({ inset, room }) => {
      const { surface, forest } = window.__villageGround;
      const [x0, y0, x1, y1] = surface.site.map;
      const best = {};
      for (const plot of surface.plots.plots) {
        const o = plot.outline;
        const n = o.length / 2;
        let [cx, cy] = [0, 0];
        for (let k = 0; k < n; k++) {
          cx += o[k * 2] / n;
          cy += o[k * 2 + 1] / n;
        }
        // The plot is convex: its middle's distance to the nearest edge.
        let clear = Infinity;
        for (let k = 0; k < n; k++) {
          const [ax, ay] = [o[k * 2], o[k * 2 + 1]];
          const [bx, by] = [o[((k + 1) % n) * 2], o[((k + 1) % n) * 2 + 1]];
          const len = Math.hypot(bx - ax, by - ay);
          clear = Math.min(clear, Math.abs((bx - ax) * (cy - ay) - (by - ay) * (cx - ax)) / len);
        }
        const open =
          cx > x0 + inset &&
          cx < x1 - inset &&
          cy > y0 + inset &&
          cy < y1 - inset &&
          clear > room &&
          surface.site.buildings.every((b) => Math.hypot(b[0] - cx, b[1] - cy) > inset) &&
          [
            [0, 0],
            [room, 0],
            [-room, 0],
            [0, room],
            [0, -room],
          ].every(([dx, dy]) => forest(cx + dx, cy + dy, 1) < 0);
        const name = surface.biome.plots[plot.kind].name;
        if (open && clear > (best[name]?.clear ?? 0))
          best[name] = { clear, at: [cx, cy], across: [...plot.across] };
      }
      return Object.fromEntries(
        Object.entries(best).map(([name, { at, across }]) => [name, { at, across }]),
      );
    },
    { inset: PLOT_INSET_M, room: PLOT_ROOM_M },
  );
}

/** How many pixels the agreement check samples, and how far a mask's
 *  distance may sit from the export's: a byte's step, and what the pixel's
 *  own width moves the point a fragment is shaded at. */
const SAMPLES = 1000;

/** The rig's own checks, on the village: the same pixels twice, and a mask
 *  that says what the simulation's export says. */
export async function groundRig(ctx) {
  const page = await openStations(ctx, "village");
  const twice = {};
  for (const station of ["bend-65", "forest-edge-65"]) {
    // Grass blades tie in depth on Metal, so the ground is compared bare.
    const shots = [];
    for (const options of [
      { view: "ground-classes" },
      { grass: false },
      { view: "ground-classes" },
      { grass: false },
    ])
      shots.push(await shoot(page, "village", station, options));
    twice[station] = {
      masks: Buffer.compare(decode(shots[0]).data, decode(shots[2]).data) === 0,
      ground: Buffer.compare(decode(shots[1]).data, decode(shots[3]).data) === 0,
    };
  }
  ctx.check(
    "a station shot twice gives the same class mask and the same ground, byte for byte",
    Object.values(twice).every((t) => t.masks && t.ground),
    JSON.stringify(twice),
  );

  // Seeded pixels over three stations: the road's bend close and far, and
  // the wood's edge.
  let seed = 62;
  const next = () => {
    seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0;
    return seed / 4294967296;
  };
  const worst = { road: 0, sides: 0, forest: 0, ground: 0, onRoad: 0, inForest: 0 };
  const stations = ["bend-65", "bend-250", "forest-edge-65"];
  for (const station of stations) {
    const mask = decode(await shoot(page, "village", station, { view: "ground-classes" }));
    const pixels = [];
    while (pixels.length < Math.ceil(SAMPLES / stations.length)) {
      const p = [Math.floor(next() * mask.width), Math.floor(next() * mask.height)];
      if (classAt(mask, ...p)) pixels.push(p);
    }
    const points = await groundUnder(page, pixels);
    const exported = await villageExport(page, points);
    pixels.forEach((p, i) => {
      const c = classAt(mask, ...p);
      const { paved, forest } = exported[i];
      const step = 1 / encoding.CLASS_STEPS_PER_M;
      const slack = step + points[i].footprint;
      worst.ground++;
      // The byte holds this far either side of the edge.
      const held = 127 * step;
      const sd = Math.min(held, Math.max(-held, -paved));
      if (Math.abs(c.roadSd - sd) > slack) worst.road++;
      if (Math.abs(paved) > slack && c.roadSd < 0 !== paved > 0) worst.sides++;
      if (Math.abs(forest) > slack && (c.forest === "inside") !== forest > 0) worst.forest++;
      if (paved > 0) worst.onRoad++;
      if (forest > 0) worst.inForest++;
    });
  }
  ctx.check(
    `the class mask says what the simulation's export says at ${SAMPLES} sampled ground pixels`,
    worst.ground >= SAMPLES &&
      worst.road === 0 &&
      worst.sides === 0 &&
      worst.forest === 0 &&
      worst.onRoad > 30 &&
      worst.inForest > 30,
    JSON.stringify(worst),
  );

  // Trees are scenery: their own switch takes them and their shadows away.
  const trees = decode(await shoot(page, "village", "forest-edge-65", { grass: false }));
  const none = decode(
    await shoot(page, "village", "forest-edge-65", { grass: false, trees: false }),
  );
  let changed = 0;
  for (let i = 0; i < trees.data.length; i += 4)
    if (Math.abs(trees.data[i + 1] - none.data[i + 1]) > 12) changed++;
  const placed = await lab(page, () => window.__lab.stats().scenery.forest.placed);
  ctx.check(
    "the trees switch off on their own, leaving the ground under the wood",
    placed > 100 && changed > trees.width * trees.height * 0.1,
    JSON.stringify({ placed, changedShare: changed / (trees.width * trees.height) }),
  );
  await page.close();
}

/** `STATIONS=map,...`: every station of those maps, as shots, masks and a
 *  sheet; `map:station+station` shoots only those. */
export async function stationSheets(ctx, maps) {
  for (const named of maps) {
    const [map, only] = named.split(":");
    const page = await openStations(ctx, map);
    await stationSheet(ctx, map, page, only?.split("+"));
    await page.close();
  }
}
