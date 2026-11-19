// One visit inside the menu shell: home, or a page under its title and a Back
// button. Escape is Back, as in a game's menus.
import { lazy, Suspense, useEffect, useRef, type ComponentType } from "react";
import { PageHead } from "./PageHead";
import { DeveloperPage } from "./DeveloperPage";
import { Home } from "./Home";
import { useLabLoading } from "../LabLoading";
import { useMenuShell } from "./MenuShell";
import { menuPage, type MenuPageId } from "./pages";
import { SettingsPage } from "./SettingsPage";
import { MultiplayerPage } from "../online/MultiplayerPage";
import { RoomPage } from "../online/RoomPage";
import { SkirmishPage } from "./SkirmishPage";

/** The Armory loads with its page: its stage, its yard document and its art
 *  library stay out of the menu until a visit asks for them, so a fault in
 *  them can only ever reach the Armory's own page. */
const ArmoryVisit = lazy(() =>
  import("../armory/ArmoryPage").then((page) => ({ default: page.ArmoryVisit })),
);

/** What each page draws under its title. */
const CONTENTS: Record<MenuPageId, ComponentType> = {
  multiplayer: MultiplayerPage,
  room: RoomPage,
  skirmish: SkirmishPage,
  armory: ArmoryVisit,
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
    const onKey = (e: KeyboardEvent) => page !== "room" && e.key === "Escape" && back();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [back, page]);
  const Contents = CONTENTS[page];
  return (
    <>
      {page !== "multiplayer" && page !== "room" && (
        <PageHead title={menuPage(page).title} onBack={back} backRef={backButton} />
      )}
      <Suspense fallback={<PageLoading />}>
        <Contents />
      </Suspense>
    </>
  );
}

/** A page whose code is still loading holds the shell's cover, so the cover
 *  lifts once, on the page as it first draws. */
function PageLoading() {
  useLabLoading("preparation", false);
  return null;
}
