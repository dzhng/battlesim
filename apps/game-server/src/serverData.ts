import { readFileSync } from "node:fs";
import { join } from "node:path";
import config from "../../../fixtures/generated-battle.json";
import { nodeCatalogSet } from "../../../web/src/battle/catalog/node.ts";
import type { MapGenerator } from "../../../web/src/maps/source.ts";
import type { SetupData } from "../../../packages/skirmish-setup/src/lobby.ts";

/** Read the same generator documents the browser bundles into a battle. */
export async function nodeSetupData(
  root: string,
): Promise<SetupData & { rules: string; root: string }> {
  const wasm = await import("../../../web/src/wasm/game_wasm.js");
  wasm.initSync({ module: readFileSync(join(root, "web/src/wasm/game_wasm_bg.wasm")) });
  const catalog = await nodeCatalogSet("game", root);
  return {
    generator: wasm as unknown as MapGenerator,
    presets: readFileSync(join(root, "fixtures/map-presets.json"), "utf8"),
    templates: readFileSync(join(root, "fixtures/prototype-building-templates.json"), "utf8"),
    limits: config.limits,
    rules: JSON.stringify(catalog.rules),
    root,
  };
}

/** How many generated maps the game tries before giving up on a battle. */
export const MAP_ATTEMPTS = config.admission.max_generated_attempts;
