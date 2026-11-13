// The menu's one shell, above every menu page's visit: the backdrop battle,
// first-launch graphics detection, the loading cover, and the plate that each
// page fills. Moving between menu pages changes only the plate's contents, so
// the film behind never restarts and the cover never comes back. A loading
// screen stands in the plate's place until the backdrop is first ready.
import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
  useSyncExternalStore,
  type ReactNode,
} from "react";
import { useLocation, useNavigate } from "react-router";
import { MenuBackdrop } from "../MenuBackdrop";
import { LOADING_STAGES, LoadingTasks, useLoadingTasks } from "../LabLoading";
import { LoadingScreen } from "../LoadingScreen";
import { Wordmark } from "../Wordmark";
import { GraphicsDetectionHost } from "../GraphicsDetectionHost";
import { graphicsDetectionState, subscribeGraphicsDetection } from "../graphicsDetection";
import { GRAPHICS_TIERS, graphicsSettings } from "../graphicsSettings";
import { allowRuntimePrefetch } from "../gameAppearances";
import type { MenuPageId } from "./pages";

/** What the shell tells the page it holds. */
export interface MenuShellState {
  /** The loading cover has lifted. It never comes back. */
  revealed: boolean;
  /** The graphics tier is settled: detection was not needed, or it is done. */
  detected: boolean;
  /** The page last left for another, or null. Home focuses its tile. */
  left: MenuPageId | null;
  /** Leave the open page for home. */
  back(): void;
}

const Shell = createContext<MenuShellState | null>(null);

/** The shell state of the menu page being drawn. */
export function useMenuShell(): MenuShellState {
  const shell = useContext(Shell);
  if (shell === null) throw new Error("a menu page is drawn inside the menu shell");
  return shell;
}

/** How a tile opens its page, so its Back can return through history. */
export const FROM_MENU = { state: { fromMenu: true } } as const;

/** `page` is the menu page at the current address: home, or one of its pages. */
export function MenuShell({ page, children }: { page: MenuPageId | "home"; children: ReactNode }) {
  const plate = useRef<HTMLDivElement>(null);
  const { report, progress } = useLoadingTasks();
  const detection = useSyncExternalStore(subscribeGraphicsDetection, graphicsDetectionState);
  const settings = useSyncExternalStore(graphicsSettings.subscribe, graphicsSettings.get);
  // Covered only until the backdrop is first ready: its restarts happen
  // behind its own veil. One that cannot be made leaves the plain ground.
  const [revealed, setRevealed] = useState(false);
  const loading = !revealed && !progress.done && !progress.error;
  const detectionNeeded = settings.mode === "auto" && !settings.detectedMode;
  const detected = !detectionNeeded || detection.status === "complete";
  const [showDetection, setShowDetection] = useState(false);
  useEffect(() => {
    if (!loading && detectionNeeded) setShowDetection(true);
  }, [loading, detectionNeeded]);
  useEffect(() => {
    if (!loading && detected) allowRuntimePrefetch();
  }, [loading, detected]);
  const detectionStageVisible = detectionNeeded || showDetection;
  const displayLoading = loading || detectionNeeded || showDetection;
  const stages = detectionStageVisible
    ? [...LOADING_STAGES, { id: "graphics", label: "Detecting Graphics" }]
    : LOADING_STAGES;
  const detectionCurrent = !loading && detectionStageVisible ? "graphics" : progress.current;
  const detectionDetail =
    detection.status === "running" && detection.currentTier
      ? `Testing ${detection.currentTier.toUpperCase()} graphics · ${Math.min(
          detection.completed + 1,
          GRAPHICS_TIERS.length,
        )} of ${GRAPHICS_TIERS.length} presets`
      : detection.status === "complete" && detection.tier
        ? `Graphics set to ${detection.tier.toUpperCase()}`
        : detection.status === "idle" && detectionStageVisible
          ? "Preparing graphics test…"
          : undefined;
  const [shown] = useState(() => {
    let resolve = () => {};
    const promise = new Promise<void>((r) => (resolve = r));
    return { promise, resolve };
  });
  useEffect(() => {
    if (loading) return;
    setRevealed(true);
    shown.resolve();
  }, [loading, shown]);
  // The page just left, by Back, Escape or history alike.
  const [at, setAt] = useState(page);
  const [left, setLeft] = useState<MenuPageId | null>(null);
  if (page !== at) {
    setAt(page);
    setLeft(at === "home" ? null : at);
  }
  const navigate = useNavigate();
  const fromMenu = (useLocation().state as { fromMenu?: boolean } | null)?.fromMenu === true;
  const back = useCallback(
    () => void (fromMenu ? navigate(-1) : navigate("/", { replace: true })),
    [fromMenu, navigate],
  );
  return (
    <Shell value={{ revealed, detected, left, back }}>
      <main className="menu" data-open={page === "home" ? undefined : page} inert={displayLoading}>
        <LoadingTasks report={report}>
          <MenuBackdrop
            plate={plate}
            shown={shown.promise}
            onDetectionSceneReady={() => setShowDetection(false)}
          />
        </LoadingTasks>
        <div className="menu-slot">
          <GraphicsDetectionHost runner={null} enabled={!loading}>
            <div className="hud-panel menu-body" ref={plate}>
              {children}
            </div>
          </GraphicsDetectionHost>
        </div>
      </main>
      {displayLoading && (
        <LoadingScreen
          title={<Wordmark />}
          stages={stages}
          current={detectionCurrent}
          stageDetail={detectionDetail}
          back={null}
        />
      )}
    </Shell>
  );
}
