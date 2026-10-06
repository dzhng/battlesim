// @vitest-environment node
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, expect, test } from "vitest";
import { mechanicsSourcePaths } from "../../apps/fixture-publication/publication";

const roots: string[] = [];
afterEach(async () => {
  await Promise.all(roots.splice(0).map((root) => rm(root, { recursive: true, force: true })));
});

test("mechanics sources exclude model-only roster manifests", async () => {
  const root = await mkdtemp(join(tmpdir(), "mechanics-publication-"));
  roots.push(root);
  await mkdir(join(root, "fixtures/units"), { recursive: true });
  await mkdir(join(root, "fixtures/props"), { recursive: true });
  await writeFile(join(root, "fixtures/game.json"), "{}\n");
  await writeFile(join(root, "fixtures/units/roster.json"), "{}\n");
  await writeFile(join(root, "fixtures/units/model-manifest.json"), "{\"description\":\"asset metadata\"}\n");

  await expect(mechanicsSourcePaths(root)).resolves.toEqual([
    "fixtures/game.json",
    "fixtures/units/roster.json",
  ]);
});
