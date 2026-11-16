// @vitest-environment node
import { execFileSync, spawn } from "node:child_process";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { expect, test } from "vitest";

const root = join(import.meta.dirname, "../../..");
test("the local Node bundle welcomes its recorded build from a different working directory", async () => {
  execFileSync("bun", ["apps/game-server/build.mjs"], { cwd: root, stdio: "pipe" });
  const folder = join(root, "throwaway/game-server");
  const build = JSON.parse(readFileSync(join(folder, "build.json"), "utf8"));
  const child = spawn("node", [join(folder, "main.mjs")], {
    cwd: join(root, "throwaway"),
    env: { ...process.env, PORT: "0", HOST: "127.0.0.1", GIT_DIR: join(folder, "no-runtime-git") },
    stdio: ["ignore", "pipe", "pipe"],
  });
  const exit = new Promise<void>((resolve) => child.once("exit", () => resolve()));
  try {
    const address = await new Promise<string>((resolve, reject) => {
      const timer = setTimeout(() => reject(new Error("bundle startup timed out")), 15_000);
      let output = "";
      child.stdout.on("data", (data) => {
        output += data;
        const match = /Game server: (http:\/\/127\.0\.0\.1:\d+)/.exec(output);
        if (match) {
          clearTimeout(timer);
          resolve(match[1]);
        }
      });
      child.once("error", (error) => {
        clearTimeout(timer);
        reject(error);
      });
      child.once("exit", () => {
        clearTimeout(timer);
        reject(new Error("bundle failed to start"));
      });
    });
    const hello = (pageBuild: typeof build) =>
      fetch(`${address}/hello`, {
        method: "POST",
        headers: { "X-Player": "1".repeat(32), "Content-Type": "application/json" },
        body: JSON.stringify({ build: pageBuild, callsign: "SCOUT" }),
      });
    expect(await (await hello(build)).json()).toEqual({ callsign: "SCOUT" });
    expect(await (await hello({ ...build, sim: "wrong" })).json()).toEqual({ refused: "build" });
  } finally {
    child.kill("SIGTERM");
    await exit;
  }
});
