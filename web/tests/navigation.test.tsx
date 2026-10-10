import { act, cleanup, fireEvent, screen, waitFor } from "@testing-library/react";
import { useState } from "react";
import { BrowserRouter, Link, useLocation, useNavigate } from "react-router";
import { afterEach, expect, test, vi } from "vitest";
import { usePublishBattleAddress } from "@apps/battle-lab/src/navigation";
import { LabRouter, screenForPath } from "@apps/battle-lab/src/router";
import { GAME_NAME } from "@apps/battle-lab/src/gameName";
import { renderSettled } from "./support/router";

const delayed = vi.hoisted(() => ({ resume: null as null | (() => void) }));

vi.mock("@apps/battle-lab/src/routes/battle", () => ({
  default: function Battle() {
    const location = useLocation();
    const navigate = useNavigate();
    const [asked] = useState(location.search);
    const publish = usePublishBattleAddress();
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
        <button
          onClick={() => {
            void new Promise<void>((resolve) => {
              delayed.resume = resolve;
            }).then(() => publish("/battle?type=open&size=small&seed=123"));
          }}
        >
          Queue publication
        </button>
        <button onClick={() => publish("/battle?type=open&size=small&seed=42")}>
          Publish admitted battle
        </button>
      </>
    );
  },
}));

afterEach(() => {
  cleanup();
  window.history.replaceState(null, "", "/");
});

test("Deploy changes the actual route without replacing the document", async () => {
  const view = await renderSettled(
    <BrowserRouter unstable_useTransitions={false}>
      <LabRouter />
    </BrowserRouter>,
  );
  const documentElement = document.documentElement;
  fireEvent.click(screen.getByRole("button", { name: "Skirmish" }));
  // Deploy enters the game set's scope, which suspends (see `renderSettled`).
  await act(async () => fireEvent.click(screen.getByTestId("menu-deploy")));
  expect(
    await screen.findByText("Battle requested: ?play=1&type=mixed&size=small&faction=us"),
  ).toBeDefined();
  expect(window.location.pathname + window.location.search).toBe(
    "/battle?play=1&type=mixed&size=small&faction=us",
  );
  expect(document.documentElement).toBe(documentElement);
  expect(view.queryByRole("heading", { name: GAME_NAME })).toBeNull();
});

test("publishing the admitted address preserves progress and unrelated history state", async () => {
  window.history.replaceState(
    { usr: { unrelated: "retained" } },
    "",
    "/battle?play=1&type=mixed&size=small&faction=us",
  );
  await renderSettled(
    <BrowserRouter unstable_useTransitions={false}>
      <LabRouter />
    </BrowserRouter>,
  );
  fireEvent.click(await screen.findByRole("button", { name: "Advance" }));
  fireEvent.click(screen.getByRole("button", { name: "Publish admitted battle" }));
  expect(
    screen.getByText("Battle requested: ?play=1&type=mixed&size=small&faction=us"),
  ).toBeDefined();
  expect(screen.getByText("Progress: 1")).toBeDefined();
  expect(window.location.search).toBe("?type=open&size=small&seed=42");
  expect(window.history.state.usr.unrelated).toBe("retained");
});

test("same-path navigation and Back/Forward prepare fresh requested visits", async () => {
  await renderSettled(
    <BrowserRouter unstable_useTransitions={false}>
      <LabRouter />
    </BrowserRouter>,
  );
  fireEvent.click(screen.getByRole("button", { name: "Skirmish" }));
  await act(async () => fireEvent.click(screen.getByTestId("menu-deploy")));
  fireEvent.click(await screen.findByRole("button", { name: "Advance" }));
  fireEvent.click(screen.getByRole("button", { name: "Publish admitted battle" }));
  fireEvent.click(screen.getByRole("link", { name: "Next battle" }));
  expect(await screen.findByText("Battle requested: ?type=open&size=small&seed=99")).toBeDefined();
  expect(screen.getByText("Progress: 0")).toBeDefined();
  fireEvent.click(screen.getByRole("button", { name: "Advance" }));
  await act(async () => {
    window.history.back();
  });
  await screen.findByText("Battle requested: ?type=open&size=small&seed=42");
  expect(screen.getByText("Progress: 0")).toBeDefined();
  await act(async () => {
    window.history.back();
  });
  await screen.findByRole("heading", { name: GAME_NAME });
  await act(async () => {
    window.history.forward();
  });
  await screen.findByText("Battle requested: ?type=open&size=small&seed=42");
  expect(screen.getByText("Progress: 0")).toBeDefined();
  fireEvent.click(screen.getByRole("link", { name: "Main menu" }));
  await waitFor(() => expect(screen.queryByText(/Battle requested:/)).toBeNull());
});

test("menu sound policy follows real page resolution, including fallback and player replay", () => {
  for (const path of ["/", "/unknown", "/battle/unknown"])
    expect(screenForPath(path), path).toBe("menu");
  for (const path of ["/battle", "/battle/"]) expect(screenForPath(path), path).toBe("loading");
  for (const path of ["/labs", "/lab/panels", "/lab/street", "/benchmark", "/workbench"])
    expect(screenForPath(path), path).toBe("other");
});

test("a rapid history round trip discards progress before the intermediate screen commits", async () => {
  await renderSettled(
    <BrowserRouter unstable_useTransitions={false}>
      <LabRouter />
    </BrowserRouter>,
  );
  fireEvent.click(screen.getByRole("button", { name: "Skirmish" }));
  await act(async () => fireEvent.click(screen.getByTestId("menu-deploy")));
  fireEvent.click(await screen.findByRole("button", { name: "Advance" }));
  fireEvent.click(screen.getByRole("button", { name: "Queue publication" }));
  await act(async () => {
    const returned = new Promise<void>((resolve) =>
      window.addEventListener("popstate", () => resolve(), { once: true }),
    );
    fireEvent.click(screen.getByRole("button", { name: "Leave and immediately return" }));
    await returned;
  });
  expect(await screen.findByText("Progress: 0")).toBeDefined();
  await act(async () => {
    delayed.resume!();
  });
  expect(window.location.search).toBe("?play=1&type=mixed&size=small&faction=us");
});
