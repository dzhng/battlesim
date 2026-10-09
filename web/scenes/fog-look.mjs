// How unseen looks. On the street test map under a 16:00
// sun, with the street recon's sight alone (ARMAPHRACT's wedge, the most
// fog beside the most shadow):
// - seen pixels are identical with fog on and off, outside the rim band;
// - the rim lies on the seen side of the boundary, within its
//   width of an unseen pixel, and draws nothing where all is seen;
// - every material path takes the style: ground, structures and the
//   translucent canopy go black under a black style, units never do;
// - structures take fog whole: a building casting fog behind it has no
//   fogged pixel on it or in its courtyard, and one none of which is seen
//   is fogged all over;
// - roofs read as their building's near side: seen from the street, unseen
//   behind a taller building;
// - a contact glyph draws over fog in its own colours: a last sighting
//   reads red through its middle, not only at its rim;
// - every style marks unseen ground: under every style, the darkest of the
//   unseen ground is darker than the same ground drawn seen, or apart in hue;
// and the frames the visual verdict reads: default and ground framings (with
// grass, as the street draws), each fixture style side by side, fog off, and
// the seen/unseen and ground masks.
import { decode } from "./_png.mjs";
import { advance, lab, snapshot } from "./_lab.mjs";
import { game, streetMap } from "./_units.mjs";

/** The street's buildings as its map places them, in the map's order (A, B
 *  and C): each one's footprint and top. Walls are named by compass, so the
 *  checks below need the buildings square to the axes. */
const [A, B] = streetMap.buildings.map(({ geometry }) => {
  const [part] = geometry.parts;
  if (geometry.parts.length !== 1 || part.yaw !== 0)
    throw new Error(`fog-look reads square one-part buildings: ${geometry.template_id}`);
  const [hx, hy, hz] = part.half_extents;
  const [x, y] = part.center;
  return { x, y, hx, hy, top: part.base_z + 2 * hz, west: x - hx };
});

/** The camera framings the verdict reads (the street's default and ground
 *  zoom, pitched by its curve): beside the recon's sight shadows, and, with
 *  every blue eye on (`eyes: "all"`), the wedge one wall of building B
 *  casts, the frames on which the fog look first failed its gate. */
const FRAMINGS = {
  "default-shadow-edge": { target: [950, 750], distance: 65, pitch: 0.85, yaw: 3.752 },
  "default-wedge": { target: [950, 750], distance: 65, pitch: 0.85, yaw: -1.57 },
  "default-wall": { target: [1078, 812], distance: 65, pitch: 0.85, yaw: 3.752, eyes: "all" },
  "ground-hill": { target: [940, 760], distance: 25, pitch: 0.22, yaw: 0.6 },
  "ground-street": { target: [950, 730], distance: 25, pitch: 0.22, yaw: -1.57 },
  "ground-wall": { target: [1085, 812], distance: 25, pitch: 0.22, yaw: 0.6, eyes: "all" },
};
/** The mask view's seen and unseen values (after the pass's own rounding);
 *  partly unseen pixels are grey between them. */
const SEEN = 250;
const UNSEEN = 3;
/** Unseen ground against the same ground seen: the darkest share of it
 *  compared, and the hue margin (CIELAB a*b* distance between the two darks'
 *  means) that tells them apart where the seen dark is not the lighter one. A
 *  framing needs this many settled unseen ground pixels to count. */
const DARKEST = 0.01;
const HUE_MARGIN = 12;
const MIN_GROUND = 2000;
/** Channels within this of the graded black count as black. */
const BLACK_TOLERANCE = 3;
/** Looking into the wood west of the street (the fixture's forest;
 *  the orchard east of it is light, and the recon sees through it): canopy
 *  tops past the recon's sight. */
const ORCHARD = { target: [790, 930], distance: 160, pitch: 0.85, yaw: 3.752 };

/** Pose the camera, with the recon's sight alone unless `eyes` is "all". */
const setCamera = (page, { eyes, ...c }) =>
  lab(
    page,
    ({ c, recon }) => {
      window.__lab.route.setReconOnly(recon);
      const z = window.__lab.route.surfaceZ(c.target[0], c.target[1]);
      window.__lab.setCamera({
        ...window.__lab.camera(),
        ...c,
        target: [c.target[0], c.target[1], z],
      });
    },
    { c, recon: eyes !== "all" },
  );

const view = (page, v) =>
  lab(
    page,
    (v) =>
      v === "mask"
        ? window.__lab.route.showMask(true)
        : v === "ground"
          ? window.__lab.route.showGround(true)
          : window.__lab.route.showWorld(v === "world"),
    v,
  );

