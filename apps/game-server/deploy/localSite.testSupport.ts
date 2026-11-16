import { execFileSync } from "node:child_process";
import { cp, mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { createServer } from "node:http";
import { join } from "node:path";
import { sourceBuildIdentity } from "../../../packages/multiplayer/buildIdentity.ts";

/** A static publication of the existing simulation, with the site's normal identity. */
export async function localSite(root: string, commit?: string) {
  await mkdir(join(root, "throwaway/multiplayer"), { recursive: true });
  const folder = await mkdtemp(join(root, "throwaway/multiplayer/site-"));
  await mkdir(join(folder, "sim"));
  for (const file of ["game_wasm.js", "game_wasm_bg.wasm", "snippets"])
    await cp(join(root, "web/src/wasm", file), join(folder, "sim", file), { recursive: true });
  const build = {
    commit:
      commit ?? execFileSync("git", ["rev-parse", "HEAD"], { cwd: root, encoding: "utf8" }).trim(),
    ...sourceBuildIdentity(root),
  };
  await writeFile(join(folder, "build.json"), JSON.stringify(build));
  const server = createServer(async (request, response) => {
    try {
      response.end(await readFile(join(folder, request.url!)));
    } catch {
      response.writeHead(404);
      response.end();
    }
  });
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  const address = `http://127.0.0.1:${(server.address() as { port: number }).port}`;
  return {
    address,
    folder,
    build,
    async close() {
      server.closeAllConnections();
      await new Promise<void>((resolve) => server.close(() => resolve()));
      await rm(folder, { recursive: true, force: true });
    },
  };
}
