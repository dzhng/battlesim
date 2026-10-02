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
/** The ground round the houses, a kind of its own. */
const SETTLEMENT = "yard";

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
      ...Object.fromEntries(
        [...CROPS, SETTLEMENT].map((kind) => [`${kind}-25`, onPlot(kind, 25, LOW)]),
      ),
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
      // Over the wood's middle, from the play camera and from twice as high.
      "forest-65": at([790, 960], 65),
      "forest-120": at([790, 960], 120),
      // The first log and the first boulder on a forest floor, where the
      // forest rule lays any.
      "floor-log-25": ({ floor }) => at(floor.log, 25, 0.6),
      "floor-boulder-25": ({ floor }) => at(floor.boulder, 25, 0.6),
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
      "track-250": at([230, 120], 250),
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
      // The town's streets nearest its centre (`townStreets`): where three
      // ways or more meet, and low along the edge of a plain street.
      "junction-65": ({ streets }) => at(streets.junction, 65),
      "street-edge-25": ({ streets }) => at(streets.edge, 25, LOW, streets.yaw),
      // Where the town's yards stop and the fields begin, west of its
      // centre; and where the country road through it leaves its streets
      // (`townEdges`).
      "town-edge-250": ({ edges }) => at(edges.yards, 250),
      "town-edge-65": ({ edges }) => at(edges.yards, 65),
      "road-join-65": ({ edges }) => at(edges.join, 65),
      "road-join-25": ({ edges }) => at(edges.join, 25, LOW, edges.joinYaw),
      "country-250": ({ start }) => at(start.at, 250),
      "country-65": ({ start }) => at(start.at, 65),
      "country-25": ({ start }) => at(start.at, 25, LOW),
      // The river's bank, on a map whose layout has a river (seed 2 has).
      "river-250": ({ river }) => at(river, 250),
      "river-65": ({ river }) => at(river, 65),
      // The west edge of the wood nearest blue's start, against the open.
      "forest-edge-250": ({ wood }) => at(wood, 250),
      "forest-edge-65": ({ wood }) => at(wood, 65),
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

/** The west edge of the wood nearest `near` on a generated map: the nearest
 *  point with forest `DEEP_M` round it, walked west to where the forest ends;
 *  undefined on a map with no wood that deep. */
const DEEP_M = 24;
const woodEdge = (page, size, near) =>
  lab(
    page,
    ({ size: [width, height], near, deep }) => {
      const wooded = (x, y) => window.__lab.route.surfaceAt(x, y)?.forest === true;
      let best;
      for (let y = deep; y < height - deep; y += 16)
        for (let x = deep; x < width - deep; x += 16) {
          const d = Math.hypot(x - near[0], y - near[1]);
          if (best && d >= best.d) continue;
          const inside = [
            [0, 0],
            [deep, 0],
            [-deep, 0],
            [0, deep],
            [0, -deep],
          ].every(([dx, dy]) => wooded(x + dx, y + dy));
          if (inside) best = { d, x, y };
        }
      if (!best) return undefined;
      let x = best.x;
      while (wooded(x - 4, best.y)) x -= 4;
      for (let step = 2; step > 0.1; step /= 2) if (wooded(x - step, best.y)) x -= step;
      return [x, best.y];
    },
    { size, near, deep: DEEP_M },
  );

/** A generated town's streets near `centre`, by what a unit finds on the
 *  ground: `junction`, the middle of the nearest meeting of three ways or
 *  more; `edge`, a point on the edge of the nearest plain street (two ways,
 *  no wider than a town street), and the `yaw` that looks along it. */
