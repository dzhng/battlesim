// The game's front door at `/`, in the HUD's look: start a battle on a
// generated map (its type, its size and, if the player cares, its region),
// play a saved battlefield of the catalogue, or watch a saved battle, and
// the sound settings. The test village, the benchmark and the labs are
// developer tools, behind the developer link.
//
// The menu is one plate over its backdrop battle. Opening one of its pages
// replaces the plate's contents, under a Back button, so the battle behind
// never changes; a loading screen stands in the plate's place until that
// battle is ready to film.
import { Link } from "react-router";
import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import { hudIcon } from "@packages/scene-assets/src/icons";
import { Icon } from "@web/battle/present/icons";
import config from "@fixtures/generated-battle.json";
import { listMaps } from "@web/maps/catalogue";
import { MAP_SIZES, MAP_TYPES, type MapSize, type MapType } from "@web/maps/source";
import { askedChoice, battleHref, playHref, REGIONS, savedBattleHref, spoken } from "./battleLinks";
import { useSavedReplay, replayRoute } from "./replayFile";
import { SoundControls } from "./SoundControls";
import { MenuBackdrop } from "./MenuBackdrop";
import { LOADING_STAGES, LoadingTasks, useLoadingTasks } from "./LabLoading";
import { LoadingScreen } from "./LoadingScreen";

interface Entry {
  label: string;
  href: string | null;
  note: string;
}

const DEVELOPER: Entry[] = [
  ...(import.meta.env.DEV
    ? [
        {
          label: "Mechanics editor",
          href: "/mechanics",
          note: "Tune unit and weapon values with validated JSON saves.",
        },
        {
          label: "Map workbench",
          href: "/map-workbench",
          note: "Tune generation and validation rules on a live plan.",
        },
        {
          label: "Sound workbench",
          href: "/sound-workbench",
          note: "Audition recordings and choose sounds for each unit type.",
        },
      ]
    : []),
  {
    label: "Village",
    href: "/battle/village",
    note: "The test village: attack it as blue.",
  },
  {
    label: "Benchmark",
    href: "/benchmark",
    note: "A scripted battle and camera tour, with frame timings.",
  },
  { label: "Labs", href: "/labs", note: "One focused fixture per mechanic." },
];

/** What each map type is, in the player's words. */
const TYPE_NOTE: Record<MapType, string> = {
  open: "Open country: villages, fields and woods.",
  mixed: "A town among fields and woods.",
  metro: "A city and the country round it.",
};

const RANDOM = "random";

/** The catalogue's battlefields a player can start: every released playable
 *  map that has the game's encounter saved on it. Each is the same map and
 *  the same deployment every time. */
export function savedBattles() {
  const recipe = config.encounter.recipe;
  return listMaps({ category: "playable", status: "released" })
    .filter((map) => map.encounters.includes(recipe))
    .map((map) => ({
      label: `Play ${map.label}`,
      href: savedBattleHref(map.id, recipe),
      note: "A fixed battlefield: attack the defended town as blue.",
    }));
}

const id = (href: string) => `menu${href.replaceAll(/[^a-z0-9]/g, "-")}`;

/** A tile's title and note, and the attributes that name the tile by its
 *  title and describe it by its note; `key` keeps its ids unique. */
function card(key: string, label: string, note: string) {
  const at = id(key);
  return {
    props: {
      className: "menu-card",
      "aria-labelledby": `${at}-label`,
      "aria-describedby": `${at}-note`,
    },
    text: (
      <>
        <span className="menu-card-label" id={`${at}-label`}>
          {label}
        </span>
        <span className="menu-card-note" id={`${at}-note`}>
          {note}
        </span>
      </>
    ),
  };
}

