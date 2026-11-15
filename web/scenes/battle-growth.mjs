// A skirmish fields the player's faction from the start and another
// faction's kind once its side meets it. That battle grows in place: the 3D
// keeps drawing, and the timeline the page logs shows the battle staged and
// its art committed, never a scene rebuild (which draws nothing until done).
// The fixture's link is a fixed small skirmish, so the meeting is the same
// every run.
import { gunzipSync } from "node:zlib";
import { readFile } from "node:fs/promises";
import { advance, lab, obs, presented } from "./_lab.mjs";

/** Minutes of play allowed for blue to meet red. */
const ROUNDS = 10;

export async function run(ctx) {
  const page = await ctx.newPage();
  // Production's diagnostic timeline logs only frames >= 1 s. Observe the
  // browser entries here as well so the smaller tripwire cannot pass silently.
  await page.addInitScript(() => {
    new PerformanceObserver((list) => {
      for (const frame of list.getEntries())
        console.log(`[growth ${Math.round(frame.startTime)} ms] long frame`, {
          ms: Math.round(frame.duration),
          scripts: frame.scripts.map((script) => [
            script.sourceFunctionName || script.invoker,
            Math.round(script.duration),
          ]),
        });
    }).observe({ type: "long-animation-frame" });
  });
  const timeline = [];
  const snapshots = [];
  const details = [];
  page.on("console", (message) => {
    const match =
      /^\[battle (\d+) ms\] (battle staged|art committed|scene rebuild|scene installed)\b/.exec(
        message.text(),
      ) ?? /^\[growth (\d+) ms\] (long frame)\b/.exec(message.text());
    if (!match) return;
    const event = { at: Number(match[1]), kind: match[2], detail: null };
    timeline.push(event);
    details.push(
      message
        .args()[1]
        ?.jsonValue()
        .then((detail) => {
          event.detail = detail;
        }),
    );
    if (event.kind === "scene installed" || event.kind === "art committed")
      snapshots.push(page.evaluate(() => window.__lab.stats().models.installed));
  });
  await ctx.openLab(page, ctx.url, 120000);
  await lab(page, () => window.__lab.route.pause());
  await page.waitForFunction(() => window.__lab?.route?.status().status === "paused");
  await advance(page, 90 - (await obs(page)).tick);
  await presented(page);
  await Promise.all(details);
  const initialWork = timeline.find((e) => e.kind === "scene installed")?.detail?.work;
  const initialNames = await snapshots.at(-1);
  console.log("battle-growth admission work", JSON.stringify(initialWork));
  // The first scene is built: from here on, only growth.
  timeline.length = 0;
  snapshots.length = 0;
  details.length = 0;

  // A tank of blue's own, placed at its entry; the battle readied.
  const entry = await page.evaluate(() => window.__lab.route.prepared().start.at);
  await page.evaluate(
    (at) => window.__lab.setCamera({ ...window.__lab.camera(), target: [...at, 0], distance: 120 }),
    entry,
  );
  const view = page.viewportSize();
  const [cx, cy] = [view.width / 2, view.height / 2];
  await page.getByRole("button", { name: "Reinforcements", exact: true }).click();
  await page.getByRole("tab", { name: "MBT", exact: true }).click();
  await page.getByRole("button", { name: "M1 Abrams", exact: true }).hover();
  await page.getByRole("button", { name: /^SEP v2 — \d+ credits$/ }).click();
  await page.mouse.move(cx, cy);
  await page.waitForTimeout(100);
  await page.mouse.click(cx, cy);
  await advance(page, 1);
  await page.getByRole("button", { name: "START BATTLE", exact: true }).click();
  await advance(page, 60 * 30);

  // It drives for the objective farthest from blue's entry, which red's army
  // contests, until blue meets a kind its battle had not fielded.
  const seen = await obs(page);
  const goal = seen.skirmish.objectives
    .map((objective) => objective.center)
    .sort(
      (a, b) =>
        Math.hypot(b[0] - entry[0], b[1] - entry[1]) - Math.hypot(a[0] - entry[0], a[1] - entry[1]),
    )[0];
  await page.evaluate(
    ([id, at]) =>
      window.__lab.route.command({
        kind: "move",
        units: [id],
        gesture: 2801,
        goal: at,
        route: "fastest",
      }),
    [seen.own[0].id, goal],
  );
  for (let round = 0; round < ROUNDS && !timeline.some((e) => e.kind === "battle staged"); round++)
    await advance(page, 60 * 30);
  // Its art is admitted while the scene keeps drawing.
  const staged = Date.now();
  const committed = () => {
    const lastStage = timeline.map((e) => e.kind).lastIndexOf("battle staged");
    return lastStage >= 0 && timeline.slice(lastStage).some((e) => e.kind === "art committed");
  };
  while (
    timeline.some((e) => e.kind === "battle staged") &&
    !committed() &&
    Date.now() - staged < 60_000
  )
    await page.waitForTimeout(250);
  await presented(page);
  // Allow the frame observer to publish the commit's last long-frame entry.
  await page.waitForTimeout(1000);
  await Promise.all(details);
  ctx.check(
    "a newly fielded kind joins the battle's scene without rebuilding it",
    timeline.some((e) => e.kind === "battle staged") &&
      committed() &&
      !timeline.some((e) => e.kind === "scene rebuild"),
    JSON.stringify(timeline),
  );
  const stages = timeline.filter((e) => e.kind === "battle staged");
  const commits = timeline.filter((e) => e.kind === "art committed");
  ctx.check(
    "one growth stages and commits exactly once",
    stages.length === 1 && commits.length === 1,
    JSON.stringify(timeline),
  );

  const names = await snapshots.at(-1);
  const catalog = JSON.parse(
    await readFile(new URL("../../assets/runtime/catalog.json", import.meta.url), "utf8"),
  );
  const identity = (name) => {
    const entry = catalog.appearances[name];
    if (!entry) throw new Error(`growth appearance ${name} is absent from the runtime catalog`);
    return {
      bundle: entry.bundle,
      skeleton: entry.skeleton ? catalog.skeletons[entry.skeleton] : null,
    };
  };
  const key = (id) => JSON.stringify([id.bundle, id.skeleton]);
  const before = new Set(
    initialNames.filter((name) => catalog.appearances[name]).map((name) => key(identity(name))),
  );
  const added = new Map(
    names
      .filter((name) => !initialNames.includes(name))
      .map((name) => {
        const id = identity(name);
        return [key(id), id];
      }),
  );
  for (const id of before) added.delete(id);
  const newContents = added.size;
  const newFiles = new Set(
    [...added.values()].flatMap((id) => [id.bundle, ...(id.skeleton ? [id.skeleton] : [])]),
  );
  const headers = await Promise.all(
    [...added.values()].map(async (id) => {
      const bytes = gunzipSync(
        await readFile(
          new URL(
            `../../assets/runtime/${catalog.gzip[id.bundle].hash}/bundle.bin`,
            import.meta.url,
          ),
        ),
      );
      return JSON.parse(bytes.subarray(16, 16 + bytes.readUInt32LE(8)).toString());
    }),
  );
  // VERTEX_BYTES in crates/renderer-core/src/models/geometry.rs: shared GPU ABI.
  const PACKED_VERTEX_BYTES = 48;
  const vertices = (header) => {
    const meshVertices = (node) => {
      if (!node || typeof node !== "object") return 0;
      if (node.positions?.$view !== undefined) return header.views[node.positions.$view][2] / 3;
      return Object.values(node).reduce((sum, child) => sum + meshVertices(child), 0);
    };
    return meshVertices(header.content);
  };
  const vertexBytes = headers.reduce(
    (sum, header) => sum + vertices(header) * PACKED_VERTEX_BYTES,
    0,
  );
  // Skinned art packs the living mesh and posed copies for corpses. Different
  // authored poses may yield identical palettes, which the art store shares;
  // headers alone bound that multiplicity. Rigid/articulated arrivals are exact.
  const posedVertexBytes = headers.reduce(
    (sum, header) =>
      sum +
      vertices(header) * PACKED_VERTEX_BYTES * (1 + (header.content.corpse_poses?.length ?? 0)),
    0,
  );
  for (const commit of commits) {
    const work = commit.detail?.work;
    console.log("battle-growth admission work", JSON.stringify(work));
    ctx.check(
      "growth encodes and decodes exactly its new contents",
      newContents > 0 &&
        work?.page?.bundlesEncoded === newContents &&
        work?.core?.bundlesDecoded === newContents,
      JSON.stringify({ added: [...added.values()], work }),
    );
    ctx.check(
      "growth packs the meshes of its new art",
      vertexBytes > 0 &&
        (vertexBytes === posedVertexBytes
          ? work?.core?.vertexBytesPacked === vertexBytes
          : work?.core?.vertexBytesPacked >= vertexBytes &&
            work?.core?.vertexBytesPacked <= posedVertexBytes),
      JSON.stringify({ vertexBytes, posedVertexBytes, core: work?.core }),
    );
    ctx.check(
      "growth carries scenery without placement or upload",
      work?.core?.sceneryPlacements === 0 && work?.core?.staticsBytesUploaded === 0,
      JSON.stringify(work?.core),
    );
    ctx.check(
      "growth installs once and reads only its missing files",
      work?.page?.appearanceLibraryInstalls === 1 &&
        work?.page?.runtimeContentsInflated <= newFiles.size,
      JSON.stringify({ newFiles: newFiles.size, page: work?.page }),
    );
    // Texture-layer and card delta assertions belong here when slices 04 and 05 land.
  }
  // A loose tripwire at 700 ms, about one quarter of slice 00's 2.5–2.9 s
  // freeze at this profile. Work counts above are the performance contract.
  const frames = timeline.filter(
    (e) =>
      e.kind === "long frame" &&
      e.at + (e.detail?.ms ?? 0) >= stages[0]?.at &&
      e.at <= commits.at(-1)?.at,
  );
  ctx.check(
    "growth has no long frame above the 700 ms tripwire",
    frames.every((e) => e.detail.ms <= 700),
    JSON.stringify(frames),
  );
  console.log(
    "battle-growth timing",
    JSON.stringify({
      stages: stages.map((e) => e.at),
      commits: commits.map((e) => e.at),
      longFrames: frames,
    }),
  );
  await page.close();
}
