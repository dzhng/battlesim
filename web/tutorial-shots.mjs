// The main menu tutorial's pictures, drawn from one real skirmish: the
// faction roster on a seeded generated map, played through the ordinary
// picker, cursor and orders while the authority is paused, so each moment is
// framed exactly. Stills are JPEGs; a gesture is a short silent loop, zoomed
// in, with the cursor that makes it.
//
//   bun run --cwd web tutorial-shots              every shot
//   bun run --cwd web tutorial-shots -- place     only the named shots
//
// It needs what a battle needs: the WebAssembly built and the battle's LFS
// assets fetched (assets/runtime, assets/third-party). It starts its own
// server unless VERIFY_URL names one, and shares the GPU like a scene
// (`lockf -k <main checkout>/throwaway/gpu.lock …`). Shots go to
// assets/runtime/tutorial/, served at /tutorial/; frames and working files
// to throwaway/tutorial-shots/.
//
// Moments are found by what the observation shows (a capture under way, a
// contact beside a seen enemy), not by tick, so a rule change moves them
// rather than breaking them; a moment that never comes fails by name. Every
// picture shows what the seed plays and how the game drew it, so a changed
// look, HUD, generator or rule set calls for drawing them again. Then look at
// each one against its caption in Tutorial.tsx (captions name the kind of
// moment, not this seed's details), and re-approve the menu scene's picture
// of the page (`UPDATE_BASELINES=ui`), whose check also fails on a picture
// left as an unfetched LFS pointer.
import { execFileSync } from "node:child_process";
import { mkdir, rm } from "node:fs/promises";
import { startServer, WEBGPU_FLAGS } from "./scene.mjs";
import { advance, lab, until } from "./scenes/_lab.mjs";

const HERE = new URL(".", import.meta.url);
const OUT = new URL("../assets/runtime/tutorial/", HERE).pathname;
const WORK = new URL("../throwaway/tutorial-shots/", HERE).pathname;
const BATTLE = "/battle?type=mixed&size=small&seed=1&faction=us";
const VIEWPORT = { width: 1600, height: 900 };
const SCALE = 2;
const FPS = 30;

/** The opening force, bought where the seed's first fights happen. */
const FORCE = [
  ["us_m1_abrams_sep_v2", [900, 1000]],
  ["us_stryker_m1126_icv", [960, 1000]],
  ["rifle_squad", [880, 960]],
  ["us_army_scouts_light_patrol", [400, 1050]],
  ["us_m1151_hmmwv_hmg", [500, 600]],
];
/** The right-drag's pair, bought onto the fields along the approach road,
 *  where a move lands where it is pressed rather than snapping to streets;
 *  close together, as a move keeps their spread and both ghosts must show. */
const ARMOUR = [
  ["us_m1_abrams_sep_v2", [835, 1330]],
  ["us_stryker_m1126_icv", [865, 1330]],
];
/** Open ground south of the town, and a building in its first block that a
 *  squad can't stand in. */
const PLACE_FROM = [930, 1010];
const PLACE_BLOCKED = [930, 994];
const SHOTS = ["overview", "buy", "place", "capture", "contact", "move"];
const asked = process.argv.slice(2).filter((a) => a !== "--");
for (const name of asked) if (!SHOTS.includes(name)) throw new Error(`no shot "${name}": ${SHOTS}`);
const wanted = (name) => asked.length === 0 || asked.includes(name);

const command = (page, order) => lab(page, (o) => window.__lab.route.command(o), order);
const css = (page, [x, y]) =>
  lab(page, ([x, y]) => window.__lab.projectToCss(x, y, window.__lab.route.surfaceZ(x, y)), [x, y]);
const settle = async (page, ms = 60) => {
  await lab(page, () => window.__lab.frame());
  await page.waitForTimeout(ms);
};

/** The rig on `at`, from `distance` metres, keeping the opening's heading. */
async function frame(page, at, distance, pitch = 0.95) {
  await lab(page, (pose) => window.__lab.placeCamera({ ...pose, yaw: window.__lab.camera().yaw }), {
    target: at,
    distance,
    pitch,
  });
  await settle(page, 250);
}

/** A 16:9 box around `boxes` with `margin`, kept on screen. */
function around(boxes, margin = 40) {
  let x0 = Math.min(...boxes.map((b) => b.x)) - margin;
  let y0 = Math.min(...boxes.map((b) => b.y)) - margin;
  let x1 = Math.max(...boxes.map((b) => b.x + b.width)) + margin;
  let y1 = Math.max(...boxes.map((b) => b.y + b.height)) + margin;
  const w = Math.max(x1 - x0, ((y1 - y0) * 16) / 9);
  const h = (w * 9) / 16;
  const cx = (x0 + x1) / 2;
  const cy = (y0 + y1) / 2;
  x0 = Math.min(Math.max(cx - w / 2, 0), VIEWPORT.width - w);
  y0 = Math.min(Math.max(cy - h / 2, 0), VIEWPORT.height - h);
  return { x: x0, y: y0, width: Math.min(w, VIEWPORT.width), height: Math.min(h, VIEWPORT.height) };
}

