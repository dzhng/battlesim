// The game's front door at `/`, in the HUD's look: a tile for each of the
// menu's pages (`pages.ts`): set up a skirmish, read how to play, and the
// settings. The benchmark and the labs are developer tools, behind the
// developer link (kept in production builds: its audience is technical).
import { useEffect, useState } from "react";
import { useNavigate } from "react-router";
import { Wordmark } from "../Wordmark";
import { APP_COMMIT, SOURCE_REPO, simFingerprint } from "../buildIdentity";
import { MENU_PAGES, menuPage, type MenuPageId } from "./pages";
import { FROM_MENU, useMenuShell } from "./MenuShell";

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

/** What a visitor should know before playing, under the pre-alpha tag beside
 *  the name: the game is unfinished, and its source is open to read and to
 *  contribute to. */
function ReleaseNotice() {
  return (
    <aside className="menu-notice" aria-label="About this build">
      <p>In active development: things can break at any time.</p>
      <p>
        Open source, and contributors are welcome:{" "}
        <a href={SOURCE_REPO} target="_blank" rel="noreferrer">
          {SOURCE_REPO.replace(/^https:\/\//, "")}
        </a>
      </p>
    </aside>
  );
}

export function Home() {
  const navigate = useNavigate();
  // Back to the list, onto the entry of the page just left.
  const { left } = useMenuShell();
  const open = (page: MenuPageId) => void navigate(menuPage(page).path, FROM_MENU);
  return (
    <>
      <h1>
        <Wordmark />
        <span className="menu-stage">Pre-alpha</span>
      </h1>
      <nav aria-label="Main menu">
        <ul>
          {MENU_PAGES.filter((row) => row.tile).map((row) => (
            <li key={row.id}>
              <button
                className="menu-card"
                data-art={row.id}
                type="button"
                data-page={row.id}
                autoFocus={left === row.id}
                onClick={() => open(row.id)}
              >
                <span className="menu-card-label">{row.title}</span>
              </button>
            </li>
          ))}
        </ul>
      </nav>
      <ReleaseNotice />
      <button
        type="button"
        className="menu-dev"
        data-page="developer"
        autoFocus={left === "developer"}
        onClick={() => open("developer")}
      >
        {menuPage("developer").title}
      </button>
      <BuildLine />
    </>
  );
}
