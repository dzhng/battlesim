// @vitest-environment jsdom
// @vitest-environment-options {"url":"https://page.invalid"}
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import { BrowserRouter, Route, Routes, useLocation } from "react-router";
import { PageVisit } from "../../../apps/battle-lab/src/navigation";
import { RoomPage } from "../../../apps/battle-lab/src/online/RoomPage";
import { browserId } from "../../../apps/battle-lab/src/online/identity";
import {
  createRoom,
  roomReduce,
  recordBattle,
  roomView,
  type Room,
} from "../../../packages/multiplayer/src/room";
import { jsonRecord, encodeRecord } from "../../../packages/multiplayer/src/stream/index";
import type { OnlineBattleEntry } from "../../../apps/battle-lab/src/onlineSession";

// These visits exercise a deliberately unconfigured production page.
vi.mock("../../../apps/battle-lab/src/online/server.json", () => ({ default: { address: "" } }));

const code = "TANGO-4821";
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
  window.history.replaceState(null, "", `/room/${code}?server=http://127.0.0.1:8787`);
  render(
    <BrowserRouter>
      <PageVisit>
        <Routes>
          <Route path="/room/:code" element={<RoomPage />} />
          <Route path="/online/battle/:id" element={<BattleArrival />} />
          <Route path="/multiplayer" element={<h1>Multiplayer</h1>} />
        </Routes>
      </PageVisit>
    </BrowserRouter>,
  );
}
function network(refused: boolean) {
  const paths: string[] = [];
  let room = createRoom({ who: browserId(), callsign: "SCOUT" }, "public", code, "77", 0);
  for (const action of [
    { kind: "set_seat", seat: "seat-2", to: "ai" },
    { kind: "ready", ready: true },
    { kind: "start" },
  ] as const)
    room = roomReduce(room, browserId(), action, 0) as Room;
  room = recordBattle(room, { id: "room-battle", mapIdentity: "generated-map" });
  vi.stubGlobal("fetch", async (input: string) => {
    const path = new URL(input, "http://localhost").pathname;
    paths.push(path);
    if (path.endsWith(".wasm")) return new Response(new Uint8Array([0, 97, 115, 109]));
    if (path === "/hello")
      return Response.json(refused ? { refused: "build" } : { callsign: "SCOUT" });
    if (path === `/rooms/${code}`)
      return new Response(
        new ReadableStream({
          start(controller) {
            controller.enqueue(
              encodeRecord(jsonRecord("state", roomView(room, browserId()), 1, 1)),
            );
          },
        }),
      );
    throw new Error("unexpected request");
  });
  return paths;
}
test("a build refusal leaves the page unavailable with no online entry or seat request", async () => {
  const paths = network(true);
  open();
  expect(await screen.findByRole("status", { name: "Online unavailable" })).toBeTruthy();
  expect(window.location.pathname).toBe(`/room/${code}`);
  expect(paths).toContain("/hello");
  expect(paths).not.toContain(`/rooms/${code}`);
  expect(screen.queryByLabelText("Arrived battle")).toBeNull();
  expect(screen.getByRole("button", { name: "Back" })).toBeTruthy();
});
test("a welcomed page requests its own room ticket after hello and reaches its battle", async () => {
  const paths = network(false);
  open();
  await waitFor(() =>
    expect(screen.getByLabelText("Arrived battle").textContent).toBe("room-battle"),
  );
  expect(paths.indexOf("/hello")).toBeGreaterThanOrEqual(0);
  expect(paths.indexOf(`/rooms/${code}`)).toBeGreaterThan(paths.indexOf("/hello"));
  expect(window.location.pathname).toBe("/online/battle/room-battle");
  expect(screen.queryByRole("alert")).toBeNull();
});
test("a production page without a committed server address offers no online entry and makes no request", async () => {
  vi.stubEnv("DEV", false);
  const paths = network(false);
  open();
  await screen.findByRole("status", { name: "Online unavailable" });
  expect(paths).toEqual([]);
  expect(screen.queryByLabelText("Arrived battle")).toBeNull();
  expect(screen.getByRole("button", { name: "Back" })).toBeTruthy();
});
