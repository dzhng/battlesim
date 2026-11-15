import { mkdir, writeFile } from "node:fs/promises";
import { capture, evidence, onlinePage, roomServer, sizes, uncovered } from "./_rooms.mjs";

const settled = async (page) => {
  await page.waitForFunction(
    () =>
      ["ready", "failed"].includes(
        document.querySelector("[data-testid=lobby-status]")?.dataset.status,
      ),
    null,
    { timeout: 120_000 },
  );
  if ((await page.getByTestId("lobby-status").getAttribute("data-status")) !== "ready")
    throw new Error(await page.getByTestId("lobby-map").textContent());
};
const roomViews = (page) => {
  const views = [];
  // Network observation is test-side only. No credential is copied to evidence.
  page.on("request", (r) => {
    if (r.url().endsWith("/act")) views.push(r.postDataJSON());
  });
  return views;
};
async function mapGeometry(page) {
  return page.getByTestId("lobby-map").evaluate((frame) => {
    const svg = frame.querySelector("svg").cloneNode(true);
    for (const tag of svg.querySelectorAll("[data-held]")) {
      tag.removeAttribute("data-held");
      tag.removeAttribute("aria-label");
      tag.removeAttribute("role");
    }
    const box = (element) => {
      const r = element.getBoundingClientRect();
      return [r.x, r.y, r.width, r.height];
    };
    return {
      svg: svg.outerHTML,
      map: box(frame),
      sides: [...document.querySelectorAll('[aria-label$=" side"]')].map(box),
    };
  });
}
async function laneAgreement(ctx, page, name) {
  const lanes = await page.getByRole("region", { name: "Room lobby" }).evaluate((lobby) =>
    ["blue", "red"].map((side) => {
      const rows = [...lobby.querySelectorAll(`.lobby-side[data-side="${side}"] li`)];
      const numbers = rows
        .filter((r) => r.dataset.kind !== "open")
        .map((r) => Number(r.querySelector(".lobby-index").textContent));
      const markers = [...lobby.querySelectorAll(`[data-entry="${side}"][data-lane]`)];
      const own = rows.find((r) => r.dataset.you);
      return {
        side,
        ready: lobby.querySelector("[data-testid=lobby-status]").dataset.status === "ready",
        numbers,
        opensUnnumbered: rows
          .filter((r) => r.dataset.kind === "open")
          .every((r) => !r.querySelector(".lobby-index").textContent),
        solid: markers
          .filter((m) => m.dataset.held !== "none")
          .map((m) => Number(m.dataset.lane))
          .sort((a, b) => a - b),
        hollow: markers.filter((m) => m.dataset.held === "none").map((m) => Number(m.dataset.lane)),
        halo: markers.filter((m) => m.dataset.held === "you").map((m) => Number(m.dataset.lane)),
        own: own ? Number(own.querySelector(".lobby-index").textContent) : null,
      };
    }),
  );
  ctx.check(
    `${name}: rows and solid lanes agree, open seats have no lane, your halo follows you`,
    lanes.every(
      (s) =>
        s.opensUnnumbered &&
        s.numbers.every((n, i) => n === i + 1) &&
        (!s.ready ||
          (JSON.stringify(s.numbers) === JSON.stringify(s.solid) &&
            s.hollow.every((n) => n > s.numbers.length) &&
            JSON.stringify(s.halo) === JSON.stringify(s.own === null ? [] : [s.own]))),
    ),
    JSON.stringify(lanes),
  );
}
async function commandGeometry(ctx, page, name) {
  const buttons = await page
    .getByRole("button", { name: /^(Reroll|Ready|Unready|Start)$/ })
    .evaluateAll((buttons) =>
      buttons.map((b) => {
        const r = b.getBoundingClientRect();
        const text = document.createRange();
        text.selectNodeContents(b);
        const t = text.getBoundingClientRect();
        return {
          height: r.height,
          bottom: r.bottom,
          textBottom: t.bottom,
          font: getComputedStyle(b).font,
        };
      }),
    );
  ctx.check(
    `${name}: room orders share height and baseline`,
    buttons.every(
      (b) =>
        Math.abs(b.height - buttons[0].height) < 0.5 &&
        Math.abs(b.bottom - buttons[0].bottom) < 0.5 &&
        Math.abs(b.textBottom - buttons[0].textBottom) < 0.5 &&
        b.font === buttons[0].font,
    ),
    JSON.stringify(buttons),
  );
}
async function openMirror(ctx, page, name) {
  const mirror = await page.evaluate(() => {
    const plate = document.querySelector(".menu-body").getBoundingClientRect();
    const rows = ["blue", "red"].map((side) =>
      document.querySelector(`.lobby-side[data-side="${side}"] [data-kind="open"]`),
    );
    if (rows.some((r) => !r)) return null;
    return {
      center: plate.left + plate.right,
      rows: rows.map((row) => {
        const r = row.getBoundingClientRect();
        // The row's word. A host has a picker there: its box is what
        // mirrors (its arrow follows the word on both sides). A guest has
        // the bare word of the sit target.
        const picker = row.querySelector(".lobby-pick > button");
        const text = document.createRange();
        text.selectNodeContents(row.querySelector(".online-sit"));
        const t = (picker ?? text).getBoundingClientRect();
        return { left: r.left, right: r.right, textLeft: t.left, textRight: t.right };
      }),
    };
  });
  if (!mirror) return;
  const [b, r] = mirror.rows;
  ctx.check(
    `${name}: Red's open row mirrors Blue about the plate centre`,
    Math.abs(b.left + r.right - mirror.center) < 0.5 &&
      Math.abs(b.right + r.left - mirror.center) < 0.5 &&
      Math.abs(b.textLeft + r.textRight - mirror.center) < 0.5,
    JSON.stringify(mirror),
  );
}
async function pair(ctx, host, guest, folder, name = "") {
  await Promise.all([uncovered(host), uncovered(guest), settled(host), settled(guest)]);
  for (const size of sizes) {
    await Promise.all([host.setViewportSize(size), guest.setViewportSize(size)]);
    for (const [role, page] of [
      ["host", host],
      ["guest", guest],
    ]) {
      await page.mouse.move(0, 0);
      await page.evaluate(
        () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))),
      );
      const label = `${role}${name ? `-${name}` : ""}-${size.width}x${size.height}`;
      await laneAgreement(ctx, page, label);
      await commandGeometry(ctx, page, label);
      await openMirror(ctx, page, label);
    }
    await Promise.all(
      [
        ["host", host],
        ["guest", guest],
      ].map(([role, page]) =>
        page.screenshot({
          path: `${folder}${role}${name ? `-${name}` : ""}-${size.width}x${size.height}.png`,
          animations: "disabled",
          caret: "hide",
        }),
      ),
    );
  }
  await Promise.all([host.setViewportSize(sizes[0]), guest.setViewportSize(sizes[0])]);
}
async function changeSetting(ctx, host, guest, guestRequests) {
  const choices = host.getByRole("radiogroup", { name: "map", exact: true }).getByRole("radio");
  const chosen = await choices.evaluateAll((buttons) =>
    buttons.findIndex((b) => b.getAttribute("aria-checked") !== "true"),
  );
  const guestActions = guestRequests.length;
  await choices.nth(chosen).click();
  await guest.waitForFunction(
    (index) =>
      document
        .querySelectorAll('[role=radiogroup][aria-labelledby="menu-map"] [role=radio]')
        [index]?.getAttribute("aria-checked") === "true",
    chosen,
  );
  await Promise.all([settled(host), settled(guest)]);
  ctx.check(
    "the guest sees the host's changed setting without sending an action",
    guestRequests.length === guestActions,
  );
}
/** A map the generator admits at this room's size, so the picture is the
 *  same every run; a room's own random seed may be refused. */
