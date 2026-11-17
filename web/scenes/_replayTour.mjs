import { advance, lab, openMenu } from "./_lab.mjs";

/** Save and watch the exact battle, including its human camera. */
export async function replayTour(ctx, page, generated, playable) {
  // The battle saved and watched: exact compiled scenario bytes restore the
  // map, encounter and rules without regenerating from the request.
  const saved = await lab(page, () => ({
    tick: window.__lab.route.tick(),
    digest: window.__lab.route.digest(),
  }));
  const file = await lab(page, () => window.__lab.route.exportReplay());
  ctx.check(
    "a human's saved battle includes their recorded camera",
    file.camera?.samples.length > 0 &&
      file.battle.report.request.players.some(
        (p) => p.id === file.camera.player && p.controller === "human",
      ),
    JSON.stringify({ player: file.camera?.player, samples: file.camera?.samples.length }),
  );
  await page.goto(ctx.urlFor({ replay: "saved" }));
  await playable(page);
  await lab(page, () => window.__lab.route.pause());
  await page.waitForFunction(() => window.__lab.route.status().status === "paused");
  await advance(page, saved.tick - (await lab(page, () => window.__lab.route.tick())));
  const replayed = await lab(page, () => ({
    tick: window.__lab.route.tick(),
    digest: window.__lab.route.digest(),
  }));
  const watched = await lab(page, () => window.__lab.route.prepared());
  const following = page.getByRole("button", { name: "Following player camera", exact: true });
  ctx.check(
    "replays follow the human camera by default",
    (await following.getAttribute("aria-pressed")) === "true",
  );
  await lab(page, async () => {
    await window.__lab.frame();
    await window.__lab.frame();
  });
  const camera = await lab(page, () => window.__lab.camera());
  const recorded = file.camera.samples.at(-1).camera;
  ctx.check(
    "the replay reaches the human's recorded framing",
    camera.target.every((v, i) => Math.abs(v - recorded.target[i]) < 0.01) &&
      Math.abs(camera.distance - recorded.distance) < 0.01,
    JSON.stringify({ camera, recorded }),
  );
  await following.click();
  await lab(page, () =>
    window.__lab.setCamera({ ...window.__lab.camera(), target: [100, 100, 0] }),
  );
  await lab(page, async () => {
    await window.__lab.frame();
    await window.__lab.frame();
  });
  const freeCamera = await lab(page, () => window.__lab.camera());
  ctx.check(
    "releasing the recorded camera lets the viewer look around",
    freeCamera.target[0] === 100 && freeCamera.target[1] === 100,
    JSON.stringify(freeCamera),
  );
  ctx.check(
    "a saved battle retains its compiled scenario and reaches the same replay digest",
    JSON.stringify(file.battle.report.request) === JSON.stringify(generated.request) &&
      typeof file.battle.scenario === "string" &&
      watched.identity.generation.map_hash === generated.generation.map_hash &&
      !!saved.digest &&
      replayed.tick === saved.tick &&
      replayed.digest === saved.digest,
    JSON.stringify({ saved, replayed }),
  );
  // The same captured scenario and commands on another engine build are refused.
  const stale = structuredClone(file);
  const commands = JSON.parse(stale.replay);
  commands.engine_build = "another-build";
  stale.replay = JSON.stringify(commands);
  await openMenu(page);
  await page.getByTestId("replay-file").setInputFiles({
    name: "stale.json",
    mimeType: "application/json",
    buffer: Buffer.from(JSON.stringify(stale)),
  });
  await page.getByTestId("error").waitFor();
  const staleMessage = await page.getByTestId("error").textContent();
  ctx.check(
    "a replay saved by another engine build is refused, saying so",
    /build/.test(staleMessage),
    staleMessage,
  );
}
