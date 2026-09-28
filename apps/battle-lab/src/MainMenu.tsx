// The game's front door at `/`: play the village, watch a saved battle, run
// the benchmark, or open the labs; and the sound settings.
import { SoundControls } from "./SoundControls";

const ENTRIES = [
  { label: "Play village", href: "/battle/village", note: "Attack the defended village as blue." },
  { label: "Watch replay", href: "/replay/village", note: "Load a saved village battle." },
  {
    label: "Benchmark",
    href: "/benchmark",
    note: "A scripted battle and camera tour, with frame timings.",
  },
  { label: "Labs", href: "/labs", note: "One focused fixture per mechanic." },
];

const id = (href: string) => `menu${href.replaceAll("/", "-")}`;

export function MainMenu() {
  return (
    <main className="menu">
      <div className="menu-body">
        <h1>Battle</h1>
        <nav aria-label="Main menu">
          <ul>
            {ENTRIES.map((e) => (
              <li key={e.href}>
                {/* The whole card is the link: its title names it, its note
                    describes it, and a click anywhere on it navigates. */}
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
        <SoundControls />
      </div>
    </main>
  );
}
