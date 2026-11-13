// @vitest-environment jsdom
// Every menu page is an address under one menu shell. Moving between pages
// never restarts the backdrop or brings back its loading cover. Back and
// Escape return home onto the tile you left, and an address that names no
// menu page opens home.
import { act, cleanup, fireEvent, screen, waitFor } from "@testing-library/react";
import { useEffect } from "react";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { graphicsSettings } from "@apps/battle-lab/src/graphicsSettings";
import { useLabLoading } from "@apps/battle-lab/src/LabLoading";
import { menuPage, type MenuPageId } from "@apps/battle-lab/src/menu/pages";
import { PreviewWorker } from "./support/previewWorker";
import { visitApp } from "./support/router";

/** The backdrop, counted: each mount would be a restarted battle. It is ready
 *  at once, so the loading cover lifts. */
const backdrop = vi.hoisted(() => ({ mounts: 0 }));
vi.mock("@apps/battle-lab/src/MenuBackdrop", () => ({
  MenuBackdrop: function MenuBackdrop() {
    useEffect(() => {
      backdrop.mounts += 1;
    }, []);
    useLabLoading("renderer", true);
    return null;
  },
}));

beforeEach(() => {
  backdrop.mounts = 0;
  // A returning player: no first-visit graphics detection over the menu.
  graphicsSettings.set({ detectedMode: "medium" }, { persist: false });
  vi.stubGlobal("Worker", PreviewWorker);
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  graphicsSettings.set({ detectedMode: null }, { persist: false });
  window.history.replaceState(null, "", "/");
});

/** The app opened at `address`, its loading cover lifted. */
async function openAt(address: string) {
  await visitApp(address);
  await waitFor(() => expect(screen.queryByTestId("loading")).toBeNull());
}

/** Home is shown. History moves land a task later, so this waits. */
const home = () => screen.findByRole("navigation", { name: "Main menu" });
/** Page `id` is open, at its own address. */
async function onPage(id: MenuPageId) {
  await screen.findByRole("heading", { name: menuPage(id).title });
  expect(window.location.pathname).toBe(menuPage(id).path);
}
/** Home's tile for page `id` has the focus. */
const focused = (id: MenuPageId) =>
  waitFor(() =>
    expect(document.activeElement).toBe(screen.getByRole("button", { name: menuPage(id).title })),
  );
/** Open page `id` from home, as the player does. */
const open = (id: MenuPageId) =>
  fireEvent.click(screen.getByRole("button", { name: menuPage(id).title }));
const back = () => fireEvent.click(screen.getByRole("button", { name: "Back" }));

test("the backdrop mounts once across every menu page", async () => {
  await openAt("/");
  await home();
  const covered = () => expect(screen.queryByTestId("loading")).toBeNull();
  open("skirmish");
  await onPage("skirmish");
  covered();
  await act(async () => window.history.back());
  await home();
  covered();
  open("settings");
  await onPage("settings");
  covered();
  back();
  await home();
  open("developer");
  await onPage("developer");
  covered();
  expect(backdrop.mounts).toBe(1);
});

test("back and escape return home and focus the tile left", async () => {
  await openAt("/");
  const entered = window.history.length;
  open("skirmish");
  back();
  await home();
  expect(window.location.pathname).toBe("/");
  await focused("skirmish");
  open("settings");
  fireEvent.keyDown(window, { key: "Escape" });
  await home();
  await focused("settings");
  // Back went back in history: Forward returns to the page just left.
  await act(async () => window.history.forward());
  await onPage("settings");
  expect(window.history.length).toBe(entered + 1);
});

test("a page opened by address goes back by replacing with home", async () => {
  await openAt(menuPage("settings").path);
  await onPage("settings");
  const entries = window.history.length;
  back();
  await home();
  expect(window.location.pathname).toBe("/");
  expect(window.history.length).toBe(entries);
});

test("an old /?type= address opens the bare menu", async () => {
  await openAt("/?type=mixed&size=small&seed=42");
  await home();
  expect(screen.queryByRole("button", { name: "Back" })).toBeNull();
  expect(window.location.pathname).toBe("/");
});

test("an address that names no page opens home", async () => {
  await openAt("/no/such/page");
  await home();
  expect(screen.queryByRole("button", { name: "Back" })).toBeNull();
});
