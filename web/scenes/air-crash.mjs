// A helicopter shot down: the side sees it fall from where it was hit, along
// its spinning arc, to the ground where its wreck then lies, and the
// airframe is drawn all the way down, trailing smoke as it did while it flew
// damaged. The simulation's tests own the arc and the publication's tests its
// fields; this judges the drawn fall and trail, and leaves a contact sheet
// from the hit to the wreck at rest and a crop of the trail.
import { writeFile } from "node:fs/promises";
import { decode, mostChanged, writeCrop } from "./_png.mjs";
import { lab, obs, advance, presented, openBattle, gpuWarnings, hideHud, aim } from "./_lab.mjs";
import { writeSheet } from "./_sheet.mjs";

const HELI = "test_heli";
/** Ticks between the sheet's frames, and how long it runs on after landing. */
const EVERY = 6;
const AFTER = 36;
/** A tick it has flown its first 25 m or so, just before the jeep hits it. */
const SMOKE_TICK = 55;

export async function run(ctx) {
  const page = await openBattle(ctx);
  const warnings = gpuWarnings(page);
  await hideHud(page);
  const css = (p) => lab(page, (q) => window.__lab.projectToCss(q[0], q[1], q[2]), p);
  const crashes = (o) => o.crashes ?? [];

  /** Draw the presented tick and save it; return the PNG buffer. */
  const shoot = async (name) => {
    await presented(page);
    await page.evaluate(() => window.__lab.frame());
    const shot = await page.screenshot();
    await writeFile(ctx.evidencePath(`frame-${name}-1280x800.png`), shot);
    return shot;
  };

  // Fly until it is hit: the last tick it is a unit. One hit from death, it
  // trails smoke all the way: shot once it is flying fast, with a crop round
  // the airframe and the 30 m of air behind it.
  let o = await obs(page);
  let alive = null;
  let trail = null;
  for (let k = 0; k < 600 && (alive = o.own.find((u) => u.kind === HELI)); k++) {
    if (!trail && o.tick >= SMOKE_TICK) {
      // Framed on the stretch of air it has just flown, then back.
      const [x, y, z] = alive.position;
      const view = await lab(page, () => window.__lab.camera());
      await aim(page, [x - 15, y, z], { distance: 60 }, { onGround: false });
      const shot = decode(await shoot("smoke"));
      // It flies east (the encounter's order): the air behind it is west.
      const a = await css([x, y, z + 1.5]);
      const b = await css([x - 30, y, z + 1.5]);
      const pad = 150;
      const [x0, x1] = [Math.min(a[0], b[0]) - pad, Math.max(a[0], b[0]) + pad];
      const [y0, y1] = [Math.min(a[1], b[1]) - pad, Math.max(a[1], b[1]) + pad];
      await writeCrop(
        shot,
        ctx.evidencePath("crop-smoke-trail-2x.png"),
        (x0 + x1) / 2,
        (y0 + y1) / 2,
        (x1 - x0) / 2,
        (y1 - y0) / 2,
        2,
      );
      const effects = await lab(page, () => window.__lab.stats().effects);
      trail = { smoking: alive.smoking, effects };
      await lab(page, (v) => window.__lab.setCamera(v), view);
    }
    await advance(page, 1);
    o = await obs(page);
  }
  ctx.check(
    "one hit from death, it publishes that it smokes, and smoke is drawn",
    trail?.smoking === true && trail.effects.instances > 0,
    JSON.stringify(trail),
  );
  ctx.check("the gun jeep shoots the helicopter down", !alive, JSON.stringify(o.own));
  if (alive) return;
  const hit = o.tick;

  // From the hit, a frame every few ticks until its wreck has lain a while;
  // the side's view of the falling airframe every tick.
  const path = [];
  const cells = [];
  let landed = null;
  let midFall = null;
  for (let tick = hit; landed === null || tick <= landed + AFTER; tick++) {
    if (tick > o.tick) {
      await advance(page, 1);
      o = await obs(page);
    }
    const falling = crashes(o);
    if (falling.length) path.push({ tick: o.tick, ...falling[0] });
    const wrecked = o.knownProps.some((p) => p.wreckOf === HELI);
    if (wrecked && landed === null) landed = o.tick;
    if ((tick - hit) % EVERY === 0 || tick === landed) {
      const z = falling[0]?.position[2];
      const state = z !== undefined ? `falling, z ${z.toFixed(1)} m` : wrecked ? "its wreck" : "-";
      const shot = await shoot(`t${tick - hit}`);
      cells.push({ png: shot, caption: `+${tick - hit} ticks: ${state}` });
      if (falling.length && midFall === null && tick - hit >= 20)
        midFall = { shot, at: falling[0].position };
    }
    if (tick - hit > 600) break;
  }
  await writeSheet(ctx, "fall-sheet.png", {
    title: "air-crash: from the hit to the wreck at rest",
    columns: 4,
    cellWidth: 400,
    cells,
  });

  ctx.check(
    "every tick it falls the side sees it, from where it was hit down to the ground",
    path.length > 20 &&
      path[0].position[2] > 15 &&
      path.every((p, i) => i === 0 || p.position[2] <= path[i - 1].position[2] + 0.05) &&
      path[path.length - 1].position[2] < 3,
    JSON.stringify({ hit, landed, first: path[0], last: path[path.length - 1], n: path.length }),
  );
  ctx.check(
    "it keeps its momentum: it falls forward, not straight down",
    path.length > 1 && path[path.length - 1].position[0] - path[0].position[0] > 20,
    JSON.stringify({ first: path[0]?.position, last: path[path.length - 1]?.position }),
  );
  const wreck = o.knownProps.find((p) => p.wreckOf === HELI);
  const end = path[path.length - 1]?.position;
  ctx.check(
    "its wreck lies where it struck the ground",
    !!wreck && !!end && Math.hypot(wreck.center[0] - end[0], wreck.center[1] - end[1]) < 3,
    JSON.stringify({ wreck: wreck?.center, end }),
  );

  // Drawn in the air mid-fall: the frame there differs from the same place
  // once the airframe has gone, with its wreck on the ground far below.
  if (midFall) {
    const at = await css([midFall.at[0], midFall.at[1], midFall.at[2] + 1.5]);
    const after = decode(cells[cells.length - 1].png);
    const during = decode(midFall.shot);
    const drawn = mostChanged(during, after, at, 14);
    ctx.check("the falling airframe is drawn mid-fall", drawn > 60, JSON.stringify({ at, drawn }));
    await writeCrop(during, ctx.evidencePath("crop-mid-fall-2x.png"), at[0], at[1], 160, 90, 2);
  } else ctx.check("the falling airframe is drawn mid-fall", false, "no frame mid-fall");
  if (end) {
    const at = await css(end);
    await writeCrop(
      decode(cells[cells.length - 1].png),
      ctx.evidencePath("crop-wreck-2x.png"),
      at[0],
      at[1],
      160,
      90,
      2,
    );
  }
  ctx.check("no GPU validation warnings", warnings.length === 0, warnings.join("\n"));
}
