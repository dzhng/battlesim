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

export function MainMenu() {
  return (
    <main className="menu">
      <div className="menu-body">
        <h1>Battle</h1>
        <nav aria-label="Main menu">
          <ul>
            {ENTRIES.map((e) => (
              <li key={e.href}>
                <a href={e.href}>{e.label}</a>
                <span>{e.note}</span>
              </li>
            ))}
          </ul>
        </nav>
        <SoundControls />
      </div>
    </main>
  );
}
