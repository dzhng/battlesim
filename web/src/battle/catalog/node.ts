// Node's session factory, for tools and scenes: the committed game view and
// a set's own documents read from `fixtures/`, composed as the browser's
// (`sets.ts`) are. The WebAssembly resolver is loaded only for a set with
// documents of its own (`bun run build:wasm` first), so a tool that reads
// only the game's units never needs it.
//
// Plain TypeScript with no bundler features, so Node scripts import it too.
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import {
  composeCatalog,
  SET_FOLDERS,
  type CatalogSetName,
  type GameView,
  type SessionCatalog,
} from "./compose.ts";

const ROOT = join(import.meta.dirname, "../../../..");
const WASM = join(import.meta.dirname, "../../wasm/game_wasm_bg.wasm");

/** Every `.json` under `dir`, in path order; none when it is absent. */
function jsonFiles(dir: string): string[] {
  if (!existsSync(dir)) return [];
  return readdirSync(dir, { recursive: true })
    .map((entry) => join(dir, String(entry)))
    .filter((path) => path.endsWith(".json") && statSync(path).isFile())
    .sort();
}

/** `set`'s own documents' source text, beyond the game's, in path order. */
export function ownDocuments(set: CatalogSetName, root = ROOT): string[] {
  return SET_FOLDERS[set]
    .flatMap((folder) => jsonFiles(join(root, "fixtures", folder)))
    .map((path) => readFileSync(path, "utf8"));
}

/** The catalog of `set` as the repository at `root` holds it. */
export async function nodeCatalogSet(set: CatalogSetName, root = ROOT): Promise<SessionCatalog> {
  const read = (path: string) => readFileSync(path, "utf8");
  const gameView = JSON.parse(read(join(root, "fixtures/catalog.json"))) as GameView;
  const added = ownDocuments(set, root);
  let resolve = (_json: string): string => {
    throw new Error("nothing to resolve");
  };
  if (added.length) {
    if (!existsSync(WASM))
      throw new Error(
        `the simulation's WebAssembly is not built (${WASM}): run \`bun run build:wasm\``,
      );
    const wasm = await import("../../wasm/game_wasm.js");
    wasm.initSync({ module: readFileSync(WASM) });
    resolve = wasm.resolve_catalog;
  }
  return composeCatalog(
    set,
    JSON.parse(read(join(root, "fixtures/game.json"))),
    gameView,
    added,
    resolve,
  );
}