/** One entry, one link: a click anywhere on its tile navigates. */
function EntryItem({ entry: e }: { entry: Entry }) {
  const { props, text: content } = card(e.href ?? e.label, e.label, e.note);
  return (
    <li>
      {e.href === null ? (
        <a {...props} aria-disabled>
          {content}
        </a>
      ) : (
        <Link {...props} to={e.href}>
          {content}
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
  const [type, setType] = useState<MapType>(asked.type ?? "mixed");
  const [size, setSize] = useState<MapSize>(asked.size ?? "small");
  // Random leaves the region to the seed.
  const [region, setRegion] = useState(asked.region ?? RANDOM);
  const choice = { type, size, ...(region !== RANDOM && { region }) };
  const deploy = card("deploy", "Deploy", "Attack the defended town as blue.");
  return (
    <section className="menu-battle" aria-label="New battle">
      <div className="menu-fields">
        <Choice label="map" options={MAP_TYPES} value={type} onChange={setType} />
        <Choice label="size" options={MAP_SIZES} value={size} onChange={setSize} />
        <Choice label="region" options={[RANDOM, ...REGIONS]} value={region} onChange={setRegion} />
      </div>
      <p className="menu-battle-note" data-testid="menu-note">
        {TYPE_NOTE[type]}
      </p>
      <Link
        {...deploy.props}
        className="menu-card menu-deploy"
        data-testid="menu-deploy"
        to={asked.seed ? battleHref({ ...choice, seed: asked.seed }) : playHref(choice)}
      >
        {deploy.text}
      </Link>
    </section>
  );
}

/** The plate's pages: each opens in the plate's place, under a Back button. */
const PAGES = {
  skirmish: { title: "Skirmish", note: "A battle on a new generated map." },
  battlefields: { title: "Battlefields", note: "A fixed battlefield of the catalogue." },
  settings: { title: "Settings", note: "Sound and volume." },
  // Opened by the quiet developer link, not a tile.
  developer: { title: "Developer" },
} as const;
type Page = keyof typeof PAGES;
type TilePage = Exclude<Page, "developer">;

/** A page's entry in the list: a tile like a link's, opening the page. */
function PageEntry({ page, open }: { page: TilePage; open: (page: Page) => void }) {
  const { props, text } = card(`page-${page}`, PAGES[page].title, PAGES[page].note);
  return (
    <li>
      <button {...props} type="button" data-page={page} onClick={() => open(page)}>
        {text}
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
  const [savedReplay] = useSavedReplay();
  const plate = useRef<HTMLDivElement>(null);
  const { report, progress } = useLoadingTasks();
  // Covered only until the backdrop is first ready: its restarts happen
  // behind its own veil. One that cannot be made leaves the plain ground.
  const [revealed, setRevealed] = useState(false);
  const loading = !revealed && !progress.done && !progress.error;
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
  const replay: Entry = {
    label: "Watch replay",
    href: savedReplay.file === undefined ? null : replayRoute(savedReplay.file),
    note: savedReplay.file === undefined ? "Reading saved battle…" : "Load a saved battle.",
  };
  const content =
    page === null ? (
      <>
        <h1>Battle</h1>
        <nav aria-label="Main menu">
          <ul>
            <PageEntry page="skirmish" open={setPage} />
            <PageEntry page="battlefields" open={setPage} />
            <EntryItem entry={replay} />
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
      </>
    ) : (
      <PageView key={page} page={page} back={back}>
        {page === "skirmish" && <NewBattle asked={asked} />}
        {page === "battlefields" && <Entries label="Battlefields" entries={savedBattles()} />}
        {page === "settings" && <SoundControls />}
        {page === "developer" && <Entries label="Developer" entries={DEVELOPER} />}
      </PageView>
    );
  return (
    <>
      <main className="menu" inert={loading}>
        <LoadingTasks report={report}>
          <MenuBackdrop plate={plate} shown={shown.promise} />
        </LoadingTasks>
        <div className="hud-panel menu-body" ref={plate}>
          {content}
        </div>
      </main>
      {loading && (
        <LoadingScreen
          title="Battle"
          stages={LOADING_STAGES}
          current={progress.current}
          back={null}
        />
      )}
    </>
  );
}
