// Units at rest with no battle, each on its own `ShowcaseStage`: the rotors
// of a parked rotorcraft turn between frames, a squad stands scattered with
// each soldier in his own pose and the launcher's operator carrying his kit,
// and every model stands on the ground. Also reports, for every aircraft
// appearance, what of it rests on the ground at rest (throwaway/armory/04/).
// With `?yard=1` each body stands on its spot in the Armory's yard, dressed
// with the yard's props, seen from the spot's camera (one shot per body).
// The tests behind the posing:
//   crates/presentation/tests/showcase.rs
//   crates/presentation/tests/sound.rs::an_idle_vehicle_sounds_as_a_stationary_battle_vehicle_does
//   crates/game-wasm/tests/showground.rs
//   web/tests/armoryYard.test.ts (the yard's layout and spots)
// This judges the drawn page.
import { mkdir, writeFile } from "node:fs/promises";
import { decode, writeCrop } from "./_png.mjs";
import { gpuWarnings } from "./_lab.mjs";

const VIEWPORT = { width: 1800, height: 700 };
/** How near the ground a model's lowest point must stand, metres. */
const GROUNDED_M = 0.02;
/** How far past a spot's clear disc its own dressing stands, metres. */
const DRESSING_M = 10;
/** A channel moving more than this is a changed pixel. */
const MOVED = 8;

const sc = (page, fn, arg) => page.evaluate(fn, arg);

/** The pixels of `png` within `r` of `at` (offset by `rect`) that moved
 *  past `MOVED` against `other`. */
function movedWithin(png, other, rect, at, r) {
  let n = 0;
  const [cx, cy] = [rect.x + at[0], rect.y + at[1]];
  for (let y = Math.floor(cy - r); y <= cy + r; y++)
    for (let x = Math.floor(cx - r); x <= cx + r; x++) {
      if (x < 0 || y < 0 || x >= png.width || y >= png.height) continue;
      if ((x - cx) ** 2 + (y - cy) ** 2 > r * r) continue;
      const i = (y * png.width + x) * 4;
      if ([0, 1, 2].some((c) => Math.abs(png.data[i + c] - other.data[i + c]) > MOVED)) n++;
    }
  return n;
}

/** Twice the area of the triangle a, b, c. */
const area2 = (a, b, c) => Math.abs((b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1]));

/** Opens `url` on `page` and waits until every stage is up, paused. */
async function openStages(ctx, page, url, logged) {
  await page.goto(url);
  try {
    await page.waitForFunction(() => window.__showcase?.ready(), undefined, { timeout: 120000 });
  } catch (error) {
    const state = await sc(page, () => ({
      probes: Boolean(window.__showcase),
      kinds: window.__showcase?.kinds(),
      text: document.body.innerText.slice(0, 300),
    }));
    await writeFile(ctx.evidencePath("showcase-timeout.png"), await page.screenshot());
    throw new Error(`the stages never came up: ${JSON.stringify({ state, logged })}`, {
      cause: error,
    });
  }
  await sc(page, () => window.__showcase.pause(true));
}

