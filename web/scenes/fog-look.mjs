// Battle-look slice 15: how unseen looks. On the village street under a 16:00
// sun, with the street recon's sight alone (ARMAPHRACT's wedge, the most
// fog beside the most shadow):
// - seen pixels are identical with fog on and off, outside the rim band;
// - the rim (slice 15b) lies on the seen side of the boundary, within its
//   width of an unseen pixel, and draws nothing where all is seen;
// - every material path takes the style: ground, structures and the
//   translucent canopy go black under a black style, units never do;
// - roofs read as their building's near side: seen from the street, unseen
//   behind a taller building;
// - a contact glyph draws over fog in its own colours: a pale hatched ghost
//   with the red glow;
// - nothing seen reads as fog (slice 19b): under every style, the darkest
//   seen ground is lighter than the darkest unseen ground, or apart in hue;
// and the frames the visual verdict reads: default and ground framings (with
// grass, as the village draws), each fixture style side by side, fog off, and
// the seen/unseen and ground masks.
import { decode } from "./_png.mjs";
import { advance, lab, snapshot } from "./_lab.mjs";

/** The camera framings the verdict reads (the village's default and ground
 *  zoom, pitched by its curve): beside the recon's sight shadows, and, with
 *  every blue eye on (`eyes: "all"`), the wedge one wall of the building at
 *  (1047, 814) casts, the frames on which slice 15's gate failed. */
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
/** The seen world's darks against fog (slice 19b): the darkest share of each
 *  side's ground compared, and the hue margin (CIELAB a*b* distance between
 *  the two darks' means) that tells them apart where the seen dark is not the
 *  lighter one. Each side needs this many settled ground pixels to count. */
const DARKEST = 0.01;
const HUE_MARGIN = 12;
const MIN_GROUND = 2000;
/** Channels within this of the graded black count as black. */
const BLACK_TOLERANCE = 3;
/** Building A (the fixture's first): its south wall faces away from the
 *  street recon, north-east of it. */
