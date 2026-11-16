// The menu's Developer page: the developer tools, each a link, and where the
// game's source lives for anyone who wants to contribute. The source editors
// are offered only in development builds.
import { Link } from "react-router";
import { SOURCE_REPO } from "../buildIdentity";

interface Entry {
  label: string;
  href: string;
}

const DEVELOPER: Entry[] = [
  ...(import.meta.env.DEV
    ? [
        { label: "Mechanics editor", href: "/labs/mechanics" },
        { label: "Map workbench", href: "/labs/map-workbench" },
        { label: "Scenario builder", href: "/labs/scenario" },
        { label: "Scenario play", href: "/labs/scenario?mode=play" },
        { label: "Sound workbench", href: "/labs/sound-workbench" },
      ]
    : []),
  { label: "Benchmark", href: "/labs/benchmark" },
  { label: "Labs", href: "/labs" },
];

export function DeveloperPage() {
  return (
    <>
      <nav aria-label="Developer">
        <ul>
          {DEVELOPER.map((e) => (
            // One entry, one link: a click anywhere on its tile navigates.
            <li key={e.href}>
              <Link className="menu-card" to={e.href} target="_self">
                <span className="menu-card-label">{e.label}</span>
              </Link>
            </li>
          ))}
        </ul>
      </nav>
      <aside className="menu-notice" aria-label="Source">
        <p>
          Open source, and contributors are welcome:{" "}
          <a href={SOURCE_REPO} target="_blank" rel="noreferrer">
            {SOURCE_REPO.replace(/^https:\/\//, "")}
          </a>
        </p>
      </aside>
    </>
  );
}