/** The box around `boxes` with `margin`, kept on screen, in their own shape. */
function fit(boxes, margin) {
  const x = Math.max(Math.min(...boxes.map((b) => b.x)) - margin, 0);
  const y = Math.max(Math.min(...boxes.map((b) => b.y)) - margin, 0);
  const right = Math.min(Math.max(...boxes.map((b) => b.x + b.width)) + margin, VIEWPORT.width);
  const bottom = Math.min(Math.max(...boxes.map((b) => b.y + b.height)) + margin, VIEWPORT.height);
  return { x, y, width: right - x, height: bottom - y };
}

/** A page point as a box. */
const spot = ([x, y]) => ({ x, y, width: 1, height: 1 });

/** A 16:9 box `width` wide centred on page point `at`, kept on screen. */
const centred = ([x, y], width) =>
  around([{ x: x - width / 2, y: y - (width * 9) / 32, width, height: (width * 9) / 16 }], 0);

/** A still: the page, or `clip` of it, as a JPEG 1440 pixels wide. */
async function still(page, name, clip) {
  if (!wanted(name)) return;
  await settle(page);
  const png = `${WORK}${name}.png`;
  await page.screenshot({ path: png, clip });
  const jpeg = ["-vf", "scale=1440:-2:flags=lanczos", "-q:v", "2"];
  execFileSync("ffmpeg", ["-loglevel", "error", "-y", "-i", png, ...jpeg, `${OUT}${name}.jpg`]);
  console.log(`  ${name}.jpg`);
}

/** A loop: `steps` drive the page one frame each, each frame cropped to
 *  `clip`, encoded as a silent H.264 loop 1280 pixels wide. */
async function loop(page, name, clip, steps) {
  if (!wanted(name)) return;
  const dir = `${WORK}${name}/`;
  await rm(dir, { recursive: true, force: true });
  await mkdir(dir, { recursive: true });
  for (const [i, step] of steps.entries()) {
    await step();
    await settle(page, 30);
    await page.screenshot({ path: `${dir}${String(i).padStart(4, "0")}.png`, clip });
  }
  const input = ["-loglevel", "error", "-y", "-framerate", String(FPS), "-i", `${dir}%04d.png`];
  const h264 = ["-c:v", "libx264", "-pix_fmt", "yuv420p", "-crf", "20", "-movflags", "+faststart"];
  const scale = "scale=1280:-2:flags=lanczos";
  execFileSync("ffmpeg", [...input, "-vf", scale, ...h264, "-an", `${OUT}${name}.mp4`]);
  console.log(`  ${name}.mp4 (${steps.length} frames)`);
}

const ease = (t) => (t < 0.5 ? 2 * t * t : 1 - (-2 * t + 2) ** 2 / 2);
/** `n` pointer moves from `a` to `b` (page points), eased. */
const glide = (page, a, b, n) =>
  Array.from({ length: n }, (_, i) => () => {
    const t = ease((i + 1) / n);
    return page.mouse.move(a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t);
  });
const hold = (n, step = async () => {}) => Array.from({ length: n }, () => step);

/** A fresh page on the seeded battle, paused at tick 10. */
async function openBattle(browser, url) {
  const context = await browser.newContext({ viewport: VIEWPORT, deviceScaleFactor: SCALE });
  const page = await context.newPage();
  page.on("pageerror", (e) => console.error(`page error: ${e.message}`));
  // The frame-rate readout is a developer's, not the player's picture.
  await page.addInitScript(() =>
    document.addEventListener("DOMContentLoaded", () => {
      const style = document.createElement("style");
      style.textContent = ".frame-rate { display: none !important; }";
      document.head.append(style);
    }),
  );
  await page.goto(url);
  await page.waitForFunction(() => window.__lab?.ready || window.__lab?.error, undefined, {
    timeout: 180_000,
  });
  const error = await lab(page, () => window.__lab.error);
  if (error) throw new Error(`battle failed: ${error}`);
  await page.waitForFunction(() => window.__lab.route?.tick() > 3, undefined, { timeout: 60_000 });
  await lab(page, () => window.__lab.route.pause());
  await advance(page, 10 - (await lab(page, () => window.__lab.route.tick())));
  return page;
}

