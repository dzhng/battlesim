// The facade lab: what a facade material can say beyond "opaque", judged on
// the facade lab's kit standing on a plain ground.
//
// Cutouts (a grille, a perforated sheet): a hole shows what is behind it and
// the metal does not, with and without fog; the shade a panel casts follows
// how much of it is there; and a panel too far away to resolve its holes is
// still drawn.
//
// Glass (free-standing panes, and the block's windows): it dims what is
// behind it without hiding it, is hidden by what stands in front, reads the
// same from its back and whatever order it is drawn in, and from the street
// a window stays darker than its wall under every sun.
//
// Rooms (behind every window of the block): dim, never a hole and never lit;
// the same room whatever the camera, the tier or the packing order; and the
// same picture under every sun.
//
// `FACADE_COST=1` measures instead: the frame's GPU time with and without
// each kind of surface (paired, interleaved) over a field of blocks.
import { writeFile } from "node:fs/promises";
import { median, rec709 as luminance } from "./_colour.mjs";
import { gpuMs, gpuWarnings, lab } from "./_lab.mjs";
import { PNG } from "pngjs";
import { decode, pixel, around, writeCrop } from "./_png.mjs";

const VIEWPORT = { width: 1920, height: 1080 };
const HIDE_PANEL = "[data-testid=facade-panel] { display: none !important; }";
const SURFACES = ["cutout", "blended", "room"];
/** Blocks in the cost run's field. */
const COST_BLOCKS = 288;

const sum = (a, b) => a.reduce((s, v, i) => s + Math.abs(v - b[i]), 0);
/** Mean colour of the pixels within `r` of `p`. */
function mean(png, p, r) {
  const total = [0, 0, 0];
  let n = 0;
  for (const [x, y] of around(png, p, r)) {
    pixel(png, x, y).forEach((v, i) => (total[i] += v));
    n++;
  }
  return total.map((v) => v / n);
}

async function open(ctx, query = "") {
  const page = await ctx.newPage({ viewport: VIEWPORT });
  const warnings = gpuWarnings(page);
  await page.goto(`${ctx.url}${query}`);
  await page.waitForFunction(
    () =>
      document.querySelector("[data-testid=error]") ||
      window.__lab?.error ||
      (window.__lab?.ready && window.__lab.route?.stand && window.__lab.stats),
    undefined,
    { timeout: 300000 },
  );
  // The kit is installed once the appearances load: draw until it is.
  for (let tries = 0; ; tries++) {
    await lab(page, () => window.__lab.frame());
    if (await lab(page, () => window.__lab.stats().models.instances > 0)) break;
    if (tries > 600) throw new Error("the facade lab drew no model");
    await page.waitForTimeout(100);
  }
  await page.addStyleTag({ content: HIDE_PANEL });
  return { page, warnings };
}

async function shot(ctx, page, file) {
  await lab(page, () => window.__lab.frame());
  const png = await page.screenshot();
  if (file) await writeFile(ctx.evidencePath(file), png);
  return decode(png);
}

/** The frame as it stands, and the same frame with no model drawn. */
async function paired(ctx, page, file) {
  const drawn = await shot(ctx, page, file);
  await lab(page, () => window.__lab.suppressModels(true));
  const bare = await shot(ctx, page, null);
  await lab(page, () => window.__lab.suppressModels(false));
  return { drawn, bare };
}

/** Cut the camera to a station as the rig places it, and wait for it to rest. */
async function stand(page, id, distance) {
  await lab(page, ([id, distance]) => window.__lab.route.stand(id, distance), [id, distance]);
  await lab(page, () => window.__lab.frame());
  await page.waitForFunction(() => window.__lab.clearance().settled, undefined, { timeout: 15000 });
}

/** The harness's own framing of `target`, drawn as given. */
const frame = (page, target, view) =>
  lab(
    page,
    ({ target, view }) => window.__lab.setCamera({ ...window.__lab.camera(), ...view, target }),
    { target, view },
  );
const project = (page, p) => lab(page, (p) => window.__lab.projectToCss(p[0], p[1], p[2]), p);
const points = (page, module, locals) =>
  lab(page, ([module, locals]) => window.__lab.route.points(module, locals), [module, locals]);

