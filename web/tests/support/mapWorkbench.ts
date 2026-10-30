import { mkdtemp, mkdir, writeFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { WorkbenchStore } from "../../../apps/map-workbench/server";
import type { NativeReporter } from "../../../apps/map-workbench/native";
import type { NativeValidation } from "../../../apps/map-workbench/src/protocol";

const roots: string[] = [];
export const stores: WorkbenchStore[] = [];
export async function closeWorkbenchFixtures() {
  await Promise.all(stores.splice(0).map((store) => store.close()));
  await Promise.all(roots.splice(0).map((root) => rm(root, { recursive: true, force: true })));
}

export const validation: NativeValidation = {
  status: "valid",
  diagnostics: [],
  fields: [
    {
      id: "density",
      document: "presets",
      path: ["density"],
      group: "towns",
      label: "Density",
      description: "Building density",
      unit: "share",
      role: "construction",
      editable: true,
    },
    {
      id: "analysis.target",
      document: "defaults",
      path: ["analysis", "target"],
      group: "analysis",
      label: "Target",
      description: "Report target",
      unit: "share",
      role: "analysis",
      editable: true,
    },
  ],
};

export const reporter: NativeReporter = async (request) => {
  if (request.operation === "validate") return validation;
  throw new Error("Unexpected operation");
};
export async function fixture(run: NativeReporter = reporter) {
  const root = await mkdtemp(join(tmpdir(), "map-workbench-"));
  roots.push(root);
  await mkdir(join(root, "fixtures"));
  const files: Record<string, unknown> = {
    "map-presets.json": { revision: "initial", density: 0.5 },
    "generated-battle.json": { analysis: { target: 0.5 } },
    "prototype-building-templates.json": [],
    "rules.json": { tick_hz: 1 },
    "catalog.json": { documents: [], units: [], props: [] },
    "encounters.json": { recipes: {} },
  };
  for (const [name, value] of Object.entries(files))
    await writeFile(join(root, "fixtures", name), JSON.stringify(value));
  const store = new WorkbenchStore(root, run);
  stores.push(store);
  return { root, store };
}
