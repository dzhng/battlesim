// @vitest-environment jsdom
// @vitest-environment-options {"url":"https://page.invalid"}
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import { BrowserRouter, Route, Routes, useLocation } from "react-router";
import { PageVisit } from "../../../apps/battle-lab/src/navigation";
import OnlineDev from "../../../apps/battle-lab/src/routes/onlineDev";
import type { OnlineBattleEntry } from "../../../apps/battle-lab/src/onlineSession";

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.unstubAllEnvs();
  window.history.replaceState(null, "", "/");
});
function BattleArrival() {
  const entry = useLocation().state as OnlineBattleEntry;
  return <output aria-label="Arrived battle">{entry.ticket.battle}</output>;
}
function open() {
  window.history.replaceState(null, "", "/online/dev?server=http://127.0.0.1:8787");
  render(
    <BrowserRouter>
      <PageVisit>
        <Routes>
          <Route path="/online/dev" element={<OnlineDev />} />
          <Route path="/online/battle/:id" element={<BattleArrival />} />
        </Routes>
      </PageVisit>
    </BrowserRouter>,
  );
}
function network(refused: boolean) {
  const paths: string[] = [];
  vi.stubGlobal("fetch", async (input: string) => {
    const path = new URL(input, "http://localhost").pathname;
    paths.push(path);
    if (path.endsWith(".wasm")) return new Response(new Uint8Array([0, 97, 115, 109]));
    if (path === "/hello")
      return Response.json(refused ? { refused: "build" } : { callsign: "SCOUT" });
    if (path === "/dev/battle") return Response.json({ battle: "dev" });
    throw new Error("unexpected request");
  });
  return paths;
}

test("a build refusal leaves the page unavailable with no online entry or seat request", async () => {
  const paths = network(true);
  open();
  const alert = await screen.findByRole("alert");
  expect(alert).toBeTruthy();
  expect(window.location.pathname).toBe("/online/dev");
  expect(paths).toContain("/hello");
  expect(paths).not.toContain("/dev/battle");
  expect(screen.queryByLabelText("Arrived battle")).toBeNull();
  const links = screen.getAllByRole("link");
  expect(links.map((link) => link.getAttribute("href"))).toEqual(["/"]);
});

test("a welcomed page requests its ticket after hello and reaches the dev battle", async () => {
  const paths = network(false);
  open();
  await waitFor(() => expect(screen.getByLabelText("Arrived battle").textContent).toBe("dev"));
  expect(paths.indexOf("/hello")).toBeGreaterThanOrEqual(0);
  expect(paths.indexOf("/dev/battle")).toBeGreaterThan(paths.indexOf("/hello"));
  expect(window.location.pathname).toBe("/online/battle/dev");
  expect(screen.queryByRole("alert")).toBeNull();
});

test("a production page without a committed server address offers no online entry and makes no request", async () => {
  vi.stubEnv("DEV", false);
  const paths = network(false);
  open();
  await screen.findByRole("alert");
  expect(paths).toEqual([]);
  expect(screen.queryByLabelText("Arrived battle")).toBeNull();
  expect(screen.getAllByRole("link").map((link) => link.getAttribute("href"))).toEqual(["/"]);
});
