// The helicopters' closing scene (D14): the Apache flies in from the map's
// edge, pops up over the village roofs and kills the tank behind them, is
// shot down by the IFV the tower hid, and its wreck comes down in the wood and
// flattens the trees there. The simulation's test (`air_closing.rs`) owns the
// beat; this holds blue's observation to the same beat as the lab draws it,
// and leaves full frames of each moment, a crop of the wreck and a contact
// sheet of the whole beat for review.
import { writeFile } from "node:fs/promises";
import { decode, writeCrop } from "./_png.mjs";
import { lab, obs, advance, presented, openBattle, gpuWarnings, hideHud, aim } from "./_lab.mjs";
import { writeSheet } from "./_sheet.mjs";
import { game } from "./_units.mjs";
import { loadEncounter, loadMap } from "../src/maps/node.ts";

const map = loadMap("air").definition;
const encounter = loadEncounter("air", "closing");
const [APACHE, TANK, IFV] = encounter.units.map((u) => u.kind);
const start = encounter.units[0].position;
const cruise = game.air.cruise_agl_m;

/** Whether XY point `q` lies inside polygon `ring`. */
function inPolygon(ring, q) {
  let inside = false;
  for (let i = 0, j = ring.length - 1; i < ring.length; j = i++) {
    const [a, b] = [ring[i], ring[j]];
    if (
      a[1] > q[1] !== b[1] > q[1] &&
      q[0] < ((b[0] - a[0]) * (q[1] - a[1])) / (b[1] - a[1]) + a[0]
    )
      inside = !inside;
  }
  return inside;
}
const inWood = (q) =>
  map.forests.some((f) => f.shape.kind === "polygon" && inPolygon(f.shape.ring, q));

