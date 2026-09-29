// The game's front door at `/`, in the HUD's look: play the village or watch
// a saved battle, and the sound settings. The benchmark and the labs are
// developer tools, behind the developer link.
import { useState } from "react";
import { SoundControls } from "./SoundControls";

interface Entry {
  label: string;
  href: string;
  note: string;
}

const ENTRIES: Entry[] = [
  { label: "Play village", href: "/battle/village", note: "Attack the defended village as blue." },
  { label: "Watch replay", href: "/replay/village", note: "Load a saved village battle." },
];

const DEVELOPER: Entry[] = [
  {
    label: "Benchmark",
    href: "/benchmark",
    note: "A scripted battle and camera tour, with frame timings.",
  },
  { label: "Labs", href: "/labs", note: "One focused fixture per mechanic." },
];

const id = (href: string) => `menu${href.replaceAll("/", "-")}`;

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

export function MainMenu() {
  const [developer, setDeveloper] = useState(false);
  return (
    <main className="menu">
      <div className="hud-panel menu-body">
        <h1>Battle</h1>
        <Entries label="Main menu" entries={ENTRIES} />
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
