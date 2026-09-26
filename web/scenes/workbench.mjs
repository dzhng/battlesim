// Slice 20: the model workbench. Drops golden-failure GLBs and expects their
// findings; renders synthetic art through the production frame and checks
// the pose paths (GPU palette = CPU pose, articulation moves pixels, the feed
// replay drives the pose driver); loads `?bundle=` through the one loader;
// bakes the impostor twice for one hash; produces a sheet headlessly; and
// captures the named views at 1920×1080 as evidence.
import { writeFile } from "node:fs/promises";
import { decode } from "./_png.mjs";

const VIEWPORT = { width: 1920, height: 1080 };

/** Pixels whose colour moved by more than `threshold` in any channel. */
function changed(a, b, threshold = 24) {
  let n = 0;
  for (let i = 0; i < a.data.length; i += 4)
    if (
      Math.abs(a.data[i] - b.data[i]) > threshold ||
      Math.abs(a.data[i + 1] - b.data[i + 1]) > threshold ||
      Math.abs(a.data[i + 2] - b.data[i + 2]) > threshold
    )
      n++;
  return n;
}

const wb = (page, fn, arg) => page.evaluate(fn, arg);

/** Generate a synthetic GLB in the page (the scene-assets test builders). */
const synthetic = (page, builder, options) =>
  wb(
    page,
    async ([builder, options]) => {
      const s = await import("/tests/sceneAssets/synthetic.ts");
      return Array.from(s[builder](options));
    },
    [builder, options ?? {}],
  );

/** Drop bytes on the workbench through a real DOM drop event. */
async function drop(page, name, bytes) {
  await wb(
    page,
    ([name, bytes]) => {
      const file = new File([new Uint8Array(bytes)], name, { type: "model/gltf-binary" });
      const data = new DataTransfer();
      data.items.add(file);
      const target = document.querySelector('[data-testid="workbench-drop"]');
      target.dispatchEvent(
        new DragEvent("drop", { dataTransfer: data, bubbles: true, cancelable: true }),
      );
    },
    [name, bytes],
  );
  // Installed on the GPU (meshes, clips, pose kernel), not just chosen.
  await page.waitForFunction(
    (name) =>
      window.__workbench?.state().model === name &&
      window.__lab.stats().models.installed.includes(name),
    name,
    { timeout: 30000 },
  );
  await wb(page, () => window.__lab.frame());
}

async function shot(ctx, page, file) {
  await wb(page, () => window.__lab.frame());
  const buffer = await page.screenshot();
  if (file) await writeFile(ctx.evidencePath(file), buffer);
  return decode(buffer);
}