/** Buy `force` and start the battle. */
async function deploy(page, force) {
  for (const [variant, destination] of force) {
    const ack = await command(page, { kind: "confirm_purchase", variant, destination });
    if (ack?.error) throw new Error(`buying ${variant}: ${JSON.stringify(ack.error)}`);
  }
  await command(page, { kind: "ready" });
}

/** A moment the battle must reach within `limit` ticks, or a named failure. */
async function moment(page, what, test, limit) {
  const o = await until(page, test, limit, 30);
  if (!o) throw new Error(`the battle never showed ${what} within ${limit} ticks`);
  return o;
}

async function preparation(page) {
  // The opening, the cursor on the order to start, clear of its label.
  const start = await page.getByRole("button", { name: "START BATTLE", exact: true }).boundingBox();
  await page.mouse.move(start.x + start.width - 14, start.y + start.height / 2);
  // The town, its flags and the bar above them: the fields around left out,
  // so the HUD's text stays near its own size.
  await still(page, "overview", { x: 450, y: 0, width: 800, height: 450 });

  // The shop: the vehicles tab, a family's variants fanned out under the cursor.
  await page.getByRole("button", { name: "Reinforcements", exact: true }).click();
  await page.getByRole("tab", { name: "VEH", exact: true }).click();
  await page.getByRole("button", { name: "M1 Abrams", exact: true }).hover();
  await settle(page, 300);
  const boxes = await page
    .locator(".hud-purchase-stack, .hud-purchase-families")
    .evaluateAll((els) => els.map((e) => e.getBoundingClientRect().toJSON()));
  await still(page, "buy", fit(boxes, 16));
  // Escape here would open the pause menu: it cancels a placement, not the shop.
  await page.getByRole("button", { name: "Reinforcements", exact: true }).click();
  await page.mouse.move(VIEWPORT.width / 2, VIEWPORT.height / 2);

  // Placing: a rifle squad's ghost over open ground, onto a building it
  // can't stand in (the blocked cursor), and back.
  await frame(
    page,
    [(PLACE_FROM[0] + PLACE_BLOCKED[0]) / 2, (PLACE_FROM[1] + PLACE_BLOCKED[1]) / 2],
    52,
  );
  const from = await css(page, PLACE_FROM);
  const blocked = await css(page, PLACE_BLOCKED);
  await page.getByRole("button", { name: "Reinforcements", exact: true }).click();
  await page.getByRole("tab", { name: "INF", exact: true }).click();
  await page.getByRole("button", { name: "Rifle Squad", exact: true }).click();
  await page.mouse.move(...from);
  await settle(page, 300);
  // The ghost stands above the cursor: the frame sits a little above the path.
  const clip = centred([(from[0] + blocked[0]) / 2, (from[1] + blocked[1]) / 2 - 10], 800);
  await loop(page, "place", clip, [
    ...hold(20),
    ...glide(page, from, blocked, 40),
    ...hold(30),
    ...glide(page, blocked, from, 40),
    ...hold(20),
  ]);
  await page.keyboard.press("Escape");
}

/** Right-drag: select the tank, press beside it, drag to face, release,
 *  and watch it set off. */
async function rightDrag(browser, url) {
  const page = await openBattle(browser, url);
  // Sound captions are their own lesson; here they would cover the gesture.
  await page.addStyleTag({ content: "[data-testid=captions] { display: none !important; }" });
  await deploy(page, ARMOUR);
  const armour = ARMOUR.map(([kind]) => kind);
  const arrived = await moment(
    page,
    "the armour at its placements",
    (o) =>
      armour.every((k) =>
        o.own.some(
          (u) =>
            u.kind === k &&
            (u.goal === null ||
              Math.hypot(u.position[0] - u.goal[0], u.position[1] - u.goal[1]) < 4),
        ),
      ),
    4000,
  );
  // Let the arrival's dust settle before the gesture starts.
  await advance(page, 90);
  // The tank alone: its ghost stands where the cursor presses, and the
  // Stryker beside it shows what is not ordered.
  const tank = arrived.own.find((u) => u.kind === ARMOUR[0][0]);
  const at = [tank.position[0], tank.position[1]];
  await frame(page, at, 115);
  await lab(page, (ids) => window.__lab.route.select(ids), [tank.id]);
  const [cx, cy] = await css(page, at);
  // Beside the tank, never behind it (a right-click just behind a lone
  // vehicle reverses it), a short drag giving the facing, so the cursor
  // stays by the ghost it makes.
  const rest = [cx + 60, cy - 130];
  const press = [cx + 160, cy + 40];
  const facing = [cx + 270, cy + 100];
  await page.mouse.move(...rest);
  await settle(page, 300);
  // Both vehicles, the press and the drag, with room for the ghost and the
  // panels, kept above the army bar.
  const bar = await page.locator(".hud-purchase-controls").boundingBox();
  const stryker = arrived.own.find((u) => u.kind === ARMOUR[1][0]).position;
  const beside = await css(page, [stryker[0], stryker[1]]);
  const clip = around([spot([cx, cy]), spot(beside), spot(press), spot(facing)], 150);
  clip.y = Math.max(0, Math.min(clip.y, bar.y - 24 - clip.height));
  await loop(page, "move", clip, [
    ...hold(15),
    ...glide(page, rest, press, 25),
    ...hold(8),
    async () => page.mouse.down({ button: "right" }),
    ...glide(page, press, facing, 30),
    ...hold(25),
    async () => page.mouse.up({ button: "right" }),
    ...hold(75, () => advance(page, 4)),
  ]);
  await page.context().close();
}