/** Where each cutout's recipe puts a hole and its metal on the panel, in the
 *  panel's own frame (`textures.py`: `grille`, bars 12.5 cm apart between
 *  rails half a metre apart; `perforated`, holes 41.7 mm apart). */
const CUTOUTS = {
  grille_panel: { hole: [0, 0, 0.5], metal: [0, 0, 0.75] },
  sheet_panel: { hole: [0.5 / 24, 0, 0.5 - 0.5 / 24 + 0.5], metal: [0, 0, 0.5 - 0.5 / 24 + 0.5] },
};
/** The share of each cutout that is there: the grille's bars (24 mm in every
 *  125) and rails (40 mm in every 500), the sheet less its holes. */
const COVERAGE = { grille_panel: 0.26, sheet_panel: 0.72 };
/** A panel's middle, in its own frame. */
const PANEL_MIDDLE = [0, 0, 1.1];

/** A hole of each cutout shows what the frame draws without the panel, and
 *  its metal does not. */
async function holes(ctx, page, tag) {
  const yaw = (await lab(page, () => window.__lab.camera())).yaw;
  const out = {};
  for (const [module, at] of Object.entries(CUTOUTS)) {
    const [hole, metal] = await points(page, module, [at.hole, at.metal]);
    await frame(page, hole, { distance: 4, pitch: 0.22, yaw });
    const { drawn, bare } = await paired(ctx, page, `${tag}-${module}-1920x1080.png`);
    const [holePx, metalPx] = [await project(page, hole), await project(page, metal)];
    await writeCrop(drawn, ctx.evidencePath(`${tag}-${module}-crop.png`), ...holePx, 120, 90, 4);
    out[module] = {
      hole: sum(mean(drawn, holePx, 1), mean(bare, holePx, 1)),
      metal: sum(mean(drawn, metalPx, 1), mean(bare, metalPx, 1)),
    };
  }
  return out;
}

