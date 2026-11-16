// @vitest-environment node
import { mkdir, mkdtemp, rm, writeFile, appendFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { runInNewContext } from "node:vm";
import { createServer } from "vite";
import { expect, test, vi } from "vitest";
import { buildInputs, sharedBuildIdentity } from "../../../packages/multiplayer/buildIdentity.ts";
import { simulationBuild } from "../../simulationBuild";

const root = join(import.meta.dirname, "../../..");
test("a development page refreshes its identity when a shared document or wire source changes", async () => {
  const folder = await mkdtemp(join(root, "throwaway/multiplayer/dev-build-"));
  const inputs = buildInputs(root);
  const wireFolder = join(folder, dirname(inputs.wire[0]));
  await mkdir(wireFolder, { recursive: true });
  await mkdir(join(folder, "web"));
  for (const file of inputs.data) {
    await mkdir(dirname(join(folder, file)), { recursive: true });
    await writeFile(join(folder, file), "{}");
  }
  await writeFile(join(wireFolder, "initial.ts"), "export const initial = true;");
  await writeFile(join(folder, "web/identity.js"), "globalThis.pageBuild = __SHARED_BUILD__;");
  const server = await createServer({
    configFile: false,
    root: join(folder, "web"),
    logLevel: "silent",
    server: { host: "127.0.0.1", port: 0 },
    plugins: [simulationBuild(folder, "development-provenance")],
  });
  try {
    await server.listen();
    const published = async () => {
      const port = (server.httpServer!.address() as { port: number }).port;
      const environment = await (await fetch(`http://127.0.0.1:${port}/@vite/env`)).text();
      const response = await fetch(`http://127.0.0.1:${port}/identity.js`);
      expect(response.ok).toBe(true);
      const context: { pageBuild?: ReturnType<typeof sharedBuildIdentity> } = {};
      runInNewContext(environment + "\n" + (await response.text()), context);
      return context.pageBuild!;
    };
    expect(await published()).toEqual(sharedBuildIdentity(folder));
    const before = sharedBuildIdentity(folder);
    await appendFile(join(folder, inputs.data[0]), "\n");
    await vi.waitFor(async () => expect(await published()).toEqual(sharedBuildIdentity(folder)), {
      timeout: 10_000,
    });
    expect((await published()).data).not.toBe(before.data);
    for (const change of [
      () => writeFile(join(wireFolder, "added.ts"), "export const added = true;"),
      () => rm(join(wireFolder, "initial.ts")),
    ]) {
      const before = sharedBuildIdentity(folder);
      await change();
      await vi.waitFor(async () => expect(await published()).toEqual(sharedBuildIdentity(folder)), {
        timeout: 10_000,
      });
      expect((await published()).wire).not.toBe(before.wire);
    }
  } finally {
    await server.close();
    await rm(folder, { recursive: true, force: true });
  }
});
