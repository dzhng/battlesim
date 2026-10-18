// The game's front door at `/`, in the HUD's look: set up a skirmish in its
// lobby (`lobby/Lobby.tsx`) and deploy it, read how to play, and the
// settings. The benchmark and the labs are
// developer tools, behind
// the developer link (kept in production builds: its audience is technical).
//
// The menu is one plate over its backdrop battle. Opening one of its pages
// replaces the plate's contents, under a Back button, so the battle behind
// never changes; a loading screen stands in the plate's place until that
// battle is ready to film.
import { Link } from "react-router";
import {
  useCallback,
  useEffect,
  useRef,
  useState,
  useSyncExternalStore,
  type ReactNode,
} from "react";
import { hudIcon } from "@packages/scene-assets/src/icons";
import { Icon } from "@web/battle/present/icons";
import { askedChoice } from "./battleLinks";
import { Lobby } from "./lobby/Lobby";
import { DiagnosticsControls } from "./DiagnosticsControls";
import { GraphicsSettingsControls } from "./GraphicsSettingsControls";
import { SoundControls } from "./SoundControls";
import { Tutorial } from "./Tutorial";
import { MenuBackdrop } from "./MenuBackdrop";
import { LOADING_STAGES, LoadingTasks, useLoadingTasks } from "./LabLoading";
import { LoadingScreen } from "./LoadingScreen";
import { Wordmark } from "./Wordmark";
import { APP_COMMIT, simFingerprint } from "./buildIdentity";
import { GraphicsDetectionHost } from "./GraphicsDetectionHost";
import { graphicsDetectionState, subscribeGraphicsDetection } from "./graphicsDetection";
import { GRAPHICS_TIERS, graphicsSettings } from "./graphicsSettings";
import { allowRuntimePrefetch } from "./gameAppearances";

interface Entry {
  label: string;
  href: string | null;
}

const DEVELOPER: Entry[] = [
  ...(import.meta.env.DEV
    ? [
        {
          label: "Mechanics editor",
          href: "/labs/mechanics",
        },
        {
          label: "Map workbench",
          href: "/labs/map-workbench",
        },
        {
          label: "Scenario builder",
          href: "/labs/scenario",
        },
        {
          label: "Scenario play",
          href: "/labs/scenario?mode=play",
        },
        {
          label: "Sound workbench",
          href: "/labs/sound-workbench",
        },
      ]
    : []),
  {
    label: "Benchmark",
    href: "/labs/benchmark",
  },
  { label: "Labs", href: "/labs" },
];

/** One entry, one link: a click anywhere on its tile navigates. */
function EntryItem({ entry: e }: { entry: Entry }) {
  return (
    <li>
      {e.href === null ? (
        <a className="menu-card" aria-disabled>
          <span className="menu-card-label">{e.label}</span>
        </a>
      ) : (
        <Link className="menu-card" to={e.href} target="_self">
          <span className="menu-card-label">{e.label}</span>
        </Link>
      )}
    </li>
  );
}

/** A list of entries. */
function Entries({ label, entries }: { label: string; entries: Entry[] }) {
  return (
    <nav aria-label={label}>
      <ul>
        {entries.map((e) => (
          <EntryItem key={e.href ?? e.label} entry={e} />
        ))}
      </ul>
    </nav>
  );
}

/** Which build is running: the app's commit and the simulation module's
 *  fingerprint (`buildIdentity.ts`), small, under everything else. */
function BuildLine() {
  const [sim, setSim] = useState<string | null>(null);
  useEffect(() => {
    let live = true;
    void simFingerprint().then((h) => live && setSim(h));
    return () => {
      live = false;
    };
  }, []);
  return (
    <p className="menu-build" data-testid="menu-build">
      build {APP_COMMIT} · sim {sim ?? "…"}
    </p>
  );
}

/** The plate's pages: each opens in the plate's place, under a Back button. */
const PAGES = {
  skirmish: { title: "Skirmish" },
  tutorial: { title: "Tutorial" },
  settings: { title: "Settings" },
  // Opened by the quiet developer link, not a tile.
  developer: { title: "Developer" },
} as const;
type Page = keyof typeof PAGES;
type TilePage = Exclude<Page, "developer">;

