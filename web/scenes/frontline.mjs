import { mkdir, writeFile } from "node:fs/promises";
import { capture, evidence, onlinePage, roomServer, sizes } from "./_rooms.mjs";
import { spills } from "./_spills.mjs";
import { purchase } from "./_battlePurchases.mjs";

const observe = (page) => page.evaluate(() => window.__lab?.route?.observation?.());
async function cardShots(ctx, page, folder, name) {
  for (const size of sizes) {
    await page.setViewportSize(size);
    const card = page.getByRole("region", { name: "Battle briefing" });
    const geometry = await card.evaluate((element) => {
      const r = element.getBoundingClientRect();
      const boundary = element.parentElement.getBoundingClientRect();
      return {
        boundary: {
          top: boundary.top,
          bottom: boundary.bottom,
          left: boundary.left,
          right: boundary.right,
        },
        top: r.top,
        bottom: r.bottom,
        left: r.left,
        right: r.right,
        height: r.height,
        windowHeight: innerHeight,
        windowWidth: innerWidth,
      };
    });
    const overflow = await spills(page, '[aria-label="Battle briefing"]');
    ctx.check(
      `${name}: the card holds all its content at ${size.width}`,
      overflow.length === 0 &&
        geometry.top >= 0 &&
        geometry.bottom <= geometry.windowHeight &&
        geometry.left >= 0 &&
        geometry.right <= geometry.windowWidth &&
        geometry.top >= geometry.boundary.top &&
        geometry.bottom <= geometry.boundary.bottom &&
        geometry.left >= geometry.boundary.left &&
        geometry.right <= geometry.boundary.right &&
        geometry.height > 200,
      JSON.stringify({ geometry, overflow }),
    );
    const headings = await card
      .locator(
        'h3, [aria-label="Assignment"], [aria-label="Battle score"], [aria-label="Commanders"], [aria-label="Deployment"]',
      )
      .evaluateAll((elements) =>
        elements.map((e) => {
          const r = e.getBoundingClientRect();
          return { y: r.y, bottom: r.bottom };
        }),
      );
    ctx.check(
      `${name}: assignment, score, roster and Deploy read in order at ${size.width}`,
      headings.every((box, i) => i === 0 || box.y >= headings[i - 1].bottom),
      JSON.stringify(headings),
    );
    await writeFile(
      `${folder}${name}-${size.width}x${size.height}-landmarks.json`,
      JSON.stringify({ geometry, headings }, null, 2),
    );
    await page.mouse.move(0, 0);
    await page.screenshot({
      path: `${folder}${name}-${size.width}x${size.height}.png`,
      animations: "disabled",
      caret: "hide",
    });
  }
  await page.setViewportSize(sizes[0]);
}