export async function run(ctx) {
  const page = await ctx.newPage({ viewport: VIEWPORT });
  const warnings = gpuWarnings(page);
  const logged = [];
  page.on("console", (m) => m.type() === "error" && logged.push(m.text().slice(0, 300)));
  page.on("pageerror", (e) => logged.push(String(e).slice(0, 300)));
  await openStages(ctx, page, ctx.url, logged);
  const kinds = await sc(page, () => window.__showcase.kinds());
  ctx.check("the lab shows three units side by side", kinds.length === 3, kinds.join(", "));

  const shot = async (time) => {
    const drawn = await sc(page, (t) => window.__showcase.draw(t), time);
    return { drawn, png: decode(await page.screenshot()) };
  };
  const a = await shot(2);
  const b = await shot(2 + 1 / 30);
  ctx.check("every stage draws", a.drawn && b.drawn);

  // ---- the rotorcraft: its rotors turn, drawn as blur, on the ground.
  const rotors = await Promise.all(
    kinds.map((k) => sc(page, (k) => window.__showcase.rotors(k), k)),
  );
  const heli = kinds[rotors.findIndex((discs) => discs.length > 0)];
  ctx.check("one unit is a rotorcraft whose rotor discs are drawn", heli !== undefined);
  if (heli) {
    const rect = await sc(page, (k) => window.__showcase.rect(k), heli);
    const discs = rotors[kinds.indexOf(heli)];
    const moved = discs.map((d) => movedWithin(a.png, b.png, rect, d.at, d.radius));
    // The control: the same disc over a stage where nothing turns stays put.
    const still = kinds.find((k, i) => k !== heli && rotors[i].length === 0);
    const stillRect = await sc(page, (k) => window.__showcase.rect(k), still);
    const control = movedWithin(a.png, b.png, stillRect, discs[0].at, discs[0].radius);
    ctx.check(
      "the parked rotorcraft's rotors turn between frames, and nothing else moves",
      moved[0] > 0 && control === 0,
      JSON.stringify({ moved, control, discs }),
    );
    const loops = await sc(page, (k) => window.__showcase.loops(k), heli);
    ctx.check(
      "the rotorcraft idles with its engine heard",
      loops.length > 0,
      JSON.stringify(loops),
    );
    await writeCrop(
      b.png,
      ctx.evidencePath("showcase-heli.png"),
      rect.x + rect.width / 2,
      rect.y + rect.height / 2,
      rect.width / 2,
      rect.height / 2,
      1,
    );
  }

  // ---- the squad: scattered, each soldier his own pose, the launcher's
  // operator in his carried kit.
  const standing = await Promise.all(
    kinds.map((k) => sc(page, (k) => window.__showcase.standing(k), k)),
  );
  const squadAt = standing.findIndex((s) => s.soldiers.length > 0);
  ctx.check("one unit is a squad", squadAt >= 0);
  if (squadAt >= 0) {
    const squad = kinds[squadAt];
    const { soldiers, operators } = standing[squadAt];
    const spots = soldiers.map((s) => s.at);
    const spread = Math.max(
      ...spots.flatMap((p, i) =>
        spots.slice(i + 1).flatMap((q, j) => spots.slice(i + j + 2).map((r) => area2(p, q, r))),
      ),
    );
    ctx.check("the squad's soldiers do not stand in a line", spread > 0.1, JSON.stringify(spots));
    const models = await sc(page, (k) => window.__showcase.models(k), squad);
    const near = (p) => models.find((m) => Math.hypot(m.at[0] - p[0], m.at[1] - p[1]) < 1e-3);
    const poses = soldiers.map((s) => near(s.at)?.pose.join(","));
    ctx.check(
      "each soldier is drawn, in his own pose",
      poses.every(Boolean) && new Set(poses).size === poses.length,
      `${new Set(poses).size} poses for ${poses.length} soldiers`,
    );
    const kits = await sc(page, (k) => window.__showcase.carried(k), squad);
    const worn = operators
      .map((id, m) => ({ m, id, kit: kits[m] }))
      .filter((o) => o.id !== null && o.kit.length > 0)
      .map((o) => ({ ...o, wears: near(soldiers.find((s) => s.id === o.id).at)?.name }));
    ctx.check(
      "each operator at rest wears his weapon's carried kit",
      worn.length > 0 && worn.every((o) => o.kit.includes(o.wears)),
      JSON.stringify(worn),
    );
    const loops = await sc(page, (k) => window.__showcase.loops(k), squad);
    ctx.check("a squad has no engine to idle", loops.length === 0, JSON.stringify(loops));
    const rect = await sc(page, (k) => window.__showcase.rect(k), squad);
    await writeCrop(
      b.png,
      ctx.evidencePath("showcase-squad.png"),
      rect.x + rect.width / 2,
      rect.y + rect.height / 2,
      rect.width / 2,
      rect.height / 2,
      1,
    );
  }

  // ---- every model stands on the ground.
  for (const kind of kinds) {
    const gaps = await sc(page, (k) => window.__showcase.grounded(k), kind);
    const off = gaps.filter((g) => g.gap === null || Math.abs(g.gap) > GROUNDED_M);
    ctx.check(
      `${kind} stands on the ground`,
      gaps.length > 0 && off.length === 0,
      JSON.stringify(off),
    );
  }
  await writeFile(ctx.evidencePath("showcase.png"), await page.screenshot());

  // ---- what of each aircraft rests on the ground, parked.
  const parked = await sc(page, () => window.__showcase.parkedAircraft());
  const report = new URL("../../throwaway/armory/04/", import.meta.url);
  await mkdir(report, { recursive: true });
  await writeFile(new URL("aircraft-ground.json", report), JSON.stringify(parked, null, 2));
  ctx.check(
    "every aircraft appearance is reported",
    parked.length > 0,
    `${parked.length} reported`,
  );

  await yardShots(ctx, page, logged);
  ctx.check("no WebGPU warnings", warnings.length === 0, warnings.join(" | "));
}

/** Each body on its spot in the yard: standing where the spot is, the
 *  yard's props drawn round it, on the ground; one shot per spot camera. */
async function yardShots(ctx, page, logged) {
  await openStages(ctx, page, ctx.urlFor({ yard: "1" }), logged);
  const kinds = await sc(page, () => window.__showcase.kinds());
  for (const kind of kinds) {
    await openStages(ctx, page, ctx.urlFor({ yard: "1", unit: kind }), logged);
    const drawn = await sc(page, () => window.__showcase.draw(2));
    const { spot, standing, props, models, gaps } = await sc(
      page,
      (k) => ({
        spot: window.__showcase.spot(k),
        standing: window.__showcase.standing(k),
        props: window.__showcase.yardModels(),
        models: window.__showcase.models(k),
        gaps: window.__showcase.grounded(k),
      }),
      kind,
    );
    ctx.check(
      `${kind} stands on its spot, ${spot?.id}`,
      drawn && spot && standing.at[0] === spot.at[0] && standing.at[1] === spot.at[1],
      JSON.stringify({ spot: spot?.at, at: standing?.at }),
    );
    // The spot's own dressing, drawn where the yard stands it; the other
    // spots' lies out of this camera's view.
    const near = props.filter(
      (p) => Math.hypot(p.at[0] - spot.at[0], p.at[1] - spot.at[1]) < spot.clear_m + DRESSING_M,
    );
    const missing = near.filter(
      (p) =>
        !models.some(
          (m) => m.name === p.name && Math.hypot(m.at[0] - p.at[0], m.at[1] - p.at[1]) < 1e-3,
        ),
    );
    ctx.check(
      `the ${near.length} props round ${spot.id} are drawn round ${kind}`,
      near.length > 0 && missing.length === 0,
      JSON.stringify(missing),
    );
    const off = gaps.filter((g) => g.gap === null || Math.abs(g.gap) > GROUNDED_M);
    ctx.check(`${kind} stands on the yard's ground`, off.length === 0, JSON.stringify(off));
    await writeFile(ctx.evidencePath(`yard-${spot?.id}-${kind}.png`), await page.screenshot());
  }
}
