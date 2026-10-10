// A helicopter shot down: the side sees it fall from where it was hit, along
// its spinning arc, to the ground where its wreck then lies, and the
// airframe is drawn all the way down. The simulation's tests own the arc and
// the publication's tests its fields; this judges the drawn fall, and leaves
// a contact sheet from the hit to the wreck at rest.
import { writeFile } from "node:fs/promises";
import { decode, mostChanged, writeCrop } from "./_png.mjs";
import { lab, obs, advance, presented, openBattle, gpuWarnings, hideHud } from "./_lab.mjs";
import { writeSheet } from "./_sheet.mjs";

const HELI = "test_heli";
/** Ticks between the sheet's frames, and how long it runs on after landing. */
const EVERY = 6;
const AFTER = 36;

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

  // Fly until it is hit: the last tick it is a unit.
  let o = await obs(page);
  let alive = null;
  for (let k = 0; k < 600 && (alive = o.own.find((u) => u.kind === HELI)); k++) {
    await advance(page, 1);
    o = await obs(page);
  }
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
