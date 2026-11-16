// Generated-map ground queries and the stations anchored to that ground.
import { vec2 } from "math";
import { polygon2 } from "math/shapes";
import { readFileSync } from "node:fs";
const presets = JSON.parse(
  readFileSync(new URL("../../fixtures/map-presets.json", import.meta.url), "utf8"),
);
/** The generated map's ground as the renderer builds it, in the page as
 *  `window.__generatedGround`: its terrain `surface` (the ground Rust
 *  resolved: its plots, its paved strokes as drawn), the map's `exports` and
 *  its world `layout`. The map is made again from the page's own request,
 *  under the rules a player's battle prepares it with (the game's catalog
 *  set: the generator reads the units' rules), and must be the map the
 *  page prepared. */
export const generatedGround = (page) =>
  page.evaluate(
    async (repo) => {
      if (window.__generatedGround) return;
      const file = (p) => `/@fs/${repo}${p}`;
      const [wasm, { rules: gameRules }, mesh, biome, presets, templates] = await Promise.all([
        import("/src/wasm/game_wasm.js"),
        import(file("web/src/battle/catalog/sets.ts")).then((m) => m.catalogSet("game")),
        import(file("packages/battle-renderer/src/worldMesh.ts")),
        import(file("fixtures/biomes/summer.json")),
        import(file("fixtures/map-presets.json?raw")),
        import(file("fixtures/prototype-building-templates.json?raw")),
      ]);
      await wasm.default();
      // The ground's shapes are resolved by the presentation module.
      await (await import(file("web/src/battle/sim/presentation.ts"))).loadPresentation();
      const prepared = window.__lab.route.prepared();
      const rules = JSON.stringify(gameRules);
      const outcome = JSON.parse(
        wasm.generate_map(
          JSON.stringify(prepared.request.map_source.request),
          presets.default,
          templates.default,
          rules,
        ),
      );
      if (outcome.status !== "ok") throw new Error(JSON.stringify(outcome.diagnostics));
      const [made, played] = [outcome.result.identity, prepared.identity.generation];
      if (made.map_hash !== played.map_hash)
        throw new Error(
          `the ground rig made map ${made.map_hash}; the page plays ${played.map_hash}`,
        );
      const view = new wasm.WorldView(JSON.stringify(outcome.result.map), rules);
      try {
        const exports = mesh.readWorldExports(view);
        const layout = JSON.parse(wasm.world_layout(rules));
        const surface = mesh.buildWorldGround(exports, layout, biome.default, "surface").terrain;
        const { presentGround } = await import(
          file("packages/battle-renderer/src/terrain/terrainSurface.ts")
        );
        window.__generatedGround = {
          surface,
          ground: presentGround(surface.input),
          exports,
          layout,
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
    ({ points, beside }) => {
      const { ground, exports, layout } = window.__generatedGround;
      const drawn = ground.ground_drawn_strokes() ?? exports.surfaceStrokes;
      // A stretch: `ax, ay, bx, by, half width, kind, cut ends` (1 at a, 2 at b).
      const stride = 7;
      const isRoad = (tag) => layout.roadAreaKinds.includes(layout.surfaceAreaKinds[tag]);
      /** How far `(x, y)` lies inside stretch `o`: its half width less the
       *  distance to it, and past a cut end the distance to that square end. */
      const strokeInside = (r, o, x, y) => {
        const [ax, ay, bx, by, half, , cuts] = r.subarray(o, o + 7);
        const [dx, dy] = [bx - ax, by - ay];
        const length = Math.hypot(dx, dy);
        const t = length
          ? Math.min(1, Math.max(0, ((x - ax) * dx + (y - ay) * dy) / length ** 2))
          : 0;
        const inside = half - Math.hypot(x - ax - dx * t, y - ay - dy * t);
        if (!cuts) return inside;
        const along = ((x - ax) * dx + (y - ay) * dy) / length;
        const past = Math.max(cuts & 1 ? -along : -Infinity, cuts & 2 ? along - length : -Infinity);
        if (past <= 0) return Math.min(inside, -past);
        const aside = Math.max(Math.abs((x - ax) * dy - (y - ay) * dx) / length - half, 0);
        return -Math.hypot(aside, past);
      };
      return points.map(([x, y]) => {
        let road = null;
        for (let o = 0; o < drawn.length; o += stride) {
          if (!isRoad(drawn[o + 5])) continue;
          // A stretch's box first: a town has thousands.
          const pad = drawn[o + 4] + beside;
          if (
            x < Math.min(drawn[o], drawn[o + 2]) - pad ||
            x > Math.max(drawn[o], drawn[o + 2]) + pad ||
            y < Math.min(drawn[o + 1], drawn[o + 3]) - pad ||
            y > Math.max(drawn[o + 1], drawn[o + 3]) + pad
          )
            continue;
          const inside = strokeInside(drawn, o, x, y);
          if (inside > -beside && inside > (road?.inside ?? -Infinity))
            road = { kind: layout.surfaceAreaKinds[drawn[o + 5]], inside };
        }
        return road;
      });
    },
    { points, beside: BESIDE_ROAD_M },
  );
}

/** Where the generated town round `centre` ends, by the renderer's own
 *  ground: `yardField`, between its main yards and the nearest drilled field;
 *  `lawnStreet`, a street beside an open yard; `join`, the nearest place to
 *  the centre where a road drawn as a street through the town turns back
 *  into the country road it is, and
 *  `joinYaw`, looking from there toward the town's centre. */
export async function townEdges(page, centre) {
  await generatedGround(page);
  const { boundaries, crops, ...stations } = await page.evaluate(
    ({ centre: [cx, cy], streetWidth }) => {
      const { surface, ground, exports, layout } = window.__generatedGround;
      const rules = surface.biome.field_rules;
      const yard = surface.biome.plots.findIndex((p) => p.name === rules.settlement_kind);
      const isYard = (x, y) => ground.ground_plot_kind_at(x, y) === yard;
      // Follow the main town outwards: a later isolated farmyard is not
      // another part of this boundary. Find its nearest drilled field, so
      // the edge frame spans the yard and the crop rather than only commons.
      const boundaries = [];
      for (let ray = 0; ray < 24; ray++) {
        const angle = (ray / 24) * 2 * Math.PI;
        let at = [cx, cy];
        for (let m = 0; m < surface.biome.field_rules.extent_m; m += 2) {
          const next = [cx + m * Math.cos(angle), cy + m * Math.sin(angle)];
          if (!isYard(...next)) break;
          at = next;
        }
        boundaries.push(at);
      }
      const crops = JSON.parse(ground.ground_plots_json())
        .filter((p) => (surface.biome.plots[p.kind].furrow_m ?? 0) > 0)
        .map((p) => p.outline);
      // A road stops, round, where it is drawn as a street from there on:
      // at a point no exported stretch ends at.
      const exported = exports.surfaceStrokes;
      const drawn = ground.ground_drawn_strokes() ?? exported;
      const stride = 7;
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
      // A street beside an open yard, not a paved court or a rural track.
      // Query its lawn past the walk, so another paved area cannot masquerade
      // as the grass against which the walk and carriageway are judged.
      const street = surface.biome.roads[surface.biome.roads.default.town.kind];
      const walk = street.walk.width_m;
      const lawns = [];
      for (let o = 0; o < drawn.length; o += stride) {
        const [ax, ay, bx, by, half, kind] = drawn.subarray(o, o + stride);
        if (layout.surfaceAreaKinds[kind] !== "road" || 2 * half !== streetWidth) continue;
        const length = Math.hypot(bx - ax, by - ay);
        if (!length) continue;
        for (const t of [0.25, 0.5, 0.75]) {
          const at = [ax + (bx - ax) * t, ay + (by - ay) * t];
          for (const side of [-1, 1]) {
            const reach = side * (half + walk + 2);
            const [x, y] = [
              at[0] - ((by - ay) / length) * reach,
              at[1] + ((bx - ax) / length) * reach,
            ];
            if (!isYard(x, y)) continue;
            lawns.push({ at, xy: [x, y], d: Math.hypot(at[0] - cx, at[1] - cy) });
          }
        }
      }
      // One field query for all candidates: constructing the field once per
      // point would measure its rebuild rather than find a station.
      const distances = ground.ground_distances(
        new Float64Array(lawns.flatMap(({ xy }) => [...xy, 0])),
      );
      const lawn = lawns
        .filter((_, i) => distances[i * 3] <= -(walk + 1))
        .sort((a, b) => a.d - b.d)[0];
      if (!lawn) throw new Error("the generated town has no street beside an open yard");
      return {
        boundaries,
        crops,
        lawnStreet: lawn.at,
        join: join?.at,
        joinYaw: join && Math.atan2(join.at[1] - cy, join.at[0] - cx),
      };
    },
    { centre, streetWidth: presets.parcels.street_width_m },
  );
  const near = [0, 0];
  let pair = null;
  for (const yard of boundaries)
    for (const outline of crops) {
      polygon2.closestPoint(near, outline, outline.length / 2, yard);
      const distance = vec2.squaredDistance(yard, near);
      if (!pair || distance < pair.distance) pair = { yard, field: [...near], distance };
    }
  if (!pair) throw new Error("the generated town has no drilled field beyond its yards");
  return { ...stations, yardField: vec2.lerp([0, 0], pair.yard, pair.field, 0.5) };
}
