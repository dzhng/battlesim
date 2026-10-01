// C69: the river lab. The water is where the simulation's distance says, a
// squad and a tank cross it only by the bridge, and the drawn river is
// measured for SG2's question: does a river carved into the 4 m grid read
// round, or stepped?
import { advance, aim, lab, obs, openBattle, snapshot } from "./_lab.mjs";
import { decode, pixel, writeCrop } from "./_png.mjs";

const VIEWPORT = { width: 1920, height: 1080 };
/** Looking north, as the lab opens. */
const YAW = -Math.PI / 2;
/** The play camera's framing (Defilade's): 65 m at 0.85 rad. */
const PLAY = { distance: 65, pitch: 0.85, yaw: YAW };
const TOP = { distance: 250, pitch: 1.5, yaw: YAW };
/** The lab's opening frame: the whole river. */
const OPENING = { target: [300, 240, 0], distance: 430, pitch: 0.95, yaw: YAW };
/** SG2's bar: the largest step in the bank's shading, as a share of the flat
 *  ground's luminance. */
const FACET_STEP_MAX = 0.08;
/** The banks are walked this far outside the water's edge, in metres: on
 *  the bank (it runs 4.8 m in the lab) and just past its top. */
const BANK_LINES_M = [1, 2.5, 4, 6];

const surfaceAt = (page, p) => lab(page, (q) => window.__lab.route.surfaceAt(q[0], q[1]), p);

/** Linear-light luminance of an sRGB pixel, in [0, 1]. */
function luminance([r, g, b]) {
  const linear = (v) => {
    const c = v / 255;
    return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b);
}

/** The mean luminance of the 3×3 pixels round `p`. */
function luminanceAt(png, p) {
  let sum = 0;
  for (let dy = -1; dy <= 1; dy++)
    for (let dx = -1; dx <= 1; dx++) sum += luminance(pixel(png, p[0] + dx, p[1] + dy));
  return sum / 9;
}

/** Lines along the river's banks, each a fixed distance outside the water's
 *  edge (`offsets`, metres) on one side, sampled every metre from the first
 *  bend to the wood: page points under the current camera, in order along the
 *  stream (`lines`, a break where a point leaves the frame); and points of
 *  flat ground well clear of the bank (`flat`). Walking a line, the ground's
 *  shading changes only as the bank turns: a jump from one metre to the next
 *  is a step. */
const bankLines = (page, offsets) =>
  lab(
    page,
    (offsets) => {
      const { rivers } = window.__lab.route.exports();
      const stride = window.__lab.route.layout().riverStride;
      const [width, height] = [window.innerWidth, window.innerHeight];
      const project = ([x, y]) => {
        const p = window.__lab.projectToCss(x, y, window.__lab.route.groundHeight(x, y) ?? 0);
        return p && p[0] > 4 && p[1] > 4 && p[0] < width - 4 && p[1] < height - 4 ? p : null;
      };
      const inside = (x, y) => {
        let best = -Infinity;
        for (let o = 0; o < rivers.length; o += stride) {
          const ax = rivers[o],
            ay = rivers[o + 1],
            dx = rivers[o + 2] - ax,
            dy = rivers[o + 3] - ay;
          const t = Math.min(1, Math.max(0, ((x - ax) * dx + (y - ay) * dy) / (dx * dx + dy * dy)));
          const half = rivers[o + 4] + (rivers[o + 5] - rivers[o + 4]) * t;
          best = Math.max(best, half - Math.hypot(x - ax - dx * t, y - ay - dy * t));
        }
        return best;
      };
      const lines = [];
      for (const offset of offsets)
        for (const side of [-1, 1]) {
          let line = [];
          for (let o = 0; o < rivers.length; o += stride) {
            const [ax, ay, bx, by, halfA, halfB] = rivers.subarray(o, o + 6);
            const length = Math.hypot(bx - ax, by - ay);
            for (let along = 0; along < length; along += 1) {
              const t = along / length;
              const reach = (halfA + (halfB - halfA) * t + offset) * side;
              const x = ax + (bx - ax) * t - ((by - ay) / length) * reach,
                y = ay + (by - ay) * t + ((bx - ax) / length) * reach;
              // West of the wood, clear of the bridge, and truly `offset`
              // out (the inside of a bend folds a line back over itself).
              const p =
                x > 100 && x < 400 && Math.abs(inside(x, y) + offset) < 0.05
                  ? project([x, y])
                  : null;
              if (p) line.push(p);
              else if (line.length) {
                lines.push(line);
                line = [];
              }
            }
          }
          if (line.length) lines.push(line);
        }
      const flat = [];
      for (let y = 100; y < 400; y += 12)
        for (let x = 100; x < 400; x += 12) {
          const at = inside(x, y);
          if (at < -20 && at > -60) flat.push(project([x, y]));
        }
      return { lines, flat: flat.filter(Boolean) };
    },
    offsets,
  );

