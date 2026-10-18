import { act, cleanup, fireEvent, screen, waitFor } from "@testing-library/react";
import { useState } from "react";
import { BrowserRouter, Link, useLocation, useNavigate } from "react-router";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { graphicsSettings } from "@apps/battle-lab/src/graphicsSettings";
import { LabRouter, screenForPath } from "@apps/battle-lab/src/router";
import { GAME_NAME } from "@apps/battle-lab/src/gameName";
import { PreviewWorker } from "./support/previewWorker";
import { renderSettled } from "./support/router";

vi.mock("@apps/battle-lab/src/routes/battle", () => ({
  default: function Battle() {
    const location = useLocation();
    const navigate = useNavigate();
    const [asked] = useState(location.search);
    const [progress, setProgress] = useState(0);
    return (
      <>
        <p>Battle requested: {asked}</p>
        <p>Progress: {progress}</p>
        <Link to="/">Main menu</Link>
        <button
          onClick={() => {
            void navigate("/");
            window.history.back();
          }}
        >
          Leave and immediately return
        </button>
        <Link to="/battle?type=open&size=small&seed=99">Next battle</Link>
        <button onClick={() => setProgress(progress + 1)}>Advance</button>
      </>
    );
  },
}));

// A returning player, whose graphics were detected on an earlier visit. A first
// visit keeps the loading screen over the (inert) menu until detection ends,
// which jsdom never does; that screen is graphics detection's to test.
beforeEach(() => {
  graphicsSettings.set({ detectedMode: "medium" }, { persist: false });
  // The lobby's map previews are admitted at once, so Deploy opens.
  PreviewWorker.admitting = true;
  vi.stubGlobal("Worker", PreviewWorker);
});

afterEach(() => {
  cleanup();
  PreviewWorker.admitting = false;
  vi.unstubAllGlobals();
  graphicsSettings.set({ detectedMode: null }, { persist: false });
  window.history.replaceState(null, "", "/");
});

/** Open Skirmish, wait for its map, and press Deploy: the battle address it
 *  went to. */
async function deploy(): Promise<string> {
  fireEvent.click(screen.getByRole("button", { name: "Skirmish" }));
  const link = await waitFor(
    () => {
      const href = screen.getByTestId("menu-deploy").getAttribute("href");
      expect(href).toBeTruthy();
      return href!;
      // The first preview waits for the simulation module to load.
    },
    { timeout: 15_000 },
  );
  // Deploy enters the game set's scope, which suspends (see `renderSettled`).
  await act(async () => fireEvent.click(screen.getByTestId("menu-deploy")));
  return link;
}

test("Deploy changes the actual route without replacing the document", async () => {
  const view = await renderSettled(
    <BrowserRouter unstable_useTransitions={false}>
      <LabRouter />
    </BrowserRouter>,
  );
  const documentElement = document.documentElement;
  const link = await deploy();
  expect(link).toMatch(/^\/battle\?/);
  expect(
    await screen.findByText(`Battle requested: ${link.slice("/battle".length)}`),
  ).toBeDefined();
  expect(window.location.pathname + window.location.search).toBe(link);
  expect(document.documentElement).toBe(documentElement);
  expect(view.queryByRole("heading", { name: GAME_NAME })).toBeNull();
});

test("same-path navigation and Back/Forward prepare fresh requested visits", async () => {
  await renderSettled(
    <BrowserRouter unstable_useTransitions={false}>
      <LabRouter />
    </BrowserRouter>,
  );
  const link = await deploy();
  const asked = `Battle requested: ${link.slice("/battle".length)}`;
  fireEvent.click(await screen.findByRole("button", { name: "Advance" }));
  fireEvent.click(screen.getByRole("link", { name: "Next battle" }));
  expect(await screen.findByText("Battle requested: ?type=open&size=small&seed=99")).toBeDefined();
  expect(screen.getByText("Progress: 0")).toBeDefined();
  fireEvent.click(screen.getByRole("button", { name: "Advance" }));
  await act(async () => {
    window.history.back();
  });
  await screen.findByText(asked);
  expect(screen.getByText("Progress: 0")).toBeDefined();
  await act(async () => {
    window.history.back();
  });
  await screen.findByRole("heading", { name: GAME_NAME });
  await act(async () => {
    window.history.forward();
  });
  await screen.findByText(asked);
  expect(screen.getByText("Progress: 0")).toBeDefined();
  fireEvent.click(screen.getByRole("link", { name: "Main menu" }));
  await waitFor(() => expect(screen.queryByText(/Battle requested:/)).toBeNull());
});

test("menu sound policy follows real page resolution, including fallback and player replay", () => {
  for (const path of ["/", "/unknown", "/battle/unknown"])
    expect(screenForPath(path), path).toBe("menu");
  for (const path of ["/battle", "/battle/"]) expect(screenForPath(path), path).toBe("loading");
  for (const path of [
    "/labs",
    "/labs/panels",
    "/labs/street",
    "/labs/benchmark",
    "/labs/workbench",
  ])
    expect(screenForPath(path), path).toBe("other");
});

test("a rapid history round trip discards progress before the intermediate screen commits", async () => {
  await renderSettled(
    <BrowserRouter unstable_useTransitions={false}>
      <LabRouter />
    </BrowserRouter>,
  );
  const link = await deploy();
  fireEvent.click(await screen.findByRole("button", { name: "Advance" }));
  await act(async () => {
    const returned = new Promise<void>((resolve) =>
      window.addEventListener("popstate", () => resolve(), { once: true }),
    );
    fireEvent.click(screen.getByRole("button", { name: "Leave and immediately return" }));
    await returned;
  });
  expect(await screen.findByText("Progress: 0")).toBeDefined();
  expect(window.location.pathname + window.location.search).toBe(link);
});
