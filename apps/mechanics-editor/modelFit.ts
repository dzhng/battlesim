import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { decodeBundle } from "../../packages/scene-assets/src/codec";
import { typeFindings } from "../../packages/scene-assets/src/validate";
import { UnitCatalog, type CatalogView } from "../../packages/scene-assets/src/units";
import {
  bundlePath,
  type Catalog,
  type RuntimeCatalog,
} from "../../packages/scene-assets/src/schema";
import type { JsonObject } from "./src/protocol";

/** Baked geometry is the model's authority; the editor never invents fit limits. */
export async function validateGeometry(root: string, view: JsonObject): Promise<void> {
  const authored: Catalog = JSON.parse(await readFile(join(root, "assets/catalog.json"), "utf8"));
  const runtime: RuntimeCatalog = JSON.parse(
    await readFile(join(root, "assets/runtime/catalog.json"), "utf8"),
  );
  const units = new UnitCatalog(view as unknown as CatalogView);
  for (const id of units.ids) {
    if (!units.hull(id)) continue;
    const name = units.type(id).appearance!;
    const source = authored.appearances[name];
    const installed = runtime.appearances[name];
    if (!source || !installed)
      throw new Error(`Unit ${id}: appearance ${name} is unavailable for model-fit validation`);
    const bytes = new Uint8Array(
      await readFile(join(root, "assets/runtime", bundlePath(installed.bundle))),
    );
    const bundle = decodeBundle(bytes);
    if (bundle.kind !== "articulated")
      throw new Error(`Unit ${id}: its model is not an articulated vehicle`);
    const errors = typeFindings(
      name,
      bundle.nodes,
      units,
      id,
      { ...authored.tolerances, ...source.tolerances },
      source.mounts ?? null,
    ).filter((finding) => finding.severity === "error");
    if (errors.length)
      throw new Error(errors.map((finding) => `${id}: ${finding.message}`).join("\n"));
  }
}