/** A page's entry in the list: a tile like a link's, opening the page. */
function PageEntry({ page, open }: { page: TilePage; open: (page: Page) => void }) {
  return (
    <li>
      <button
        className="menu-card"
        data-art={page}
        type="button"
        data-page={page}
        onClick={() => open(page)}
      >
        <span className="menu-card-label">{PAGES[page].title}</span>
      </button>
    </li>
  );
}

/** An open page: its title beside the Back button, then its contents. */
function PageView({ page, back, children }: { page: Page; back: () => void; children: ReactNode }) {
  const backButton = useRef<HTMLButtonElement>(null);
  useEffect(() => backButton.current?.focus(), []);
  return (
    <>
      <header className="menu-page-head">
        <button
          ref={backButton}
          type="button"
          className="menu-back"
          aria-label="Back"
          onClick={back}
        >
          <Icon path={hudIcon("back")} />
        </button>
        <h2>{PAGES[page].title}</h2>
      </header>
      {children}
    </>
  );
}

export function MainMenu() {
  const [asked] = useState(() => askedChoice(window.location.search));
  // An address asking for a battle opens on the page that starts it.
  const [page, setPage] = useState<Page | null>(
    Object.values(asked).some((v) => v !== undefined) ? "skirmish" : null,
  );
  const plate = useRef<HTMLDivElement>(null);
  const { report, progress } = useLoadingTasks();
  const detection = useSyncExternalStore(subscribeGraphicsDetection, graphicsDetectionState);
  const settings = useSyncExternalStore(graphicsSettings.subscribe, graphicsSettings.get);
  // Covered only until the backdrop is first ready: its restarts happen
  // behind its own veil. One that cannot be made leaves the plain ground.
  const [revealed, setRevealed] = useState(false);
  const loading = !revealed && !progress.done && !progress.error;
  const detectionNeeded = settings.mode === "auto" && !settings.detectedMode;
  const [showDetection, setShowDetection] = useState(false);
  useEffect(() => {
    if (!loading && detectionNeeded) setShowDetection(true);
  }, [loading, detectionNeeded]);
  useEffect(() => {
    if (loading || (detectionNeeded && detection.status !== "complete")) return;
    allowRuntimePrefetch();
  }, [loading, detectionNeeded, detection.status]);
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
  const detectionFailure =
    detection.status === "failed" && showDetection
      ? { message: "Graphics detection failed.", details: [detection.error ?? "Unknown error"] }
      : null;
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
  // Back to the list, onto the entry of the page just left.
  const left = useRef<Page | null>(null);
  const back = useCallback(() => {
    left.current = page;
    setPage(null);
  }, [page]);
  useEffect(() => {
    if (page !== null || left.current === null) return;
    plate.current?.querySelector<HTMLElement>(`[data-page="${left.current}"]`)?.focus();
    left.current = null;
  }, [page]);
  useEffect(() => {
    if (page === null) return;
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && back();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [page, back]);
  const content =
    page === null ? (
      <>
        <h1>
          <Wordmark />
        </h1>
        <nav aria-label="Main menu">
          <ul>
            <PageEntry page="skirmish" open={setPage} />
            <PageEntry page="tutorial" open={setPage} />
            <PageEntry page="settings" open={setPage} />
          </ul>
        </nav>
        <button
          type="button"
          className="menu-dev"
          data-page="developer"
          onClick={() => setPage("developer")}
        >
          Developer
        </button>
        <BuildLine />
      </>
    ) : (
      <PageView key={page} page={page} back={back}>
        {page === "skirmish" && <Lobby asked={asked} />}
        {page === "tutorial" && <Tutorial />}
        {page === "settings" && (
          <>
            <GraphicsSettingsControls />
            <SoundControls />
            <DiagnosticsControls />
          </>
        )}
        {page === "developer" && <Entries label="Developer" entries={DEVELOPER} />}
      </PageView>
    );
  return (
    <>
      <main className="menu" data-open={page ?? undefined} inert={displayLoading}>
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
              {content}
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
          failure={detectionFailure}
          recovery={
            detectionFailure ? (
              <button
                type="button"
                className="hud-menu-item"
                onClick={() => window.location.reload()}
              >
                Reload
              </button>
            ) : undefined
          }
          back={null}
        />
      )}
    </>
  );
}
