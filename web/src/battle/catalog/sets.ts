// The session factory: the one module that imports the committed game
// catalog. A page asks it for its battle's set (`catalogSet`), and the
// session hands what it gets to its views; nothing else reads a catalog at
// module level. Sets are resolved once per page and kept.
import view from "@fixtures/catalog.json";
import game from "@fixtures/game.json";
import { loadWasm } from "../sim/module";
import {
  composeCatalog,
  newDocuments,
  type CatalogSetName,
  type GameView,
  type SessionCatalog,
} from "./compose";

export { admitScenario, type CatalogSetName, type SessionCatalog } from "./compose";

/** Each set's own documents' source text, beyond the game's. */
const OWN: Record<CatalogSetName, Record<string, () => Promise<string>>> = {
  game: {},
  test: import.meta.glob<string>("../../../../fixtures/units/generic/**/*.json", {
    query: "?raw",
    import: "default",
  }),
  menu: import.meta.glob<string>("../../../../fixtures/units/menu/**/*.json", {
    query: "?raw",
    import: "default",
  }),
};

const resolved = new Map<CatalogSetName, Promise<SessionCatalog>>();

/** `set`'s documents beyond the game's, in path order, resolved with the
 *  game's (`own` stands in for the set's files, as a test's fakes do). */
export async function resolveCatalogSet(
  set: CatalogSetName,
  own?: readonly string[],
): Promise<SessionCatalog> {
  const files = OWN[set];
  const texts =
    own ??
    (await Promise.all(
      Object.keys(files)
        .sort()
        .map((path) => files[path]()),
    ));
  const gameView = view as unknown as GameView;
  const added = newDocuments(gameView, texts);
  const wasm = added.length ? await loadWasm() : null;
  return composeCatalog(set, game, gameView, added, (json) => wasm!.resolve_catalog(json));
}

/** The catalog of `set` for this page, resolved on first request. */
export function catalogSet(set: CatalogSetName): Promise<SessionCatalog> {
  let catalog = resolved.get(set);
  if (!catalog) {
    catalog = resolveCatalogSet(set);
    resolved.set(set, catalog);
  }
  return catalog;
}

/** Mechanics startup edits the committed view in place (installing the
 *  editor's accepted generation, then freezing it) before any set is
 *  resolved from it. */
export function editGameView(edit: (view: object) => void): void {
  if (resolved.size) throw new Error("the game catalog was edited after a session read it");
  edit(view);
}