export async function run(ctx) {
  const folder = evidence("frontline");
  await mkdir(folder, { recursive: true });
  const local = await roomServer();
  try {
    const { page } = await onlinePage(ctx, local, "VIPER-41");
    await capture(page, folder, "multiplayer");
    // Inspect the small JSON reply as the page receives it. A long map draw
    // plus background art downloads can evict an old request's body from
    // Chromium's inspector cache before Playwright asks for it.
    await page.evaluate(() => {
      const original = window.fetch.bind(window);
      window.fetch = async (input, init) => {
        const response = await original(input, init);
        const kind = String(input).endsWith("/frontline/join")
          ? "join"
          : String(input).endsWith("/deploy")
            ? "deploy"
            : null;
        if (kind)
          void response
            .clone()
            .json()
            .then((value) => {
              window.__frontlineReply = { kind, value };
            });
        return response;
      };
    });
    console.log("      Frontline: Play; the server is preparing a real game");
    let streams = 0;
    page.on("request", (request) => {
      if (/\/battles\/[^/]+\/stream/.test(request.url())) streams++;
    });
    let briefing;
    for (let attempt = 0; attempt < 3; attempt++) {
      await page.evaluate(() => {
        window.__frontlineReply = null;
      });
      await page.getByRole("button", { name: "Play", exact: true }).click();
      ctx.check(
        "Play stays busy while the server prepares a new game",
        await page.getByRole("button", { name: "Finding a battle…" }).isDisabled(),
      );
      await page.waitForFunction(() => window.__frontlineReply?.kind === "join", null, {
        timeout: 240_000,
      });
      briefing = await page.evaluate(() => window.__frontlineReply.value);
      if (!briefing.refused) break;
      if (briefing.refused !== "unavailable")
        throw new Error(`Frontline join refused: ${briefing.refused}`);
      console.log("      Frontline: drawn maps refused; retrying Play");
    }
    if (briefing.refused) throw new Error("No battlefield admitted after bounded Play attempts");
    await page.getByRole("region", { name: "Battle briefing" }).waitFor();
    ctx.check(
      "Deploy is shut while the browser prepares its map",
      await page.getByRole("button", { name: "Deploy", exact: true }).isDisabled(),
    );
    await cardShots(ctx, page, folder, "fresh-preparing");
    await page.getByRole("button", { name: "Deploy", exact: true }).waitFor();
    await page.waitForFunction(
      (button) => !button.disabled,
      await page.getByRole("button", { name: "Deploy", exact: true }).elementHandle(),
      { timeout: 180_000 },
    );
    ctx.check("the briefing does not open a battle stream", streams === 0);
    await cardShots(ctx, page, folder, "fresh-ready");
    console.log("      Frontline: map matched; Deploy with snapshot deliberately delayed");
    await page.evaluate(() => {
      const original = window.fetch.bind(window);
      let release;
      const gate = new Promise((resolve) => {
        release = resolve;
      });
      window.__releaseSnapshot = () => release();
      window.fetch = async (input, init) => {
        const response = await original(input, init);
        if (!/\/battles\/[^/]+\/stream/.test(String(input))) return response;
        window.__snapshotWaiting = true;
        const reader = response.body.getReader();
        return new Response(
          new ReadableStream({
            async pull(controller) {
              await gate;
              const chunk = await reader.read();
              if (chunk.done) controller.close();
              else controller.enqueue(chunk.value);
            },
            cancel: () => reader.cancel(),
          }),
          { status: response.status, headers: response.headers },
        );
      };
    });
    await page.evaluate(() => {
      window.__frontlineReply = null;
    });
    await page.getByRole("button", { name: "Deploy", exact: true }).click();
    await page.waitForFunction(() => window.__frontlineReply?.kind === "deploy");
    const ticket = await page.evaluate(() => window.__frontlineReply.value);
    await page.waitForFunction(() => window.__snapshotWaiting, null, { timeout: 60_000 });
    ctx.check(
      "loading remains until the first snapshot arrives",
      (await page.getByTestId("loading").isVisible()) && !(await observe(page)),
    );
    await page.screenshot({ path: `${folder}joining-snapshot-1280x720.png` });
    await page.evaluate(() => window.__releaseSnapshot());
    await page.getByTestId("loading").waitFor({ state: "hidden", timeout: 180_000 });
    const start = await observe(page);
    const economy = start.skirmish.players.find((p) => p.id === ticket.player);
    ctx.check(
      "the opening wallet agrees with the briefing",
      economy.credits >= briefing.credits,
      JSON.stringify({ shown: briefing.credits, received: economy.credits }),
    );
    ctx.check(
      "the arriving commander owns no units",
      start.own.filter((u) => u.owner === economy.index).length === 0,
    );
    console.log("      Frontline: in battle; first purchase through the real picker");
    // A destination inland leaves a visible road-in after dispatch; buying
    // on the entry itself can finish before the first observed unit frame.
    const destination = await page.evaluate(() => {
      const { start, size } = window.__lab.route.prepared();
      const center = size.map((extent) => extent / 2);
      const distance = Math.hypot(center[0] - start.at[0], center[1] - start.at[1]);
      return start.at.map((value, i) => value + (center[i] - value) * Math.min(1, 200 / distance));
    });
    await purchase(page, ticket, destination);
    const ready = page.getByRole("button", { name: "START BATTLE", exact: true });
    if (await ready.count()) await ready.click();
    await page.waitForFunction(
      (owner) => window.__lab.route.observation().own.some((u) => u.owner === owner),
      economy.index,
      { timeout: 120_000 },
    );
    const first = (await observe(page)).own.find((u) => u.owner === economy.index);
    const before = first.position;
    await page.waitForFunction(
      ({ owner, id, before }) => {
        const unit = window.__lab.route
          .observation()
          .own.find((u) => u.owner === owner && u.id === id);
        return unit && Math.hypot(unit.position[0] - before[0], unit.position[1] - before[1]) > 2;
      },
      { owner: economy.index, id: first.id, before },
      { timeout: 30_000 },
    );
    ctx.check("the first purchase drives in", true);
    for (const name of ["Your victory score", "Enemy victory score"]) {
      const shown = await page.getByLabel(name, { exact: true }).getAttribute("aria-valuemax");
      ctx.check(
        `${name}: the battle uses the briefing's actual target`,
        Number(shown) === briefing.target,
        shown,
      );
    }
    for (const size of sizes) {
      await page.setViewportSize(size);
      await page.evaluate((id) => {
        const unit = window.__lab.route.observation().own.find((u) => u.id === id);
        window.__lab.route.select([id]);
        const [x, y] = unit.position;
        const camera = window.__lab.camera();
        window.__lab.setCamera({
          ...camera,
          target: [x, y, window.__lab.route.surfaceZ(x, y)],
          distance: 160,
        });
      }, first.id);
      await page.mouse.move(0, 0);
      await page.evaluate(
        () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))),
      );
      await page.screenshot({ path: `${folder}first-purchase-${size.width}x${size.height}.png` });
    }
    await page.setViewportSize(sizes[0]);
    await page.getByRole("button", { name: "Menu", exact: true }).click();
    await page.getByRole("button", { name: "Multiplayer", exact: true }).click();
    const leaving = page.waitForResponse((r) => r.url().endsWith("/leave"));
    await page.getByRole("button", { name: "Leave battle", exact: true }).click();
    await leaving;
    await page.getByRole("region", { name: "Frontline" }).waitFor();
    ctx.check(
      "the battle menu returns the seat and opens Multiplayer",
      await page.getByRole("button", { name: "Play", exact: true }).isVisible(),
    );
    // Only the late-game card's wire data is synthetic: waiting for a real
    // hour would prove nothing about its reading order. Map work stays real.
    const late = {
      ...briefing,
      minutesIn: 38,
      scores: [Math.round(briefing.target * 0.46), Math.round(briefing.target * 0.53)],
      // The player keeps their own seat; other commanders take free ones.
      roster: (() => {
        const others = { [briefing.side]: ["MAKO", "IRONBARK"] };
        return briefing.roster.map((seat) => ({
          ...seat,
          callsign: seat.callsign ?? (others[seat.side] ??= ["LENA#2"]).shift() ?? null,
        }));
      })(),
    };
    await page.route("**/frontline/join", (route) => route.fulfill({ json: late }));
    await page.getByRole("button", { name: "Play", exact: true }).click();
    await page.getByRole("region", { name: "Battle briefing" }).waitFor();
    await page.waitForFunction(
      (button) => !button.disabled,
      await page.getByRole("button", { name: "Deploy", exact: true }).elementHandle(),
      { timeout: 180_000 },
    );
    await cardShots(ctx, page, folder, "late-ready");
    await writeFile(`${folder}briefings.json`, JSON.stringify({ fresh: briefing, late }, null, 2));
    await page.getByRole("button", { name: "Back to Multiplayer" }).click();
    await page.close();
  } finally {
    await local.stop(folder);
  }
}
