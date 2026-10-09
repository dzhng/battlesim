// A battle's unit catalog, composed from one of three document sets and
// resolved by the simulation's one resolver (the WebAssembly
// `resolve_catalog`; `sim::fixtures::catalog_documents` natively):
//
// - game: the roster, its profiles and roles, and the props. The only set
//   committed, as `fixtures/catalog.json`.
// - test: the game's plus the test units tests, labs, scenes and art checks
//   run (`fixtures/units/test`, `test_*`, never game content).
// - menu: the game's plus the test units and the menu reel's own units
//   (`fixtures/units/menu`), which extend them.
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

/** Each set's own folders under `fixtures/`, beyond the game's: the folders
 *  `sim::fixtures::CatalogSet` reads natively (the browser's `sets.ts` globs
 *  them by literal path). The menu's units extend the test units, so the
 *  menu set holds both. */
export const SET_FOLDERS: Record<CatalogSetName, readonly string[]> = {
  game: [],
  test: ["units/test"],
  menu: ["units/test", "units/menu"],
};

/** Whether the catalog document at `path` (from the repository root) is the
 *  game's: under no other set's own folder. */
export function isGameDocument(path: string): boolean {
  return Object.values(SET_FOLDERS)
    .flat()
    .every((folder) => !path.startsWith(`fixtures/${folder}/`));
}

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

/** The catalog of `set`: the game's documents plus `added` (the set's own
 *  documents' source text) resolved by `resolve` (`resolve_catalog`), or
 *  with nothing added the game's view as committed, which the same resolver
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

/** A scenario's rules as the page reads them: each weapon row as the
 *  catalog resolves it (`extends` followed, so a row inherits its icon,
 *  full load and range), beside any row the scenario adds. Panels, the
 *  range ruler and poses read these; the authority is sent the scenario's
 *  own text and resolves it itself. */
export function presentationRules<R extends { weapons: object }>(
  rules: R,
  catalog: SessionCatalog,
): R {
  return { ...rules, weapons: { ...rules.weapons, ...catalog.weapons } };
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
