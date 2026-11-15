import { spawn, execFileSync } from "node:child_process";
import { once } from "node:events";
import { mkdir, writeFile } from "node:fs/promises";
import { randomBytes } from "node:crypto";

export const root = new URL("../../", import.meta.url).pathname;
export const sizes = [
  { width: 1280, height: 720 },
  { width: 1920, height: 1080 },
];
export const evidence = (scene) => `${root}throwaway/multiplayer/${scene}/`;

export async function roomServer() {
  execFileSync("bun", ["apps/game-server/build.mjs"], { cwd: root });
  const child = spawn("node", ["throwaway/game-server/main.mjs"], {
    cwd: root,
    env: { ...process.env, PORT: "0" },
    stdio: ["ignore", "pipe", "pipe"],
  });
  let log = "";
  child.stderr.on("data", (chunk) => {
    log += chunk;
  });
  const address = await new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      child.kill();
      reject(new Error(`Room server startup timed out: ${log}`));
    }, 30_000);
    child.once("exit", (code) => {
      clearTimeout(timer);
      reject(new Error(`Room server exited ${code}: ${log}`));
    });
    child.stdout.on("data", (chunk) => {
      log += chunk;
      const match = /Game server: (http:\/\/127\.0\.0\.1:\d+)/.exec(log);
      if (match) {
        clearTimeout(timer);
        resolve(match[1]);
      }
    });
  });
  return {
    address,
    async post(path, who, value) {
      return (
        await fetch(`${address}${path}`, {
          method: "POST",
          headers: { "X-Player": who, "Content-Type": "application/json" },
          body: JSON.stringify(value),
        })
      ).json();
    },
    async stop(folder) {
      const exited = once(child, "exit");
      child.kill();
      await exited;
      await writeFile(`${folder}server.log`, log);
    },
  };
}

export async function onlinePage(ctx, local, name, path = "/multiplayer") {
  const page = await ctx.newPage({ viewport: sizes[0] });
  await page.addInitScript((name) => {
    localStorage.setItem("battle.callsign", name);
    localStorage.setItem(
      "battle.graphics",
      JSON.stringify({ mode: "high", detectedMode: "high", lastResult: null }),
    );
  }, name);
  const handshake = page.waitForRequest((r) => r.url().endsWith("/hello"));
  await page.goto(`${new URL(ctx.url).origin}${path}?server=${encodeURIComponent(local.address)}`);
  const build = (await handshake).postDataJSON().build;
  if (path.startsWith("/room/"))
    await page.getByRole("region", { name: "Room lobby" }).waitFor({ timeout: 120_000 });
  else await page.getByRole("textbox", { name: "Callsign" }).waitFor({ timeout: 120_000 });
  await page.getByTestId("loading").waitFor({ state: "hidden", timeout: 120_000 });
  await page.evaluate(() => document.fonts.ready);
  return { page, build };
}

export async function commander(local, build, name) {
  const who = randomBytes(16).toString("hex");
  await local.post("/hello", who, { build, callsign: name });
  const room = await local.post("/rooms", who, { visibility: "public" });
  if (room.refused) throw new Error(`Fixture room refused: ${room.refused}`);
  const lifetime = new AbortController();
  await fetch(`${local.address}/rooms/${room.code}`, {
    headers: { "X-Player": who },
    signal: lifetime.signal,
  });
  return { who, code: room.code, close: () => lifetime.abort() };
}

export async function uncovered(page) {
  await page.getByTestId("loading").waitFor({ state: "hidden", timeout: 120_000 });
  await page.locator("main.menu:not([inert])").waitFor({ timeout: 120_000 });
  await page.evaluate(() => document.fonts.ready);
}

export async function capture(page, folder, name, allSizes = true) {
  await uncovered(page);
  await mkdir(folder, { recursive: true });
  for (const size of allSizes ? sizes : [page.viewportSize()]) {
    await page.setViewportSize(size);
    await page.mouse.move(0, 0);
    await page.evaluate(
      () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))),
    );
    await page.screenshot({
      path: `${folder}${name}-${size.width}x${size.height}.png`,
      animations: "disabled",
      caret: "hide",
    });
  }
  await page.setViewportSize(sizes[0]);
}

export async function mockScreens(ctx) {
  const folder = evidence("mock");
  await mkdir(folder, { recursive: true });
  const page = await ctx.newPage({ viewport: sizes[0] });
  await page.goto(`file://${root}specs/multiplayer/visualizations/lobby-mock.html`);
  for (const [index, name] of [
    [1, "Main menu"],
    [2, "Multiplayer"],
    [3, "Room lobby"],
  ]) {
    await page.getByRole("button", { name: `${index} ${name}`, exact: true }).click();
    await page.locator("#stage").screenshot({ path: `${folder}screen${index}.png` });
  }
  await page.close();
}