async function cutouts(ctx) {
  const { page, warnings } = await open(ctx);
  const station = {};
  for (const distance of ["close", "tactical", "far"]) {
    await stand(page, "cutouts", distance);
    station[distance] = await paired(ctx, page, `cutouts-${distance}-1920x1080.png`);
  }

  // Colour and depth: through a hole the ground behind is drawn as if the
  // panel were not there; the metal beside it is the panel's.
  const clear = await holes(ctx, page, "cutouts-hole");
  ctx.check(
    "a cutout's hole shows the ground behind it and its metal does not, in colour and at the prepass's depth",
    Object.values(clear).every((c) => c.hole <= 6 && c.metal >= 40),
    JSON.stringify(clear),
  );

  // Shadow: from nearly overhead, how far the ground in each panel's shade is
  // dimmed, as a share of how far a solid wall's shade dims it: the share of
  // the panel that is there.
  const { azimuth, elevation } = await lab(page, () => window.__lab.route.light());
  const toSun = [
    Math.cos(elevation) * Math.cos(azimuth),
    Math.cos(elevation) * Math.sin(azimuth),
    Math.sin(elevation),
  ];
  const yaw = (await lab(page, () => window.__lab.camera())).yaw;
  /** How much darker the ground is under the shade of a point of `module`
   *  than with nothing drawn: 0 sunlit, about 0.2 in full shade. */
  const dimmed = async (module, local, file) => {
    const [caster] = await points(page, module, [local]);
    const k = caster[2] / toSun[2];
    const ground = [caster[0] - toSun[0] * k, caster[1] - toSun[1] * k, 0];
    await frame(page, ground, { distance: 22, pitch: 1.35, yaw });
    const above = await paired(ctx, page, file);
    const px = await project(page, ground);
    if (file)
      await writeCrop(
        above.drawn,
        ctx.evidencePath(file.replace("-1920x1080", "-crop")),
        ...px,
        200,
        140,
        3,
      );
    return 1 - luminance(mean(above.drawn, px, 6)) / luminance(mean(above.bare, px, 6));
  };
  const shade = {
    // A point of the block's side wall, four metres up.
    solid: await dimmed("block_shell", [-4.8, 2.6, 4], null),
    grille: await dimmed("grille_panel", PANEL_MIDDLE, "cutouts-shadow-grille-1920x1080.png"),
    sheet: await dimmed("sheet_panel", PANEL_MIDDLE, "cutouts-shadow-sheet-1920x1080.png"),
  };
  const share = { grille: shade.grille / shade.solid, sheet: shade.sheet / shade.solid };
  ctx.check(
    "a cutout casts its share of shade: the grille (a quarter there) about a quarter of a wall's, the sheet (seven tenths there) about seven tenths",
    shade.solid > 0.08 &&
      Math.abs(share.grille - COVERAGE.grille_panel) < 0.12 &&
      Math.abs(share.sheet - COVERAGE.sheet_panel) < 0.15,
    JSON.stringify({ shade, share }),
  );

  // Mips: 250 m away a panel is a few pixels of unresolved holes, and still drawn.
  await stand(page, "cutouts", "far");
  const far = {};
  for (const module of Object.keys(CUTOUTS)) {
    const [at] = await points(page, module, [PANEL_MIDDLE]);
    const px = await project(page, at);
    far[module] = sum(mean(station.far.drawn, px, 2), mean(station.far.bare, px, 2));
    await writeCrop(
      station.far.drawn,
      ctx.evidencePath(`cutouts-far-${module}-crop.png`),
      ...px,
      40,
      30,
      8,
    );
  }
  ctx.check(
    "a cutout too far away to resolve its holes is still drawn",
    Object.values(far).every((d) => d >= 12),
    JSON.stringify(far),
  );
  ctx.check(
    "no WebGPU validation warning was logged",
    warnings.length === 0,
    warnings.slice(0, 3).join(" | "),
  );
  const meta = {
    browser: ctx.browser,
    adapter: await page.evaluate(() => window.__lab.adapter),
    viewport: [VIEWPORT.width, VIEWPORT.height],
    layout: await lab(page, () => window.__lab.route.layout()),
    light: { azimuth, elevation },
    cameras: {},
  };
  for (const distance of ["close", "tactical", "far"]) {
    await stand(page, "cutouts", distance);
    meta.cameras[`cutouts-${distance}`] = await lab(page, () => window.__lab.camera());
  }
  await page.context().close();

  // Fog: sight ends just behind the panels, so the ground through a hole is
  // unseen ground, drawn as the frame draws it without the panel.
  const fogged = await open(ctx, "?fog=1");
  await stand(fogged.page, "cutouts", "close");
  await shot(ctx, fogged.page, "cutouts-fog-close-1920x1080.png");
  const unseen = await holes(ctx, fogged.page, "cutouts-fog-hole");
  ctx.check(
    "across a fog boundary a cutout's hole shows the unseen ground behind it, and its metal does not",
    Object.values(unseen).every((c) => c.hole <= 6 && c.metal >= 40),
    JSON.stringify(unseen),
  );
  await fogged.page.context().close();
  return meta;
}

/** The same frame with and without one kind of surface drawn. */
async function withAndWithout(ctx, page, surface, file) {
  const drawn = await shot(ctx, page, file);
  await lab(page, (s) => window.__lab.suppressSurface(s, true), surface);
  const without = await shot(ctx, page, null);
  await lab(page, (s) => window.__lab.suppressSurface(s, false), surface);
  return { drawn, without };
}

/** Points of the glass station's panes, each in its pane's own frame: the
 *  first three step back and across, the fourth is turned round. */
const PANES = {
  // Glass with nothing but ground behind it.
  single: { pane: 0, at: [-0.5, 0, 1.2] },
  // The first pane's glass over the second's.
  double: { pane: 0, at: [0.4, 0, 1.2] },
  // The first pane's frame (its near face), in front of the second pane's glass.
  frame: { pane: 0, at: [0.78, -0.03, 1.2], within: 0 },
  // The turned pane, seen from its back.
  back: { pane: 3, at: [0.3, 0, 1.2] },
};
/** A window pane of the block's middle bay, a floor up, and its wall beside it. */
const WINDOW = { pane: [0, 0.13, 1.7], wall: [-1.1, 0, 1.7] };
/** Sun azimuths the block is judged under from the street, by where the sun
 *  stands for a camera south of the facade: as the fixture has it, behind the
 *  camera, in front of it (the facade in its own shade) and along the street. */
