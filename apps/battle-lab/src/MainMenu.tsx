// The game's front door at `/`, in the HUD's look: start a battle on a
// generated map (its type, its size and, if the player cares, its region),
// read how to play, and the sound settings. The benchmark and the labs are
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
import { MAP_SIZES, MAP_TYPES, type MapSize, type MapType } from "@web/maps/source";
import { FACTIONS } from "@packages/scene-assets/src/units";
import { askedChoice, battleHref, playHref, REGIONS, spoken } from "./battleLinks";
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
import {
  GRAPHICS_TIERS,
  graphicsDetectionState,
  subscribeGraphicsDetection,
} from "./graphicsDetection";
import { graphicsSettings } from "./graphicsSettings";

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
          href: "/labs/scenario-builder",
        },
        {
          label: "Scenario play",
          href: "/labs/scenario-play",
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

const RANDOM = "random";

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

/** One of a few named options, as the pause menu's variants are chosen. */
function Choice<T extends string>({
  label,
  options,
  value,
  onChange,
}: {
  label: string;
  options: readonly T[];
  value: T;
  onChange: (value: T) => void;
}) {
  return (
    <>
      <span className="menu-field" id={`menu-${label}`}>
        {label}
      </span>
      <div role="radiogroup" aria-labelledby={`menu-${label}`} className="menu-options">
        {options.map((option) => (
          <button
            key={option}
            type="button"
            role="radio"
            aria-checked={option === value}
            className="hud-menu-choice"
            data-testid={`menu-${label}-${option}`}
            onClick={() => onChange(option)}
          >
            {spoken(option)}
          </button>
        ))}
      </div>
    </>
  );
}

/** Ordinary Play selects a battlefield after deployment. An explicit seed
 * carried by a menu address still asks for that exact battle. */
function NewBattle({ asked }: { asked: ReturnType<typeof askedChoice> }) {
  const [faction, setFaction] = useState(asked.faction ?? "us");
  const [type, setType] = useState<MapType>(asked.type ?? "mixed");
  const [size, setSize] = useState<MapSize>(asked.size ?? "small");
  // Random leaves the region to the seed.
  const [region, setRegion] = useState(asked.region ?? RANDOM);
  const choice = {
    type,
    size,
    faction,
    ...(region !== RANDOM && { region }),
  };
  return (
    <section className="menu-battle" aria-label="New battle">
      <div className="menu-fields">
        <Choice label="faction" options={FACTIONS} value={faction} onChange={setFaction} />
        <Choice label="map" options={MAP_TYPES} value={type} onChange={setType} />
        <Choice label="size" options={MAP_SIZES} value={size} onChange={setSize} />
        <Choice label="region" options={[RANDOM, ...REGIONS]} value={region} onChange={setRegion} />
      </div>
      <Link
        className="menu-card menu-deploy"
        data-testid="menu-deploy"
        target="_self"
        to={asked.seed ? battleHref({ ...choice, seed: asked.seed }) : playHref(choice)}
      >
        <span className="menu-card-label">Deploy</span>
      </Link>
    </section>
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
    if (detection.status !== "complete") return;
    const dismiss = setTimeout(() => setShowDetection(false), 900);
    return () => clearTimeout(dismiss);
  }, [detection.status]);
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
        {page === "skirmish" && <NewBattle asked={asked} />}
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
          <MenuBackdrop plate={plate} shown={shown.promise} />
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
