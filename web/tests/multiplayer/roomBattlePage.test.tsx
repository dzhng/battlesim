// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, waitFor, within } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import { MemoryRouter, Route, Routes, useLocation } from "react-router";
import { RoomPage } from "@apps/battle-lab/src/online/RoomPage";
import { MultiplayerPage } from "@apps/battle-lab/src/online/MultiplayerPage";
import { OnlineMatchResult } from "@apps/battle-lab/src/online/OnlineMatchResult";
import { browserId } from "@apps/battle-lab/src/online/identity";
import type { BattleHost, BattleEnd } from "@apps/game-server/src/battleHost/index";
import type { OnlineBattleEntry } from "@apps/battle-lab/src/onlineSession";
import { WithTestCatalog } from "../catalog";
import { PreviewWorker } from "../support/previewWorker";
import { roomHttp, build } from "./support/roomHttp";

vi.mock("@apps/battle-lab/src/buildIdentity", () => ({ buildIdentity: async () => build }));
vi.mock("@apps/battle-lab/src/menu/MenuShell", () => ({ useMenuShell: () => ({ back() {} }) }));
const address = vi.hoisted(() => ({ value: "" }));
vi.mock("@apps/battle-lab/src/online/server", () => ({ serverAddress: () => address.value }));
let service: Awaited<ReturnType<typeof roomHttp>>;
afterEach(async () => {
  cleanup();
  await service?.close();
  vi.unstubAllGlobals();
});
function Arrival() {
  const { state } = useLocation();
  return <output aria-label="Battle ticket">{JSON.stringify(state)}</output>;
}
async function open() {
  vi.stubGlobal("Worker", PreviewWorker);
  let ready!: (value: Awaited<BattleHost["ready"]>) => void;
  let end!: (value: BattleEnd) => void;
  service = await roomHttp(1, () => ({
    ready: new Promise((resolve) => {
      ready = resolve;
    }),
    onEnd: (handle) => {
      end = handle;
    },
    attach() {},
    setController() {},
    referee() {
      return Promise.reject(new Error("battle still preparing"));
    },
    seatHuman() {
      return Promise.reject(new Error("battle still preparing"));
    },
    onOrder() {},
    endSeat() {},
    dispose() {},
    replay: async () => "",
  }));
  address.value = service.address;
  await service.post("/hello", browserId(), { build, callsign: "SCOUT" });
  const { code } = await service.post("/rooms", browserId(), { visibility: "public" });
  const page = render(
    <MemoryRouter initialEntries={[`/room/${code}`]}>
      <Routes>
        <Route path="/room/:code" element={<RoomPage />} />
        <Route path="/online/battle/:id" element={<Arrival />} />
        <Route path="/multiplayer" element={<MultiplayerPage />} />
      </Routes>
    </MemoryRouter>,
  );
  await page.findByRole("region", { name: "Room lobby" });
  await service.post(`/rooms/${code}/act`, browserId(), {
    kind: "set_seat",
    seat: "seat-2",
    to: "ai",
  });
  fireEvent.click(page.getByRole("button", { name: "Ready" }));
  await waitFor(() =>
    expect((page.getByRole("button", { name: "Start" }) as HTMLButtonElement).disabled).toBe(false),
  );
  fireEvent.click(page.getByRole("button", { name: "Start" }));
  await page.findByRole("status", { name: "Starting" });
  return {
    page,
    code,
    ready: () => ready({ layout: "layout", tickHz: 30, mapIdentity: "map-identity" }),
    crash: () => end({ kind: "crashed", message: "private worker failure" }),
  };
}
test("Start shows server preparation until its view carries the member's ticket, then follows the battle route", async () => {
  const { page, ready } = await open();
  expect(page.queryByRole("region", { name: "Room lobby" })).toBeNull();
  expect(page.queryByRole("button", { name: "Start" })).toBeNull();
  expect(page.queryByRole("status", { name: "Battle ticket" })).toBeNull();
  await act(async () => ready());
  const arrived = JSON.parse(
    (await page.findByLabelText("Battle ticket")).textContent!,
  ) as OnlineBattleEntry;
  expect(arrived.ticket).toMatchObject({
    side: "blue",
    player: "blue-1",
    mapIdentity: "map-identity",
    callsigns: { "blue-1": "SCOUT" },
  });
  expect(arrived.address).toBe(service.address);
  expect(JSON.stringify(arrived)).not.toContain(browserId());
});
test("a battle failing during server preparation sends the member to Multiplayer with a failure message", async () => {
  const { page, crash } = await open();
  await act(async () => crash());
  await page.findByRole("heading", { name: "Multiplayer" });
  expect((await page.findByRole("alert")).textContent).toMatch(/battle.*could not continue/i);
  expect(page.queryByLabelText("Battle ticket")).toBeNull();
  expect(page.container.textContent).not.toContain("private worker failure");
});
test("the final online observation names callsigns in player rows, heads the sides by what they are to the viewer, and returns to Multiplayer", async () => {
  const page = render(
    <MemoryRouter initialEntries={["/online/battle/finished"]}>
      <Routes>
        <Route
          path="/online/battle/:id"
          element={
            <WithTestCatalog>
              <OnlineMatchResult
                match={{
                  phase: "finished",
                  result: "blue",
                  scores: [1, 0],
                  objectives: [],
                  preparationRemainingS: 0,
                  players: [],
                  report: { totals: [], events: [] },
                }}
                seats={[
                  { id: "blue-1", side: "blue", controller: "human" },
                  { id: "red-1", side: "red", controller: "human" },
                ]}
                viewer="red-1"
                callsigns={{ "blue-1": "SCOUT", "red-1": "WING" }}
              />
            </WithTestCatalog>
          }
        />
        <Route path="/multiplayer" element={<h1>Multiplayer</h1>} />
      </Routes>
    </MemoryRouter>,
  );
  expect(page.getByRole("dialog", { name: "Match result" })).toBeTruthy();
  expect(page.getByRole("row", { name: "SCOUT" })).toBeTruthy();
  expect(page.getByRole("row", { name: "WING (YOU)" })).toBeTruthy();
  // The viewer commands Red: their own side comes first, under these words.
  expect(
    page
      .getAllByRole("group")
      .map((side) => [
        side.getAttribute("aria-label"),
        within(side).getAllByRole("row")[1].textContent,
      ]),
  ).toEqual([
    ["YOUR SIDE", expect.stringContaining("WING (YOU)")],
    ["ENEMY", expect.stringContaining("SCOUT")],
  ]);
  fireEvent.click(page.getByRole("button", { name: "Return to Multiplayer" }));
  await page.findByRole("heading", { name: "Multiplayer" });
  expect(page.queryByRole("dialog", { name: "Match result" })).toBeNull();
});