const SUNS = { fixture: null, behind: -Math.PI / 2, ahead: Math.PI / 2, along: Math.PI };

async function glass(ctx) {
  const { page, warnings } = await open(ctx);
  for (const distance of ["close", "tactical", "far"]) {
    await stand(page, "glass", distance);
    await shot(ctx, page, `glass-${distance}-1920x1080.png`);
  }
  await stand(page, "glass", "close");
  const { drawn, without } = await withAndWithout(ctx, page, "blended", null);
  const at = {};
  for (const [name, p] of Object.entries(PANES)) {
    const [world] = await lab(
      page,
      ([locals, nth]) => window.__lab.route.points("pane", locals, nth),
      [[p.at], p.pane],
    );
    const px = await project(page, world);
    const [with_, bare] = [mean(drawn, px, p.within ?? 2), mean(without, px, p.within ?? 2)];
    at[name] = { kept: luminance(with_) / luminance(bare), changed: sum(with_, bare) };
  }
  await writeCrop(drawn, ctx.evidencePath("glass-close-crop.png"), 960, 560, 300, 160, 3);
  ctx.check(
    "glass dims what is behind it without hiding it, two panes more than one, and the same from its back",
    at.single.kept > 0.3 &&
      at.single.kept < 0.92 &&
      at.double.kept < at.single.kept - 0.04 &&
      at.double.kept > 0.1 &&
      Math.abs(at.back.kept - at.single.kept) < 0.12,
    JSON.stringify(at),
  );
  ctx.check(
    "an opaque frame in front of glass is drawn as it is without the glass",
    at.frame.changed <= 3,
    JSON.stringify(at.frame),
  );
  const cameras = { "glass-close": await lab(page, () => window.__lab.camera()) };
  await page.context().close();

  // Order: the same modules handed over in the opposite order blend to the
  // same picture, so glass needs no sort.
  const reversed = await open(ctx, "?order=reversed");
  await stand(reversed.page, "glass", "close");
  const other = await shot(ctx, reversed.page, "glass-close-reversed-1920x1080.png");
  let most = 0;
  let differing = 0;
  for (let i = 0; i < drawn.data.length; i += 4) {
    const delta = Math.max(
      ...[0, 1, 2].map((c) => Math.abs(drawn.data[i + c] - other.data[i + c])),
    );
    most = Math.max(most, delta);
    if (delta > 2) differing++;
  }
  ctx.check(
    "overlapping panes drawn in the opposite order blend to the same picture",
    most <= 6,
    JSON.stringify({ largest: most, pixelsOver2: differing }),
  );
  await reversed.page.context().close();

  // Fog: sight ends behind the first panes.
  const fogged = await open(ctx, "?fog=1");
  await stand(fogged.page, "glass", "close");
  await shot(ctx, fogged.page, "glass-fog-close-1920x1080.png");
  await fogged.page.context().close();

  // From the street, under each sun: a window is darker than its wall.
  const windows = {};
  for (const [name, azimuth] of Object.entries(SUNS)) {
    const sunned = await open(ctx, azimuth === null ? "" : `?azimuth=${azimuth}&sun=0.5`);
    await stand(sunned.page, "facade", "close");
    const { drawn: street, without: bare } = await withAndWithout(
      ctx,
      sunned.page,
      "blended",
      `facade-street-${name}-1920x1080.png`,
    );
    const [pane, wall] = await lab(
      sunned.page,
      (locals) => window.__lab.route.points("bay_window", locals, 1),
      [WINDOW.pane, WINDOW.wall],
    );
    const [panePx, wallPx] = [await project(sunned.page, pane), await project(sunned.page, wall)];
    await writeCrop(
      street,
      ctx.evidencePath(`facade-street-${name}-crop.png`),
      ...panePx,
      160,
      110,
      4,
    );
    windows[name] = {
      pane: luminance(mean(street, panePx, 4)),
      wall: luminance(mean(street, wallPx, 4)),
      // The room behind the pane, with no glass over it.
      room: mean(bare, panePx, 4),
    };
    if (name === "fixture")
      cameras["facade-close"] = await lab(sunned.page, () => window.__lab.camera());
    warnings.push(...sunned.warnings);
    await sunned.page.context().close();
  }
  ctx.check(
    "from the street a window is a dark opening under every sun, never a plate as bright as its wall",
    Object.values(windows).every((w) => w.pane < 0.8 * w.wall),
    JSON.stringify(windows),
  );
  ctx.check(
    "no WebGPU validation warning was logged with glass drawn",
    warnings.length === 0,
    warnings.slice(0, 3).join(" | "),
  );
  return { cameras, windows };
}

