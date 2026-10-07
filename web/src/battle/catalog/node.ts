// Node's session factory, for tools and scenes: the committed game view and
// a set's own documents read from `fixtures/`, composed as the browser's
// (`sets.ts`) are. The WebAssembly resolver is loaded only when a set adds
// documents the game's lacks (`bun run build:wasm` first), so a tool that
// reads only the game's units never needs it.
//
// Plain TypeScript with no bundler features, so Node scripts import it too.
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import {
  composeCatalog,
  newDocuments,
  type CatalogSetName,
  type GameView,
  type SessionCatalog,
} from "./compose.ts";

const ROOT = join(import.meta.dirname, "../../../..");
const WASM = join(import.meta.dirname, "../../wasm/game_wasm_bg.wasm");

/** Each set's own folder under `fixtures/`, beyond the game's (the browser's
 *  `sets.ts` globs the same folders). */
const OWN: Record<CatalogSetName, string | null> = {
  game: null,
  test: "units/generic",
  menu: "units/menu",
};

/** Every `.json` under `dir`, in path order; none when it is absent. */
function jsonFiles(dir: string): string[] {
  if (!existsSync(dir)) return [];
  return readdirSync(dir, { recursive: true })
    .map((entry) => join(dir, String(entry)))
    .filter((path) => path.endsWith(".json") && statSync(path).isFile())
    .sort();
}

/** The catalog of `set` as the repository at `root` holds it. */
export async function nodeCatalogSet(set: CatalogSetName, root = ROOT): Promise<SessionCatalog> {
  const read = (path: string) => readFileSync(path, "utf8");
  const own = OWN[set];
  const gameView = JSON.parse(read(join(root, "fixtures/catalog.json"))) as GameView;
  const added = newDocuments(gameView, own ? jsonFiles(join(root, "fixtures", own)).map(read) : []);
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
