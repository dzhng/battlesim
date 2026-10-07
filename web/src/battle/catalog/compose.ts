// A battle's unit catalog, composed from one of three document sets and
// resolved by the simulation's one resolver (the WebAssembly
// `resolve_catalog`; `sim::fixtures::catalog_documents` natively):
//
// - game: the roster, its profiles and roles, and the props. The only set
//   committed, as `fixtures/catalog.json`.
// - test: the game's plus the test units labs, scenes and art checks run
//   (`fixtures/units/generic` until unit-models slice 05 names them).
// - menu: the game's plus the menu reel's own units (`fixtures/units/menu`).
//
// The browser factory (`sets.ts`) and the Node one (`node.ts`) read the
// files their own way and compose them here. Relative imports only: the
// asset CLI runs this under Node's type stripping.
import {
  UnitCatalog,
  type CatalogView,
  type WeaponRow,
} from "../../../../packages/scene-assets/src/units.ts";

export type CatalogSetName = "game" | "test" | "menu";

/** The game's committed view: `Catalog::view` plus `game.json`'s resolved weapons. */
export type GameView = CatalogView & { weapons: Record<string, WeaponRow> };

/** One battle's unit catalog, owned by its session. */
export interface SessionCatalog {
  readonly set: CatalogSetName;
  /** The resolved unit, soldier, card and prop types. */
  readonly units: UnitCatalog;
  /** `game.json`'s weapon rows, `extends` resolved, by the names mounts give them. */
  readonly weapons: Record<string, WeaponRow>;
  /** The rules a scenario on this catalog carries: `game.json` with this
   *  set's documents as `catalog`, so the authority runs what the page draws. */
  readonly rules: GameRules;
}

/** `fixtures/game.json` with a set's documents as its `catalog`. */
export type GameRules = Omit<typeof import("../../../../fixtures/game.json"), "default"> & {
  catalog: unknown[];
};

/** Which of a set's own documents (each one's source text) the game's view
 *  lacks. Transitional (until unit-models slice 05): the game's set still
 *  holds the generic units, so an own document whose every concrete entry
 *  the game's view already defines is the game's own, not added twice (the
 *  view's documents are resolved: abstract bases are folded away). A
 *  document that adds anything is passed whole, and the resolver refuses an
 *  entry defined twice by name. */
export function newDocuments(gameView: GameView, own: readonly string[]): string[] {
  const defined = (section: string, id: string) =>
    gameView.documents.some((d) => Object.hasOwn((d as Record<string, object>)[section] ?? {}, id));
  return own.filter((text) => {
    const doc = JSON.parse(text) as Record<string, Record<string, { abstract?: boolean }>>;
    return Object.entries(doc).some(([section, entries]) =>
      Object.entries(entries).some(([id, entry]) => !entry.abstract && !defined(section, id)),
    );
  });
}

/** The catalog of `set`: the game's documents plus `added` (from
 *  `newDocuments`) resolved by `resolve` (`resolve_catalog`), or with
 *  nothing added the game's view as committed, which the same resolver
 *  wrote. */
export function composeCatalog(
  set: CatalogSetName,
  game: Omit<GameRules, "catalog">,
  gameView: GameView,
  added: readonly string[],
  resolve: (documentsJson: string) => string,
): SessionCatalog {
  const documents = [...gameView.documents.map((d) => JSON.stringify(d)), ...added];
  const view: CatalogView = added.length
    ? (JSON.parse(resolve(`[${documents.join(",")}]`)) as CatalogView)
    : gameView;
  return {
    set,
    units: new UnitCatalog(view),
    weapons: gameView.weapons,
    rules: { ...game, catalog: view.documents },
  };
}

/** The units a scenario fields, admitted against its session's catalog:
 *  a unit the catalog lacks is refused by name, never silently undrawn. */
export function admitScenario(
  scenario: { units?: readonly { kind: string }[] },
  catalog: SessionCatalog,
): UnitCatalog {
  const missing = [
    ...new Set((scenario.units ?? []).map((u) => u.kind).filter((k) => !catalog.units.has(k))),
  ];
  if (missing.length)
    throw new Error(
      `${missing.length > 1 ? "unit types" : "unit type"} ${missing.join(", ")} ${
        missing.length > 1 ? "are" : "is"
      } not in the ${catalog.set} catalog`,
    );
  return catalog.units;
}