export async function run(ctx) {
  const page = await ctx.newPage({ viewport: VIEWPORT });
  await ctx.openLab(page);

  // ---- a bad GLB: specific findings, and still rendered for judging.
  const started = Date.now();
  // The spike's stub gun (3 m) against the fixture's realistic muzzle.
  const bad = await synthetic(page, "tankGlb", { muzzleX: 3, lods: "none" });
  await drop(page, "bad-tank.glb", bad);
  const badState = await wb(page, () => window.__workbench.state());
  const codes = new Set(badState.findings.map((f) => f.code));
  for (const code of ["fit.tank_muzzle", "structure.tier_count", "provenance.unlisted"])
    ctx.check(`a bad GLB shows ${code}`, codes.has(code), [...codes].join(", "));
  const listed = await page.locator('.wb-finding[data-code="fit.tank_muzzle"]').count();
  ctx.check("findings are listed in the panel with their code", listed === 1, `${listed}`);
  const stats = await wb(page, () => window.__lab.stats().models);
  ctx.check(
    "the bad GLB still renders through the models layer",
    stats.instances === 1 && stats.triangles > 0,
    JSON.stringify(stats),
  );
  ctx.check(
    "drop to drawn in seconds",
    Date.now() - started < 10000,
    `${Date.now() - started} ms (validate ${Math.round(badState.loadMs)} ms)`,
  );

  // ---- a valid tank: articulation moves pixels, and the palette is the CPU's.
  await drop(page, "tank.glb", await synthetic(page, "tankGlb", { muzzleX: 5.9 }));
  await wb(page, () => window.__workbench.show({ hitBox: false, sockets: false, figure: false }));
  await wb(page, () => window.__workbench.setView("q-front"));
  const rest = {
    turret_yaw: 0,
    gun_pitch: 0,
    hmg_yaw: 0,
    hmg_pitch: 0,
    travel_l: 0,
    travel_r: 0,
    deploy: 0,
  };
  await wb(page, (a) => window.__workbench.setPose({ kind: "articulated", articulation: a }), rest);
  const tankRest = await shot(ctx, page, "tank-rest.png");
  await wb(page, (a) => window.__workbench.setPose({ kind: "articulated", articulation: a }), {
    ...rest,
    turret_yaw: Math.PI / 2,
    gun_pitch: 0.3,
  });
  const tankYawed = await shot(ctx, page, "tank-turret-90.png");
  const moved = changed(tankRest, tankYawed);
  ctx.check("turret yaw and gun pitch move the drawn tank", moved > 2000, `${moved} px`);
  const tankError = await wb(page, () => window.__workbench.paletteError());
  ctx.check(
    "the articulated palette equals the CPU's node worlds",
    tankError < 1e-4,
    `${tankError}`,
  );

  // ---- the feed replay drives the pose driver.
  await wb(page, () => window.__workbench.setFeed(10));
  const slewed = await wb(page, () => window.__workbench.models()[0].pose.articulation);
  ctx.check(
    "a tank feed slewing its gun turns the turret relative to the hull",
    Math.abs(slewed.turret_yaw) > 0.5 && slewed.travel_l !== slewed.travel_r,
    JSON.stringify(slewed),
  );

  // ---- a skinned soldier: clips on the GPU equal the CPU pose. The
  // synthetic rig faces +Z in glTF, as the Quaternius rig does: basis yaw 90.
  await page.getByTestId("workbench-yaw").selectOption("90");
  await drop(page, "rifleman.glb", await synthetic(page, "soldierGlb"));
  const soldier = await wb(page, () => window.__workbench.state());
  ctx.check(
    "a valid synthetic rifleman validates clean",
    soldier.findings.every((f) => f.code.startsWith("provenance")),
    JSON.stringify(soldier.findings.map((f) => f.code)),
  );
  let worst = 0;
  for (const [clip, phase] of [
    ["walk", 0.3],
    ["death", 0.7],
    ["run", 0.95],
  ]) {
    await wb(page, (p) => window.__workbench.setPose(p), {
      kind: "skinned",
      clip,
      phase,
      blend: null,
    });
    worst = Math.max(worst, await wb(page, () => window.__workbench.paletteError()));
  }
  ctx.check("the pose kernel's palette equals scene-assets' CPU pose", worst < 1e-4, `${worst}`);
  await wb(page, () => window.__workbench.setView("left"));
  await wb(page, (p) => window.__workbench.setPose(p), {
    kind: "skinned",
    clip: "death",
    phase: 0,
    blend: null,
  });
  const standing = await shot(ctx, page, "soldier-death-0.png");
  await wb(page, (p) => window.__workbench.setPose(p), {
    kind: "skinned",
    clip: "death",
    phase: 1,
    blend: null,
  });
  const fallen = await shot(ctx, page, "soldier-death-1.png");
  ctx.check(
    "a clip's phase moves the drawn body",
    changed(standing, fallen) > 2000,
    `${changed(standing, fallen)} px`,
  );

  await wb(page, () => window.__workbench.setFeed(4));
  const walking = await wb(page, () => window.__workbench.models().map((m) => m.pose.clip));
  ctx.check(
    "an infantry feed poses each soldier on his own (walking)",
    walking.length === 4 && walking.every((c) => c === "walk"),
    walking.join(","),
  );
  await wb(page, () => window.__workbench.setFeed(14));
  const pinned = await wb(page, () => window.__workbench.models().map((m) => m.pose.clip));
  ctx.check(
    "suppression pins the squad prone",
    pinned.every((c) => c === "prone_pinned"),
    pinned.join(","),
  );
  await wb(page, () => window.__workbench.setFeed(17.5));
  const fell = await wb(page, () => window.__workbench.models().map((m) => m.pose.clip));
  ctx.check("the fallen soldier plays his death", fell.includes("death"), fell.join(","));
  await wb(page, () => window.__workbench.setFeed(null));

  // ---- impostor: same inputs, same hash.
  const first = await wb(page, () => window.__workbench.bakeImpostor());
  const second = await wb(page, () => window.__workbench.bakeImpostor());
  ctx.check(
    "the impostor bake is deterministic",
    first.hash === second.hash,
    `${first.hash} ${second.hash}`,
  );
  const alpha = Buffer.from(first.albedo, "base64");
  let covered = 0;
  for (let i = 3; i < alpha.length; i += 4) if (alpha[i] > 0) covered++;
  const cells = (first.width * first.height) / 100;
  ctx.check(
    "the impostor atlas holds the model, framed with a clear margin",
    covered > cells && covered < first.width * first.height * 0.5 && alpha[3] === 0,
    `${covered} covered of ${first.width}×${first.height}`,
  );

  // ---- a sheet, produced headlessly.
  await drop(page, "truck.glb", await synthetic(page, "truckGlb"));
  const sheet = await wb(page, () => window.__workbench.sheet());
  const png = (dataUrl) => Buffer.from(dataUrl.split(",")[1], "base64");
  await writeFile(ctx.evidencePath("sheet-truck-contact.png"), png(sheet.contact));
  for (const strip of sheet.strips)
    await writeFile(ctx.evidencePath(`sheet-truck-${strip.name}.png`), png(strip.png));
  await ctx.writeEvidence("sheet-truck-stats.json", sheet.stats);
  const contact = decode(png(sheet.contact));
  ctx.check(
    "the sheet has the eight views and the vehicle strips",
    contact.width === 2048 &&
      contact.height > 1024 &&
      ["articulation", "running-gear", "deploy"].every((n) =>
        sheet.strips.some((s) => s.name === n),
      ),
    `${contact.width}×${contact.height}; ${sheet.strips.map((s) => s.name)}`,
  );
  const deploy = decode(png(sheet.strips.find((s) => s.name === "deploy").png));
  const half = deploy.width / 8;
  let deployMoved = 0;
  for (let y = 40; y < deploy.height; y++)
    for (let x = 0; x < half; x++) {
      const a = (y * deploy.width + x) * 4;
      const b = (y * deploy.width + x + 7 * half) * 4;
      if (Math.abs(deploy.data[a] - deploy.data[b]) > 24) deployMoved++;
    }
  ctx.check(
    "the deploy strip's first and last frames differ",
    deployMoved > 500,
    `${deployMoved} px`,
  );

  // ---- scenery: a tree and a wall, with what the simulation knows of them.
  await page.getByTestId("workbench-yaw").selectOption("0");
  await page.getByTestId("workbench-unit").selectOption("scenery");
  await page.getByTestId("workbench-scenery").selectOption("tree");
  await drop(page, "tree.glb", await synthetic(page, "buildingGlb", 12));
  const tree = await wb(page, () => window.__workbench.state());
  const treeLabel = await page.getByTestId("workbench-footprint").textContent();
  ctx.check(
    "a tree is scenery, validated and drawn with the forest's trunk and canopy",
    tree.unit === "scenery" &&
      tree.findings.every((f) => f.code.startsWith("provenance")) &&
      /canopy at 12 m/.test(treeLabel),
    `${JSON.stringify(tree.findings.map((f) => f.code))} ${treeLabel}`,
  );
  await wb(page, () => window.__workbench.show({ hitBox: true, sockets: true, figure: true }));
  await wb(page, () => window.__workbench.setView("q-front"));
  await shot(ctx, page, "scenery-tree.png");
  const treeSheet = await wb(page, () => window.__workbench.sheet());
  await writeFile(
    ctx.evidencePath("sheet-tree-contact.png"),
    Buffer.from(treeSheet.contact.split(",")[1], "base64"),
  );
  ctx.check(
    "a scenery sheet has its states strip",
    treeSheet.strips.some((s) => s.name === "states"),
    treeSheet.strips.map((s) => s.name).join(","),
  );
  await page.getByTestId("workbench-scenery").selectOption("wall");
  await page.waitForFunction(() =>
    /stops/.test(document.querySelector('[data-testid="workbench-footprint"]')?.textContent ?? ""),
  );
  const wallLabel = await page.getByTestId("workbench-footprint").textContent();
  ctx.check(
    "a wall carries the simulation's blocking and sight classes",
    /stops infantry and vehicle/.test(wallLabel) && /hides what is behind it/.test(wallLabel),
    wallLabel,
  );
  await page.getByTestId("workbench-unit").selectOption("auto");
  await page.getByTestId("workbench-yaw").selectOption("auto");

  // ---- the named views at 1920×1080, with the figure, hit box and sockets.
  await drop(page, "rifleman.glb", await synthetic(page, "soldierGlb"));
  await wb(page, () => window.__workbench.show({ hitBox: true, sockets: true, figure: true }));
  for (const view of [
    "q-front",
    "front",
    "left",
    "rear",
    "top",
    "battle-near",
    "battle-mid",
    "battle-far",
  ]) {
    await wb(page, (v) => window.__workbench.setView(v), view);
    await shot(ctx, page, `view-${view}.png`);
  }

  // ---- `?bundle=`: a baked catalog served at the site root, through the loader.
  const bakeModule = `/@fs${new URL("../../packages/scene-assets/src/bake.ts", import.meta.url).pathname}`;
  const files = await wb(
    page,
    async (bakeModule) => {
      const s = await import("/tests/sceneAssets/synthetic.ts");
      const { bakeCatalog, runtimeCatalogText } = await import(bakeModule);
      const sources = s.testSources();
      const result = await bakeCatalog(s.testCatalog(), async (path) => sources[path], {
        authority: s.AUTHORITY,
      });
      const out = {
        "catalog.json": Array.from(new TextEncoder().encode(runtimeCatalogText(result.runtime))),
      };
      for (const [path, bytes] of result.files) out[path] = Array.from(bytes);
      return out;
    },
    bakeModule,
  );
  const served = await ctx.newPage({ viewport: VIEWPORT });
  await served.route(/\/(catalog\.json|[0-9a-f]{64}\/bundle\.bin)$/, (route) => {
    const path = new URL(route.request().url()).pathname.slice(1);
    const bytes = files[path];
    return bytes
      ? route.fulfill({ status: 200, body: Buffer.from(bytes) })
      : route.fulfill({ status: 404, body: "" });
  });
  await ctx.openLab(served, `${ctx.url}?bundle=tank`);
  await served.waitForFunction(
    () =>
      window.__workbench?.state().model === "tank" &&
      window.__lab.stats().models.installed.includes("tank"),
    undefined,
    {
      timeout: 30000,
    },
  );
  const fromCatalog = await served.evaluate(() => ({
    state: window.__workbench.state(),
    models: window.__lab.stats().models,
  }));
  ctx.check(
    "?bundle= installs a catalog appearance through the loader and draws it",
    fromCatalog.state.catalog.includes("rifleman") && fromCatalog.models.instances === 1,
    JSON.stringify(fromCatalog),
  );
  await served.evaluate(() => window.__lab.frame());
  await writeFile(ctx.evidencePath("bundle-tank.png"), await served.screenshot());

  // ---- one mesh per kind, two armies: the side's tint recolours the masked cloth.
  await served.evaluate(() => window.__workbench.select("rifleman"));
  await served.waitForFunction(() => window.__workbench?.state().model === "rifleman");
  await served.evaluate(() => window.__workbench.setSide("blue"));
  const blue = await shot(ctx, served, "bundle-rifleman-blue.png");
  await served.evaluate(() => window.__workbench.setSide("red"));
  const red = await shot(ctx, served, "bundle-rifleman-red.png");
  ctx.check(
    "the red side's tint recolours the rifleman's tint-masked cloth",
    changed(blue, red) > 500,
    `${changed(blue, red)} px`,
  );

  // ---- battle-look slice 18: a grass kind from the real catalog, the clump
  // the battle's grass field instances, shown and sheeted like any scenery.
  const grass = await ctx.newPage({ viewport: VIEWPORT });
  await ctx.openLab(grass, `${ctx.url}?bundle=grass_meadow`);
  await grass.waitForFunction(
    () =>
      window.__workbench?.state().model === "grass_meadow" &&
      window.__lab.stats().models.installed.includes("grass_meadow"),
    undefined,
    { timeout: 30000 },
  );
  const meadow = await grass.evaluate(() => ({
    state: window.__workbench.state(),
    models: window.__lab.stats().models,
  }));
  ctx.check(
    "a grass kind is a catalog appearance the workbench installs and draws",
    meadow.models.instances === 1 && meadow.state.catalog.includes("grass_wheat"),
    JSON.stringify({ model: meadow.state.model, models: meadow.models }),
  );
  await grass.evaluate(() => window.__workbench.setView("q-front"));
  await grass.evaluate(() => window.__lab.frame());
  await writeFile(ctx.evidencePath("bundle-grass_meadow.png"), await grass.screenshot());
  const grassSheet = await grass.evaluate(() => window.__workbench.sheet());
  await writeFile(
    ctx.evidencePath("sheet-grass_meadow-contact.png"),
    Buffer.from(grassSheet.contact.split(",")[1], "base64"),
  );
  ctx.check(
    "a grass kind's sheet has its states strip",
    grassSheet.strips.some((s) => s.name === "states"),
    grassSheet.strips.map((s) => s.name).join(","),
  );
}
