// How a page's UI reads its battle's unit catalog: a scope resolves the
// page's set (suspending until it has) and provides it; views read it with
// `useSessionCatalog`. Renderer, audio and overlay code take it as an argument.
import { createContext, use, useContext, type ReactNode } from "react";
import { catalogSet, type CatalogSetName, type SessionCatalog } from "./sets";

const Catalog = createContext<SessionCatalog | null>(null);

/** Provide `catalog` to the views below. */
export function SessionCatalogProvider({
  catalog,
  children,
}: {
  catalog: SessionCatalog;
  children: ReactNode;
}) {
  return <Catalog value={catalog}>{children}</Catalog>;
}

/** Resolve the page's catalog set and provide it; suspends until resolved. */
export function SessionCatalogScope({
  set,
  children,
}: {
  set: CatalogSetName;
  children: ReactNode;
}) {
  return <SessionCatalogProvider catalog={use(catalogSet(set))}>{children}</SessionCatalogProvider>;
}

/** The catalog of the battle this view belongs to. */
export function useSessionCatalog(): SessionCatalog {
  const catalog = useContext(Catalog);
  if (!catalog) throw new Error("no session catalog: the page has no SessionCatalogScope");
  return catalog;
}