/** Draw with grass on or off and wait for the frame. */
async function grass(page, on) {
  await lab(page, (on) => window.__lab.route.setGrass(on), on);
  await page.evaluate(() => window.__lab.frame());
}

/** Draw with fog on or off and wait for the frame. */
async function fog(page, on) {
  await lab(page, (on) => window.__lab.route.setFogOn(on), on);
  await page.evaluate(() => window.__lab.frame());
}

const rgb = (png, x, y) => {
  const i = (Math.round(y) * png.width + Math.round(x)) * 4;
  return [png.data[i], png.data[i + 1], png.data[i + 2]];
};

/** Close over the street recon, every eye on: a framing with nothing unseen. */
const ALL_SEEN = { target: [1004, 788], distance: 18, pitch: 1.3, yaw: 3.752, eyes: "all" };

/** Pixels where `a` and `b` differ (the rim), and of those, how many are
 *  unseen in the mask or farther than `reach` from an unseen pixel. */
function rimPixels(mask, a, b, reach) {
  const { width, height, data } = mask;
  const unseenAt = (x, y) => data[(y * width + x) * 4] <= UNSEEN;
  let rim = 0;
  let unseenSide = 0;
  let far = 0;
  let unseen = 0;
  const r = Math.ceil(reach);
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      if (unseenAt(x, y)) unseen++;
      const i = (y * width + x) * 4;
      if (
        a.data[i] === b.data[i] &&
        a.data[i + 1] === b.data[i + 1] &&
        a.data[i + 2] === b.data[i + 2]
      )
        continue;
      rim++;
      if (unseenAt(x, y)) {
        unseenSide++;
        continue;
      }
      let near = false;
      for (let dy = -r; dy <= r && !near; dy++)
        for (let dx = -r; dx <= r; dx++) {
          const u = x + dx;
          const v = y + dy;
          if (u < 0 || v < 0 || u >= width || v >= height || dx * dx + dy * dy > reach * reach)
            continue;
          if (unseenAt(u, v)) {
            near = true;
            break;
          }
        }
      if (!near) far++;
    }
  }
  return { rim, unseenSide, far, unseen };
}

/** Mask pixels whose whole `r`-neighbourhood is seen, or unseen. */
function settled(mask, seen, r) {
  const out = [];
  const { width, height, data } = mask;
  const test = seen ? (v) => v >= SEEN : (v) => v <= UNSEEN;
  for (let y = r; y < height - r; y += 2) {
    for (let x = r; x < width - r; x += 2) {
      let ok = true;
      for (let dy = -r; dy <= r && ok; dy++)
        for (let dx = -r; dx <= r; dx++) {
          if (!test(data[((y + dy) * width + x + dx) * 4])) {
            ok = false;
            break;
          }
        }
      if (ok) out.push([x, y]);
    }
  }
  return out;
}

/** Ground pixels (in the ground mask) whose whole `r`-neighbourhood is ground
 *  and seen (or unseen) in the fog mask: away from the rim and soft edge. */
function settledGround(mask, ground, seen, r) {
  const out = [];
  const { width, height } = mask;
  const side = seen ? (v) => v >= SEEN : (v) => v <= UNSEEN;
  for (let y = r; y < height - r; y += 2) {
    for (let x = r; x < width - r; x += 2) {
      let ok = true;
      for (let dy = -r; dy <= r && ok; dy++)
        for (let dx = -r; dx <= r; dx++) {
          const i = ((y + dy) * width + x + dx) * 4;
          if (ground.data[i] < SEEN || !side(mask.data[i])) {
            ok = false;
            break;
          }
        }
      if (ok) out.push([x, y]);
    }
  }
  return out;
}

/** Rec. 709 luma of a display pixel, 0–255. */
const luma = ([r, g, b]) => 0.2126 * r + 0.7152 * g + 0.0722 * b;

/** CIELAB (D65) of an sRGB display pixel. */
function lab709([r, g, b]) {
  const lin = (c) => ((c /= 255) <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4);
  const [R, G, B] = [lin(r), lin(g), lin(b)];
  const X = (0.4124 * R + 0.3576 * G + 0.1805 * B) / 0.95047;
  const Y = 0.2126 * R + 0.7152 * G + 0.0722 * B;
  const Z = (0.0193 * R + 0.1192 * G + 0.9505 * B) / 1.08883;
  const f = (t) => (t > 0.008856 ? Math.cbrt(t) : 7.787 * t + 16 / 116);
  return [116 * f(Y) - 16, 500 * (f(X) - f(Y)), 200 * (f(Y) - f(Z))];
}

