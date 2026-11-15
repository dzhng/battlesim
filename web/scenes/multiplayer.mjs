import { mkdir, readFile } from "node:fs/promises";
import {
  capture,
  commander,
  evidence,
  mockScreens,
  onlinePage,
  offlinePictures,
  root,
  roomServer,
} from "./_rooms.mjs";

export async function run(ctx) {
  const folder = evidence("slice16");
  await mkdir(folder, { recursive: true });
  await mockScreens(ctx);
  await offlinePictures(ctx, folder, true);
  const local = await roomServer();
  try {
    const { page, build } = await onlinePage(ctx, local, "VIPER-41");
    await page.getByRole("status", { name: "No public rooms" }).waitFor();
    ctx.check(
      "an empty stream says no public rooms",
      await page.getByRole("status", { name: "No public rooms" }).isVisible(),
    );
    await capture(page, folder, "empty");
    const a = await commander(local, build, "MAKO");
    await local.post(`/rooms/${a.code}/act`, a.who, { kind: "add_seat", side: "blue" });
    const b = await commander(local, build, "SALTMARSH");
    const regions = JSON.parse(await readFile(`${root}fixtures/map-presets.json`, "utf8")).parcels
      .regional_families;
    const changed = await local.post(`/rooms/${b.code}/act`, b.who, {
      kind: "settings",
      patch: { map: { type: "open", region: regions[0] } },
    });
    if (!changed.ok) throw new Error("Fixture map setting refused");
    const full = await commander(local, build, "KESTREL");
    await local.post(`/rooms/${full.code}/act`, full.who, {
      kind: "set_seat",
      seat: "seat-2",
      to: "ai",
    });
    await page.getByRole("listitem", { name: "KESTREL's room" }).waitFor();
    ctx.check(
      "the streamed list includes open and full public rooms",
      (await page.getByRole("list", { name: "Rooms" }).getByRole("listitem").count()) === 3 &&
        (await page
          .getByRole("list", { name: "Rooms" })
          .getByRole("button", { name: /^Join .*room$/ })
          .count()) === 2,
    );
    await capture(page, folder, "rooms");
    await page.goto(new URL(ctx.url).origin);
    await page.locator("main.menu:not([inert])").waitFor({ timeout: 120_000 });
    await page.getByRole("navigation", { name: "Main menu" }).waitFor();
    await page.getByTestId("loading").waitFor({ state: "hidden", timeout: 120_000 });
    const menu = page.getByRole("navigation", { name: "Main menu" });
    const labels = await menu.getByRole("button").allTextContents();
    ctx.check(
      "Multiplayer precedes Skirmish in the menu",
      labels[0] === "Multiplayer" && labels[1] === "Skirmish",
    );
    await capture(page, folder, "menu");
    // Refusal is served by the real server, after the page sends the wrong build.
    await page.close();
    const refused = await ctx.newPage({
      viewport: { width: 1280, height: 720 },
      allowErrors: true,
    });
    await refused.addInitScript(() =>
      localStorage.setItem(
        "battle.graphics",
        JSON.stringify({ mode: "high", detectedMode: "high" }),
      ),
    );
    await refused.route("**/hello", (route) => {
      const body = route.request().postDataJSON();
      return route.continue({
        postData: JSON.stringify({
          ...body,
          build: { ...body.build, commit: "refused-scene-build" },
        }),
      });
    });
    await refused.goto(
      `${new URL(ctx.url).origin}/multiplayer?server=${encodeURIComponent(local.address)}`,
    );
    await refused.getByRole("status", { name: "Online unavailable" }).waitFor();
    ctx.check(
      "a refused build offers no online action",
      (await refused.getByRole("button", { name: "Create room" }).count()) === 0 &&
        (await refused.getByRole("button", { name: "Join private room" }).count()) === 0,
    );
    await capture(refused, folder, "refused");
    await refused.close();
  } finally {
    await local.stop(folder);
  }
}
