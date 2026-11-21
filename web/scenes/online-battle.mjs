import { purchase } from "./_battlePurchases.mjs";
// Two real pages, two credentials, one battle process.
import { mkdir, writeFile } from "node:fs/promises";
import { captureSeats, resultColours } from "./_battle-colours.mjs";
import { openingView, screenControls, screenUpOrder } from "./_battleOrientation.mjs";
import { capture, onlinePage, roomServer } from "./_rooms.mjs";
import { encodeRecord, jsonRecord } from "../../packages/multiplayer/src/stream/index.ts";

const evidence = new URL("../../throwaway/multiplayer/online-battle/", import.meta.url).pathname;
const observation = (page) => page.evaluate(() => window.__lab.route.observation());

async function controls(ctx, page, side) {
  const before = (await observation(page)).tick;
  const action = /^(?:pause|resume|restart)\b|speed|(?:export|save).*replay/i;
  const forbidden = page
    .getByRole("button", { name: action })
    .or(page.getByRole("link", { name: action }))
    .or(page.getByRole("slider", { name: /speed/i }))
    .or(page.getByRole("spinbutton", { name: /speed/i }))
    .or(page.getByRole("combobox", { name: /speed/i }));
  ctx.check(
    `${side}: the battle HUD offers no pause, speed or replay export`,
    (await forbidden.count()) === 0,
  );
  await page.getByRole("button", { name: "Menu", exact: true }).click();
  await page.getByRole("dialog", { name: "Menu", exact: true }).waitFor();
  ctx.check(
    `${side}: local authority controls cannot be found in the online menu`,
    (await forbidden.count()) === 0,
  );
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  // A seated player may save the live battle's replay (both sides' orders,
  // as the server's replay export gives them); the authority's own controls
  // stay offline-only.
  const authority = page
    .getByRole("button", { name: /^(?:pause|resume|restart)\b|speed/i })
    .or(page.getByRole("slider", { name: /speed/i }))
    .or(page.getByRole("spinbutton", { name: /speed/i }))
    .or(page.getByRole("combobox", { name: /speed/i }));
  ctx.check(
    `${side}: settings offer Save replay, and no pause, restart or speed`,
    (await authority.count()) === 0 &&
      (await page.getByRole("button", { name: "Save replay", exact: true }).count()) === 1,
  );
  await page.getByRole("button", { name: "Close settings", exact: true }).click();
  await page.waitForFunction((tick) => window.__lab.route.tick() > tick, before);
  ctx.check(
    `${side}: opening the menu leaves the battle running`,
    (await observation(page)).tick > before,
  );
  await page.getByRole("button", { name: "Back to battle", exact: true }).click();
}

async function orderMove(page, goal, fast = false) {
  const unit = (await observation(page)).own[0];
  const acknowledged = await page.evaluate(() =>
    Math.max(0, ...window.__lab.route.acks().map((entry) => entry.seq)),
  );
  await page.evaluate(
    ({ id, goal }) => {
      window.__lab.route.select([id]);
      const camera = window.__lab.camera();
      window.__lab.setCamera({ ...camera, target: [...goal, 0], distance: 220 });
    },
    { id: unit.id, goal },
  );
  await page.waitForTimeout(200);
  const css = await page.evaluate(
    ([x, y]) => window.__lab.projectToCss(x, y, window.__lab.route.surfaceZ(x, y)),
    goal,
  );
  if (!css) throw new Error("move destination is not drawn");
  if (fast) await page.mouse.dblclick(css[0], css[1], { button: "right" });
  else await page.mouse.click(css[0], css[1], { button: "right" });
  await page.waitForFunction(
    (before) =>
      window.__lab.route
        .acks()
        .some(
          ({ seq, order, ack }) =>
            seq > before && order.kind === "move" && !ack.error && ack.refused.length === 0,
        ),
    acknowledged,
  );
  return unit;
}