/** The darkest 1% of `pixels` in `png`: its luma bound, and its mean a*b*. */
function darkest(png, pixels) {
  const colours = pixels.map(([x, y]) => rgb(png, x, y)).sort((a, b) => luma(a) - luma(b));
  const dark = colours.slice(0, Math.max(1, Math.ceil(colours.length * DARKEST)));
  const ab = [0, 0];
  for (const c of dark) {
    const [, a, b] = lab709(c);
    ab[0] += a / dark.length;
    ab[1] += b / dark.length;
  }
  return { luma: luma(dark.at(-1)), ab };
}

/** The commonest colour among `pixels`. */
function commonest(png, pixels) {
  const counts = new Map();
  for (const [x, y] of pixels) {
    const k = rgb(png, x, y).join();
    counts.set(k, (counts.get(k) ?? 0) + 1);
  }
  const [k] = [...counts].sort((a, b) => b[1] - a[1])[0] ?? ["0,0,0"];
  return k.split(",").map(Number);
}

const near = (a, b) => a.every((c, k) => Math.abs(c - b[k]) <= BLACK_TOLERANCE);
const onScreen = (p) => p && p[0] >= 0 && p[1] >= 0 && p[0] < 1920 && p[1] < 1080;

/** Run `fn` in the page, which rebuilds the lab's frame, and wait for it. */
async function rebuild(page, fn) {
  await page.evaluate(() => (window.__labBefore = window.__lab));
  await page.evaluate(fn);
  await page.waitForFunction(
    () =>
      window.__lab &&
      window.__lab !== window.__labBefore &&
      window.__lab.ready &&
      typeof window.__lab.frame === "function",
    undefined,
    { timeout: 60000 },
  );
  await page.evaluate(() => window.__lab.frame());
}

/** Surface points at the projected pixel, probed for fog. */
async function probeAt(page, points) {
  const seen = [...(await lab(page, (p) => window.__lab.route.probe(p), points))];
  const px = await lab(
    page,
    (p) => p.map((q) => window.__lab.projectToCss(q.position[0], q.position[1], q.position[2])),
    points,
  );
  return points.map((p, i) => ({ ...p, seen: seen[i], px: px[i] }));
}

async function contactGlyphOverFog(ctx, page) {
  // A contact's glyph over fog: its own colours, red through its middle.
  await setCamera(page, { target: [1120, 930], distance: 260, pitch: 0.85, yaw: 3.752 });
  await page.evaluate(() => window.__lab.frame());
  await view(page, "world");
  const bare = decode(await snapshot(ctx, page, "glyph-world-1920x1080.png"));
  await view(page, "final");
  const drawn = decode(await snapshot(ctx, page, "glyph-1920x1080.png"));
  const specimen = (await lab(page, () => window.__lab.route.specimens()))[0];
  // The interior stays red over fog. Its common white outline is covered
  // by contactGlyph.test.ts; rim colour is diagnostic here.
  const rim = await lab(
    page,
    (c) =>
      Array.from({ length: 180 }, (_, k) => {
        const a = (k / 180) * 2 * Math.PI;
        const x = c.center[0] + Math.cos(a) * c.radius * 0.97;
        const y = c.center[1] + Math.sin(a) * c.radius * 0.97;
        return window.__lab.projectToCss(x, y, window.__lab.route.surfaceZ(x, y));
      }),
    specimen,
  );
  const inside = await lab(
    page,
    (c) => {
      const out = [];
      for (let k = 0; k < 4000; k++) {
        // A deterministic spread over the inner 80% of the disc.
        const r = c.radius * 0.8 * Math.sqrt((k + 0.5) / 4000);
        const a = k * 2.399963;
        const x = c.center[0] + Math.cos(a) * r;
        const y = c.center[1] + Math.sin(a) * r;
        out.push(window.__lab.projectToCss(x, y, window.__lab.route.surfaceZ(x, y)));
      }
      return out;
    },
    specimen,
  );
  const reddened = (p) => {
    if (!onScreen(p)) return false;
    const [r, g] = rgb(drawn, p[0], p[1]);
    const [r0, g0] = rgb(bare, p[0], p[1]);
    return r - g > r0 - g0 + 10;
  };
  const shown = inside.filter(onScreen).length;
  const middle = inside.filter(reddened).length;
  const red = rim.filter(reddened).length;
  ctx.check(
    "a last sighting reads red through its middle over fog",
    shown > 0 && middle >= 0.9 * shown,
    JSON.stringify({ middle, shown, red, rim: rim.length, historicalRedRimTargetMet: red >= 90 }),
  );
}