/** The block's windows: its six flats (two floors of three bays) and its
 *  three shops, each by its pane's middle in its bay's own frame. */
const WINDOWS = [
  ...[0, 1, 2, 3, 4, 5].map((nth) => ({ module: "bay_window", nth, at: [0, 0.13, 1.7] })),
  ...[0, 1, 2].map((nth) => ({ module: "bay_shop", nth, at: [0, 0.13, 1.45] })),
];
/** The mean colour behind each window's pane in `png`, as `page` frames it. */
async function behindWindows(page, png, r = 5) {
  const out = [];
  for (const w of WINDOWS) {
    const [world] = await lab(
      page,
      ([module, locals, nth]) => window.__lab.route.points(module, locals, nth),
      [w.module, [w.at], w.nth],
    );
    out.push(mean(png, await project(page, world), r));
  }
  return out;
}
const farthest = (a, b) => Math.max(...a.map((colour, i) => sum(colour, b[i])));

async function rooms(ctx, suns) {
  const { page, warnings } = await open(ctx);
  const cameras = {};
  const target = (
    await lab(page, () => window.__lab.route.points("block_shell", [[0, 0, 4.5]]))
  )[0];
  const yaw = -Math.PI / 2 - 0.35;
  // The facade from 30 m and 80 m as the rig pitches the camera there, and a
  // shopfront from the street.
  const views = {
    "30m": { target, distance: 30, pitch: 0.3, yaw },
    "80m": { target, distance: 80, pitch: 0.85, yaw },
    storefront: {
      target: [target[0], target[1], 1.4],
      distance: 9,
      pitch: 0.05,
      yaw: -Math.PI / 2 - 0.2,
    },
  };
  const seen = {};
  for (const [name, view] of Object.entries(views)) {
    await frame(page, view.target, view);
    const { drawn, without } = await withAndWithout(
      ctx,
      page,
      "blended",
      `rooms-${name}-1920x1080.png`,
    );
    await writeFile(
      ctx.evidencePath(`rooms-${name}-no-glass-1920x1080.png`),
      PNG.sync.write(without),
    );
    await writeCrop(drawn, ctx.evidencePath(`rooms-${name}-crop.png`), 960, 540, 240, 135, 4);
    const wall = (
      await lab(page, () => window.__lab.route.points("bay_window", [[-1.1, 0, 1.7]], 1))
    )[0];
    seen[name] = {
      rooms: (await behindWindows(page, without)).map(luminance),
      glazed: (await behindWindows(page, drawn)).map(luminance),
      wall: luminance(mean(drawn, await project(page, wall), 4)),
    };
    cameras[`rooms-${name}`] = await lab(page, () => window.__lab.camera());
  }
  // Neither a hole (black) nor lit (as bright as the sunlit wall beside it).
  const dim = (s) =>
    [...s.rooms, ...s.glazed].every((l) => l >= 6) &&
    [...s.rooms, ...s.glazed].every((l) => l <= 0.7 * s.wall);
  ctx.check(
    "behind every window is a dim room: never black as a hole, never as bright as the wall beside it, with glass over it or without",
    dim(seen["30m"]) && dim(seen["80m"]),
    JSON.stringify(seen),
  );

  // Which room a window shows depends on where it stands and nothing else:
  // not on the camera, the tier, or the order the frame packs its models in.
  await frame(page, views["30m"].target, views["30m"]);
  const before = await shot(ctx, page, null);
  const picked = await behindWindows(page, before);
  await stand(page, "cutouts", "far");
  await shot(ctx, page, null);
  await frame(page, views["30m"].target, views["30m"]);
  const after = await shot(ctx, page, null);
  const returned = farthest(picked, await behindWindows(page, after));
  const reversed = await open(ctx, "?order=reversed");
  await frame(reversed.page, views["30m"].target, views["30m"]);
  const other = await shot(ctx, reversed.page, "rooms-30m-reversed-1920x1080.png");
  const reordered = farthest(picked, await behindWindows(reversed.page, other));
  await reversed.page.context().close();
  // How unlike two windows' rooms are: the nearest pair of the flats.
  const flats = picked.slice(0, 6);
  let alike = Infinity;
  for (let i = 0; i < flats.length; i++)
    for (let j = i + 1; j < flats.length; j++) alike = Math.min(alike, sum(flats[i], flats[j]));
  const distinct = new Set(flats.map((c) => c.map((v) => Math.round(v / 6)).join()));
  ctx.check(
    "a window shows the same room after the camera has been away and when the models are packed in the opposite order, and the windows do not all show one room",
    returned <= 2 && reordered <= 2 && distinct.size >= 3,
    JSON.stringify({ returned, reordered, distinct: distinct.size, nearestPair: alike }),
  );

  // Unlit: a room is the same picture whichever way the sun stands (the three
  // suns at one elevation), while the wall beside it changes with it.
  const lit = ["behind", "ahead", "along"].map((name) => suns[name]);
  const roomSpread = Math.max(...lit.map((a) => Math.max(...lit.map((b) => sum(a.room, b.room)))));
  const wallSpread = Math.max(
    ...lit.map((a) => Math.max(...lit.map((b) => Math.abs(a.wall - b.wall)))),
  );
  ctx.check(
    "a room takes no sun: its picture is the same under every sun azimuth, while the wall beside it is lit and shaded",
    roomSpread <= 4 && wallSpread >= 25,
    JSON.stringify({ roomSpread, wallSpread, suns: lit }),
  );
  ctx.check(
    "no WebGPU validation warning was logged with rooms drawn",
    warnings.length === 0,
    warnings.slice(0, 3).join(" | "),
  );
  await page.context().close();
  return cameras;
}

