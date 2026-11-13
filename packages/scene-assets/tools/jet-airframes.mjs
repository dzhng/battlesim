// Write every fixed-wing roster jet's nozzles, wingtips and wing planforms,
// derived from its source GLB (`src/jetAirframes.ts`), into
// fixtures/presentation.json's `effects.jet_exhaust.airframes`:
//
//   bun run --cwd web jet-airframes
//
// Run it after a jet's source art changes or a fixed-wing jet joins the
// roster (author its `engine` there first). web/tests/sceneAssets/
// jetAirframes.test.ts fails while the fixture and the art disagree.
import { readdirSync, readFileSync, writeFileSync } from "node:fs";
import { registerHooks } from "node:module";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repo = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");
// packages/* own no node_modules: resolve their bare imports from web/, as
// web/asset.mjs does.
registerHooks({
  resolve(specifier, context, next) {
    if (
      /^[a-z@]/.test(specifier) &&
      !specifier.startsWith("node:") &&
      context.parentURL?.includes("/packages/")
    )
      return next(specifier, { ...context, parentURL: `file://${repo}/web/asset.mjs` });
    return next(specifier, context);
  },
});
const { deriveRosterJets, withAirframes } = await import("../src/jetAirframes.ts");

const path = join(repo, "fixtures/presentation.json");
const read = (p) => readFileSync(join(repo, p));
const derived = deriveRosterJets(read, (dir) => readdirSync(join(repo, dir)));
const before = readFileSync(path, "utf8");
const after = withAirframes(before, derived);
if (after === before) console.log(`jet airframes: ${Object.keys(derived).length} up to date`);
else {
  writeFileSync(path, after);
  console.log(`jet airframes: wrote ${Object.keys(derived).length} to fixtures/presentation.json`);
}