export async function run(ctx) {
  const page = await ctx.newPage({ viewport: { width: 1920, height: 1080 } });
  await ctx.openLab(page);
  await page.waitForFunction(() => window.__lab.route?.tick() > 3, undefined, { timeout: 60000 });
  // The frames are the verdict's evidence: the panel stays out of them, and
  // so does the frame-rate readout, which follows the wall clock and would
  // differ between two frames compared pixel for pixel.
  await page.addStyleTag({
    content: "[data-testid=fog-look-panel], .frame-rate { display: none; }",
  });
  await lab(page, () => window.__lab.route.pause());
  await advance(page, 2);
  await lab(page, () => window.__lab.route.setReconOnly(true));
  const fixtureStyle = await lab(page, () => window.__lab.route.style());
  const styles = await lab(page, () => window.__lab.route.styles());
  const surfaceZ = (x, y) => lab(page, ([x, y]) => window.__lab.route.surfaceZ(x, y), [x, y]);

  // Seen pixels, fog on and off: identical, away from the edge (whose pixels
  // blend both sides at 4× MSAA, and carry the rim) and with bloom off, which
  // would spread unseen's dimming a little into them. The framings with no
  // translucent canopy: a seen canopy over unseen ground is partly unseen by
  // design.
  await rebuild(page, () => window.__lab.route.setBloom(false));
  const band = Math.max(3, Math.ceil(fixtureStyle.rim.width_px) + 1);
  const identical = {};
  for (const name of ["default-shadow-edge", "default-wedge", "ground-hill"]) {
    const framing = FRAMINGS[name];
    await setCamera(page, framing);
    await fog(page, true);
    await view(page, "mask");
    const mask = decode(await snapshot(ctx, page, `${name}-mask-1920x1080.png`));
    await view(page, "world");
    const on = decode(await snapshot(ctx, page, `${name}-world-1920x1080.png`));
    await fog(page, false);
    const off = decode(await snapshot(ctx, page, `${name}-world-fog-off-1920x1080.png`));
    await fog(page, true);
    const seen = settled(mask, true, band);
    const unseen = settled(mask, false, band);
    const moved = seen.filter(([x, y]) => rgb(on, x, y).some((c, k) => c !== rgb(off, x, y)[k]));
    identical[name] = { seen: seen.length, unseen: unseen.length, moved: moved.length };
  }
  await rebuild(page, () => window.__lab.route.setBloom(true));
  // The gate frames, with the grass the street draws.
  await grass(page, true);
  for (const [name, framing] of Object.entries(FRAMINGS)) {
    await setCamera(page, framing);
    await fog(page, true);
    await view(page, "final");
    await snapshot(ctx, page, `${name}-1920x1080.png`);
    await view(page, "mask");
    await snapshot(ctx, page, `${name}-mask-1920x1080.png`);
    await view(page, "final");
  }
  await grass(page, false);
  // The rim: the world with the style's rim against the same style without
  // one. What differs is the rim, and each such pixel is seen and within the
  // rim's width (plus the MSAA edge pixel) of an unseen one.
  const rimless = { ...fixtureStyle, rim: { ...fixtureStyle.rim, alpha: 0 } };
  const rims = {};
  for (const name of ["default-shadow-edge", "default-wedge", "ground-hill"]) {
    await setCamera(page, FRAMINGS[name]);
    await page.evaluate(() => window.__lab.frame());
    await view(page, "mask");
    const mask = decode(await snapshot(ctx, page, `${name}-mask-1920x1080.png`));
    await view(page, "world");
    const withRim = decode(await snapshot(ctx, page, `${name}-world-1920x1080.png`));
    await lab(page, (s) => window.__lab.route.setStyle(s), rimless);
    await page.evaluate(() => window.__lab.frame());
    const without = decode(await snapshot(ctx, page, `${name}-world-rimless-1920x1080.png`));
    await lab(page, (s) => window.__lab.route.setStyle(s), fixtureStyle);
    rims[name] = rimPixels(mask, withRim, without, fixtureStyle.rim.width_px + 1.5);
  }
  // Close over the recon with every eye: nothing unseen, so nothing to rim.
  await setCamera(page, ALL_SEEN);
  await page.evaluate(() => window.__lab.frame());
  await view(page, "mask");
  const allMask = decode(await snapshot(ctx, page, "all-seen-mask-1920x1080.png"));
  await view(page, "world");
  const allRim = decode(await snapshot(ctx, page, "all-seen-world-1920x1080.png"));
  await lab(page, (s) => window.__lab.route.setStyle(s), rimless);
  await page.evaluate(() => window.__lab.frame());
  const allBare = decode(await snapshot(ctx, page, "all-seen-world-rimless-1920x1080.png"));
  await lab(page, (s) => window.__lab.route.setStyle(s), fixtureStyle);
  rims["all-seen"] = rimPixels(allMask, allRim, allBare, 0);
  await view(page, "final");
  ctx.check(
    "the rim lies on the seen side of the boundary, within its width; none where all is seen",
    ["default-shadow-edge", "default-wedge", "ground-hill"].every(
      (n) => rims[n].rim > 1000 && rims[n].unseenSide === 0 && rims[n].far === 0,
    ) &&
      rims["all-seen"].unseen === 0 &&
      rims["all-seen"].rim === 0,
    JSON.stringify(rims),
  );
  ctx.check(
    "seen pixels are identical with fog on and off, outside the rim band",
    Object.values(identical).every((f) => f.seen > 20000 && f.unseen > 5000 && f.moved === 0),
    JSON.stringify(identical),
  );

  // Every material path takes the style. Under a black style (no light
  // kept, no lines) every unseen pixel is the graded black: the ground, the
  // structures' faces and the translucent canopy alike.
  const black = {
    ...fixtureStyle,
    edge_softness: 0,
    veil: 0,
    dim: 0,
    lines: { ...fixtureStyle.lines, strength: 0, floor: 0 },
  };
  // A structure takes fog whole (fogTerm.ts), so its unseen faces are
  // those of a building none of which is seen. Every building on the street
  // shows the side something, so one eye stands at head height 5 m from
  // building A's west wall, which hides what lies east of it: the largest
  // building it sees nothing of (the flags don't depend on the view), framed
  // as the default framing is.
  await setCamera(page, FRAMINGS["default-shadow-edge"]);
  const hideAt = [A.west - 5, A.y];
  const hideEye = [
    {
      key: "scene:0",
      position: [...hideAt, (await surfaceZ(...hideAt)) + 1.7],
      forward: 0,
      shape: { front: 1, side: 1, rear: 1 },
      range: 400,
    },
  ];
  const withHideEye = async (fn) => {
    await lab(page, (e) => window.__lab.route.setEyes(e), hideEye);
    await page.evaluate(() => window.__lab.frame());
    try {
      return await fn();
    } finally {
      await lab(page, () => window.__lab.route.setEyes(null));
      await page.evaluate(() => window.__lab.frame());
    }
  };
  const wholesHidden = await withHideEye(() => lab(page, () => window.__lab.route.wholes()));
  const unseenBuilding = wholesHidden
    .filter((b) => !b.seen && b.top - b.base > 3)
    .reduce((best, b) => (!best || b.hx * b.hy > best.hx * best.hy ? b : best), null);
  const unseenFraming = unseenBuilding && {
    ...FRAMINGS["default-shadow-edge"],
    target: [unseenBuilding.x, unseenBuilding.y],
  };
  const unseenWalls = unseenBuilding
    ? [
        [1, 0],
        [-1, 0],
        [0, 1],
        [0, -1],
      ].map(([a, b]) => {
        const c = Math.cos(unseenBuilding.yaw);
        const s = Math.sin(unseenBuilding.yaw);
        const n = [a * c - b * s, a * s + b * c, 0];
        const h = a !== 0 ? unseenBuilding.hx : unseenBuilding.hy;
        return {
          kind: "wall",
          position: [
            unseenBuilding.x + n[0] * h,
            unseenBuilding.y + n[1] * h,
            (unseenBuilding.base + unseenBuilding.top) / 2,
          ],
          normal: n,
        };
      })
    : [];
  await lab(page, (s) => window.__lab.route.setStyle(s), black);
  const paths = {};
  let graded = null;
  for (const [name, framing] of [
    ["default-shadow-edge", FRAMINGS["default-shadow-edge"]],
    ["orchard", ORCHARD],
    ...(unseenFraming ? [["unseen-building", unseenFraming]] : []),
  ]) {
    await setCamera(page, framing);
    if (name === "unseen-building") await lab(page, (e) => window.__lab.route.setEyes(e), hideEye);
    await page.evaluate(() => window.__lab.frame());
    await view(page, "mask");
    const mask = decode(await snapshot(ctx, page, `black-style-${name}-mask-1920x1080.png`));
    await view(page, "world");
    const dark = decode(await snapshot(ctx, page, `black-style-${name}-world-1920x1080.png`));
    // Two pixels clear of any seen one. Under this style nothing softens the
    // edge; the residue is pixels partly seen. An upward face judges itself
    // by the air above it, not the body it belongs to, so on an unseen side
    // the sub-pixel ledges (a sandbag's courses, a lintel under the roof
    // rule) count as seen and the pixel mixes by its coverage (the mask
    // shows a pixel more than half unseen as unseen). Each frame judges its
    // own path whole: the street's ground, the wood's canopy, an unseen
    // building's faces. A prop on the street's seen ground can turn an
    // unseen face with seen ledges to the camera, which is none of those.
    let unseen;
    if (name === "default-shadow-edge") {
      await view(page, "ground");
      const ground = decode(await snapshot(ctx, page, `black-style-${name}-ground-1920x1080.png`));
      unseen = settledGround(mask, ground, false, 2);
    } else unseen = settled(mask, false, 2);
    graded ??= commonest(dark, unseen);
    const lit = unseen.filter(([x, y]) => !near(rgb(dark, x, y), graded));
    paths[name] = { unseen: unseen.length, notBlack: lit.length };
    // Named points of each path, probed unseen, then read.
    const candidates =
      name === "unseen-building"
        ? unseenWalls
        : name === "orchard"
          ? await lab(page, () => {
              const out = [];
              for (let x = 710; x < 880; x += 10)
                for (let y = 830; y < 1040; y += 10)
                  out.push({
                    kind: "canopy",
                    position: [x, y, window.__lab.route.surfaceZ(x, y) + 12],
                    normal: [0, 0, 1],
                  });
              return out;
            })
          : [{ kind: "ground", position: [945, 720, await surfaceZ(945, 720)] }];
    const probed = await probeAt(page, candidates);
    // A canopy pixel is black only if the ground seen through it is too.
    const under = await probeAt(
      page,
      probed.map((p) => ({ position: [p.position[0], p.position[1], p.position[2] - 12] })),
    );
    if (name === "unseen-building") await lab(page, () => window.__lab.route.setEyes(null));
    for (const [i, p] of probed.entries()) {
      if (p.seen || !onScreen(p.px)) continue;
      if (p.kind === "canopy" && under[i].seen) continue;
      const c = rgb(dark, p.px[0], p.px[1]);
      (paths[p.kind] ??= []).push(near(c, graded) ? "black" : c.join());
    }
  }
  // A canopy point's pixel may show a trunk or another crown in front of it,
  // so most, not all, of the orchard's must be black; an unstyled path would
  // leave none black.
  const blackCount = (kind) => (paths[kind] ?? []).filter((c) => c === "black").length;
  const canopy = paths.canopy ?? [];
  ctx.check(
    "every material path takes the style (ground, structures, translucent canopy)",
    Math.max(...graded) < 60 &&
      ["default-shadow-edge", "orchard", "unseen-building"].every(
        (n) => paths[n]?.unseen > 2000 && paths[n].notBlack <= paths[n].unseen * 0.002,
      ) &&
      blackCount("ground") === 1 &&
      (paths.wall ?? []).length >= 1 &&
      blackCount("wall") === paths.wall.length &&
      canopy.length >= 5 &&
      blackCount("canopy") >= 0.9 * canopy.length,
    JSON.stringify({ graded, ...paths, canopy: canopy.join(" ") }),
  );
  // Own soldiers standing on unseen ground (outside the recon's sight) are
  // drawn by identification: never fogged, even under the black style.
  await setCamera(page, FRAMINGS["ground-street"]);
  await page.evaluate(() => window.__lab.frame());
  const street = decode(await snapshot(ctx, page, "black-style-units-1920x1080.png"));
  const own = (await lab(page, () => window.__lab.route.observation())).own;
  const members = own.flatMap((u) => u.members.map((m) => ({ position: [m[0], m[1], m[2]] })));
  // A soldier counts when he stands on unseen ground and his body, a metre
  // up, is on screen (his feet may be below the frame's edge).
  const standing = (await probeAt(page, members)).filter((m) => m.seen === 0);
  const units = await lab(
    page,
    (ms) =>
      ms.map((m) => window.__lab.projectToCss(m.position[0], m.position[1], m.position[2] + 1)),
    standing,
  );
  const unitRgb = units.filter(onScreen).map((p) => rgb(street, p[0], p[1]));
  ctx.check(
    "units standing in fog are never fogged",
    unitRgb.length >= 3 && unitRgb.every((c) => Math.max(...c) > 40),
    JSON.stringify({ members: members.length, inFog: standing.length, unitRgb }),
  );
  await lab(page, (s) => window.__lab.route.setStyle(s), fixtureStyle);

  // Roofs above every eye read as their building's near side: each
  // building's roof, at its middle.
  const roofs = await lab(
    page,
    (b) =>
      b.map(({ x, y, top }) => ({
        position: [x, y, window.__lab.route.surfaceZ(x, y) + top],
        normal: [0, 0, 1],
      })),
    streetMap.buildings.map(({ geometry }) => ({
      x: geometry.parts[0].center[0],
      y: geometry.parts[0].center[1],
      top: geometry.height_m,
    })),
  );
  const roofSeen = [...(await lab(page, (p) => window.__lab.route.probe(p), roofs))];
  // A lower roof 6 m up in building B's sight shadow stays unseen. A roof
  // reads as the air on its near side, up to `roof_reach_m` toward the eye
  // (fogTerm.ts), so the roof stands that far and 10 m more past B's far
  // wall, on the line from the recon through B's middle: its near side's
  // air is 10 m into B's sight shadow.
  const recon = (await lab(page, () => window.__lab.route.observation())).own.find(
    (u) => u.kind === "test_recon",
  );
  const away = [B.x - recon.position[0], B.y - recon.position[1]];
  const along = Math.hypot(...away);
  const u = away.map((c) => c / along);
  const past =
    Math.min(B.hx / Math.abs(u[0]), B.hy / Math.abs(u[1])) +
    game.presentation.fog_geometry.roof_reach_m +
    10;
  const lower = [B.x + u[0] * past, B.y + u[1] * past];
  if (B.top <= 6) throw new Error(`building B (${B.top} m) is no taller than the lower roof`);
  const hidden = [
    ...(await lab(page, (p) => window.__lab.route.probe(p), [
      { position: [...lower, (await surfaceZ(...lower)) + 6], normal: [0, 0, 1] },
    ])),
  ];
  ctx.check(
    "roofs read as their building's near side: seen from the street, hidden behind a taller one",
    roofSeen.every((s) => s === 1) && hidden[0] === 0,
    JSON.stringify({ roofSeen, hidden, lower }),
  );

  // Structures take fog whole. With every blue eye on, building B casts the
  // wedge behind it: something of it is seen, so none of it (walls, roofs,
  // courtyard ground) is fogged. With the recon alone, a building none of
  // which he sees is fogged all over.
  const wholeFrame = async (framing, name, pick) => {
    await setCamera(page, framing);
    await page.evaluate(() => window.__lab.frame());
    const boxes = await lab(page, () => window.__lab.route.wholes());
    const box = pick(boxes);
    if (!box) return { name, box: null };
    await view(page, "mask");
    const mask = decode(await snapshot(ctx, page, `whole-${name}-mask-1920x1080.png`));
    await view(page, "final");
    await snapshot(ctx, page, `whole-${name}-1920x1080.png`);
    const points = await lab(
      page,
      (b) => {
        // A grid over the footprint's ground, half a metre inside its edges:
        // each point's pixel shows what of the building stands over it (a
        // roof, a wall) or the courtyard ground itself; nothing outside the
        // box stands between it and the camera but the odd unit.
        const out = [];
        const c = Math.cos(b.yaw);
        const s = Math.sin(b.yaw);
        for (let u = -b.hx + 0.5; u <= b.hx - 0.5; u += 1)
          for (let v = -b.hy + 0.5; v <= b.hy - 0.5; v += 1) {
            const x = b.x + u * c - v * s;
            const y = b.y + u * s + v * c;
            out.push(window.__lab.projectToCss(x, y, window.__lab.route.surfaceZ(x, y) + 0.05));
          }
        return out;
      },
      box,
    );
    const values = points
      .filter(onScreen)
      .map(([x, y]) => mask.data[(Math.floor(y) * mask.width + Math.floor(x)) * 4]);
    return {
      name,
      box: [box.x, box.y],
      seen: box.seen,
      points: values.length,
      fogged: values.filter((v) => v < SEEN).length,
      unseen: values.filter((v) => v <= UNSEEN).length,
    };
  };
  const within = (b, [x, y]) => {
    const dx = x - b.x;
    const dy = y - b.y;
    const u = dx * Math.cos(b.yaw) + dy * Math.sin(b.yaw);
    const v = -dx * Math.sin(b.yaw) + dy * Math.cos(b.yaw);
    return Math.abs(u) <= b.hx && Math.abs(v) <= b.hy;
  };
  const caster = await wholeFrame(FRAMINGS["default-wall"], "caster", (bs) =>
    bs.find((b) => within(b, [B.x, B.y])),
  );
  const unseenWhole = unseenBuilding
    ? await withHideEye(() =>
        wholeFrame(unseenFraming, "unseen", (bs) =>
          bs.find((b) => b.x === unseenBuilding.x && b.y === unseenBuilding.y),
        ),
      )
    : { name: "unseen", box: null };
  ctx.check(
    "a building casting fog is unfogged whole, courtyard too; one wholly unseen is fogged whole",
    caster.seen === true &&
      caster.points >= 50 &&
      caster.fogged === 0 &&
      unseenWhole.seen === false &&
      unseenWhole.points >= 20 &&
      unseenWhole.unseen >= 0.95 * unseenWhole.points,
    JSON.stringify({
      caster,
      unseen: unseenWhole,
      fromHideEye: `${wholesHidden.filter((b) => b.seen).length} of ${wholesHidden.length} seen`,
    }),
  );

  await contactGlyphOverFog(ctx, page);

  // Every style marks unseen ground: at every gate framing, under every
  // fixture style, the darkest 1% of settled unseen ground is darker than the
  // darkest 1% of the same pixels with fog off (the same ground, under the
  // same light, drawn as seen), or differs from it in hue by HUE_MARGIN. Like
  // is compared with like: seen ground in a long cast shadow can be as dark as
  // a style's fog on sunlit ground, and is still told apart by its colour
  // and its place, so a style is judged on what it does to the ground it
  // covers. Each style's frames also make the A/B sheet. With grass, as the
  // street draws.
  await grass(page, true);
  const sides = {};
  for (const [name, framing] of Object.entries(FRAMINGS)) {
    await setCamera(page, framing);
    await page.evaluate(() => window.__lab.frame());
    await view(page, "mask");
    const mask = decode(await snapshot(ctx, page, `${name}-mask-1920x1080.png`));
    await view(page, "ground");
    const ground = decode(await snapshot(ctx, page, `${name}-ground-1920x1080.png`));
    await view(page, "final");
    await fog(page, false);
    const seenLook = decode(await snapshot(ctx, page, `${name}-fog-off-1920x1080.png`));
    await fog(page, true);
    sides[name] = { unseen: settledGround(mask, ground, false, band), seenLook };
  }
  const darks = {};
  const failing = [];
  for (const style of styles) {
    await lab(page, (s) => window.__lab.route.setStyle(s), style);
    darks[style] = {};
    for (const [name, framing] of Object.entries(FRAMINGS)) {
      await setCamera(page, framing);
      const frame = decode(await snapshot(ctx, page, `style-${style}-${name}-1920x1080.png`));
      const { unseen, seenLook } = sides[name];
      if (unseen.length < MIN_GROUND) {
        darks[style][name] = { unseen: unseen.length };
        continue;
      }
      const s = darkest(seenLook, unseen);
      const u = darkest(frame, unseen);
      const hue = Math.hypot(s.ab[0] - u.ab[0], s.ab[1] - u.ab[1]);
      const by = s.luma > u.luma ? "darker" : hue >= HUE_MARGIN ? "hue" : "none";
      darks[style][name] = {
        seen: Math.round(s.luma),
        unseen: Math.round(u.luma),
        hue: Math.round(hue * 10) / 10,
        by,
      };
      if (by === "none") failing.push(`${style}/${name}`);
    }
  }
  const measured = Object.values(darks).flatMap((f) => Object.values(f).filter((d) => d.by));
  await ctx.writeEvidence("darks.json", darks);
  ctx.check(
    "every measured style and framing marks unseen ground apart from the same ground seen",
    failing.length === 0 && measured.length >= styles.length * 4,
    JSON.stringify({ failing, darks }),
  );
  await ctx.writeEvidence("meta.json", {
    adapter: await page.evaluate(() => window.__lab.adapter),
    viewport: [1920, 1080],
    dpr: 1,
    tick: await lab(page, () => window.__lab.route.tick()),
    sun: await lab(page, () => window.__lab.route.sun()),
    style: fixtureStyle,
  });
  await page.close();
}
