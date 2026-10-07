// Buildings destroyed in a battle, drawn as each side knows them. On the
// camera lab's map the shelling encounter brings a five-floor U block down
// and guts a twenty-floor tower while blue watches and red, behind two slabs,
// does not; later red walks out and sees. The scene holds what each side
// draws to what it was published:
//
// - blue draws the block as its ruin, no higher than the remains it knows,
//   and the tower as its gutted shell, standing at full height and burnt;
// - red draws both intact, exactly as before the shelling, until it has seen
//   them, and then draws what blue draws;
// - switching the observed side switches the picture;
// - the same holds from where the coarsest tier draws and at the whole-map
//   view: a destroyed building's far picture is its damage state's own;
// - the camera's obstacles and the fog's occluders are the same boxes;
// - what a side knows destroyed smokes for it, and for no one else.
//
// Pictures (`throwaway/evidence/city-ruins/`): `<phase>-<side>-<station>.png`
// for the phases before, after and learned, at fixed cameras with grass,
// effects, cast lights and units off; `crop-<subject>-<phase>-<side>.png`,
// each building alone; and `battle-*.png`, the battle as it is played, with
// units, grass and smoke.
import { readFile, writeFile } from "node:fs/promises";
import {
  advance,
  buildingsSettled,
  buildingStats as stats,
  gpuWarnings,
  groundClasses,
  lab,
  presented,
  route,
  until,
} from "./_lab.mjs";
import { nodeCatalogSet } from "../src/battle/catalog/node.ts";
import { eyeOf, gap } from "./_cameraClearance.mjs";
import { rec709 } from "./_colour.mjs";
import { crop, decode, pixel } from "./_png.mjs";
import { bounds, boxRound, isBody, judge, projected, setFits } from "./_templateFit.mjs";

const REPO = new URL("../../", import.meta.url);
const HIDE_PANEL = "[data-testid=city-ruins-panel] { display: none !important; }";
/** The buildings the encounter destroys, and where each is looked at from:
 *  the block from the north, into its yard, and the tower's shelled face from
 *  the south; `lift` raises the near camera's target up a tall building.
 *  Every station looks down steeply enough that the map's own ground, not the
 *  country past its edge, lies behind the whole building: the ground-classes
 *  view is black past the edge too. */
const SUBJECTS = {
  block: {
    template: "china-apartment-block-u-5f",
    ends: "ruin",
    yaw: Math.PI / 2,
    near: { distance: 95, pitch: 0.62, lift: 0 },
  },
  tower: {
    template: "china-tower-20f",
    ends: "gutted",
    yaw: -Math.PI / 2,
    near: { distance: 200, pitch: 0.9, lift: 20 },
  },
};
/** How far past the last tier boundary a far station stands, as a factor:
 *  far enough that both subjects are past it from either one's station. */
const FAR = 1.3;
const FAR_PITCH = 0.8;
/** A pixel has changed when a channel differs by more than this (of 255):
 *  above what one state drawn twice differs by on this hardware. */
const CHANGED = 16;
/** The share of a frame that may change between two drawings of one state. */
const SAME_FRAME = 0.001;
/** How long the battle runs on before its last pictures, ticks: longer than
 *  a puff of a destroyed building's smoke lives. */
const SMOKE_RISES_TICKS = 600;
/** Clear picture round a building in its crop, pixels. */
const CROP_PX = 12;

/** A length to the millimetre: boxes cross the page as 32-bit floats. */
const metres = (m) => Number(m.toFixed(3));

async function settle(page) {
  await lab(page, () => window.__lab.frame());
  await buildingsSettled(page);
  await lab(page, () => window.__lab.frame());
}

/** Observe as `side`: its publication of the tick after this one, drawn. */
async function observeAs(page, side) {
  await route(page, "setSide", side);
  await advance(page, 1);
  await page.waitForFunction((side) => window.__lab.route.side() === side, side);
  await presented(page);
  await settle(page);
}