async function move(page, goal) {
  const unit = await orderMove(page, goal);
  await page.waitForFunction(
    ({ id, position, goal }) => {
      const u = window.__lab.route.observation().own.find((u) => u.id === id);
      const before = Math.hypot(goal[0] - position[0], goal[1] - position[1]);
      return u && Math.hypot(goal[0] - u.position[0], goal[1] - u.position[1]) < before * 0.75;
    },
    { id: unit.id, position: unit.position, goal },
    { timeout: 60_000 },
  );
}

export async function run(ctx) {
  await mkdir(evidence, { recursive: true });
  const colours = new URL("../../throwaway/multiplayer/online-battle/colours/", import.meta.url)
    .pathname;
  const captures = process.env.SLICE11_BEFORE ? `${colours}before/` : colours;
  await mkdir(captures, { recursive: true });
  await resultColours(ctx, new URL(ctx.url).origin, captures);
  const local = await roomServer();
  const base = `${new URL(ctx.url).origin}`;
  const onlineQuery = `?server=${encodeURIComponent(local.address)}&quality=${ctx.graphicsQuality}`;
  const pages = [];
  const refusedRequests = [];
  const trace = (page) => {
    page.on("response", (response) => {
      if (response.status() >= 400)
        refusedRequests.push({
          path: new URL(response.url()).pathname,
          method: response.request().method(),
          status: response.status(),
        });
    });
    return page;
  };
  try {
    const { page: host } = await onlinePage(ctx, local, "SCOUT");
    console.log("      online room: host at Multiplayer");
    pages.push(trace(host));
    // The harness can cut just this document's streams without adding any
    // disconnect control to the product or a second server-side timer.
    await host.evaluate(() => {
      const original = window.fetch.bind(window);
      const pending = new Set();
      let cut = false;
      window.fetch = (input, init = {}) => {
        if (!/\/(?:battles\/[^/]+\/stream|rooms\/[^/?]+(?:\?|$))/.test(String(input)))
          return original(input, init);
        if (cut) {
          const waiting = Promise.withResolvers();
          init.signal?.addEventListener("abort", () => waiting.reject(init.signal.reason), {
            once: true,
          });
          return waiting.promise;
        }
        const lifetime = new AbortController();
        pending.add(lifetime);
        return original(input, {
          ...init,
          signal: init.signal ? AbortSignal.any([init.signal, lifetime.signal]) : lifetime.signal,
        });
      };
      window.__cutBattleStreams = () => {
        cut = true;
        for (const stream of pending) stream.abort();
      };
    });
    await host.getByRole("button", { name: "Create room", exact: true }).click();
    const creation = host.waitForResponse(
      (r) => r.url().endsWith("/rooms") && r.request().method() === "POST",
    );
    await host.getByRole("button", { name: "Private room", exact: true }).click();
    await host.getByRole("region", { name: "Room lobby" }).waitFor();
    const made = await creation;
    const { code } = await made.json();
    const credential = (await made.request().allHeaders())["x-player"];
    const setting = await fetch(`${local.address}/rooms/${code}/act`, {
      method: "POST",
      headers: { "X-Player": credential, "Content-Type": "application/json" },
      body: JSON.stringify({ kind: "settings", patch: { map: { type: "open", seed: "1" } } }),
    });
    if (!(await setting.json()).ok) throw new Error("Fixture seed refused");
    const roomUrl = `${base}/room/${code}${onlineQuery}`;
    const { page: guest } = await onlinePage(ctx, local, "WING", `/room/${code}`);
    console.log("      online room: both commanders in the room");
    pages.push(trace(guest));
    for (const page of pages)
      await page.getByRole("button", { name: "Ready", exact: true }).click();
    await host.getByRole("button", { name: "Start", exact: true }).click();
    console.log("      online room: Start accepted, preparing");
    const tickets = [];
    for (const page of pages) {
      await page.waitForURL(/\/online\/battle\//, { timeout: 120_000 });
      await page.waitForFunction(() => window.__lab?.ready && window.__lab?.route?.prepared, null, {
        timeout: 120_000,
      });
      await page.getByTestId("loading").waitFor({ state: "hidden", timeout: 120_000 });
      tickets.push(await page.evaluate(() => history.state.usr.ticket));
    }
    for (let i = 0; i < pages.length; i++) {
      await openingView(ctx, pages[i], tickets[i].side, evidence);
      await screenControls(ctx, pages[i], tickets[i].side);
    }
    const mismatch = trace(await ctx.newPage());
    // This is the same member receiving a bad ticket, not a stranger. Its
    // room-presence stream may resume normally while map verification fails.
    await mismatch.addInitScript((credential) => {
      localStorage.setItem("battle.browser-id", credential);
      localStorage.setItem("battle.callsign", "SCOUT");
    }, credential);
    let streams = 0;
    mismatch.on("request", (request) => {
      if (/\/battles\/[^/]+\/stream/.test(request.url())) streams++;
    });
    // A malformed server ticket still travels the real room-to-page path.
    const firstView = {
      code,
      visibility: "private",
      seats: [],
      settings: {
        factions: tickets[0].setup.factions,
        map: tickets[0].setup.map,
        lighting: tickets[0].setup.lighting,
      },
      you: "seat-1",
      canStart: false,
      phase: { kind: "battle", ticket: { ...tickets[0], mapIdentity: "wrong-map" } },
    };
    await mismatch.route(`**/rooms/${code}`, (route) =>
      route.fulfill({
        contentType: "application/octet-stream",
        body: Buffer.from(encodeRecord(jsonRecord("state", firstView, 1, 1))),
      }),
    );
    await mismatch.goto(roomUrl);
    await mismatch.getByRole("alert").waitFor({ timeout: 120_000 });
    const details = mismatch.getByRole("button", { name: "Details", exact: true });
    await details.click();
    ctx.check(
      "a mismatched map refuses before any battle stream opens",
      streams === 0 && /local map differs/.test(await mismatch.getByRole("alert").textContent()),
      `streams opened: ${streams}`,
    );
    if (streams !== 0) throw new Error("A mismatched page opened a battle stream");
    await mismatch.close();
    ctx.check(
      "the first page commands Blue and the second commands Red in the same battle",
      tickets[0].side === "blue" &&
        tickets[1].side === "red" &&
        tickets[0].battle === tickets[1].battle,
    );
    await Promise.all(pages.map((page, i) => purchase(page, tickets[i])));
    await captureSeats(ctx, local, pages, "preparation", captures);
    for (const page of pages) {
      const ready = page.getByRole("button", { name: "START BATTLE", exact: true });
      if (await ready.isVisible()) await ready.click();
    }
    await Promise.all(
      pages.map((page) =>
        page.waitForFunction(
          () =>
            window.__lab.route.observation().own.length > 0 &&
            window.__lab.route.observation().skirmish.phase === "active",
          null,
          { timeout: 120_000 },
        ),
      ),
    );
    const initial = await Promise.all(pages.map(observation));
    for (let i = 0; i < pages.length; i++) {
      const o = initial[i];
      const ticket = tickets[i];
      const owner = o.skirmish.players.find((p) => p.id === ticket.player)?.index;
      ctx.check(
        `${ticket.side}: own records belong to its seated player, never the opposite side`,
        Number.isInteger(owner) &&
          o.groundPatch.side === ticket.side &&
          o.own.length > 0 &&
          o.own.every((u) => u.owner === owner),
      );
      ctx.check(
        `${ticket.side}: the enemy is absent while the two armies are out of sight`,
        o.identified.length === 0,
      );
      await controls(ctx, pages[i], ticket.side);
    }
    await Promise.all(
      pages.map((page, i) => screenUpOrder(ctx, page, tickets[i].side, move, evidence)),
    );
    const starts = await Promise.all(
      pages.map((page) => page.evaluate(() => window.__lab.route.prepared().start.at)),
    );
    ctx.check(
      "commanders open on different home roads",
      JSON.stringify(starts[0]) !== JSON.stringify(starts[1]),
    );
    // The map admits an objective's approaches for both sides. An arbitrary
    // midpoint can be inside a building and strands a scout short of sight.
    const middle = initial[0].skirmish.objectives[0].center;
    const objectives = initial.map(
      (o, i) =>
        [...o.skirmish.objectives].sort(
          (a, b) =>
            Math.hypot(a.center[0] - starts[i][0], a.center[1] - starts[i][1]) -
            Math.hypot(b.center[0] - starts[i][0], b.center[1] - starts[i][1]),
        )[0],
    );
    try {
      await Promise.all(pages.map((page, i) => orderMove(page, objectives[i].center, true)));
    } catch (error) {
      await ctx.writeEvidence("failed-objective-move.json", {
        objectives,
        observations: await Promise.all(pages.map(observation)),
      });
      throw error;
    }
    await pages[0].waitForTimeout(1500);
    await captureSeats(ctx, local, pages, "moving", captures);
    await Promise.all(
      pages.map((page, i) =>
        page.waitForFunction(
          (side) =>
            window.__lab.route
              .observation()
              .skirmish.objectives.some((o) => o.capturing === side || o.owner === side),
          tickets[i].side,
          { timeout: 300_000 },
        ),
      ),
    );
    await captureSeats(ctx, local, pages, "capture", captures);
    await Promise.all(
      pages.map((page, i) =>
        page.waitForFunction(
          (side) =>
            window.__lab.route.observation().skirmish.objectives.some((o) => o.owner === side),
          tickets[i].side,
          { timeout: 60_000 },
        ),
      ),
    );
    await captureSeats(ctx, local, pages, "held", captures);
    // Sight and reconnect are the subjects here. Both commanders issue the
    // ordinary Return fire order so a scout's death cannot erase the army
    // needed by the later reconnect and result checks.
    await Promise.all(
      pages.map(async (page) => {
        await page.getByRole("button", { name: /^Fire at will/ }).click();
        await page.waitForFunction(() =>
          window.__lab.route
            .observation()
            .own.every((unit) => unit.engagement === "return_fire_only"),
        );
      }),
    );
    // Both drive toward the same admitted meeting ground. The page issues
    // the order through picking and the ordinary right-click command path.
    await Promise.all(pages.map((page) => move(page, middle)));
    for (let i = 0; i < pages.length; i++) {
      ctx.check(
        `${tickets[i].side}: its right-click order moves its own unit`,
        (await observation(pages[i])).own.some(
          (u) =>
            Math.hypot(
              u.position[0] - initial[i].own[0].position[0],
              u.position[1] - initial[i].own[0].position[1],
            ) > 5,
        ),
      );
    }
    // The two armies are fighting by now, so what a page sees changes from
    // one frame to the next: the proof is taken in the frame where it holds.
    const sighted = await Promise.all(
      pages.map((page) =>
        page
          .waitForFunction(
            () => {
              const o = window.__lab.route.observation();
              const onSeenGround = o.identified.every((unit) => {
                const cell =
                  Math.floor(unit.position[1] / o.fog.cellM) * o.fog.nx +
                  Math.floor(unit.position[0] / o.fog.cellM);
                return !!(o.fog.bits[cell >>> 5] & (1 << (cell & 31)));
              });
              return o.identified.length > 0 && onSeenGround ? o.identified.length : false;
            },
            null,
            { timeout: 240_000 },
          )
          .then(
            async (count) => {
              await page.keyboard.press("Backspace");
              return count.jsonValue();
            },
            () => 0,
          ),
      ),
    );
    await captureSeats(ctx, local, pages, "contact", captures);
    for (let i = 0; i < pages.length; i++) {
      const page = pages[i];
      const side = tickets[i].side;
      ctx.check(
        `${side}: enemy records appear when the enemy enters its sight`,
        sighted[i] > 0,
        `${sighted[i]} identified, each on ground it sees`,
      );
      const o = await observation(page);
      await page.evaluate((at) => {
        const camera = window.__lab.camera();
        window.__lab.setCamera({ ...camera, target: at, distance: 200 });
      }, o.own[0].position);
      await page.waitForTimeout(500);
      ctx.check(
        `${side}: the running battlefield has no error overlay`,
        !(await page.getByRole("alert").isVisible()) &&
          !(await page.getByTestId("loading").isVisible()),
      );
      await page.screenshot({ path: `${evidence}${side}.png` });
    }
    // The AI that runs a dropped seat fights what its army sees, so Red pulls
    // back out of sight first: the reconnect checks need both armies whole.
    await orderMove(pages[1], starts[1], true);
    await pages[1].waitForFunction(
      () => window.__lab.route.observation().identified.length === 0,
      null,
      { timeout: 120_000 },
    );
    // Keep every combat and orientation check above. Then actually close a
    // commander's document: the only disconnect clock is server presence.
    const storage = await pages[0].context().storageState();
    const beforeDrop = await observation(pages[1]);
    const departingArmy = await observation(pages[0]);
    await pages[0].evaluate(() => window.__cutBattleStreams());
    await pages[1].waitForTimeout(6000);
    const handedOver = await (
      await fetch(`${tickets[0].address ?? local.address}/battles/${tickets[0].battle}/up`, {
        method: "POST",
        headers: { "X-Player": credential, "Content-Type": "application/json" },
        body: JSON.stringify({
          seq: Date.now(),
          items: [{ id: 1, order: { kind: "ready" }, queued: false }],
        }),
      })
    ).json();
    ctx.check(
      "a disconnected commander's seat is run by AI after the presence grace",
      handedOver.results?.[0]?.ack.error?.reason === "ai_controlled",
      JSON.stringify(handedOver),
    );
    for (const [i, page] of pages.entries())
      await page.screenshot({ path: `${evidence}${tickets[i].side}-after-drop.png` });
    await pages[0].close();
    await pages[1].waitForTimeout(1000);
    ctx.check(
      "the surviving commander keeps playing after the other page closes",
      (await observation(pages[1])).tick > beforeDrop.tick,
    );
    const returned = trace(await ctx.newPage({ viewport: { width: 1280, height: 720 } }));
    await returned.addInitScript((storage) => {
      for (const entry of storage.origins.flatMap((o) => o.localStorage))
        localStorage.setItem(entry.name, entry.value);
    }, storage);
    // Reopening through the room restores the ticket and human controller.
    // Observe before the open restores it, using the surviving side if in sight.
    await returned.goto(roomUrl);
    await returned.waitForURL(/\/online\/battle\//, { timeout: 120_000 });
    await returned.waitForFunction(
      () => window.__lab?.ready && window.__lab?.route?.observation()?.own.length > 0,
      null,
      { timeout: 120_000 },
    );
    const returnTicket = await returned.evaluate(() => history.state.usr.ticket);
    ctx.check(
      "reopening the room link returns the same battle and seat",
      returnTicket.battle === tickets[0].battle && returnTicket.player === tickets[0].player,
    );
    pages[0] = returned;
    const inherited = await observation(returned);
    ctx.check(
      "the returning seat still has the army it dropped",
      inherited.own.some((u) => departingArmy.own.some((before) => before.id === u.id)),
    );
    await captureSeats(ctx, local, pages, "returned", evidence);
    // A new order must be applied after the AI's orders, not merely queued
    // behind an obsolete sequence number from before the drop.
    await orderMove(returned, objectives[0].center, true);
    ctx.check(
      "the returned commander can issue an applied order",
      (await returned.evaluate(() => window.__lab.route.acks())).some(
        ({ order, ack }) => order.kind === "move" && !ack.error && ack.refused.length === 0,
      ),
    );
    // A chosen departure takes one press. The battle goes on for the other
    // commander, so Multiplayer offers the way back to the same seat.
    await returned.getByRole("button", { name: "Menu", exact: true }).click();
    await returned.getByRole("dialog", { name: "Menu", exact: true }).waitFor();
    await returned.screenshot({ path: `${evidence}menu.png` });
    const told = returned.waitForResponse((r) => r.url().endsWith(`/rooms/${code}/act`));
    await returned.getByRole("button", { name: "Leave battle", exact: true }).click();
    await told;
    await returned.getByRole("group", { name: "Your battle" }).waitFor({ timeout: 60_000 });
    const left = await (
      await fetch(`${tickets[0].address ?? local.address}/battles/${tickets[0].battle}/up`, {
        method: "POST",
        headers: { "X-Player": credential, "Content-Type": "application/json" },
        body: JSON.stringify({
          seq: Date.now(),
          items: [{ id: 1, order: { kind: "ready" }, queued: false }],
        }),
      })
    ).json();
    ctx.check(
      "leaving by the menu hands the seat to its AI at once and offers the way back",
      new URL(returned.url()).pathname === "/multiplayer" &&
        left.results?.[0]?.ack.error?.reason === "ai_controlled",
      JSON.stringify(left),
    );
    await capture(returned, evidence, "rejoin", false);
    await returned.getByRole("button", { name: "Rejoin battle", exact: true }).click();
    await returned.waitForURL(/\/online\/battle\//, { timeout: 120_000 });
    await returned.waitForFunction(
      () => window.__lab?.ready && window.__lab?.route?.observation()?.own.length > 0,
      null,
      { timeout: 120_000 },
    );
    ctx.check(
      "rejoining returns to the same battle and seat",
      (await returned.evaluate(() => history.state.usr.ticket.player)) === tickets[0].player &&
        (await returned.evaluate(() => history.state.usr.ticket.battle)) === tickets[0].battle,
    );
    await orderMove(returned, objectives[0].center, true);
    console.log("      online room: returned, taking every objective for the normal skirmish end");
    // Keep Red out of the capture route. Blue wins by taking every objective
    // under the unmodified skirmish rules, exactly as at the service seam.
    await orderMove(pages[1], starts[1], true);
    for (const objective of initial[0].skirmish.objectives) {
      await orderMove(returned, objective.center, true);
      await returned.waitForFunction(
        (center) => {
          const match = window.__lab.route.observation().skirmish;
          return (
            match.phase === "finished" ||
            match.objectives.find((o) => o.center[0] === center[0] && o.center[1] === center[1])
              ?.owner === "blue"
          );
        },
        objective.center,
        { timeout: 180_000 },
      );
      if ((await observation(returned)).skirmish.phase === "finished") break;
    }
    const final = [];
    for (const [i, page] of pages.entries()) {
      await page.getByRole("dialog", { name: "Match result" }).waitFor({ timeout: 180_000 });
      ctx.check(
        `${tickets[i].side}: the final observation shows both commanders' callsigns`,
        (await page
          .getByRole("row", { name: i === 0 ? "SCOUT (YOU)" : "SCOUT", exact: true })
          .isVisible()) &&
          (await page
            .getByRole("row", { name: i === 1 ? "WING (YOU)" : "WING", exact: true })
            .isVisible()),
      );
      await page.screenshot({ path: `${evidence}${tickets[i].side}-result.png` });
      final.push(await observation(page));
      await ctx.writeEvidence(`${tickets[i].side}-final.json`, final.at(-1));
      await page.getByRole("button", { name: "Return to Multiplayer", exact: true }).click();
      await page.getByRole("heading", { name: "Multiplayer", exact: true }).waitFor();
      ctx.check(
        `${tickets[i].side}: result returns to the multiplayer menu`,
        new URL(page.url()).pathname === "/multiplayer",
      );
    }

    await ctx.writeEvidence("observations.json", { initial, final });
    for (const page of pages) await page.close();
  } catch (error) {
    for (const [index, page] of pages.entries())
      if (!page.isClosed()) {
        await page.screenshot({ path: `${evidence}failure-${index}.png` }).catch(() => {});
        await ctx.writeEvidence(
          `failure-${index}.json`,
          await page
            .evaluate(() => ({
              observation: window.__lab?.route?.observation?.() ?? null,
              path: location.pathname,
              text: document.body.innerText,
            }))
            .catch(() => null),
        );
      }
    throw error;
  } finally {
    for (const page of pages) if (!page.isClosed()) await page.close();
    await local.stop(evidence);
    await writeFile(`${evidence}http-refusals.json`, JSON.stringify(refusedRequests, null, 2));
  }
}
