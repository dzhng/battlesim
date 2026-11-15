// @vitest-environment node
import { execFileSync, spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { createServer } from "node:http";
import { join } from "node:path";
import { expect, test } from "vitest";

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
            commit: fault === "commit" ? "wrong" : commit,
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
      const result = await new Promise<{ code: number | null; output: string }>(
        (resolve, reject) => {
          const child = spawn(
            "bun",
            ["run", "deploy:server", "--site", `http://127.0.0.1:${address.port}`],
            { cwd: root, stdio: ["ignore", "pipe", "pipe"] },
          );
          let output = "";
          child.stdout.on("data", (data) => (output += data));
          child.stderr.on("data", (data) => (output += data));
          child.once("error", reject);
          child.once("exit", (code) => resolve({ code, output }));
        },
      );
      expect(result.code).toBe(1);
      expect(result.output).toContain(
        fault === "commit"
          ? "Site commit differs from checkout"
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
