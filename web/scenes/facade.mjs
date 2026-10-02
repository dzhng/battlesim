// The facade lab: what a facade material can say beyond "opaque", judged on
// the facade lab's kit standing on a plain ground.
//
// Cutouts (a grille, a perforated sheet): a hole shows what is behind it and
// the metal does not, with and without fog; the shade a panel casts follows
// how much of it is there; and a panel too far away to resolve its holes is
// still drawn.
//
// `FACADE_COST=1` measures instead: the frame's GPU time with and without
// each kind of surface (paired, interleaved) over a field of blocks.
import { writeFile } from "node:fs/promises";
import { lab } from "./_lab.mjs";
import { decode, pixel, around, writeCrop } from "./_png.mjs";

const VIEWPORT = { width: 1920, height: 1080 };
const HIDE_PANEL = "[data-testid=facade-panel] { display: none !important; }";
const SURFACES = ["cutout"];
/** Blocks in the cost run's field. */
const COST_BLOCKS = 288;

const sum = (a, b) => a.reduce((s, v, i) => s + Math.abs(v - b[i]), 0);
const luminance = ([r, g, b]) => 0.2126 * r + 0.7152 * g + 0.0722 * b;
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
  const warnings = [];
  page.on("console", (m) => {
    if (m.type() === "warning" && /webgpu|validation|gpu\w*error/i.test(m.text()))
      warnings.push(m.text().slice(0, 200));
  });
  await page.goto(`${ctx.url}${query}`);
  await page.waitForFunction(
    () =>
      document.querySelector("[data-testid=error]") ||
      window.__lab?.error ||
      (window.__lab?.ready && window.__lab.route?.stand && window.__lab.stats),
    undefined,
    { timeout: 120000 },
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
  await ctx.writeEvidence("meta.json", meta);
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
}

/** The frame's GPU time over `frames` forced redraws from now. */
async function gpuMs(page, frames = 120) {
  return lab(
    page,
    async (frames) => {
      await window.__lab.setFrameView("final");
      for (let i = 0; i < frames; i++) await window.__lab.frame();
      return window.__lab.stats().gpu;
    },
    frames,
  );
}

/** `FACADE_COST=1`: paired GPU time, each kind of surface drawn against every
 *  other frame batch without it, over a field of blocks. */
async function cost(ctx) {
  const { page } = await open(ctx, `?blocks=${COST_BLOCKS}`);
  const median = (list) => [...list].sort((a, b) => a - b)[Math.floor(list.length / 2)];
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
  await cutouts(ctx);
}
