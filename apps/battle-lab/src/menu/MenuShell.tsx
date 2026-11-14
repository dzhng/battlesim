// The menu's one shell, above every menu page's visit: the backdrop battle,
// first-launch graphics detection, the loading covers, and the plate that each
// page fills. Moving between menu pages changes only the plate's contents, so
// the film behind never restarts and the menu's own cover never comes back. A
// loading screen stands in the plate's place until the backdrop is first
// ready; after that, a page still loading (the Armory, until its stage draws)
// stands under a cover of its own, so the film never stands in for it.
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
import { menuPage, type MenuPageId } from "./pages";

/** What the shell tells the page it holds. */
export interface MenuShellState {
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

/** The stages of a page's own cover: its contents, then its scene. */
const PAGE_STAGES = [
  { id: "preparation", label: "Preparing" },
  { id: "renderer", label: "Setting the stage" },
] as const;

/** How a tile opens its page, so its Back can return through history. */
export const FROM_MENU = { state: { fromMenu: true } } as const;

/** `page` is the menu page at the current address: home, or one of its pages. */
export function MenuShell({ page, children }: { page: MenuPageId | "home"; children: ReactNode }) {
  const plate = useRef<HTMLDivElement>(null);
  const { report, progress } = useLoadingTasks();
  // What the page reports: it holds the first cover with the backdrop, and
  // after that a cover of its own.
  const pageTasks = useLoadingTasks();
  const detection = useSyncExternalStore(subscribeGraphicsDetection, graphicsDetectionState);
  const settings = useSyncExternalStore(graphicsSettings.subscribe, graphicsSettings.get);
  // Covered only until the backdrop is first ready: its restarts happen
  // behind its own veil. One that cannot be made leaves the plain ground.
  const [revealed, setRevealed] = useState(false);
  const loading = !revealed && !progress.error && (!progress.done || pageTasks.progress.pending);
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
  const pageLoading = revealed && !displayLoading && page !== "home" && pageTasks.progress.pending;
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
    <Shell value={{ detected, left, back }}>
      <main
        className="menu"
        data-open={page === "home" ? undefined : page}
        inert={displayLoading || pageLoading}
      >
        {/* A page with a scene of its own (the Armory's stage) reports it, so
            its cover holds until the scene draws. */}
        <LoadingTasks report={report}>
          <MenuBackdrop
            plate={plate}
            shown={shown.promise}
            onDetectionSceneReady={() => setShowDetection(false)}
          />
        </LoadingTasks>
        <LoadingTasks report={pageTasks.report}>
          <div className="menu-slot">
            <GraphicsDetectionHost runner={null} enabled={!loading}>
              <div className="hud-panel menu-body" ref={plate}>
                {children}
              </div>
            </GraphicsDetectionHost>
          </div>
        </LoadingTasks>
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
      {pageLoading && (
        <LoadingScreen
          title={menuPage(page).title}
          stages={PAGE_STAGES}
          current={pageTasks.progress.current}
          back={null}
        />
      )}
    </Shell>
  );
}
