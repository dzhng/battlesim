// One visit inside the menu shell: home, or a page under its title and a Back
// button. Escape is Back, as in a game's menus.
import { useEffect, useRef, type ComponentType } from "react";
import { hudIcon } from "@packages/scene-assets/src/icons";
import { Icon } from "@web/battle/present/icons";
import { Tutorial } from "../Tutorial";
import { DeveloperPage } from "./DeveloperPage";
import { Home } from "./Home";
import { useMenuShell } from "./MenuShell";
import { menuPage, type MenuPageId } from "./pages";
import { SettingsPage } from "./SettingsPage";
import { SkirmishPage } from "./SkirmishPage";

/** What each page draws under its title. */
const CONTENTS: Record<MenuPageId, ComponentType> = {
  skirmish: SkirmishPage,
  tutorial: Tutorial,
  settings: SettingsPage,
  developer: DeveloperPage,
};

export function MenuVisit({ page }: { page: MenuPageId | "home" }) {
  return page === "home" ? <Home /> : <PageView page={page} />;
}

/** An open page: its title beside the Back button, then its contents. */
function PageView({ page }: { page: MenuPageId }) {
  const { back } = useMenuShell();
  const backButton = useRef<HTMLButtonElement>(null);
  useEffect(() => backButton.current?.focus(), []);
  // Bubbling on the window, so a control that handles Escape itself (the
  // lobby's dropdowns) has it first.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && back();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [back]);
  const Contents = CONTENTS[page];
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
        <h2>{menuPage(page).title}</h2>
      </header>
      <Contents />
    </>
  );
}
