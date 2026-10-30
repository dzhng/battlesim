// The model workbench. Drops golden-failure GLBs and expects their
// findings; renders synthetic art through the production frame and checks
// the pose paths (articulation and a clip's phase move pixels); loads
// `?bundle=` through the one loader;
// bakes the impostor twice for one hash; produces a sheet headlessly; and
// captures the named views at 1920×1080 as evidence.
import { writeFile } from "node:fs/promises";
import { PNG } from "pngjs";
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

/** Drop bytes on the workbench through a real DOM drop event, fitted to
 *  (and replayed as) the unit type `type` ("auto" for none). */
async function drop(page, name, bytes, type = "auto", wait = true) {
  await page.getByTestId("workbench-type").selectOption(type);
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
  if (wait) await drawnOnBench(page, name);
}

/** Wait until `name` is on the bench, installed on the GPU (meshes and
 *  cards), and drawn. Installing swaps the appearances in between
 *  frames, so the models layer's stats (instances, draws) only count the
 *  model once a frame has packed it: draw one before reading them. */
async function drawnOnBench(page, name) {
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
  await drop(page, "bad-tank.glb", bad, "test_tank");
  const badState = await wb(page, () => window.__workbench.state());
  const codes = new Set(badState.findings.map((f) => f.code));
  for (const code of ["fit.vehicle_muzzle", "structure.tier_count"])
    ctx.check(`a bad GLB shows ${code}`, codes.has(code), [...codes].join(", "));
  // One per mount off its row: the synthetic cannon and roof HMG both.
  const muzzles = badState.findings.filter((f) => f.code === "fit.vehicle_muzzle").length;
  const listed = await page.locator('.wb-finding[data-code="fit.vehicle_muzzle"]').count();
  ctx.check(
    "findings are listed in the panel with their code",
    muzzles > 0 && listed === muzzles,
    `${listed} of ${muzzles}`,
  );
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
  await drop(page, "tank.glb", await synthetic(page, "tankGlb", { muzzleX: 5.9 }), "test_tank");
  await wb(page, () => window.__workbench.show({ hitBox: false, sockets: false, figure: false }));
  await wb(page, () => window.__workbench.setView("q-front"));
  const rest = {
    turret_yaw: 0,
    gun_pitch: 0,
    recoil: 0,
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
  // ---- a skinned soldier. The synthetic rig faces +Z in glTF, as the
  // Quaternius rig does: basis yaw 90.
  await page.getByTestId("workbench-yaw").selectOption("90");
  await drop(page, "rifleman.glb", await synthetic(page, "soldierGlb"), "test_rifle");
  const soldier = await wb(page, () => window.__workbench.state());
  ctx.check(
    "a valid synthetic rifleman validates clean",
    soldier.findings.length === 0,
    JSON.stringify(soldier.findings.map((f) => f.code)),
  );
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
  await drop(page, "truck.glb", await synthetic(page, "truckGlb"), "test_supply");
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
  await drop(page, "tree.glb", await synthetic(page, "treeGlb", 11));
  const tree = await wb(page, () => window.__workbench.state());
  const treeLabel = await page.getByTestId("workbench-footprint").textContent();
  ctx.check(
    "a tree is scenery, validated and drawn with the forest's trunk and canopy",
    tree.unit === "scenery" && tree.findings.length === 0 && /canopy at 12 m/.test(treeLabel),
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
  await drop(page, "rifleman.glb", await synthetic(page, "soldierGlb"), "test_rifle");
  await wb(page, () => window.__workbench.show({ hitBox: true, sockets: true, figure: true }));
  for (const view of ["q-front", "battle-mid"]) {
    await wb(page, (v) => window.__workbench.setView(v), view);
    await shot(ctx, page, `view-${view}.png`);
  }

  // ---- `?bundle=`: a baked catalog served at the site root, through the loader.
  // The workbench loads the unit art its session's (test) units wear, so the
  // synthetic bake is served under the test units' appearance names.
  const WORN = { tank: "test_tank", rifleman: "test_rifle" };
  const bakeModule = `/@fs${new URL("../../packages/scene-assets/src/bake.ts", import.meta.url).pathname}`;
  const files = await wb(
    page,
    async ([bakeModule, worn]) => {
      const s = await import("/tests/sceneAssets/synthetic.ts");
      const { bakeCatalog, runtimeCatalogText } = await import(bakeModule);
      const sources = s.testSources();
      const result = await bakeCatalog(s.testCatalog(), async (path) => sources[path], {
        authority: s.AUTHORITY,
      });
      result.runtime.appearances = Object.fromEntries(
        Object.entries(result.runtime.appearances).map(([name, entry]) => [
          worn[name] ?? name,
          entry,
        ]),
      );
      const out = {
        "catalog.json": Array.from(new TextEncoder().encode(runtimeCatalogText(result.runtime))),
      };
      for (const [path, bytes] of result.files) out[path] = Array.from(bytes);
      return out;
    },
    [bakeModule, WORN],
  );
  const served = await ctx.newPage({ viewport: VIEWPORT });
  await served.route(/\/(catalog\.json|[0-9a-f]{64}\/bundle\.bin)$/, (route) => {
    const path = new URL(route.request().url()).pathname.slice(1);
    const bytes = files[path];
    return bytes
      ? route.fulfill({ status: 200, body: Buffer.from(bytes) })
      : route.fulfill({ status: 404, body: "" });
  });
  await ctx.openLab(served, ctx.urlFor({ bundle: WORN.tank }));
  await drawnOnBench(served, WORN.tank);
  const fromCatalog = await served.evaluate(() => ({
    state: window.__workbench.state(),
    models: window.__lab.stats().models,
  }));
  ctx.check(
    "?bundle= installs a catalog appearance through the loader and draws it",
    fromCatalog.state.catalog.includes(WORN.rifleman) && fromCatalog.models.instances === 1,
    JSON.stringify(fromCatalog),
  );
  await served.evaluate(() => window.__lab.frame());
  await writeFile(ctx.evidencePath("bundle-tank.png"), await served.screenshot());

  // ---- one mesh per kind, two armies: the side's tint recolours the masked cloth.
  await served.evaluate((name) => window.__workbench.select(name), WORN.rifleman);
  await served.waitForFunction((name) => window.__workbench?.state().model === name, WORN.rifleman);
  await served.evaluate(() => window.__workbench.setSide("blue"));
  const blue = await shot(ctx, served, "bundle-rifleman-blue.png");
  await served.evaluate(() => window.__workbench.setSide("red"));
  const red = await shot(ctx, served, "bundle-rifleman-red.png");
  ctx.check(
    "the red side's tint recolours the rifleman's tint-masked cloth",
    changed(blue, red) > 500,
    `${changed(blue, red)} px`,
  );

  // ---- a grass kind from the real catalog, the clump
  // the battle's grass field instances, shown and sheeted like any scenery.
  const grass = await ctx.newPage({ viewport: VIEWPORT });
  await ctx.openLab(grass, ctx.urlFor({ bundle: "grass_meadow" }));
  await drawnOnBench(grass, "grass_meadow");
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
  await Promise.all([page.close(), served.close(), grass.close()]);
  await appearanceRace(ctx);
}

/** Real GPU catalog replacement, current presentation, failure and departure contract. */
export async function appearanceRace(ctx) {
  const race = await ctx.newPage({ viewport: VIEWPORT });
  await ctx.openLab(race);
  const rifle = await synthetic(race, "soldierGlb");
  const tank = await synthetic(race, "tankGlb", { muzzleX: 5.9 });
  await drop(race, "committed.glb", await synthetic(race, "truckGlb"), "test_supply");
  await race.evaluate(() =>
    window.__workbench.show({ figure: false, hitBox: false, sockets: false }),
  );
  await race.getByTestId("workbench-yaw").selectOption("90");
  const committedBefore = await race.evaluate(() => window.__lab.stats().models.installed);
  await race.evaluate(
    async (cameraModule) => {
      const { ScenePresentation } = await import("/src/battle/sim/presentation.ts");
      const { presentedCamera } = await import(cameraModule);
      const canvas = document.querySelector("canvas");
      const camera = presentedCamera({
        camera3d: window.__lab.camera(),
        width: canvas.width,
        height: canvas.height,
      });
      const prepare = ScenePresentation.prototype.prepareRendererAppearances;
      ScenePresentation.prototype.prepareRendererAppearances = function (...args) {
        ScenePresentation.prototype.prepareRendererAppearances = prepare;
        const renderer = args[0];
        window.committedPresentation = this;
        window.captureCommitted = async () => {
          this.frame(
            0,
            camera,
            { selected: [], xray: false, ghosts: [] },
            renderer.static_generation(),
          );
          this.render(renderer);
          const frame = this.lastFrame();
          const facts = {
            rows: this.appearanceRows(),
            instances: Array.from(frame.instances.f32),
            palettes: Array.from(frame.palettes.f32),
          };
          const { width, height, rgba } = await renderer.read_rgba(false);
          let binary = "";
          for (let i = 0; i < rgba.length; i += 8192)
            binary += String.fromCharCode(...rgba.subarray(i, i + 8192));
          return { width, height, rgba: btoa(binary), facts };
        };
        window.committedBeforePixels = window.captureCommitted();
        const before = this.appearanceRows().map((row) => row.name);
        window.heldAppearanceFacts = () => ({
          before,
          current: this.appearanceRows().map((row) => row.name),
        });
        return prepare.apply(this, args).then(
          (assets) =>
            new Promise((resolve) => {
              window.releaseOlderAssets = () => resolve(assets);
            }),
        );
      };
    },
    `/@fs${new URL("../../apps/battle-lab/src/presentedFrame.ts", import.meta.url).pathname}`,
  );
  await drop(race, "older.glb", rifle, "test_rifle", false);
  await race.waitForFunction(() => window.releaseOlderAssets);
  await drop(race, "latest.glb", tank, "test_tank", false);
  await race.waitForFunction(() => window.__workbench.state().model === "latest.glb");
  const baking = await race.evaluate(() => window.__lab.stats().models.installed);
  const [beforePixels, duringPixels] = await race.evaluate(async () => [
    await window.committedBeforePixels,
    await window.captureCommitted(),
  ]);
  const raw = (snapshot) => ({ ...snapshot, data: Buffer.from(snapshot.rgba, "base64") });
  ctx.check(
    "the replacement race begins with a nonempty rendered generation",
    committedBefore.includes("committed.glb"),
    JSON.stringify(committedBefore),
  );
  ctx.check(
    "pending and superseded generations preserve frozen committed pixels",
    changed(raw(beforePixels), raw(duringPixels), 0) === 0,
    `${changed(raw(beforePixels), raw(duringPixels), 0)} changed pixels`,
  );
  ctx.check(
    "pending generation preserves canonical committed instances and poses",
    JSON.stringify(beforePixels.facts) === JSON.stringify(duringPixels.facts),
    JSON.stringify({
      rows: duringPixels.facts.rows,
      instanceWords: duringPixels.facts.instances.length,
      paletteWords: duringPixels.facts.palettes.length,
    }),
  );
  for (const [name, snapshot] of [
    ["committed-before", beforePixels],
    ["committed-pending", duringPixels],
  ])
    await writeFile(ctx.evidencePath(`${name}.png`), PNG.sync.write(raw(snapshot)));
  ctx.check(
    "candidate preparation preserves the committed GPU generation",
    JSON.stringify(baking) === JSON.stringify(committedBefore),
    JSON.stringify(baking),
  );
  const heldFacts = await race.evaluate(() => window.heldAppearanceFacts());
  ctx.check(
    "candidate preparation preserves the committed presentation model facts",
    JSON.stringify(heldFacts.current) === JSON.stringify(heldFacts.before),
    JSON.stringify(heldFacts),
  );
  const [movedPixels, restoredPixels] = await race.evaluate(async () => {
    const p = window.committedPresentation;
    p.setLabModels([
      { appearance: "committed.glb", x: 1, y: 0, z: 0, yaw: 0, pose: { kind: "rest" } },
    ]);
    const moved = await window.captureCommitted();
    p.setLabModels([
      { appearance: "committed.glb", x: 0, y: 0, z: 0, yaw: 0, pose: { kind: "rest" } },
    ]);
    return [moved, await window.captureCommitted()];
  });
  ctx.check(
    "pending GPU work still renders changes from the current presentation",
    changed(raw(beforePixels), raw(movedPixels), 0) > 0,
    `${changed(raw(beforePixels), raw(movedPixels), 0)} changed pixels`,
  );
  ctx.check(
    "restoring the current presentation restores its exact pixels",
    changed(raw(beforePixels), raw(restoredPixels), 0) === 0,
    `${changed(raw(beforePixels), raw(restoredPixels), 0)} changed pixels`,
  );
  await race.evaluate(() => window.releaseOlderAssets());
  await drawnOnBench(race, "latest.glb");
  const latest = await race.evaluate(() => window.__lab.stats().models);
  ctx.check(
    "overlapping appearance changes finish with the newest model drawn",
    latest.installed.length === 1 &&
      latest.installed[0] === "latest.glb" &&
      latest.triangles > 0 &&
      latest.atlasLayers === 0,
    JSON.stringify(latest),
  );
  await race.screenshot({ path: ctx.evidencePath("latest-appearance.png") });
  await race.evaluate(async () => {
    const { ScenePresentation } = await import("/src/battle/sim/presentation.ts");
    const prepare = ScenePresentation.prototype.prepareRendererAppearances;
    ScenePresentation.prototype.prepareRendererAppearances = function (...args) {
      ScenePresentation.prototype.prepareRendererAppearances = prepare;
      return prepare.apply(this, args).then((assets) => {
        assets.free();
        throw new Error("deliberate latest candidate failure");
      });
    };
  });
  await race.evaluate((bytes) => {
    void window.__workbench.drop("failed.glb", new Uint8Array(bytes), {
      type: "test_rifle",
      yaw: 90,
    });
  }, rifle);
  await race.getByTestId("error").waitFor();
  await race.getByRole("button", { name: /details/i }).click();
  await race.getByText("deliberate latest candidate failure", { exact: false }).first().waitFor();
  await race.waitForFunction(() => !window.__workbench && !window.__lab);
  const failure = await race.evaluate(() => ({
    reported: document.body.innerText.includes("deliberate latest candidate failure"),
    workbench: !!window.__workbench,
    lab: !!window.__lab,
  }));
  ctx.check(
    "failed newest GPU generation reports its error and unmounts the failed renderer",
    failure.reported && !failure.workbench && !failure.lab,
    JSON.stringify(failure),
  );
  await race.screenshot({ path: ctx.evidencePath("failed-appearance.png") });
  await race.close();

  const leaving = await ctx.newPage({ viewport: VIEWPORT });
  await ctx.openLab(leaving);
  await drop(leaving, "departure-current.glb", tank, "test_tank");
  await leaving.evaluate(async () => {
    const { ScenePresentation } = await import("/src/battle/sim/presentation.ts");
    const prepare = ScenePresentation.prototype.prepareRendererAppearances;
    const commit = ScenePresentation.prototype.commitRendererAppearances;
    window.departureCommits = window.departureFrees = 0;
    ScenePresentation.prototype.commitRendererAppearances = function (...args) {
      window.departureCommits++;
      return commit.apply(this, args);
    };
    ScenePresentation.prototype.prepareRendererAppearances = function (...args) {
      ScenePresentation.prototype.prepareRendererAppearances = prepare;
      return prepare.apply(this, args).then((assets) => {
        const free = assets.free.bind(assets);
        assets.free = () => {
          window.departureFrees++;
          free();
        };
        return new Promise((resolve) => {
          window.releaseDepartingAssets = () => resolve(assets);
        });
      });
    };
  });
  await drop(leaving, "departure-pending.glb", rifle, "test_rifle", false);
  await leaving.waitForFunction(() => window.releaseDepartingAssets);
  await leaving.evaluate(() => {
    history.pushState({ key: "workbench-departure" }, "", "/labs");
    dispatchEvent(new PopStateEvent("popstate"));
  });
  await leaving.waitForFunction(() => !window.__workbench);
  await leaving.evaluate(() => window.releaseDepartingAssets());
  await leaving.waitForFunction(() => window.departureFrees > 0);
  const departure = await leaving.evaluate(() => ({
    commits: window.departureCommits,
    frees: window.departureFrees,
  }));
  ctx.check(
    "departure frees the prepared candidate without committing to freed presentation",
    departure.commits === 0 && departure.frees === 1,
    JSON.stringify(departure),
  );
  await leaving.close();
}
