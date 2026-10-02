// The game's front door at `/`, in the HUD's look: start a battle on a
// generated map (its type and its size), play a saved battlefield of the
// catalogue, or watch a saved battle, and the sound settings. The test
// village, the benchmark and the labs are
// developer tools, behind the developer link.
import { useState } from "react";
import config from "@fixtures/generated-battle.json";
import { listMaps } from "@web/maps/catalogue";
import {
  canonicalSeed,
  MAP_SIZES,
  MAP_TYPES,
  newSeed,
  type MapSize,
  type MapType,
} from "@web/maps/source";
import { askedChoice, battleHref, savedBattleHref } from "./battleLinks";
import { readSavedReplay, replayRoute } from "./replayFile";
import { SoundControls } from "./SoundControls";

interface Entry {
  label: string;
  href: string;
  note: string;
}

const DEVELOPER: Entry[] = [
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

/** The catalogue's battlefields a player can start: every released playable
 *  map that has the game's encounter saved on it. Each is the same map and
 *  the same deployment every time. */
export function savedBattles(): Entry[] {
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

/** A list of entries, each one link: its title names it, its note describes
 *  it, and a click anywhere on it navigates. */
function Entries({ label, entries }: { label: string; entries: Entry[] }) {
  return (
    <nav aria-label={label}>
      <ul>
        {entries.map((e) => (
          <li key={e.href}>
            <a
              className="menu-card"
              href={e.href}
              aria-labelledby={`${id(e.href)}-label`}
              aria-describedby={`${id(e.href)}-note`}
            >
              <span className="menu-card-label" id={`${id(e.href)}-label`}>
                {e.label}
              </span>
              <span className="menu-card-note" id={`${id(e.href)}-note`}>
                {e.note}
              </span>
            </a>
          </li>
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
            {option}
          </button>
        ))}
      </div>
    </>
  );
}

/** A new battle: the map's type and size, and the order to deploy. Every
 *  visit to the menu is a new map: the seed is drawn here and never shown.
 *  It rides in the battle's address, which is how a battle is shared or
 *  returned to, and an address that already names one is kept. */
function NewBattle() {
  const [asked] = useState(() => askedChoice(window.location.search));
  const [type, setType] = useState<MapType>(asked.type ?? "mixed");
  const [size, setSize] = useState<MapSize>(asked.size ?? "small");
  const [seed] = useState(() => canonicalSeed(asked.seed ?? "") ?? newSeed());
  return (
    <section className="menu-battle" aria-label="New battle">
      <h2>Skirmish</h2>
      <div className="menu-fields">
        <Choice label="map" options={MAP_TYPES} value={type} onChange={setType} />
        <Choice label="size" options={MAP_SIZES} value={size} onChange={setSize} />
      </div>
      <p className="menu-battle-note" data-testid="menu-note">
        {TYPE_NOTE[type]}
      </p>
      <a
        className="menu-card menu-deploy"
        data-testid="menu-deploy"
        href={battleHref({ type, size, seed })}
      >
        <span className="menu-card-label">Deploy</span>
        <span className="menu-card-note">Attack the defended town as blue.</span>
      </a>
    </section>
  );
}

export function MainMenu() {
  const [developer, setDeveloper] = useState(false);
  const [entries] = useState<Entry[]>(() => [
    ...savedBattles(),
    { label: "Watch replay", href: replayRoute(readSavedReplay()), note: "Load a saved battle." },
  ]);
  return (
    <main className="menu">
      <div className="hud-panel menu-body">
        <h1>Battle</h1>
        <NewBattle />
        <Entries label="Main menu" entries={entries} />
        <SoundControls />
        <button
          type="button"
          className="menu-dev"
          aria-expanded={developer}
          onClick={() => setDeveloper(!developer)}
        >
          Developer
        </button>
        {developer && <Entries label="Developer" entries={DEVELOPER} />}
      </div>
    </main>
  );
}
