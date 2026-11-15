import { mkdir, writeFile } from "node:fs/promises";
import { capture, evidence, onlinePage, roomServer, sizes } from "./_rooms.mjs";

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
    for (const tag of svg.querySelectorAll("[data-held]")) tag.removeAttribute("data-held");
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
/** A map the generator admits at this room's size, so the picture is the
 *  same every run; a room's own random seed may be refused. */
const ROOM_SEED = "77";

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
    const hostBlue = host.getByRole("group", { name: "Blue side" });
    const hostRed = host.getByRole("group", { name: "Red side" });
    await hostBlue.getByRole("button", { name: /Add seat/ }).click();
    console.log("      room lobby: added Blue seat");
    await host.getByRole("button", { name: "Set Blue 2 open", exact: true }).click();
    console.log("      room lobby: opened Blue seat");
    await hostRed.getByRole("button", { name: /Add seat/ }).click();
    console.log("      room lobby: added Red AI");
    await hostRed.getByRole("button", { name: "Remove Red 2", exact: true }).waitFor();
    await guest.getByRole("button", { name: "Ready", exact: true }).click();
    await host.getByRole("status", { name: "LENA ready" }).waitFor();
    ctx.check(
      "the host sees the guest ready beside an open seat and an AI",
      (await host.getByRole("button", { name: "Sit Blue 2" }).isVisible()) &&
        (await host.getByRole("button", { name: "Set Red 2 open" }).isVisible()),
    );
    await settled(host);
    await settled(guest);
    ctx.check(
      "host and guest share the same map geometry and fixed sides",
      JSON.stringify(await mapGeometry(host)) === JSON.stringify(await mapGeometry(guest)),
    );
    // No actions while these concurrent captures are taken: both see the same revision.
    for (const size of sizes) {
      await Promise.all([host.setViewportSize(size), guest.setViewportSize(size)]);
      await Promise.all([host.mouse.move(0, 0), guest.mouse.move(0, 0)]);
      await Promise.all([
        host.screenshot({
          path: `${folder}host-${size.width}x${size.height}.png`,
          animations: "disabled",
          caret: "hide",
        }),
        guest.screenshot({
          path: `${folder}guest-${size.width}x${size.height}.png`,
          animations: "disabled",
          caret: "hide",
        }),
      ]);
    }
    await Promise.all([host.setViewportSize(sizes[0]), guest.setViewportSize(sizes[0])]);
    await guest.evaluate(
      () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))),
    );
    const before = await mapGeometry(guest);
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
    await capture(guest, folder, "guest-moved");
    // Streamed host setting: the guest's selected map follows it, with no action of its own.
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
    await settled(guest);
    ctx.check(
      "the guest sees the host's changed setting without sending an action",
      guestRequests.length === guestActions,
    );
    await capture(guest, folder, "guest-setting-changed");
    await host.getByRole("button", { name: "Set Red 1 AI", exact: true }).click();
    await guest.getByRole("button", { name: "Ready", exact: true }).click();
    const start = host.getByRole("button", { name: "Start", exact: true });
    ctx.check("the host's Start waits for its own ready mark", await start.isDisabled());
    await host.getByRole("button", { name: "Ready", exact: true }).click();
    await host.waitForFunction(
      () =>
        ![...document.querySelectorAll("button")].find((b) => b.textContent === "Start").disabled,
    );
    ctx.check("the host's Start lights after both humans ready", await start.isEnabled());
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