/** Cut the camera to `pose` exactly (a raw framing: no clearance moves it,
 *  so both sides are drawn from one eye). */
async function cut(page, pose) {
  await lab(page, (pose) => window.__lab.setCamera({ ...window.__lab.camera(), ...pose }), pose);
  await settle(page);
}

async function shot(page) {
  await lab(page, () => window.__lab.frame());
  return page.screenshot();
}

/** The share of the pixels of `a` and `b` inside `box` (the whole frame
 *  without one) of which a channel differs by more than `CHANGED`. */
function changed(a, b, box = { x: 0, y: 0, w: a.width, h: a.height }) {
  const [x0, y0] = [Math.max(0, Math.floor(box.x)), Math.max(0, Math.floor(box.y))];
  const [x1, y1] = [Math.min(a.width, box.x + box.w), Math.min(a.height, box.y + box.h)];
  let count = 0;
  let total = 0;
  for (let y = y0; y < y1; y++)
    for (let x = x0; x < x1; x++) {
      total++;
      const [pa, pb] = [pixel(a, x, y), pixel(b, x, y)];
      if (pa.some((v, i) => Math.abs(v - pb[i]) > CHANGED)) count++;
    }
  return total ? count / total : 0;
}

/** The mean brightness (0 to 255) of `picture` where `mask` has a body
 *  inside `box`. */
function brightness(picture, mask, box) {
  let sum = 0;
  let count = 0;
  for (let y = Math.max(0, Math.floor(box.y)); y < Math.min(mask.height, box.y + box.h); y++)
    for (let x = Math.max(0, Math.floor(box.x)); x < Math.min(mask.width, box.x + box.w); x++) {
      if (!isBody(pixel(mask, x, y))) continue;
      sum += rec709(pixel(picture, x, y));
      count++;
    }
  return count ? sum / count : 0;
}

