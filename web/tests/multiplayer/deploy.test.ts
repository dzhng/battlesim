// @vitest-environment node
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { createServer } from "node:http";
import { join } from "node:path";
import { expect, test, vi } from "vitest";

vi.mock("node:child_process", async (original) => {
  const actual = await original<typeof import("node:child_process")>();
  return {
    ...actual,
    execFileSync: (...args: Parameters<typeof actual.execFileSync>) => {
      if (!["git", "tar", "bun"].includes(String(args[0])))
        throw new Error("Host execution forbidden");
      return actual.execFileSync(...args);
    },
  };
});
import { deployServer } from "../../../apps/game-server/deploy/deployment.ts";

const root = join(import.meta.dirname, "../../..");
const commit = execFileSync("git", ["rev-parse", "HEAD"], { cwd: root, encoding: "utf8" }).trim();
const bytes = new Uint8Array([0, 97, 115, 109]);
const sim = createHash("sha256").update(bytes).digest("hex");

test.each(["commit", "hash"])(
  "deploy refuses a site's wrong %s before bundling or pushing",
  async (fault) => {
    const visited: string[] = [];
    const server = createServer((request, response) => {
      visited.push(request.url!);
      if (request.url === "/build.json")
        response.end(
          JSON.stringify({
            commit:
              fault === "commit"
                ? createHash("sha1").update("missing local source").digest("hex")
                : commit,
            sim: fault === "hash" ? "wrong" : sim,
          }),
        );
      else if (request.url === "/sim/game_wasm_bg.wasm") response.end(bytes);
      else {
        response.writeHead(500);
        response.end();
      }
    });
    await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
    const address = server.address();
    if (!address || typeof address === "string") throw new Error("no TCP address");
    try {
      const lines: string[] = [];
      const code = await deployServer(
        { site: `http://127.0.0.1:${address.port}` },
        { root, report: (line) => lines.push(line) },
      );
      const result = { code, output: lines.join("\n") };
      expect(result.code).toBe(1);
      expect(result.output).toContain(
        fault === "commit"
          ? "Site commit does not exist in the local repository"
          : "Site simulation hash differs from build.json",
      );
      expect(visited).toEqual(
        fault === "commit" ? ["/build.json"] : ["/build.json", "/sim/game_wasm_bg.wasm"],
      );
      expect(result.output).not.toContain("Would push:");
    } finally {
      server.closeAllConnections();
      await new Promise<void>((resolve) => server.close(() => resolve()));
    }
  },
);

test("deploy check asks a real lobby without a credential and reports compatibility without building or pushing", async () => {
  const { sourceBuildIdentity } = await import("../../../packages/multiplayer/buildIdentity.ts");
  const { createLobbyHttp } = await import("../../../apps/game-server/src/http/index.ts");
  const { inProcessBattleMachines } =
    await import("../../../apps/game-server/src/machines/inProcess.ts");
  const { pendingBattle } = await import("./support/pendingBattle");
  const { timerClock } = await import("./timerClock");
  const build = { commit, ...sourceBuildIdentity(root) };
  const lobby = createLobbyHttp(build, {
    machines: inProcessBattleMachines({
      capacity: 0,
      createBattle: pendingBattle,
      ordersPerSecond: 100,
    }),
    mapAttempts: 1,
    clock: timerClock(),
    random: Math.random,
    clientAddressHeader: null,
  });
  const site = createServer((_request, response) => response.end(JSON.stringify(page)));
  let page = build;
  await new Promise<void>((resolve) => lobby.listen(0, "127.0.0.1", resolve));
  await new Promise<void>((resolve) => site.listen(0, "127.0.0.1", resolve));
  const address = `http://127.0.0.1:${(site.address() as { port: number }).port}`;
  const request: typeof fetch = async (input, init) => {
    expect(["/build", "/deployment"]).toContain(new URL(String(input)).pathname);
    expect(init?.headers).toBeUndefined();
    return fetch(
      `http://127.0.0.1:${(lobby.address() as { port: number }).port}${new URL(String(input)).pathname}`,
      init,
    );
  };
  try {
    for (const part of [undefined, "sim", "data", "wire"] as const) {
      page = part
        ? {
            ...build,
            [part]: createHash("sha256")
              .update(build[part] + "changed")
              .digest("hex"),
          }
        : build;
      const lines: string[] = [];
      expect(
        await deployServer(
          { site: address, commit, check: true },
          { request, report: (line) => lines.push(line) },
        ),
      ).toBe(part ? 2 : 0);
      expect(lines.join("\n")).toContain(part ? "Lobby needs a deploy" : "Lobby is in step");
      expect(lines.join("\n")).not.toContain("Would push:");
    }
    for (const request of [
      async () => new Response("missing", { status: 404 }),
      async () => {
        throw new Error("not reachable");
      },
    ]) {
      expect(
        await deployServer({ site: address, check: true }, { request, report: () => {} }),
      ).toBe(2);
    }
  } finally {
    for (const server of [lobby, site]) {
      server.closeAllConnections();
      await new Promise<void>((resolve) => server.close(() => resolve()));
    }
  }
});