const townStreets = (page, centre) =>
  lab(
    page,
    ([cx, cy]) => {
      const road = (x, y) => window.__lab.route.surfaceAt(x, y)?.kind === "road";
      // The ways leaving (x, y): the middle angle of each run of road round
      // a circle of radius r.
      const ways = (x, y, r) => {
        const n = 90;
        const on = Array.from({ length: n }, (_, k) =>
          road(x + r * Math.cos((k / n) * 2 * Math.PI), y + r * Math.sin((k / n) * 2 * Math.PI)),
        );
        const first = on.indexOf(false);
        if (first < 0) return [];
        const out = [];
        for (let k = 1, from = -1; k <= n; k++) {
          const here = on[(first + k) % n];
          if (here && from < 0) from = k;
          if (!here && from >= 0) {
            out.push((((first + (from + k - 1) / 2) % n) / n) * 2 * Math.PI);
            from = -1;
          }
        }
        return out;
      };
      const meets = (x, y) =>
        road(x, y) && ways(x, y, 14).length >= 3 && ways(x, y, 22).length >= 3;
      // How far the road runs from (x, y) along (dx, dy).
      const run = (x, y, dx, dy) => {
        let m = 0;
        while (m < 12 && road(x + dx * (m + 0.25), y + dy * (m + 0.25))) m += 0.25;
        return m;
      };
      const plain = (x, y) => {
        if (!road(x, y)) return null;
        const [near, far] = [ways(x, y, 14), ways(x, y, 30)];
        if (near.length !== 2 || far.length !== 2) return null;
        const along = [Math.cos(far[0]), Math.sin(far[0])];
        const back = [Math.cos(far[1]), Math.sin(far[1])];
        if (along[0] * back[0] + along[1] * back[1] > -0.97) return null;
        const [left, right] = [run(x, y, -along[1], along[0]), run(x, y, along[1], -along[0])];
        if (left + right > 7.6) return null;
        return { edge: [x - along[1] * left, y + along[0] * left], along };
      };
      const found = {};
      for (let ring = 0; ring < 150 && !(found.junction && found.street); ring += 2)
        for (let k = 0, n = Math.max(1, Math.ceil(Math.PI * ring)); k < n; k++) {
          const a = (k / n) * 2 * Math.PI;
          const [x, y] = [cx + ring * Math.cos(a), cy + ring * Math.sin(a)];
          if (!found.junction && meets(x, y)) {
            // Its middle: the mean of the points round it where ways meet.
            let [sx, sy, count] = [0, 0, 0];
            for (let dy = -12; dy <= 12; dy++)
              for (let dx = -12; dx <= 12; dx++)
                if (meets(x + dx, y + dy)) {
                  sx += x + dx;
                  sy += y + dy;
                  count++;
                }
            found.junction = [sx / count, sy / count];
          }
          found.street ??= plain(x, y);
        }
      const { edge, along } = found.street ?? {};
      return {
        junction: found.junction,
        edge,
        yaw: along && Math.atan2(-along[1], -along[0]),
      };
    },
    centre,
  );

/** The generated map's ground as the renderer builds it, in the page as
 *  `window.__generatedGround`: its terrain `surface` (its plots, its paved
 *  strokes as drawn). The map is made again from the page's own request. */
const generatedGround = (page) =>
  page.evaluate(
    async (repo) => {
      if (window.__generatedGround) return;
      const file = (p) => `/@fs/${repo}${p}`;
      const [wasm, { GAME_RULES }, mesh, biome, presets, templates] = await Promise.all([
        import("/src/wasm/game_wasm.js"),
        import(file("apps/battle-lab/src/scenarios.ts")),
        import(file("packages/battle-renderer/src/worldMesh.ts")),
        import(file("fixtures/biomes/summer.json")),
        import(file("fixtures/map-presets.json?raw")),
        import(file("fixtures/prototype-building-templates.json?raw")),
      ]);
      await wasm.default();
      const request = window.__lab.route.prepared().request.map_source.request;
      const outcome = JSON.parse(
        wasm.generate_map(
          JSON.stringify(request),
          presets.default,
          templates.default,
          JSON.stringify(GAME_RULES.catalog),
        ),
      );
      if (outcome.status !== "ok") throw new Error(JSON.stringify(outcome.diagnostics));
      const rules = JSON.stringify(GAME_RULES);
      const view = new wasm.WorldView(JSON.stringify(outcome.result.map), rules);
      try {
        window.__generatedGround = {
          surface: mesh.buildWorldLayers(
            mesh.readWorldExports(view),
            JSON.parse(wasm.world_layout(rules)),
            biome.default,
            "surface",
          ).terrain,
        };
      } finally {
        view.free();
      }
    },
    new URL("../../", import.meta.url).pathname,
  );

/** How far beside a carriageway `drawnRoads` still names it. */
const BESIDE_ROAD_M = 10;

/** At each point `[x, y]` of the generated map, the carriageway it lies
 *  deepest in or nearest beside, by the renderer's own strokes: the `kind`
 *  it is drawn as, and how far `inside` its edge the point is (negative
 *  beside it). Null with none within `BESIDE_ROAD_M`. */