export async function run(ctx) {
  const page = await ctx.newPage({ viewport: { width: 1920, height: 1080 } });
  const warnings = gpuWarnings(page);
  await ctx.openLab(page, ctx.url, 120000);
  await page.waitForFunction(() => window.__lab.route?.tick() > 3, undefined, { timeout: 60000 });
  await lab(page, () => window.__lab.route.pause());
  await page.addStyleTag({ content: HIDE_PANEL });

  // The simulation's rule for a collapsed building's remains, from the
  // catalog's building row, and each set's fit.
  const catalog = (await nodeCatalogSet("test")).units.view;
  const { fit, setOf } = await setFits();
  const boundaries = await route(page, "boundaries");
  const overview = await route(page, "overview");

  /** Each subject as the observed side knows it now: its building (the
   *  probe's row, with its parts as known) and the entry of its authored
   *  parts. */
  const subjects = async () => {
    const buildings = await route(page, "buildings");
    return Object.fromEntries(
      Object.entries(SUBJECTS).map(([name, subject]) => {
        const building = buildings.find((b) => b.template === subject.template);
        return [
          name,
          {
            ...subject,
            building,
            authored: { parts: building.authored, ...boxRound(building.authored) },
          },
        ];
      }),
    );
  };
  const authored = await subjects();
  /** The remains the simulation leaves of a building of `parts`. */
  const remainsOf = (parts) => {
    const into = catalog.props[parts[0].kind].destroyed.into;
    const height = Math.max(...parts.map((p) => p.baseZ + 2 * p.half[2])) - parts[0].baseZ;
    return Math.min(
      Math.max(height * into.building.height_fraction, into.height_m),
      into.building.max_height_m,
    );
  };

  /** The stations: each subject near and from where the coarsest tier draws,
   *  and the whole map. */
  const stations = {};
  for (const [name, s] of Object.entries(authored)) {
    const { min, max } = s.authored;
    const middle = [(min[0] + max[0]) / 2, (min[1] + max[1]) / 2];
    stations[`${name}-near`] = {
      subject: name,
      pose: {
        target: [...middle, s.near.lift],
        distance: s.near.distance,
        pitch: s.near.pitch,
        yaw: s.yaw,
      },
    };
    stations[`${name}-far`] = {
      subject: name,
      far: true,
      pose: { target: [...middle, 0], distance: boundaries[2] * FAR, pitch: FAR_PITCH, yaw: s.yaw },
    };
  }
  stations.overview = {
    far: true,
    pose: { ...overview, target: [...overview.target, 0] },
  };

  /** Every station as the observed side draws it now: the picture, the
   *  ground-classes view, the buildings' statistics, and each subject there
   *  as its authored parts and as the remains the simulation would leave. */
  async function capture(phase, side) {
    const out = {};
    for (const [id, station] of Object.entries(stations)) {
      await cut(page, station.pose);
      const png = await shot(page);
      await writeFile(ctx.evidencePath(`${phase}-${side}-${id}.png`), png);
      const seen = {};
      for (const [name, s] of Object.entries(authored)) {
        if (station.subject && station.subject !== name) continue;
        const set = fit[setOf[s.template]];
        const remains = remainsOf(s.authored.parts) / 2;
        seen[name] = {
          standing: await projected(page, s.authored, set),
          remains: await projected(
            page,
            {
              ...s.authored,
              parts: s.authored.parts.map((p) => ({ ...p, half: [p.half[0], p.half[1], remains] })),
            },
            { side_m: set.side_m, top_m: set.ruin_top_m ?? 0 },
          ),
        };
      }
      const mask = await groundClasses(page);
      await writeFile(ctx.evidencePath(`classes-${phase}-${side}-${id}.png`), mask);
      out[id] = { picture: decode(png), mask: decode(mask), stats: await stats(page), seen };
      if (station.subject && !station.far) {
        const at = bounds(seen[station.subject].standing.box, CROP_PX);
        await writeFile(
          ctx.evidencePath(`crop-${station.subject}-${phase}-${side}.png`),
          crop(out[id].picture, at.x, at.y, at.w, at.h),
        );
      }
    }
    return out;
  }

  /** What the lab hands the camera and the fog of each subject, as the
   *  observed side knows it: whether an eye inside the building's authored
   *  volume, above where its remains would reach, is held off it, and the
   *  heights of the fog's occluders on its parts. */
  async function colliders() {
    const out = {};
    const occluders = await route(page, "fogOccluders");
    for (const [name, s] of Object.entries(authored)) {
      const part = s.authored.parts[0];
      const remains = remainsOf(s.authored.parts);
      // An orbit whose eye lands over the part's middle, between the top of
      // the remains and the top of the part.
      const height = (remains + 2 * part.half[2]) / 2;
      const pose = {
        distance: 30,
        pitch: Math.asin(Math.min(0.95, height / 30)),
        yaw: -Math.PI / 2,
      };
      const back = 30 * Math.cos(pose.pitch);
      pose.target = [part.center[0], part.center[1] + back];
      const flown = await lab(page, (pose) => window.__lab.flyClearance([pose], 1 / 60), pose);
      const eye = eyeOf({ ...pose, target: [...pose.target, part.baseZ] });
      out[name] = {
        eyeInAuthored: gap(eye, s.authored.parts) === 0,
        eyeHeight: eye[2],
        hold: flown.frames[0].hold,
        occluders: s.authored.parts.map((p) => {
          const o = occluders.find((o) => o.x === p.center[0] && o.y === p.center[1]);
          return o ? metres(o.top - o.base) : null;
        }),
      };
    }
    return out;
  }

  // Before a round lands: tick 30.
  await advance(page, 30 - (await route(page, "tick")));
  await presented(page);
  await settle(page);
  // The battle as it is played, units and grass and all.
  await writeFile(ctx.evidencePath("battle-before-blue.png"), await shot(page));
  await lab(page, async () => {
    await window.__lab.suppressGrass(true);
    await window.__lab.suppressEffects(true);
    await window.__lab.suppressCastLights(true);
    await window.__lab.suppressModels(true);
  });
  const before = {
    blue: await capture("before", "blue"),
    blueKnows: await subjects(),
    smoke: (await route(page, "effects")).sources,
  };
  before.blueColliders = await colliders();
  await observeAs(page, "red");
  before.red = await capture("before", "red");
  before.redColliders = await colliders();
  await observeAs(page, "blue");

  // The shelling, until blue has been published every part of both.
  const parts = Object.values(authored).reduce((n, s) => n + s.authored.parts.length, 0);
  const shelled = await until(
    page,
    (o) => o.knownProps.filter((p) => p.authoredProp !== null).length === parts,
    900,
    15,
  );
  await presented(page);
  await settle(page);
  const after = { tick: shelled?.tick ?? null };
  const smoking = async () => (await route(page, "effects")).sources;
  after.blueKnows = await subjects();
  after.blueSmoke = await smoking();
  after.blue = await capture("after", "blue");
  after.blueColliders = await colliders();
  await observeAs(page, "red");
  after.redKnows = await subjects();
  after.redSmoke = await smoking();
  after.red = await capture("after", "red");
  after.redColliders = await colliders();
  // And back: the same side draws the same picture again.
  await observeAs(page, "blue");
  after.blueAgain = await capture("again", "blue");
  after.blueSmokeAgain = await smoking();

  const states = (known) =>
    Object.fromEntries(Object.entries(known).map(([name, s]) => [name, s.building.state]));
  const heights = (known) =>
    Object.fromEntries(
      Object.entries(known).map(([name, s]) => [name, s.building.parts.map((p) => 2 * p.half[2])]),
    );
  const fullHeights = heights(authored);
  const remains = Object.fromEntries(
    Object.entries(authored).map(([name, s]) => [name, remainsOf(s.authored.parts)]),
  );
  ctx.check(
    "blue, who watched, is published the block's parts as remains at the simulation's ruin height and the tower's as a shell at full height; red, who did not, knows both as the map has them",
    shelled !== null &&
      JSON.stringify(states(before.blueKnows)) ===
        JSON.stringify({ block: "intact", tower: "intact" }) &&
      JSON.stringify(states(after.blueKnows)) ===
        JSON.stringify({ block: "ruin", tower: "gutted" }) &&
      JSON.stringify(states(after.redKnows)) ===
        JSON.stringify({ block: "intact", tower: "intact" }) &&
      heights(after.blueKnows).block.every((h) => Math.abs(h - remains.block) < 1e-6) &&
      heights(after.blueKnows).block.length === fullHeights.block.length &&
      remains.block < Math.min(...fullHeights.block) / 2 &&
      JSON.stringify(heights(after.blueKnows).tower) === JSON.stringify(fullHeights.tower) &&
      after.blueKnows.tower.building.parts.every((p) => p.kind !== "building") &&
      JSON.stringify(heights(after.redKnows)) === JSON.stringify(fullHeights),
    JSON.stringify({
      tick: after.tick,
      blue: { states: states(after.blueKnows), heights: heights(after.blueKnows) },
      red: { states: states(after.redKnows), heights: heights(after.redKnows) },
      remains,
    }),
  );

  // What is drawn, against the boxes each side knows, at every station that
  // frames a subject and at the whole-map view.
  const verdicts = [];
  for (const [id, station] of Object.entries(stations))
    for (const name of station.subject ? [station.subject] : Object.keys(authored)) {
      const of = (frames) => {
        const { mask, seen } = frames[id];
        return {
          standing: judge(mask, seen[name].standing),
          remains: judge(mask, seen[name].remains),
        };
      };
      verdicts.push({
        station: id,
        subject: name,
        intact: of(before.blue),
        blue: of(after.blue),
        red: of(after.red),
      });
    }
  const brief = (v) => ({
    covered: v.covered,
    stray: v.stray,
    worst: v.worst,
    cores: v.cores.map((c) => +c.toFixed(2)),
  });
  const block = verdicts.filter((v) => v.subject === "block");
  ctx.check(
    "the block blue saw collapse is drawn inside the remains it knows, at every station; intact, and for red, it stands in its parts, far above them",
    block.every(
      (v) =>
        v.blue.remains.inside &&
        v.intact.standing.inside &&
        v.intact.remains.stray > 0 &&
        v.red.standing.inside &&
        v.red.remains.stray > 0 &&
        // Near, an intact block fills each part; far, a part is a few pixels.
        (v.station !== "block-near" || (v.intact.standing.stands && v.red.standing.stands)),
    ),
    JSON.stringify(
      block.map((v) => ({
        station: v.station,
        blueInRemains: brief(v.blue.remains),
        intactOverRemains: v.intact.remains.stray,
        redOverRemains: v.red.remains.stray,
      })),
    ),
  );
  const tower = verdicts.filter((v) => v.subject === "tower");
  ctx.check(
    "the tower blue saw gutted still stands through its part's full height at every station, far above where remains would reach, and inside that part as it is intact and for red",
    tower.every(
      (v) =>
        v.blue.remains.stray > 0 &&
        v.blue.standing.cores.every((c) => c === 1) &&
        // The whole-map view has the map's edge, black in this view, within
        // reach of the tower's roof: nothing there says what lies outside it.
        (v.station === "overview" ||
          (v.blue.standing.inside && v.intact.standing.inside && v.red.standing.inside)) &&
        (v.station !== "tower-near" || (v.blue.standing.stands && v.intact.standing.stands)),
    ),
    JSON.stringify(
      tower.map((v) => ({
        station: v.station,
        blue: brief(v.blue.standing),
        blueOverRemains: v.blue.remains.stray,
        intact: brief(v.intact.standing),
      })),
    ),
  );

  // The pictures: blue's changed where the buildings stand; red's did not
  // change at all; and switching sides switches them.
  const pictures = Object.entries(stations).map(([id, station]) => {
    const names = station.subject ? [station.subject] : Object.keys(authored);
    const boxes = names.map((name) =>
      bounds(
        before.blue[id].seen[name].standing.parts.flatMap((p) => p.grown),
        0,
      ),
    );
    const inBoxes = (a, b) =>
      boxes.map((box) => +changed(a[id].picture, b[id].picture, box).toFixed(4));
    return {
      station: id,
      blueChanged: inBoxes(before.blue, after.blue),
      redChanged: +changed(before.red[id].picture, after.red[id].picture).toFixed(5),
      sidesDiffer: inBoxes(after.blue, after.red),
      blueAgain: +changed(after.blue[id].picture, after.blueAgain[id].picture).toFixed(5),
      // The gutted tower against the intact one, where each is drawn.
      towerBrightness: names.includes("tower")
        ? [before.blue, after.blue].map(
            (frames) =>
              +brightness(
                frames[id].picture,
                frames[id].mask,
                boxes[names.indexOf("tower")],
              ).toFixed(1),
          )
        : null,
    };
  });
  ctx.check(
    "blue's picture changes where each destroyed building stands, at every station",
    pictures.every((p) => p.blueChanged.every((share) => share > 0.1)),
    JSON.stringify(pictures.map((p) => [p.station, p.blueChanged])),
  );
  ctx.check(
    "red's picture, having seen neither, is its picture from before the shelling at every station",
    pictures.every((p) => p.redChanged <= SAME_FRAME),
    JSON.stringify(pictures.map((p) => [p.station, p.redChanged])),
  );
  ctx.check(
    "switching the observed side switches the picture, and switching back draws the same one again",
    pictures.every(
      (p) => p.sidesDiffer.every((share) => share > 0.1) && p.blueAgain <= SAME_FRAME,
    ) &&
      Object.values(after.blue).every((f) => f.stats.fallen === 2) &&
      Object.values(after.red).every((f) => f.stats.fallen === 0) &&
      Object.values(after.blueAgain).every((f) => f.stats.fallen === 2),
    JSON.stringify(pictures.map((p) => [p.station, p.sidesDiffer, p.blueAgain])),
  );
  ctx.check(
    "the gutted tower reads burnt: darker than the intact one where it is drawn, at every station",
    pictures.every((p) => !p.towerBrightness || p.towerBrightness[1] < 0.85 * p.towerBrightness[0]),
    JSON.stringify(pictures.map((p) => [p.station, p.towerBrightness])),
  );

  // The far tier: with both subjects past the last tier boundary, what blue
  // draws of the destroyed is their damage states' rows at the coarsest tier
  // and at no other (the checks above judged those pictures); near, a
  // subject's rows at a finer tier. Red draws none.
  const tiered = Object.entries(stations).map(([id, station]) => ({
    id,
    far: !!station.far,
    // The eye's distance to the nearest part of each subject.
    ranges: Object.values(authored).map((s) =>
      Math.round(gap(eyeOf(station.pose), s.authored.parts)),
    ),
    blue: after.blue[id].stats.ruinTiers,
    red: after.red[id].stats.ruinTiers,
  }));
  ctx.check(
    "with both destroyed buildings past the last tier boundary, and at the whole-map view, blue draws their damage states' rows at the coarsest tier alone; near, at a finer one; red draws none",
    tiered.every(
      (t) =>
        t.red.every((n) => n === 0) &&
        (t.far
          ? t.ranges.every((m) => m > boundaries[2]) &&
            t.blue[3] > 0 &&
            t.blue.slice(0, 3).every((n) => n === 0)
          : t.ranges.some((m) => m < boundaries[2]) && t.blue.slice(0, 3).some((n) => n > 0)),
    ),
    JSON.stringify({ lastBoundary: Math.round(boundaries[2]), tiered }),
  );

  // A destroyed building smokes, part by part, for the side that knows.
  ctx.check(
    "each part blue knows destroyed is a smoke source for blue, before the shelling and for red nothing smokes, and switching back to blue it smokes again",
    before.smoke === 0 &&
      after.blueSmoke === parts &&
      after.redSmoke === 0 &&
      after.blueSmokeAgain === parts,
    JSON.stringify({
      before: before.smoke,
      blue: after.blueSmoke,
      red: after.redSmoke,
      blueAgain: after.blueSmokeAgain,
      parts,
    }),
  );

  // The camera and the fog are handed the same boxes.
  ctx.check(
    "the camera's obstacles and the fog's occluders are the boxes the side knows: blue's eye passes over the block's remains and is held off the gutted tower, red's is held off both, and each occluder is as tall as the part known",
    Object.values(before.blueColliders).every((c) => c.eyeInAuthored && c.hold !== "none") &&
      after.blueColliders.block.hold === "none" &&
      after.blueColliders.block.eyeHeight > remains.block &&
      after.blueColliders.tower.hold !== "none" &&
      after.redColliders.block.hold !== "none" &&
      after.redColliders.tower.hold !== "none" &&
      JSON.stringify(after.blueColliders.block.occluders) ===
        JSON.stringify(fullHeights.block.map(() => metres(remains.block))) &&
      JSON.stringify(after.blueColliders.tower.occluders) ===
        JSON.stringify(fullHeights.tower.map(metres)) &&
      JSON.stringify(after.redColliders.block.occluders) ===
        JSON.stringify(fullHeights.block.map(metres)) &&
      JSON.stringify(after.redColliders.tower.occluders) ===
        JSON.stringify(fullHeights.tower.map(metres)),
    JSON.stringify({ blue: after.blueColliders, red: after.redColliders }),
  );

  // Red walks out from behind the slabs and sees both: it draws what blue
  // draws. Until its scripted walk starts it has learned nothing, however
  // long ago the buildings went.
  await observeAs(page, "red");
  const encounter = JSON.parse(
    await readFile(new URL("fixtures/maps/camera-lab/encounters/shelling.json", REPO), "utf8"),
  );
  const walk = encounter.scripts.find((s) => s.side === "red" && s.order.kind === "move").tick;
  await advance(page, walk - (await route(page, "tick")));
  await presented(page);
  await settle(page);
  const waiting = { tick: await route(page, "tick"), knows: states(await subjects()) };
  const learned = await until(
    page,
    (o) => o.knownProps.filter((p) => p.authoredProp !== null).length === parts,
    4500,
    30,
  );
  await presented(page);
  await settle(page);
  const late = { redKnows: await subjects(), red: await capture("learned", "red") };
  const lateVerdicts = Object.entries(stations)
    .filter(([, station]) => station.subject)
    .map(([id, station]) => ({
      station: id,
      remains: judge(late.red[id].mask, late.red[id].seen[station.subject].remains),
      standing: judge(late.red[id].mask, late.red[id].seen[station.subject].standing),
    }));
  ctx.check(
    "once red has seen them it knows what blue knows and draws it: the block inside its remains, the tower standing gutted",
    learned !== null &&
      waiting.tick === walk &&
      waiting.tick > after.tick &&
      JSON.stringify(waiting.knows) === JSON.stringify({ block: "intact", tower: "intact" }) &&
      learned.tick > walk &&
      JSON.stringify(states(late.redKnows)) === JSON.stringify(states(after.blueKnows)) &&
      JSON.stringify(heights(late.redKnows)) === JSON.stringify(heights(after.blueKnows)) &&
      Object.values(late.red).every((f) => f.stats.fallen === 2) &&
      lateVerdicts.every((v) =>
        v.station.startsWith("block") ? v.remains.inside : v.standing.inside && v.remains.stray > 0,
      ),
    JSON.stringify({
      destroyedBy: after.tick,
      stillIntactAt: waiting,
      learnedAt: learned?.tick ?? null,
      states: states(late.redKnows),
      verdicts: lateVerdicts.map((v) => [v.station, v.remains.stray, v.standing.stray]),
    }),
  );

  // The battle as it is played at its end, for the eye: a while on, so the
  // smoke over what red has just learned of has had time to rise.
  await observeAs(page, "blue");
  await advance(page, SMOKE_RISES_TICKS);
  await presented(page);
  await lab(page, async () => {
    await window.__lab.suppressGrass(false);
    await window.__lab.suppressEffects(false);
    await window.__lab.suppressCastLights(false);
    await window.__lab.suppressModels(false);
    window.__lab.reset();
    await window.__lab.frame();
  });
  await settle(page);
  await writeFile(ctx.evidencePath("battle-after-blue.png"), await shot(page));
  // And each destroyed building close, under its smoke.
  for (const id of ["block-near", "tower-near"]) {
    await cut(page, stations[id].pose);
    await writeFile(ctx.evidencePath(`battle-after-blue-${id}.png`), await shot(page));
  }

  ctx.check(
    "no WebGPU validation warning was logged",
    warnings.length === 0,
    warnings.slice(0, 3).join(" | "),
  );
  console.log(
    `METRIC city-ruins: blue knew both destroyed by tick ${after.tick}; red walked from tick ${walk} and knew by tick ${learned?.tick}; far station ${(boundaries[2] * FAR).toFixed(0)} m, whole-map view ${overview.distance.toFixed(0)} m; tower brightness intact/gutted ${pictures
      .map((p) => p.towerBrightness?.join("/"))
      .filter(Boolean)
      .join(", ")}`,
  );
  await ctx.writeEvidence("meta.json", {
    browser: ctx.browser,
    adapter: await page.evaluate(() => window.__lab.adapter),
    viewport: [1920, 1080],
    stations: Object.fromEntries(Object.entries(stations).map(([id, s]) => [id, s.pose])),
    remains,
    fullHeights,
    ticks: { shelled: after.tick, walk, learned: learned?.tick ?? null },
    pictures,
    verdicts: verdicts.map((v) => ({
      station: v.station,
      subject: v.subject,
      blue: { standing: brief(v.blue.standing), remains: brief(v.blue.remains) },
      red: { standing: brief(v.red.standing), remains: brief(v.red.remains) },
    })),
    tiered,
    colliders: { before: before.blueColliders, blue: after.blueColliders, red: after.redColliders },
  });
}
