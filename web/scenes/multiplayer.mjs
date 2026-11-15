import { mkdir, readFile } from "node:fs/promises";
import {
  capture,
  commander,
  evidence,
  mockScreens,
  onlinePage,
  root,
  roomServer,
  sizes,
  uncovered,
} from "./_rooms.mjs";

export async function run(ctx) {
  const folder = evidence("multiplayer");
  await mkdir(folder, { recursive: true });
  await mockScreens(ctx);
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
    for (const size of sizes) {
      await page.setViewportSize(size);
      const rows = await page
        .getByRole("list", { name: "Rooms" })
        .getByRole("listitem")
        .evaluateAll((rows) =>
          rows.map((row) => {
            const map = row.querySelector('[aria-label="Map"]');
            return {
              x: map.getBoundingClientRect().x,
              case: getComputedStyle(map).textTransform,
              labelCase: getComputedStyle(map.querySelector("small")).textTransform,
            };
          }),
        );
      ctx.check(
        `room map columns align at ${size.width}`,
        rows.every((r) => r.x === rows[0].x),
        JSON.stringify(rows),
      );
      ctx.check(
        `map rows use the label's upper case at ${size.width}`,
        rows.every((r) => r.case === r.labelCase),
        JSON.stringify(rows),
      );
      const legend = page.locator(".online-seat-key");
      ctx.check(
        "seat legend describes shapes without a team",
        (await legend.locator("[data-side]").count()) === 0,
      );
      const ai = await page.locator('.online-room-seats [data-kind="ai"]').evaluateAll((seats) =>
        seats.map((s) => {
          const sample = document.querySelector(
            `.online-room-seats [data-side="${s.parentElement.dataset.side}"] i`,
          );
          const role = getComputedStyle(sample);
          return {
            side: s.parentElement.dataset.side,
            outline: getComputedStyle(s).borderTopColor,
            role: sample.dataset.kind === "open" ? role.borderTopColor : role.backgroundColor,
            stroke: getComputedStyle(s).borderTopStyle,
            rowStroke: getComputedStyle(s.closest("li")).borderLeftStyle,
          };
        }),
      );
      ctx.check(
        "AI blocks have equal strength on both sides",
        ai.some((s) => s.side === "red") &&
          ai.some((s) => s.side === "blue") &&
          ai.every((s) => s.outline === s.role && s.stroke === s.rowStroke),
        JSON.stringify(ai),
      );
    }
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
        JSON.stringify({ mode: "high", detectedMode: "high", lastResult: null }),
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
    await uncovered(refused);
    ctx.check(
      "the refusal's words are visible after the cover leaves",
      (await refused.getByRole("status", { name: "Online unavailable" }).isVisible()) &&
        (await refused.getByRole("status", { name: "Online unavailable" }).innerText()) ===
          "Online play is not available on this build.",
    );
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
