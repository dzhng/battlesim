import { createContext, useCallback, useContext, useEffect, useRef, type ReactNode } from "react";
import { useLocation, useNavigate } from "react-router";

const Visit = createContext<string | null>(null);

/** Every navigation remounts the page. Only publishing an admitted battle
 * carries its current visit through the router's replacement key. */
export function PageVisit({ children }: { children: ReactNode }) {
  const location = useLocation();
  const key = location.state?.battleVisit ?? location.key;
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
    return live.current && (state?.usr?.battleVisit ?? state?.key ?? "default") === visit;
  }, [visit]);
}

/** Publish an address without changing the admitted battle or other history state.
 * The callback stays stable so its replacement cannot restart preparation effects. */
export function usePublishBattleAddress() {
  const visit = useContext(Visit);
  const active = usePageVisitActive();
  const navigate = useNavigate();
  const location = useLocation();
  const current = useRef({ navigate, state: location.state });
  current.current = { navigate, state: location.state };
  if (visit === null) throw new Error("A battle address requires a page visit");
  return useCallback(
    (href: string) => {
      if (!active()) return;
      const { navigate, state } = current.current;
      void navigate(href, { replace: true, state: { ...state, battleVisit: visit } });
    },
    [visit, active],
  );
}
