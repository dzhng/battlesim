// A skirmish fields the player's faction from the start and another
// faction's kind once its side meets it. That battle grows in place: the 3D
// keeps drawing, and the timeline the page logs shows the battle staged and
// its art committed, never a scene rebuild (which draws nothing until done).
// The growth's art is held at the network edge until the new kind's stand-in
// blocks are judged and attacked (`_stand-ins.mjs`), then released. The
// fixture's link is a fixed small skirmish, so the meeting is the same every
// run.
import { readFile } from "node:fs/promises";
import { advance, lab, obs, presented } from "./_lab.mjs";
import { growthArt } from "./_growthArt.mjs";
import { approachContact, followEnemyObjective } from "./_contact-battle.mjs";
import { heldUnit, holdArt, judgeStandIns } from "./_stand-ins.mjs";

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
  const record = (message) => {
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
  };
  page.on("console", record);
  const network = await holdArt(page);
  await ctx.openLab(page, ctx.url, 120000);
  await lab(page, () => window.__lab.route.pause());
  await page.waitForFunction(() => window.__lab?.route?.status().status === "paused");
  await advance(page, 90 - (await obs(page)).tick);
  await presented(page);
  await Promise.all(details);
  const initialWork = timeline.find((e) => e.kind === "scene installed")?.detail?.work;
  const initialNames = await snapshots.at(-1);
  console.log("battle-growth admission work", JSON.stringify(initialWork));
  // The first scene is built: from here on, only growth, its art held.
  timeline.length = 0;
  snapshots.length = 0;
  details.length = 0;
  network.start();

  // Recon's longer sight reaches the firing vehicles before they can kill a
  // short-sighted tank on the approach. This fixture is a meeting, not an assault.
  const scoutId = await approachContact(page);
  // Stop at the first held growth, rather than fast-forwarding through a
  // whole minute in which a second kind could arrive as well. Staging waits
  // on the art, so a held growth shows as blocks before it logs anything.
  let meeting = await obs(page);
  let unit = null;
  for (let ticks = 0; ticks < ROUNDS * 60 * 30; ticks += 30) {
    await advance(page, 30);
    await presented(page);
    meeting = await obs(page);
    unit = network.held.length ? await heldUnit(page, meeting) : null;
    if (unit) break;
    await followEnemyObjective(page, meeting, scoutId);
  }
  const committed = () => {
    const lastStage = timeline.map((e) => e.kind).lastIndexOf("battle staged");
    return lastStage >= 0 && timeline.slice(lastStage).some((e) => e.kind === "art committed");
  };
  const attacker = meeting.own.find((u) => u.id === scoutId);
  ctx.check(
    "the held growth draws a seen enemy unit as stand-ins, the scout alive",
    !!unit && !!attacker,
    JSON.stringify({
      tick: meeting.tick,
      held: network.held.length,
      identified: meeting.identified,
      own: meeting.own,
    }),
  );
  if (unit && attacker)
    await judgeStandIns(ctx, page, { unit, attacker, release: network.release, committed });
  // Released, its art is admitted while the scene keeps drawing.
  await network.release();
  const released = Date.now();
  while (!committed() && Date.now() - released < 60_000) await page.waitForTimeout(250);
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
  if (!names) {
    const observation = await obs(page);
    ctx.check(
      stages.length ? "the staged battle never committed its growth" : "the battle never grew",
      false,
      JSON.stringify({
        timeline,
        tick: observation.tick,
        own: observation.own,
        contacts: observation.contacts,
      }),
    );
    page.off("console", record);
    await Promise.all(details);
    await page.close();
    return;
  }
  const catalog = JSON.parse(
    await readFile(new URL("../../assets/runtime/catalog.json", import.meta.url), "utf8"),
  );
  const art = await growthArt(page, catalog, initialNames, names);
  const { newContents, newFiles, vertexBytes } = art;
  for (const commit of commits) {
    const work = commit.detail?.work;
    console.log("battle-growth admission work", JSON.stringify(work));
    ctx.check(
      "growth encodes and decodes exactly its new contents",
      newContents > 0 &&
        work?.page?.bundlesEncoded === newContents &&
        work?.core?.bundlesDecoded === newContents,
      JSON.stringify({ added: art.added, work }),
    );
    ctx.check(
      "growth packs the meshes of its new art",
      vertexBytes > 0 && work?.core?.vertexBytesPacked === vertexBytes,
      JSON.stringify({ vertexBytes, core: work?.core }),
    );
    ctx.check(
      "growth carries scenery without placement or upload",
      work?.core?.sceneryPlacements === 0 && work?.core?.staticsBytesUploaded === 0,
      JSON.stringify(work?.core),
    );
    ctx.check(
      "growth installs once and reads only its missing files",
      work?.page?.appearanceLibraryInstalls === 1 &&
        work?.page?.runtimeContentsInflated <= newFiles,
      JSON.stringify({ newFiles: newFiles, page: work?.page }),
    );
    ctx.check(
      "growth derives rest facts only for its new content and poses",
      work?.core?.restPosesDerived === art.restPoses,
      JSON.stringify({ restPoses: art.restPoses, core: work?.core }),
    );
    ctx.check(
      "growth writes only new referenced textures without rebuilding arrays",
      work?.core?.textureLayersWritten === art.textureLayers &&
        work?.core?.textureArraysRebuilt === 0,
      JSON.stringify({ textureLayers: art.textureLayers, core: work?.core }),
    );
    ctx.check(
      "growth bakes only new runtime card keys",
      work?.core?.cardsBaked === art.cards && (art.cards > 0 || work?.core?.cardReadWaits === 0),
      JSON.stringify({ cards: art.cards, core: work?.core }),
    );
  }
  // A loose tripwire at 700 ms. Work counts above are the performance contract.
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
  page.off("console", record);
  await Promise.all(details);
  await page.close();
}