export async function drawnRoads(page, points) {
  await generatedGround(page);
  return page.evaluate(
    async ({ points, repo, beside }) => {
      const file = (p) => `/@fs/${repo}${p}`;
      const [strokes, surfaces] = await Promise.all([
        import(file("packages/battle-renderer/src/terrain/strokes.ts")),
        import(file("packages/battle-renderer/src/terrain/surfaces.ts")),
      ]);
      const { surface } = window.__generatedGround;
      const drawn = surface.strokes;
      const stride = surface.site.surfaceStrokeStride;
      return points.map(([x, y]) => {
        let road = null;
        for (let o = 0; o < drawn.length; o += stride) {
          if (!surfaces.isRoad(drawn[o + 5])) continue;
          // A stretch's box first: a town has thousands.
          const pad = drawn[o + 4] + beside;
          if (
            x < Math.min(drawn[o], drawn[o + 2]) - pad ||
            x > Math.max(drawn[o], drawn[o + 2]) + pad ||
            y < Math.min(drawn[o + 1], drawn[o + 3]) - pad ||
            y > Math.max(drawn[o + 1], drawn[o + 3]) + pad
          )
            continue;
          const inside = strokes.strokeInside(drawn, o, x, y);
          if (inside > -beside && inside > (road?.inside ?? -Infinity))
            road = { kind: surfaces.SURFACE_AREA_KINDS[drawn[o + 5]], inside };
        }
        return road;
      });
    },
    { points, repo: new URL("../../", import.meta.url).pathname, beside: BESIDE_ROAD_M },
  );
}

/** A yard's last ground is followed by this much of anything else before
 *  the town has ended. */
const TOWN_END_M = 300;

/** Where the generated town round `centre` ends, by the renderer's own
 *  ground: `yards`, the last of its yards due west of its centre, where the
 *  fields begin; `join`, the nearest place to the centre where a road drawn
 *  as a street through the town turns back into the country road it is, and
 *  `joinYaw`, looking from there toward the town's centre. */
async function townEdges(page, centre) {
  await generatedGround(page);
  return page.evaluate(
    async ({ centre: [cx, cy], repo, end }) => {
      const { plotAt } = await import(`/@fs/${repo}packages/battle-renderer/src/terrain/plots.ts`);
      const { surface } = window.__generatedGround;
      const rules = surface.biome.field_rules;
      const yard = surface.biome.plots.findIndex((p) => p.name === rules.settlement_kind);
      const isYard = (x, y) => surface.plots.plots[plotAt(surface.plots, x, y).plot].kind === yard;
      let last = cx;
      for (let x = cx; x > last - end && x > 0; x -= 2) if (isYard(x, cy)) last = x;
      // A road stops, round, where it is drawn as a street from there on:
      // at a point no exported stretch ends at.
      const drawn = surface.strokes;
      const exported = surface.site.surfaceStrokes;
      const stride = surface.site.surfaceStrokeStride;
      const ends = new Set();
      for (let o = 0; o < exported.length; o += stride)
        for (const k of [0, 2]) ends.add(`${exported[o + k]},${exported[o + k + 1]}`);
      let join = null;
      for (let o = 0; o < drawn.length; o += stride)
        for (const [k, cut] of [
          [0, 1],
          [2, 2],
        ]) {
          const [x, y] = [drawn[o + k], drawn[o + k + 1]];
          if (drawn[o + 6] & cut || ends.has(`${x},${y}`)) continue;
          const d = Math.hypot(x - cx, y - cy);
          if (!join || d < join.d) join = { d, at: [x, y] };
        }
      return {
        yards: [last, cy],
        join: join?.at,
        joinYaw: join && Math.atan2(join.at[1] - cy, join.at[0] - cx),
      };
    },
    { centre, repo: new URL("../../", import.meta.url).pathname, end: TOWN_END_M },
  );
}

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
      ? { plots: await villagePlots(page), floor: await villageFloor(page) }
      : report && {
          ...report,
          edges: await townEdges(page, report.objective.center),
          river: await riverBank(page, report.size),
          wood: await woodEdge(page, report.size, report.start.at),
          streets: await townStreets(page, report.objective.center),
        },
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
 *  preparation report, the village's plots by kind (`villagePlots`) and the
 *  bodies on its forest floors (`villageFloor`). */
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
      // The forest floor's dressing is laid over a few frames after a cut.
      while (l.stats().scenery.dressing.pending) await l.frame();
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
 *  with a row a station: the shot, the bare ground, the mask made legible.
 *  `<map>-stations.json` holds the pose each was shot from. */
