// The skirmish lobby at `/`, through its real previews: what it shows for
// each match, which changes draw a new map, the quiet retry and the failure,
// and that Deploy starts the map it showed. Each state is shot at 1280×720
// and 1920×1080 into the evidence folder; an XL 5v5 preview's preparation
// time is printed.
const VIEWPORTS = [
  { width: 1280, height: 720 },
  { width: 1920, height: 1080 },
];

/** On the menu, the first two preparation workers are stand-ins that refuse
 *  their map for a reason another seed can fix, as the generator does for
 *  some seeds; the second takes its time, so "retrying" can be seen. Every
 *  later worker is real. */
function refuseFirstPreviews() {
  if (location.pathname !== "/") return;
  const Real = Worker;
  let refused = 0;
  window.__refused = [];
  window.Worker = function (url, options) {
    if (refused === 2 || !String(url).includes("/prepare/worker")) return new Real(url, options);
    const delay = refused++ === 0 ? 0 : 2000;
    const stand = {
      onmessage: null,
      onerror: null,
      terminate() {},
      postMessage(message) {
        window.__refused.push(message.request.map_source.request.seed);
        setTimeout(
          () =>
            stand.onmessage?.({
              data: {
                type: "refused",
                stage: "map",
                diagnostics: [
                  { code: "generation_failed", feature: null, location: "$", message: "stand-in" },
                ],
              },
            }),
          delay,
        );
      },
    };
    return stand;
  };
}

const PREVIEW_TIMEOUT = 60_000;

/** The requests the page posts to its preparation workers, in order: the
 *  lobby's previews, and a battle page's own preparation. */
function recordPreviews() {
  window.__previews = [];
  window.__battles = [];
  const post = Worker.prototype.postMessage;
  Worker.prototype.postMessage = function (message, ...rest) {
    if (message?.type === "prepare")
      (message.preview ? window.__previews : window.__battles).push(message.request);
    return post.call(this, message, ...rest);
  };
}

/** A returning player's graphics, so no first-visit detection holds the menu. */
function seedGraphics() {
  localStorage.setItem(
    "battle.graphics",
    JSON.stringify({
      mode: "high",
      detectedMode: "high",
      lastResult: {
        averageFps: 60,
        onePercentLowFps: 60,
        selectedMode: "high",
      },
    }),
  );
}