/** `FACADE_COST=1`: paired GPU time, each kind of surface drawn against every
 *  other frame batch without it, over a field of blocks. */
async function cost(ctx) {
  const { page } = await open(ctx, `?blocks=${COST_BLOCKS}`);
  const rows = [];
  // The field from the default camera, and from far enough to take it all in.
  for (const [name, view] of Object.entries({
    tactical: { target: [150, 110, 0], distance: 65, pitch: 0.85 },
    field: { target: [150, 110, 0], distance: 420, pitch: 0.85 },
  })) {
    await frame(page, view.target, {
      distance: view.distance,
      pitch: view.pitch,
      yaw: -Math.PI / 2,
    });
    await shot(ctx, page, `cost-${name}-1920x1080.png`);
    const drawn = await lab(page, () => window.__lab.stats().models);
    for (const surface of SURFACES) {
      const differences = [];
      const absolute = [];
      for (let round = 0; round < 6; round++) {
        await lab(page, (s) => window.__lab.suppressSurface(s, false), surface);
        const on = await gpuMs(page);
        await lab(page, (s) => window.__lab.suppressSurface(s, true), surface);
        const off = await gpuMs(page);
        if (on && off) {
          differences.push(on.meanMs - off.meanMs);
          absolute.push(on.meanMs);
        }
      }
      await lab(page, (s) => window.__lab.suppressSurface(s, false), surface);
      rows.push({
        view: name,
        surface,
        median: median(differences),
        frame: median(absolute),
        differences,
      });
      console.log(
        `METRIC facade cost ${name} ${surface}: ${median(differences).toFixed(2)} ms of a ${median(absolute).toFixed(2)} ms frame (paired differences ${differences.map((d) => d.toFixed(2)).join(", ")}); ${drawn.instances} models, ${Math.round(drawn.triangles)} triangles, ${drawn.draws} draws`,
      );
    }
  }
  await ctx.writeEvidence("cost.json", {
    adapter: await page.evaluate(() => window.__lab.adapter),
    blocks: COST_BLOCKS,
    rows,
  });
}

export async function run(ctx) {
  if (process.env.FACADE_COST === "1") return cost(ctx);
  const meta = await cutouts(ctx);
  const glazed = await glass(ctx);
  Object.assign(meta.cameras, glazed.cameras, await rooms(ctx, glazed.windows));
  await ctx.writeEvidence("meta.json", meta);
}