export async function stationSheet(
  ctx,
  map,
  page,
  stations = Object.keys(STATION_MAPS[map].stations),
) {
  const rows = [];
  await ctx.writeEvidence(
    `${map}-stations.json`,
    Object.fromEntries(stations.map((station) => [station, stationPose(page, map, station)])),
  );
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
 *  as `window.__villageGround`: its terrain surface (the plots among it),
 *  where the `first` prop of each type stands, and
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
        const exported = mesh.readWorldExports(view);
        const layout = JSON.parse(wasm.world_layout(rules));
        const surface = mesh.buildWorldLayers(exported, layout, biome.default, "surface").terrain;
        const field = terrain.terrainField(surface);
        // Where the first prop of each type stands.
        const first = {};
        const [kind, x, y] = ["kind", "x", "y"].map((f) => layout.propFields.indexOf(f));
        for (let o = 0; o < exported.props.length; o += layout.propStride)
          first[layout.propKinds[exported.props[o + kind]]] ??= [
            exported.props[o + x],
            exported.props[o + y],
          ];
        window.__villageGround = {
          surface,
          first,
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
export async function villageExport(page, points) {
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

/** Where the village's first log and first boulder lie (undefined where the
 *  forest rule lays none). */
async function villageFloor(page) {
  await villageGround(page);
  return page.evaluate(() => {
    const { log, boulder } = window.__villageGround.first;
    return { log, boulder };
  });
}

/** A plot must keep this far inside the map and from any building to stand
 *  for its kind, and its middle this far from its own edge (a yard's
 *  station, this far from its buildings too). */
const PLOT_INSET_M = 60;
const PLOT_ROOM_M = 12;

/** Per plot kind's name, the village's roomiest open plot of that kind
 *  (inside the map, clear of buildings and woods): its middle `at`, and the
 *  unit vector `across` its rows. A settlement's yard is built on: its
 *  station is the yards' roomiest point between the buildings. */
async function villagePlots(page) {
  await villageGround(page);
  return page.evaluate(
    ({ inset, room }) => {
      const { surface, forest } = window.__villageGround;
      const [x0, y0, x1, y1] = surface.site.map;
      const yard = surface.biome.field_rules.settlement_kind;
      /** How far inside convex outline `o` a point lies. */
      const inside = (o, x, y) => {
        const n = o.length / 2;
        let clear = Infinity;
        for (let k = 0; k < n; k++) {
          const [ax, ay] = [o[k * 2], o[k * 2 + 1]];
          const [bx, by] = [o[((k + 1) % n) * 2], o[((k + 1) % n) * 2 + 1]];
          const len = Math.hypot(bx - ax, by - ay);
          clear = Math.min(clear, ((bx - ax) * (y - ay) - (by - ay) * (x - ax)) / len);
        }
        return clear;
      };
      /** How far a point lies from the nearest static prop's footprint. */
      const fromProps = (x, y) => {
        const f = surface.site.footprints;
        let clear = Infinity;
        for (let k = 0; k < f.length; k += 5) {
          const [dx, dy] = [x - f[k], y - f[k + 1]];
          const [c, s] = [Math.cos(f[k + 2]), Math.sin(f[k + 2])];
          const u = Math.abs(dx * c + dy * s) - f[k + 3];
          const v = Math.abs(-dx * s + dy * c) - f[k + 4];
          clear = Math.min(clear, Math.hypot(Math.max(u, 0), Math.max(v, 0)));
        }
        return clear;
      };
      const best = {};
      for (const plot of surface.plots.plots) {
        const o = plot.outline;
        const n = o.length / 2;
        const name = surface.biome.plots[plot.kind].name;
        let [cx, cy] = [0, 0];
        for (let k = 0; k < n; k++) {
          cx += o[k * 2] / n;
          cy += o[k * 2 + 1] / n;
        }
        // The plot is convex: its middle's distance to the nearest edge.
        let clear = inside(o, cx, cy);
        let apart = inset;
        if (name === yard) {
          // The point of a 4 m lattice over it farthest from its edge and
          // from anything standing on it.
          clear = 0;
          apart = room;
          let [lx, ly, hx, hy] = [Infinity, Infinity, -Infinity, -Infinity];
          for (let k = 0; k < n; k++) {
            lx = Math.min(lx, o[k * 2]);
            hx = Math.max(hx, o[k * 2]);
            ly = Math.min(ly, o[k * 2 + 1]);
            hy = Math.max(hy, o[k * 2 + 1]);
          }
          for (let y = ly; y <= hy; y += 4)
            for (let x = lx; x <= hx; x += 4) {
              const here = Math.min(inside(o, x, y), fromProps(x, y));
              if (here > clear) [clear, cx, cy] = [here, x, y];
            }
        }
        const open =
          cx > x0 + inset &&
          cx < x1 - inset &&
          cy > y0 + inset &&
          cy < y1 - inset &&
          clear > room &&
          surface.site.buildings.every((b) => Math.hypot(b[0] - cx, b[1] - cy) > apart) &&
          [
            [0, 0],
            [room, 0],
            [-room, 0],
            [0, room],
            [0, -room],
          ].every(([dx, dy]) => forest(cx + dx, cy + dy, 1) < 0);
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