export async function run(ctx) {
  const origin = new URL(ctx.url).origin;
  let page;
  /** Scene steps act on `next` from here on. */
  const use = async (next) => {
    page = next;
    await page.addInitScript(seedGraphics);
    await page.addInitScript(recordPreviews);
  };
  await use(await ctx.newPage({ viewport: VIEWPORTS[0] }));

  const status = () => page.getByTestId("lobby-status").getAttribute("data-status");
  const vs = () => page.getByTestId("lobby-vs").textContent();
  const seed = async () =>
    (await page.getByTestId("lobby-status").textContent())
      .match(/seed ([\d ]+)/)?.[1]
      .replaceAll(" ", "");
  const previews = () => page.evaluate(() => window.__previews.length);
  const settled = async (timeout = PREVIEW_TIMEOUT) => {
    await page.waitForFunction(
      () =>
        ["ready", "failed"].includes(
          document.querySelector("[data-testid=lobby-status]")?.dataset.status,
        ),
      null,
      { timeout },
    );
    return status();
  };
  const shoot = async (name) => {
    for (const viewport of VIEWPORTS) {
      await page.setViewportSize(viewport);
      await page.mouse.move(0, 0);
      await page.screenshot({
        path: ctx.evidencePath(`${name}-${viewport.width}x${viewport.height}.png`),
      });
    }
    await page.setViewportSize(VIEWPORTS[0]);
  };
  const open = async (search = "") => {
    await page.goto(`${origin}/${search}`);
    await page.locator("main.menu:not(.loading):not([inert])").waitFor({ timeout: 120_000 });
    await page.getByTestId("loading").waitFor({ state: "hidden", timeout: 120_000 });
    await page.evaluate(() => document.fonts.ready);
    if (!search) await page.getByRole("button", { name: "Skirmish", exact: true }).click();
  };

  // The default: a 1v1 whose map is coming from its first frame.
  await open();
  ctx.check(
    "the lobby shows its map coming from the first frame",
    (await status()) === "loading",
    await status(),
  );
  ctx.check(
    "Deploy waits for the map",
    (await page.getByTestId("menu-deploy").getAttribute("aria-disabled")) === "true",
  );
  await shoot("loading");
  ctx.check("the default 1v1 previews a map", (await settled()) === "ready", await status());
  ctx.check("the default is 1 VS 1", (await vs()) === "1 VS 1", await vs());
  await shoot("1v1");

  // A map input draws a new map; Reroll draws another seed.
  const first = await seed();
  await page.getByTestId("menu-map-open").click();
  ctx.check(
    "a new map type previews again",
    (await status()) !== "ready" && (await settled()) === "ready",
  );
  await page.getByTestId("lobby-reroll").click();
  ctx.check(
    "Reroll previews another seed",
    (await settled()) === "ready" && (await seed()) !== first,
    `${first} → ${await seed()}`,
  );

  // Presets, Add and Delete.
  await page.getByTestId("menu-match-3v3").click();
  ctx.check(
    "the 3v3 preset seats three a side on a Large map",
    (await vs()) === "3 VS 3" &&
      (await page.getByTestId("menu-size-large").getAttribute("aria-checked")) === "true",
  );
  ctx.check(
    "sizes below a 3v3's floor are closed, with why",
    (await page.getByTestId("menu-size-medium").isDisabled()) &&
      /too small for 3 a side/.test(await page.getByTestId("lobby-size-reason").textContent()),
  );
  await page.getByTestId("lobby-add-red").click();
  ctx.check("Add makes it 3 VS 4", (await vs()) === "3 VS 4");
  ctx.check("the 3v4 previews a map", (await settled()) === "ready", await status());
  const entries = await page.locator('[data-testid=lobby-map] [data-entry="blue"]').count();
  ctx.check(
    "the 3v4 map has three entries a side",
    entries === 3 &&
      (await page.locator('[data-testid=lobby-map] [data-entry="red"]').count()) === 3,
    String(entries),
  );
  await shoot("3v4");
  const drawn = await previews();
  await page.getByTestId("lobby-remove-blue-3").click();
  await page.getByTestId("lobby-remove-red-4").click();
  ctx.check("Delete makes it 2 VS 3", (await vs()) === "2 VS 3");
  // Taking another Blue seat, and dropping to 2v3 (still three entries a
  // side, still Large), keep the map.
  await page.getByTestId("lobby-controller-blue-2").click();
  await page.getByTestId("lobby-controller-blue-2-you").click();
  ctx.check(
    "you can take another Blue seat",
    (await page.locator('[data-player="blue-2"][data-you]').count()) === 1 &&
      (await page.locator("[data-you]").count()) === 1,
  );
  ctx.check(
    "seating and a 2v3 with the same entries keep the previewed map",
    (await previews()) === drawn && (await status()) === "ready",
    `${drawn} → ${await previews()} previews, ${await status()}`,
  );

  // 5v5: the size ratchets to XL; time its preview.
  await page.getByTestId("menu-match-5v5").click();
  const started = Date.now();
  ctx.check(
    "the 5v5 preset is XL",
    (await vs()) === "5 VS 5" &&
      (await page.getByTestId("menu-size-xl").getAttribute("aria-checked")) === "true",
  );
  await shoot("5v5-loading");
  const xl = await settled(120_000);
  const seconds = ((Date.now() - started) / 1000).toFixed(1);
  console.log(
    `      lobby: an XL 5v5 preview ${xl === "ready" ? "was ready" : "failed"} after ${seconds} s`,
  );
  ctx.check(
    "the 5v5 XL previews a map within the game's deadline",
    xl === "ready",
    `${xl} after ${seconds} s`,
  );
  await shoot("5v5");
  await page.getByTestId("menu-match-1v1").click();
  ctx.check(
    "back to 1v1 the map is Small again",
    (await page.getByTestId("menu-size-small").getAttribute("aria-checked")) === "true",
  );

  // A refused seed is redrawn quietly, and Deploy names the seed that won.
  await page.close();
  await use(await ctx.newPage({ viewport: VIEWPORTS[0] }));
  await page.addInitScript(refuseFirstPreviews);
  const warnings = [];
  page.on("console", (m) => m.type() === "warning" && warnings.push(m.text().slice(0, 300)));
  await open();
  let retried = false;
  try {
    await page
      .getByTestId("lobby-status")
      .and(page.locator('[data-status="retrying"]'))
      .waitFor({ timeout: PREVIEW_TIMEOUT });
    retried = true;
    await shoot("retrying");
  } catch {
    // Checked below.
  }
  ctx.check(
    "a refused seed retries quietly",
    retried &&
      (await settled()) === "ready" &&
      !(await page.evaluate(() => window.__refused)).includes(await seed()),
    `retried ${retried}, refused ${await page.evaluate(() => window.__refused)}, seed ${await seed()} ${warnings.join(" | ")}`,
  );

  // Deploy starts the very request the preview admitted.
  const admitted = await seed();
  const request = await page.evaluate(
    (seed) => window.__previews.findLast((r) => r.map_source.request.seed === seed),
    admitted,
  );
  await page.getByTestId("menu-deploy").click();
  await page.waitForFunction(
    () =>
      window.__lab?.ready || window.__lab?.error || document.querySelector("[data-testid=error]"),
    null,
    { timeout: 120_000 },
  );
  const battle = await page.evaluate(
    () =>
      (window.__lab?.ready && window.__battles.at(-1)) ||
      document.querySelector("[data-testid=error]")?.textContent,
  );
  ctx.check(
    "Deploy prepares the request the preview admitted",
    JSON.stringify(battle) === JSON.stringify(request),
    typeof battle === "string"
      ? battle
      : `${JSON.stringify(battle?.map_source?.request)} vs ${JSON.stringify(request?.map_source?.request)}`,
  );

  await page.close();

  // A map that cannot be made: the preparation worker never starts (its
  // load error is expected on this page).
  await use(await ctx.newPage({ viewport: VIEWPORTS[0], allowErrors: true }));
  await page.route("**/prepare/worker.ts*", (route) => route.abort());
  await open("?seed=5");
  ctx.check("a map that cannot be made says so", (await settled()) === "failed", await status());
  ctx.check(
    "Deploy stays shut without a map",
    (await page.getByTestId("menu-deploy").getAttribute("aria-disabled")) === "true",
  );
  await shoot("failed");
  await page.close();
}