export async function run(ctx) {
  const page = await openBattle(ctx);
  const warnings = gpuWarnings(page);
  await hideHud(page);
  const surface = (p) => lab(page, (q) => window.__lab.route.surfaceZ(q[0], q[1]), p);
  const cells = [];

  /** Draw the presented tick and save it as a full frame; return the PNG. */
  const shoot = async (name, caption) => {
    await presented(page);
    await page.evaluate(() => window.__lab.frame());
    const shot = await page.screenshot();
    await writeFile(ctx.evidencePath(`frame-${name}-1280x800.png`), shot);
    cells.push({ png: shot, caption });
    return shot;
  };
  const apacheOf = (o) => o.own.find((u) => u.kind === APACHE);

  // Fly-in: it enters at the west edge at cruise height, and the village and
  // its roofs lie ahead. Framed from behind its shoulder, the village beyond.
  let o = await obs(page);
  let heli = apacheOf(o);
  ctx.check(
    "the Apache enters at the map's west edge at cruise height",
    !!heli &&
      heli.position[0] < 40 &&
      Math.abs(heli.position[2] - (await surface(heli.position)) - cruise) < 1.5,
    JSON.stringify({ start, at: heli?.position }),
  );
  await advance(page, 75);
  o = await obs(page);
  heli = apacheOf(o);
  await aim(page, [heli.position[0] + 70, heli.position[1], 10], {
    distance: 150,
    pitch: 0.32,
    yaw: -Math.PI / 2 - 0.55,
  });
  await shoot("fly-in", `fly-in, tick ${o.tick}: the Apache at the west edge, the village ahead`);

  // The pop-up: blue first identifies the tank once the Apache has climbed
  // over the roofs, then kills it while the Apache lives.
  let seen = null;
  let killed = null;
  for (let k = 0; k < 900 && !killed; k++) {
    await advance(page, 1);
    o = await obs(page);
    heli = apacheOf(o);
    if (!seen && o.identified.some((u) => u.kind === TANK) && heli)
      seen = { tick: o.tick, height: heli.position[2] - (await surface(heli.position)) };
    if (seen && o.knownProps.some((p) => p.wreckOf === TANK))
      killed = { tick: o.tick, alive: !!heli };
  }
  ctx.check(
    "blue sees the tank only once the Apache has popped up over the roofs",
    !!seen && seen.height > cruise + 5,
    JSON.stringify(seen),
  );
  ctx.check(
    "the Apache kills the tank and lives",
    !!killed && killed.alive,
    JSON.stringify(killed),
  );
  if (!killed) return;
  const tankWreck = o.knownProps.find((p) => p.wreckOf === TANK);
  // From the south-east, low: the tank burning in front, the Apache beyond
  // it over the roofs against the sky.
  await aim(page, [(heli.position[0] + tankWreck.center[0]) / 2, heli.position[1], 16], {
    distance: 120,
    pitch: 0.2,
    yaw: -Math.PI / 4,
  });
  await advance(page, 12);
  o = await obs(page);
  await shoot("tank-kill", `tank kill, tick ${o.tick}: the Apache over the roofs, the tank hit`);

  // The shoot-down: flying on east past the tower, the Apache sees the IFV
  // it hid, and is downed in the air by it. Once the IFV is seen the camera
  // stands behind it, low, looking west over the wood: the IFV in front, the
  // Apache coming on beyond. Frames: the IFV's first rounds in the air (blue
  // still sees them), then the tick it is hit.
  let downed = null;
  let last = null;
  let ifv = null;
  let fired = false;
  for (let k = 0; k < 900 && !downed; k++) {
    await advance(page, 1);
    o = await obs(page);
    heli = apacheOf(o);
    if (heli) last = heli.position;
    else downed = { tick: o.tick, at: last };
    const seen = heli && o.identified.find((u) => u.kind === IFV);
    if (seen && !ifv) {
      ifv = seen;
      await aim(page, [480, 470, 14], { distance: 150, pitch: 0.15, yaw: -0.35 });
    }
    if (ifv && heli && !fired && o.projectiles.some((p) => !p.own)) {
      fired = true;
      await shoot("ifv-fires", `the IFV fires, tick ${o.tick}: its rounds climb at the Apache`);
    }
  }
  ctx.check(
    "the Apache is shot down in the air after the tank's kill",
    !!downed && downed.tick > killed.tick && downed.at[2] - (await surface(downed.at)) > cruise - 3,
    JSON.stringify(downed),
  );
  ctx.check("blue saw the IFV that downed it before it fell", !!ifv, JSON.stringify(o.identified));
  ctx.check("blue saw the IFV's rounds climb at the Apache", fired, "no enemy round seen");
  if (!downed) return;
  // Close on the airframe the tick it is hit, from the south, low: it hangs
  // against the sky over the wood it will fall into.
  const hitAt = o.crashes?.[0]?.position ?? downed.at;
  await aim(page, hitAt, { distance: 55, pitch: 0.12, yaw: -Math.PI / 2 }, { onGround: false });
  await shoot("shoot-down", `shoot-down, tick ${o.tick}: hit, it falls on toward the wood`);

  // Its fall, then its wreck at rest in the wood, among the trees it felled.
  let landed = null;
  let midFall = false;
  for (let k = 0; k < 600 && !landed; k++) {
    await advance(page, 1);
    o = await obs(page);
    const falling = o.crashes?.[0];
    if (falling && !midFall && falling.position[2] < downed.at[2] - 8) {
      midFall = true;
      await aim(
        page,
        falling.position,
        { distance: 70, pitch: 0.18, yaw: -Math.PI / 2 },
        {
          onGround: false,
        },
      );
      await shoot("falling", `falling, tick ${o.tick}: z ${falling.position[2].toFixed(1)} m`);
    }
    const wreck = o.knownProps.find((p) => p.wreckOf === APACHE);
    if (wreck) landed = { tick: o.tick, wreck };
  }
  ctx.check("the falling airframe is drawn on its way down", midFall, "no frame mid-fall");
  ctx.check("its wreck comes to rest", !!landed, "no Apache wreck");
  if (!landed) return;
  await advance(page, 150);
  o = await obs(page);
  const wreck = o.knownProps.find((p) => p.wreckOf === APACHE);
  const at = wreck.center;
  ctx.check("its wreck lies in the wood", inWood(at), JSON.stringify(at));
  const felled = o.fallenBodies.filter((f) => Math.hypot(f.at[0] - at[0], f.at[1] - at[1]) < 12);
  // The wreck at rest, from nearly overhead (lower, the crowns round the
  // gap hide it): the airframe burning in the gap its crash tore, the
  // stumps of the trees it felled.
  await aim(page, at, { distance: 48, pitch: 1.2, yaw: -1.25 });
  const rest = decode(await shoot("wreck", `wreck at rest, tick ${o.tick}: in the felled wood`));
  const scenery = await lab(page, () => window.__lab.stats().scenery.felled);
  ctx.check(
    "blue, which saw it go down, sees the trees its crash felled lying there",
    felled.length > 0 && scenery.trees > 0 && !scenery.falling,
    JSON.stringify({ felled: felled.length, scenery, at }),
  );
  const c = await lab(page, (q) => window.__lab.projectToCss(q[0], q[1], q[2]), [
    at[0],
    at[1],
    await surface(at),
  ]);
  await writeCrop(rest, ctx.evidencePath("crop-wreck-2x.png"), c[0], c[1], 200, 120, 2);

  await writeSheet(ctx, "beat-sheet.png", {
    title: "air-closing: fly-in, tank kill, shoot-down, fall, wreck at rest",
    columns: 3,
    cellWidth: 420,
    cells,
  });
  ctx.check("no GPU validation warnings", warnings.length === 0, warnings.join("\n"));
}