const ROOM_SEED = "77";

/** The host chooses what holds a seat nobody sits in, from its picker. */
async function setSeat(host, seat, to) {
  await host.getByRole("button", { name: `${seat} seat`, exact: true }).click();
  await host.getByRole("option", { name: to, exact: true }).click();
}

export async function run(ctx) {
  const folder = evidence("room-lobby");
  await mkdir(folder, { recursive: true });
  const local = await roomServer();
  const pages = [];
  try {
    const { page: host } = await onlinePage(ctx, local, "VIPER-41");
    pages.push(host);
    console.log("      room lobby: host connected");
    const hostRequests = roomViews(host);
    await host.getByRole("button", { name: "Create room", exact: true }).click();
    const creation = host.waitForResponse(
      (response) => response.url().endsWith("/rooms") && response.request().method() === "POST",
    );
    await host.getByRole("button", { name: "Private room", exact: true }).click();
    await host.getByRole("region", { name: "Room lobby" }).waitFor();
    console.log("      room lobby: host entered room");
    await capture(host, folder, "host-entered", false);
    await laneAgreement(ctx, host, "host-entered");
    const path = new URL(host.url()).pathname;
    const created = await creation;
    const code = (await created.json()).code;
    const who = (await created.request().allHeaders())["x-player"];
    const setting = await local.post(`/rooms/${code}/act`, who, {
      kind: "settings",
      patch: { map: { seed: ROOM_SEED } },
    });
    if (!setting.ok) throw new Error("Host fixture seed refused");
    const { page: guest } = await onlinePage(ctx, local, "LENA", path);
    pages.push(guest);
    console.log("      room lobby: guest connected");
    const guestRequests = roomViews(guest);
    await guest.getByRole("region", { name: "Room lobby" }).waitFor();
    await guest.getByTestId("loading").waitFor({ state: "hidden", timeout: 120_000 });
    console.log("      room lobby: guest entered room");
    await capture(host, folder, "host-after-guest", false);
    await laneAgreement(ctx, host, "host-after-guest");
    const hostBlue = host.getByRole("group", { name: "Blue side" });
    const hostRed = host.getByRole("group", { name: "Red side" });
    await hostBlue.getByRole("button", { name: /Add seat/ }).click();
    console.log("      room lobby: added Blue seat");
    await setSeat(host, "Blue 2", "open");
    console.log("      room lobby: opened Blue seat");
    await hostRed.getByRole("button", { name: /Add seat/ }).click();
    console.log("      room lobby: added Red AI");
    await hostRed.getByRole("button", { name: "Remove Red 2", exact: true }).waitFor();
    await guest.getByRole("button", { name: "Ready", exact: true }).click();
    await host.getByRole("status", { name: "LENA ready" }).waitFor();
    ctx.check(
      "the host sees the guest ready beside an open seat and an AI",
      (await host.getByRole("button", { name: "Sit Blue 2" }).isVisible()) &&
        (await host.getByRole("button", { name: "Red 2 seat", exact: true }).isVisible()),
    );
    await settled(host);
    await settled(guest);
    ctx.check(
      "host and guest share the same map geometry and fixed sides",
      JSON.stringify(await mapGeometry(host)) === JSON.stringify(await mapGeometry(guest)),
    );
    // Both browsers are captured without actions between them at each revision.
    await pair(ctx, host, guest, folder);
    // Show matching open rows on both sides without adding a fighter that
    // would ratchet the map to the next size tier.
    await setSeat(host, "Red 2", "open");
    await guest.getByRole("button", { name: "Sit Red 2", exact: true }).waitFor();
    await pair(ctx, host, guest, folder, "open-seats");
    await setSeat(host, "Red 2", "ai");
    await guest
      .getByRole("button", { name: "Sit Red 2", exact: true })
      .waitFor({ state: "hidden" });
    const guestSettingsActions = guestRequests.length;
    const settingsBefore = await guest
      .getByRole("radio")
      .evaluateAll((buttons) => buttons.map((b) => b.getAttribute("aria-checked")));
    for (const radio of await guest.getByRole("radio").all()) await radio.dispatchEvent("click");
    for (const label of ["Blue faction", "Red faction", "region", "lighting"]) {
      const pick = guest.getByRole("button", { name: label, exact: true });
      if (await pick.count()) await pick.dispatchEvent("click");
    }
    ctx.check(
      "guest settings send nothing and leave the screen unchanged",
      (await guest.getByRole("listbox").count()) === 0 &&
        guestRequests.length === guestSettingsActions &&
        JSON.stringify(settingsBefore) ===
          JSON.stringify(
            await guest
              .getByRole("radio")
              .evaluateAll((buttons) => buttons.map((b) => b.getAttribute("aria-checked"))),
          ),
      JSON.stringify({
        before: settingsBefore,
        after: await guest
          .getByRole("radio")
          .evaluateAll((buttons) => buttons.map((b) => b.getAttribute("aria-checked"))),
        requestsBefore: guestSettingsActions,
        requestsAfter: guestRequests.length,
      }),
    );
    // The plain setting-change case is captured before moving the guest.
    await changeSetting(ctx, host, guest, guestRequests);
    await pair(ctx, host, guest, folder, "setting-changed");
    const before = await mapGeometry(guest);
    await guest.getByRole("button", { name: "Sit Blue 2", exact: true }).click();
    await guest
      .getByRole("group", { name: "Blue side" })
      .getByText("LENA · you", { exact: true })
      .waitFor();
    const after = await mapGeometry(guest);
    ctx.check(
      "changing side keeps the map geometry and both columns fixed",
      JSON.stringify(before) === JSON.stringify(after),
      JSON.stringify({
        before: { map: before.map, sides: before.sides },
        after: { map: after.map, sides: after.sides },
      }),
    );
    await host
      .getByRole("group", { name: "Blue side" })
      .getByText("LENA", { exact: true })
      .waitFor();
    await pair(ctx, host, guest, folder, "moved");
    await changeSetting(ctx, host, guest, guestRequests);
    await pair(ctx, host, guest, folder, "moved-setting-changed");
    await setSeat(host, "Red 1", "ai");
    await guest.getByRole("button", { name: "Ready", exact: true }).click();
    const start = host.getByRole("button", { name: "Start", exact: true });
    ctx.check("the host's Start waits for its own ready mark", await start.isDisabled());
    await host.getByRole("button", { name: "Ready", exact: true }).click();
    await host.waitForFunction(
      () =>
        ![...document.querySelectorAll("button")].find((b) => b.textContent === "Start").disabled,
    );
    ctx.check("the host's Start lights after both humans ready", await start.isEnabled());
    await pair(ctx, host, guest, folder, "ready-to-start");
    await start.click();
    await Promise.all([
      host.getByRole("status", { name: "Starting", exact: true }).waitFor(),
      guest.getByRole("status", { name: "Starting", exact: true }).waitFor(),
    ]);
    ctx.check(
      "Start sends its action and both pages stop at starting",
      hostRequests.at(-1).kind === "start" &&
        new URL(host.url()).pathname === path &&
        new URL(guest.url()).pathname === path,
    );
    await capture(host, folder, "starting");
    await host.close();
    await guest.close();
  } catch (error) {
    for (const [index, page] of pages.entries())
      if (!page.isClosed()) {
        await page.screenshot({ path: `${folder}failure-${index}.png` }).catch(() => {});
        await writeFile(
          `${folder}failure-${index}.json`,
          JSON.stringify(
            {
              path: new URL(page.url()).pathname,
              text: await page
                .locator("body")
                .innerText()
                .catch(() => "page closed"),
            },
            null,
            2,
          ),
        );
      }
    throw error;
  } finally {
    for (const page of pages) if (!page.isClosed()) await page.close();
    await local.stop(folder);
  }
}
