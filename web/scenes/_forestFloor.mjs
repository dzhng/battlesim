// The forest floor at the ground rig's stations: the bodies that lie on it
// (logs, boulders) and its dressing (ferns, bushes, saplings, small rocks,
// fallen branches), seen under the crowns and with the crowns off.
//
//   FLOOR_ONLY=1 bun run --cwd web scene -- ground    the checks below
//   FLOOR_SHOTS=1 bun run --cwd web scene -- ground   every station's frame
//   FLOOR_COST=1 bun run --cwd web scene -- ground    the dressing's frame cost
//
// The shots go to throwaway/evidence/ground/ as
// `floor-<map>-<station>[-open|-classes].png`: the frame, the frame without
// trees, and the ground's classes.
import { writeFile } from "node:fs/promises";
import { lab } from "./_lab.mjs";
import { decode } from "./_png.mjs";
import { classAt, openStations, shoot } from "./_groundStations.mjs";

/** Where the floor is judged: inside the village's west wood and at its
 *  edge, at a log and a boulder where the forest rule lays any, the river
 *  lab's wood, and a generated map's wood edge. */
const FLOOR_STATIONS = {
  village: ["forest-deep-25", "forest-edge-65", "forest-65", "floor-log-25", "floor-boulder-25"],
  river: ["wood-65"],
  generated: ["forest-edge-65", "forest-edge-250"],
};

/** The rig hides every model; the floor's bodies are models. */
const showBodies = (page) => page.evaluate(() => window.__lab.suppressModels(false));

/** `FLOOR_SHOTS=1`, or `FLOOR_SHOTS=village,river` for those maps alone. A
 *  station with nothing to stand on (no log on the map) is left out. */
export async function forestFloorShots(ctx, only) {
  const maps = only === "1" ? Object.keys(FLOOR_STATIONS) : only.split(",");
  for (const map of maps) {
    const page = await openStations(ctx, map);
    await showBodies(page);
    for (const station of FLOOR_STATIONS[map]) {
      const save = async (suffix, options) =>
        writeFile(
          ctx.evidencePath(`floor-${map}-${station}${suffix}.png`),
          await shoot(page, map, station, options),
        );
      try {
        await save("", {});
      } catch (error) {
        if (!/nothing to stand/.test(error.message)) throw error;
        continue;
      }
      await save("-open", { trees: false });
      await save("-classes", { view: "ground-classes" });
    }
    await page.close();
  }
}

/** A pixel is the dressing's where the frame differs this much with it off. */
const CHANGED = 10;

/** The dressing is drawn, and only on the forest's floor: at the wood's edge
 *  with the crowns off, the pixels that change with the dressing lie over
 *  forest ground (a piece at the very edge leans a little over the verge). */
export async function forestFloor(ctx) {
  const page = await openStations(ctx, "village");
  const station = "forest-edge-65";
  const frame = async (dressing) => {
    await lab(page, (off) => window.__lab.suppressDressing(off), !dressing);
    return decode(await shoot(page, "village", station, { trees: false, grass: false }));
  };
  const [dressed, bare] = [await frame(true), await frame(false)];
  // The ground's classes under the same pixels, with no scenery over them.
  await lab(page, () => window.__lab.suppressDressing(true));
  const mask = decode(
    await shoot(page, "village", station, { view: "ground-classes", trees: false, grass: false }),
  );
  await lab(page, () => window.__lab.suppressDressing(false));
  const pixels = { changed: 0, floor: 0, verge: 0, outside: 0, forest: 0 };
  for (let y = 0; y < mask.height; y++)
    for (let x = 0; x < mask.width; x++) {
      const forest = classAt(mask, x, y)?.forest;
      if (forest === "inside") pixels.forest++;
      const i = (y * mask.width + x) * 4;
      let d = 0;
      for (let c = 0; c < 3; c++) d = Math.max(d, Math.abs(dressed.data[i + c] - bare.data[i + c]));
      if (d <= CHANGED) continue;
      pixels.changed++;
      pixels[forest === "inside" ? "floor" : forest === "verge" ? "verge" : "outside"]++;
    }
  const stats = await lab(page, () => window.__lab.stats().scenery.dressing);
  await ctx.writeEvidence("forest-floor.json", { pixels, stats });
  ctx.check(
    "the forest floor is dressed, and nothing is dressed outside a forest",
    stats.placed > 1000 &&
      // The dressing is there to be seen: a hundredth of the floor in view
      // at least (thin fronds, mostly, from 65 m).
      pixels.changed > 0.01 * pixels.forest &&
      pixels.outside + pixels.verge < 0.03 * pixels.changed &&
      pixels.outside < 0.005 * pixels.changed,
    JSON.stringify({ pixels, stats }),
  );
  await page.close();
}

/** Frames a cost batch draws before its GPU time is read, and the pairs a
 *  station is measured over. */
const COST_FRAMES = 120;
const COST_PAIRS = 4;
const median = (values) => [...values].sort((a, b) => a - b)[Math.floor(values.length / 2)];

/** FLOOR_COST=1: what the dressing costs a frame over a wood from the play
 *  camera and at a wood's edge from the tactical camera: the same frozen
 *  frame with the dressing on and off in a few interleaved batches, the
 *  median of the paired differences. Run it alone, under the GPU lock. */
export async function forestFloorCost(ctx) {
  const result = { stations: {} };
  for (const [map, stations] of [
    ["village", ["forest-65"]],
    ["generated", ["forest-edge-250"]],
  ]) {
    const page = await openStations(ctx, map);
    for (const station of stations) {
      await shoot(page, map, station);
      // The page draws on demand, and the timer's mean runs over the frames
      // drawn: a view change resets it, then every frame is forced.
      const batch = (off) =>
        lab(
          page,
          async ({ off, frames }) => {
            await window.__lab.suppressDressing(off);
            await window.__lab.setFrameView("final");
            for (let i = 0; i < frames; i++) await window.__lab.frame();
            return window.__lab.stats().gpu.meanMs;
          },
          { off, frames: COST_FRAMES },
        );
      const rows = { on: [], off: [] };
      for (let pair = 0; pair < COST_PAIRS; pair++) {
        rows.off.push(await batch(true));
        rows.on.push(await batch(false));
      }
      // The frame's scenery draw calls, counted over the last whole frame.
      const draws = (off) =>
        lab(
          page,
          async (off) => {
            await window.__lab.suppressDressing(off);
            await window.__lab.frame();
            await window.__lab.frame();
            return window.__lab.stats().scenery.draws;
          },
          off,
        );
      result.stations[`${map} ${station}`] = {
        bareMs: +median(rows.off).toFixed(3),
        dressingMs: +median(rows.on.map((v, i) => v - rows.off[i])).toFixed(3),
        differences: rows.on.map((v, i) => +(v - rows.off[i]).toFixed(3)),
        draws: { bare: await draws(true), dressed: await draws(false) },
        drawn: await lab(page, () => window.__lab.stats().scenery.dressing),
      };
    }
    result.adapter = await page.evaluate(() => window.__lab.adapter);
    await page.close();
  }
  await ctx.writeEvidence("forest-floor-cost.json", result);
  ctx.check(
    "the forest floor's dressing costs under a millisecond of GPU a frame",
    Object.values(result.stations).every((s) => s.dressingMs < 1),
    JSON.stringify(result.stations),
  );
}
