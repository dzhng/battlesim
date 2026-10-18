import { createContext, useCallback, useContext, useEffect, useRef, type ReactNode } from "react";
import { useLocation } from "react-router";

const Visit = createContext<string | null>(null);

/** Every navigation remounts the page. */
export function PageVisit({ children }: { children: ReactNode }) {
  const key = useLocation().key;
  return (
    <Visit key={key} value={key}>
      {children}
    </Visit>
  );
}

/** History changes before React commits the next page. A queued completion
 * must stop at that boundary, even while the old page is still mounted. */
export function usePageVisitActive() {
  const visit = useContext(Visit);
  const live = useRef(true);
  useEffect(() => {
    live.current = true;
    return () => {
      live.current = false;
    };
  }, []);
  if (visit === null) throw new Error("A page continuation requires a visit");
  return useCallback(() => {
    const state = window.history.state;
    return live.current && (state?.key ?? "default") === visit;
  }, [visit]);
}