const A = { center: [975, 752], half: [15, 12, 4] };
/** Looking into the wood west of the street (the fixture's medium forest;
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

export async function run(ctx) {
  const page = await ctx.newPage({ viewport: { width: 1920, height: 1080 } });
  await ctx.openLab(page);
  await page.waitForFunction(() => window.__lab.route?.tick() > 3, undefined, { timeout: 60000 });
  // The frames are the verdict's evidence: the panel stays out of them.
  await page.addStyleTag({ content: "[data-testid=fog-look-panel] { display: none; }" });
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
  // The gate frames, with the grass the village draws.
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
  await lab(page, (s) => window.__lab.route.setStyle(s), black);
  const paths = {};
  let graded = null;
  for (const [name, framing] of [
    ["default-shadow-edge", FRAMINGS["default-shadow-edge"]],
    ["orchard", ORCHARD],
  ]) {
    await setCamera(page, framing);
    await page.evaluate(() => window.__lab.frame());
    await view(page, "mask");
    const mask = decode(await snapshot(ctx, page, `black-style-${name}-mask-1920x1080.png`));
    await view(page, "world");
    const dark = decode(await snapshot(ctx, page, `black-style-${name}-world-1920x1080.png`));
    // Two pixels clear of any seen one: the mask pass softens the edge, so
    // a thin unseen sliver between seen faces (a sandbag's side under its
    // seen top, slice 37) is blended, not styled flat.
    const unseen = settled(mask, false, 2);
    graded ??= commonest(dark, unseen);
    const lit = unseen.filter(([x, y]) => !near(rgb(dark, x, y), graded));
    paths[name] = { unseen: unseen.length, notBlack: lit.length };
    // Named points of each path, probed unseen, then read.
    const candidates =
      name === "orchard"
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
        : [
            {
              kind: "wall",
              position: [A.center[0], A.center[1] - A.half[1], (await surfaceZ(975, 740)) + 4],
              normal: [0, -1, 0],
            },
            { kind: "ground", position: [945, 720, await surfaceZ(945, 720)] },
          ];
    const probed = await probeAt(page, candidates);
    // A canopy pixel is black only if the ground seen through it is too.
    const under = await probeAt(
      page,
      probed.map((p) => ({ position: [p.position[0], p.position[1], p.position[2] - 12] })),
    );
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
      ["default-shadow-edge", "orchard"].every(
        (n) => paths[n].unseen > 2000 && paths[n].notBlack <= paths[n].unseen * 0.002,
      ) &&
      blackCount("ground") === 1 &&
      blackCount("wall") === 1 &&
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

  // Roofs above every eye read as their building's near side.
  const roofs = await lab(
    page,
    (b) =>
      b.map(([x, y]) => ({
        position: [x, y, window.__lab.route.surfaceZ(x, y) + 8],
        normal: [0, 0, 1],
      })),
    [
      [975, 752],
      [1047, 814],
      [983, 871],
    ],
  );
  const roofSeen = [...(await lab(page, (p) => window.__lab.route.probe(p), roofs))];
  // A lower roof 6 m up in building B's sight shadow stays unseen.
  const hidden = [
    ...(await lab(page, (p) => window.__lab.route.probe(p), [
      { position: [1090, 822, (await surfaceZ(1090, 822)) + 6], normal: [0, 0, 1] },
    ])),
  ];
  ctx.check(
    "roofs read as their building's near side: seen from the street, hidden behind a taller one",
    roofSeen.every((s) => s === 1) && hidden[0] === 0,
    JSON.stringify({ roofSeen, hidden }),
  );

  // A contact's glyph over fog: its own colours, a pale hatch and red glow.
  await setCamera(page, { target: [1120, 930], distance: 260, pitch: 0.85, yaw: 3.752 });
  await page.evaluate(() => window.__lab.frame());
  await view(page, "world");
  const bare = decode(await snapshot(ctx, page, "glyph-world-1920x1080.png"));
  await view(page, "final");
  const drawn = decode(await snapshot(ctx, page, "glyph-1920x1080.png"));
  const specimen = (await lab(page, () => window.__lab.route.specimens()))[0];
  // Inside the ghost: pixels the glyph turned pale; at its rim: pixels it
  // turned red.
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
  const pale = inside.filter((p) => {
    if (!onScreen(p)) return false;
    // Lifted toward white in every channel, the blue most (a pale line over
    // olive or slate).
    const c = rgb(drawn, p[0], p[1]);
    const b = rgb(bare, p[0], p[1]);
    return c.every((v, k) => v > b[k] + 12) && c[2] - b[2] >= c[0] - b[0];
  }).length;
  const red = rim.filter((p) => {
    if (!onScreen(p)) return false;
    const [r, g] = rgb(drawn, p[0], p[1]);
    const [r0, g0] = rgb(bare, p[0], p[1]);
    return r - g > r0 - g0 + 10;
  }).length;
  ctx.check(
    "a last sighting is a pale hatched ghost with a red glow, over fog",
    pale >= 80 && red >= 90,
    JSON.stringify({ pale, inside: inside.length, red, rim: rim.length }),
  );

  // Nothing seen reads as fog (slice 19b): at every gate framing, under every
  // fixture style, the darkest 1% of seen ground is lighter than the darkest
  // 1% of unseen ground, or differs from it in hue by HUE_MARGIN. Each style's
  // frames also make the A/B sheet. With grass, as the village draws.
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
    sides[name] = {
      seen: settledGround(mask, ground, true, band),
      unseen: settledGround(mask, ground, false, band),
    };
  }
  const darks = {};
  const failing = [];
  for (const style of styles) {
    await lab(page, (s) => window.__lab.route.setStyle(s), style);
    darks[style] = {};
    for (const [name, framing] of Object.entries(FRAMINGS)) {
      await setCamera(page, framing);
      const frame = decode(await snapshot(ctx, page, `style-${style}-${name}-1920x1080.png`));
      const { seen, unseen } = sides[name];
      if (seen.length < MIN_GROUND || unseen.length < MIN_GROUND) {
        darks[style][name] = { seen: seen.length, unseen: unseen.length };
        continue;
      }
      const s = darkest(frame, seen);
      const u = darkest(frame, unseen);
      const hue = Math.hypot(s.ab[0] - u.ab[0], s.ab[1] - u.ab[1]);
      const by = s.luma > u.luma ? "lighter" : hue >= HUE_MARGIN ? "hue" : "none";
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
    "the darkest seen ground is lighter than the darkest unseen, or apart in hue, under every style",
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
