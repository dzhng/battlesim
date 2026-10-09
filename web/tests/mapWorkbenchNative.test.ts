// @vitest-environment node
import { mkdtemp, mkdir, copyFile, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { expect, test } from "vitest";
import { WorkbenchStore } from "../../apps/map-workbench/server";
import { nativeReporter } from "../../apps/map-workbench/native";

// This is the composed source/publication boundary: only its filesystem is temporary.
test("real Rust admits settings independently of the selected seed and new runs capture the reviewed saved bytes", async () => {
  const repository = new URL("../../", import.meta.url).pathname;
  const root = await mkdtemp(join(tmpdir(), "map-workbench-native-"));
  await mkdir(join(root, "fixtures"));
  for (const file of [
    "map-presets.json",
    "generated-battle.json",
    "prototype-building-templates.json",
    "game.json",
    "catalog.json",
    "encounters.json",
  ])
    await copyFile(join(repository, "fixtures", file), join(root, "fixtures", file));
  const store = new WorkbenchStore(root, nativeReporter(repository));
  try {
    const draft = structuredClone(await store.snapshot());
    (draft.documents.defaults.limits as { max_authored_parts: number }).max_authored_parts = 1;
    const refused = await store.generate({
      purpose: "preview",
      retainedArtifactIds: [],
      draft,
      choice: { type: "mixed", size: "small", seed: "1" },
    });
    expect(refused.status).toBe("refused");
    const review = await store.preview(draft);
    expect(review.diagnostics).toEqual([]);
    expect(review.files.map((file) => file.path)).toEqual(["fixtures/generated-battle.json"]);
    const saved = await store.save(review);
    expect(await readFile(join(root, review.files[0].path), "utf8")).toBe(review.files[0].after);
    expect(saved.documents.presets.revision).toBe(draft.documents.presets.revision);
    const next = await store.generate({
      purpose: "preview",
      retainedArtifactIds: [],
      draft: saved,
      choice: refused.choice,
    });
    const exported = await store.export(next.artifactId!);
    expect((exported.inputs as { defaults: string }).defaults).toBe(review.files[0].after);
    expect(next.status).toBe("refused");
  } finally {
    await store.close();
    await rm(root, { recursive: true, force: true });
  }
}, 30000);