export async function run(ctx) {
  // A pinned tick: the orders below then land on the same tick every run.
  const page = await openBattle(ctx, { viewport: VIEWPORT, grass: true, timeout: 60000, tick: 30 });
  await page.addStyleTag({ content: ".lab-panel { display: none }" });

  // --- the rule, through the simulation's own surface query -----------------
  const stations = await lab(page, () => window.__lab.route.stations);
  const midstream = await surfaceAt(page, stations.midstream);
  ctx.check(
    "the middle of the 30 m stretch is water no mover stands on",
    midstream.kind === "water" && !midstream.traversable,
    JSON.stringify(midstream),
  );
  const deck = await surfaceAt(page, [60, 240]);
  ctx.check(
    "the bridge deck over the water is walkable",
    deck.kind === "bridge" && deck.traversable,
    JSON.stringify(deck),
  );
  // West of its first bend the 12 m stretch runs along y = 240: its edge is
  // 6 m off the centreline, and the water lies 1.2 m under the land.
  const edge = await lab(page, () => ({
    wet: window.__lab.route.surfaceAt(78, 240 - 5.9),
    dry: window.__lab.route.surfaceAt(78, 240 - 6.1),
    waterline: window.__lab.route.groundHeight(78, 234),
  }));
  ctx.check(
    "the 12 m stretch's edge is where the distance puts it, and the ground meets the surface there",
    edge.wet.kind === "water" &&
      edge.dry.kind === "ground" &&
      edge.dry.traversable &&
      Math.abs(edge.waterline - -1.2) < 0.05,
    JSON.stringify(edge),
  );

  // --- the look: frames for review, and SG2's measures ------------------------
  await lab(page, () => window.__lab.suppressFog(true));
  await aim(page, [260, 255], TOP);
  await snapshot(ctx, page, "top-250.png");
  for (const [name, at] of [
    ["meander", [240, 300]],
    ["bridge", [60, 232]],
    ["wide", [500, 222]],
    ["narrow", [100, 236]],
  ]) {
    await aim(page, at, PLAY);
    await snapshot(ctx, page, `play-65-${name}.png`);
  }
  await aim(page, [150, 258], { ...PLAY, distance: 25 });
  await snapshot(ctx, page, "close-25-bank.png");
  await aim(page, [300, 150], { distance: 200, pitch: 0.3, yaw: YAW });
  await snapshot(ctx, page, "low-200.png");

  // The ground's shading alone: every dry triangle one tint, lit as the
  // ground is, so a step from one metre of a bank to the next is shading and
  // nothing else. SG2's bar is 8% of the flat ground's luminance.
  await lab(page, () => window.__lab.route.setGroundView("shading"));
  await page.waitForTimeout(300);
  const facets = {};
  for (const [name, at, view] of [
    ["top-250", [260, 255], TOP],
    ["play-65", [240, 300], PLAY],
  ]) {
    await aim(page, at, view);
    const frame = decode(await snapshot(ctx, page, `facets-${name}.png`));
    const bank = await bankLines(page, BANK_LINES_M);
    const flats = bank.flat.map((p) => luminanceAt(frame, p)).sort((x, y) => x - y);
    const flat = flats[flats.length >> 1];
    const steps = bank.lines
      .flatMap((line) =>
        line.slice(1).map((p, k) => ({
          at: p,
          step: Math.abs(luminanceAt(frame, p) - luminanceAt(frame, line[k])) / flat,
        })),
      )
      .sort((x, y) => x.step - y.step);
    const share = (q) => +(steps[Math.floor((steps.length - 1) * q)]?.step ?? 0).toFixed(4);
    facets[name] = {
      steps: steps.length,
      flat: +flat.toFixed(4),
      median: share(0.5),
      p95: share(0.95),
      p99: share(0.99),
      max: share(1),
      overBar: steps.filter((s) => s.step > FACET_STEP_MAX).length,
    };
    const worst = steps[steps.length - 1];
    if (worst)
      await writeCrop(
        frame,
        ctx.evidencePath(`facets-${name}-worst-4x.png`),
        worst.at[0],
        worst.at[1],
        60,
        40,
        4,
      );
  }
  await lab(page, () => window.__lab.route.setGroundView("surface"));
  await page.waitForTimeout(300);
  await ctx.writeEvidence("sg2.json", { facets });
  ctx.check(
    "walking a bank, the ground's shading never steps by more than 8% of the flat ground's luminance",
    facets["top-250"].steps > 1000 &&
      facets["play-65"].steps > 100 &&
      Object.values(facets).every((f) => f.max <= FACET_STEP_MAX),
    JSON.stringify(facets),
  );

  // --- a squad and a tank cross by the bridge --------------------------------
  await lab(page, (view) => window.__lab.setCamera({ ...window.__lab.camera(), ...view }), OPENING);
  await lab(page, () => window.__lab.route.demo("Over the bridge"));
  const wet = [];
  const onDeck = new Set();
  let own = [];
  let framed = false;
  for (let step = 0; step < 200; step++) {
    await advance(page, 15);
    own = (await obs(page)).own;
    const standing = await lab(
      page,
      (units) =>
        units.map((u) =>
          (u.members.length ? u.members : [u.position]).map(
            (p) => window.__lab.route.surfaceAt(p[0], p[1])?.kind,
          ),
        ),
      own,
    );
    standing.forEach((kinds, k) => {
      if (kinds.includes("water") || kinds.includes(undefined))
        wet.push({ unit: own[k].kind, tick: step * 15, at: own[k].position });
      if (kinds.includes("bridge")) onDeck.add(own[k].kind);
    });
    if (onDeck.size === 2 && !framed) {
      framed = true;
      await aim(page, [60, 240], PLAY);
      await snapshot(ctx, page, "crossing-play-65.png");
    }
    if (own.every((u) => u.state === "idle")) break;
  }
  // The far bank begins 6 m north of the 12 m stretch's centreline.
  const across = (p) => p[1] > 246;
  const tank = own.find((u) => u.kind === "tank");
  const squad = own.find((u) => u.kind === "rifle");
  const crossing = {
    tank: { state: tank.state, at: tank.position.map((v) => +v.toFixed(1)) },
    squad: {
      state: squad.state,
      across: squad.members.filter(across).length,
      of: squad.members.length,
    },
    onDeck: [...onDeck],
  };
  // The river's part: the way across is the deck, and both take it. A squad
  // wider than the deck can leave soldiers standing at the water beside it:
  // they do not sidestep ground they cannot enter. That is the movement
  // rules' to fix; the scenario table holds it as pending
  // (`c69-river-bridge`, `c69-river-around`), and the count is recorded here.
  ctx.check(
    "ordered up the road, the tank and the squad cross on the deck and the tank reaches its place",
    tank.state === "idle" &&
      Math.hypot(tank.position[0] - stations.north[0], tank.position[1] - stations.north[1] - 30) <
        3 &&
      onDeck.size === 2 &&
      crossing.squad.across > 0,
    JSON.stringify(crossing),
  );
  ctx.check(
    "nobody stood in the water on the way",
    wet.length === 0,
    JSON.stringify(wet.slice(0, 3)),
  );

  await lab(page, () => window.__lab.route.demo("Into the river"));
  await advance(page, 30);
  const sent = (await obs(page)).own;
  const goals = await lab(
    page,
    (units) => units.map((u) => u.goal && window.__lab.route.surfaceAt(u.goal[0], u.goal[1])?.kind),
    sent,
  );
  const ordered = sent.map((u, k) => ({
    kind: u.kind,
    state: u.state,
    goal: u.goal,
    on: goals[k],
  }));
  // Joint placement may move every member onto a bank; no order grants
  // permission to stand in the water.
  ctx.check(
    "ordered into the water, no unit is given a way in: its route is blocked, or its goal is moved to dry ground",
    ordered.every((u) => u.state === "route_blocked" || !u.goal || (u.on && u.on !== "water")),
    JSON.stringify(ordered),
  );
  await ctx.writeEvidence("meta.json", {
    browser: ctx.browser,
    adapter: await page.evaluate(() => window.__lab.adapter),
    viewport: [VIEWPORT.width, VIEWPORT.height],
    dpr: 1,
  });

  // The same frames under a low sun, where a facet shows most.
  await page.close();
  const dusk = await openBattle(ctx, {
    viewport: VIEWPORT,
    url: `${ctx.url}?sun=0.3`,
    grass: true,
    timeout: 60000,
  });
  await dusk.addStyleTag({ content: ".lab-panel { display: none }" });
  await lab(dusk, () => window.__lab.suppressFog(true));
  await aim(dusk, [260, 255], TOP);
  await snapshot(ctx, dusk, "low-sun-top-250.png");
  await aim(dusk, [240, 300], PLAY);
  await snapshot(ctx, dusk, "low-sun-play-65-meander.png");
  await aim(dusk, [150, 258], { ...PLAY, distance: 25 });
  await snapshot(ctx, dusk, "low-sun-close-25-bank.png");
}