async function battle(page) {
  await deploy(page, FORCE);

  // Capturing: a unit of ours in a ring, its flag counting up.
  const capture = await moment(
    page,
    "an objective half captured by blue",
    (o) =>
      o.skirmish.objectives.some(
        (x) => x.capturing === "blue" && x.captureProgress > 0.45 && x.captureProgress < 0.75,
      ),
    6000,
  );
  const ring = capture.skirmish.objectives.find((x) => x.capturing === "blue");
  const [taker] = capture.own
    .map((u) => ({
      u,
      d: Math.hypot(u.position[0] - ring.center[0], u.position[1] - ring.center[1]),
    }))
    .sort((a, b) => a.d - b.d);
  await frame(page, ring.center, ring.radiusM * 4.4);
  const [ux, uy] = await css(page, [taker.u.position[0], taker.u.position[1]]);
  // The cursor near the unit, never over it.
  await page.mouse.move(ux + 70, uy + 40);
  // The whole ring, its flag and the unit taking it, the command bar below
  // left out.
  await still(page, "capture", centred(await css(page, ring.center), 820));

  // Seeing the enemy: a contact (heard, or last seen) near an identified enemy.
  const near = (o) => {
    for (const c of o.contacts)
      for (const e of o.identified) {
        const d = Math.hypot(c.center[0] - e.position[0], c.center[1] - e.position[1]);
        if (d > 40 && d < 180) return { c, e, d };
      }
    return null;
  };
  const sighting = await moment(page, "a contact beside an identified enemy", near, 12000);
  const { c, e, d } = near(sighting);
  const between = [(c.center[0] + e.position[0]) / 2, (c.center[1] + e.position[1]) / 2];
  let distance = Math.max(260, d * 1.9);
  await frame(page, between, distance);
  // The contact and every enemy seen near it, names and all, the camera
  // drawn back until they span a crop small enough for the names to read.
  const seen = sighting.identified.filter(
    (x) => Math.hypot(x.position[0] - c.center[0], x.position[1] - c.center[1]) < d + 40,
  );
  const spread = async () => {
    const points = [];
    for (const p of [c.center, ...seen.map((x) => x.position)]) {
      points.push(spot(await css(page, [p[0], p[1]])));
    }
    return around(points, 0);
  };
  let box = await spread();
  if (box.width > 440) {
    distance *= box.width / 440;
    await frame(page, between, distance);
    box = await spread();
  }
  const [ex, ey] = await css(page, [e.position[0], e.position[1]]);
  await page.mouse.move(ex + 30, ey + 40);
  // Panels stand up and right of their units: more room above than below;
  // the score bar along the top left out.
  const clip = around([{ ...box, y: box.y - 60, height: box.height + 60 }], 110);
  const top = await page.getByLabel("Objectives", { exact: true }).boundingBox();
  clip.y = Math.min(Math.max(clip.y, top.y + top.height + 12), VIEWPORT.height - clip.height);
  await still(page, "contact", clip);
}

async function main() {
  await rm(WORK, { recursive: true, force: true });
  await mkdir(WORK, { recursive: true });
  await mkdir(OUT, { recursive: true });
  const server = await startServer();
  const { chromium } = await import("playwright");
  const browser = await chromium.launch({
    channel: "chromium",
    args: [...WEBGPU_FLAGS, "--mute-audio"],
  });
  try {
    const url = `${server.url}${BATTLE}`;
    // Every shot but the right-drag is one battle's; that one plays its own.
    if (SHOTS.some((name) => name !== "move" && wanted(name))) {
      const page = await openBattle(browser, url);
      await preparation(page);
      if (wanted("capture") || wanted("contact")) await battle(page);
      await page.context().close();
    }
    if (wanted("move")) await rightDrag(browser, url);
  } finally {
    await browser.close();
    await server.close();
  }
}

await main();
