// @vitest-environment jsdom
import { cleanup, fireEvent, render } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import { MemoryRouter, Route, Routes, useLocation } from "react-router";
import OnlineBattle from "@apps/battle-lab/src/routes/onlineBattle";
import { PageVisit } from "@apps/battle-lab/src/navigation";
import type { OnlineBattleEntry } from "@apps/battle-lab/src/onlineSession";
import { preset, setOccupant } from "@packages/skirmish-setup/src/lobby";
import { encodeRecord, jsonRecord } from "@packages/multiplayer/src/stream/index";
import { toBattleSetup } from "@packages/multiplayer/src/setup";

vi.mock("@web/battle/remote/connection", () => ({
  remoteConnection: () => () => ({ send() {}, close() {} }),
}));

const preparation = vi.hoisted(() => ({
  calls: 0,
  hold: false,
  endRoom: () => {},
  request: { map_source: { request: { seed: "1" } }, battle_seed: 1 },
}));
vi.mock("@apps/battle-lab/src/useBuiltScenario", () => ({
  useBuiltScenario: () => preparation.request,
  buildFailed: () => false,
}));
vi.mock("@web/battle/catalog/context", () => ({ useSessionCatalog: () => ({ rules: {} }) }));
vi.mock("@web/battle/sim/module", () => ({ loadWasm: async () => ({}) }));
vi.mock("@web/battle/prepare/lanes", () => ({
  lanesOf: () => ({ entries: [{ holder: "blue-1" }] }),
}));
vi.mock("@apps/battle-lab/src/gameLight", () => ({ setMatchLighting() {} }));
vi.mock("@web/battle/prepare/client", () => ({
  prepareBattle: () => {
    preparation.calls++;
    return {
      cancel() {},
      battle: preparation.hold
        ? new Promise(() => {})
        : Promise.resolve({
            scenario: JSON.stringify({
              players: [],
              skirmish: { sites: { entries: [{ center: [0, 0], yaw: 0 }] } },
            }),
            report: {
              identity: { kind: "generated", generation: { map_hash: "map" } },
              size: [100, 100],
              extents: { rendered: [0, 0, 100, 100] },
            },
          }),
    };
  },
}));
vi.mock("@apps/battle-lab/src/BattleView", () => ({
  // The viewport's failure boundary is independent of WebGPU admission.
  BattleView: ({ onFailure }: { onFailure: (message: string) => void }) => (
    <button onClick={() => onFailure("worker crashed")}>Report battle failure</button>
  ),
}));
afterEach(() => {
  cleanup();
  preparation.calls = 0;
  preparation.hold = false;
  vi.unstubAllGlobals();
});
function Menu() {
  const location = useLocation();
  return (
    <>
      <h1>Multiplayer</h1>
      <p role="alert">{location.state?.roomNotice}</p>
    </>
  );
}
function open(entry: OnlineBattleEntry | null) {
  vi.stubGlobal(
    "fetch",
    vi.fn(async (input: string, init: RequestInit) => {
      if (!String(input).endsWith("/rooms/TANGO-4821")) throw new Error("unexpected request");
      return new Response(
        new ReadableStream({
          start(controller) {
            preparation.endRoom = () =>
              controller.enqueue(encodeRecord(jsonRecord("end", { reason: "battle_failed" })));
            init.signal?.addEventListener("abort", () => controller.close(), { once: true });
          },
        }),
      );
    }),
  );
  return render(
    <MemoryRouter initialEntries={[{ pathname: "/online/battle/room-match", state: entry }]}>
      <PageVisit>
        <Routes>
          <Route path="/online/battle/:id" element={<OnlineBattle />} />
          <Route path="/multiplayer" element={<Menu />} />
        </Routes>
      </PageVisit>
    </MemoryRouter>,
  );
}
test("deep-linking a battle route without a ticket refuses before preparing a world", async () => {
  const page = open(null);
  expect(await page.findByRole("alert")).toBeTruthy();
  expect(preparation.calls).toBe(0);
  expect(fetch).not.toHaveBeenCalled();
  expect(page.queryByRole("button", { name: "Report battle failure" })).toBeNull();
});
function entry(): OnlineBattleEntry {
  const setup = toBattleSetup(
    setOccupant(preset("1v1"), "seat-2", { kind: "human", who: "other" }),
    "skirmish",
  );
  return {
    room: "TANGO-4821",
    address: "http://127.0.0.1:8787",
    ticket: {
      battle: "room-match",
      setup,
      mapIdentity: "map",
      side: "blue",
      player: "blue-1",
      callsigns: { "blue-1": "SCOUT", "red-1": "WING" },
    },
  };
}
test("a running battle failure returns to the multiplayer menu with an honest message", async () => {
  const page = open(entry());
  fireEvent.click(await page.findByRole("button", { name: "Report battle failure" }));
  await page.findByRole("heading", { name: "Multiplayer" });
  expect((await page.findByRole("alert")).textContent).toMatch(/battle.*could not continue/i);
  expect(page.queryByRole("button", { name: "Report battle failure" })).toBeNull();
});

test("local map preparation keeps a member stream open and hears a room battle failure before the battle stream exists", async () => {
  preparation.hold = true;
  const page = open(entry());
  await page.findByRole("heading", { name: "Deploying" });
  expect(fetch).toHaveBeenCalledTimes(1);
  expect(vi.mocked(fetch).mock.calls[0][1]?.signal?.aborted).toBe(false);
  preparation.endRoom();
  await page.findByRole("heading", { name: "Multiplayer" });
  expect((await page.findByRole("alert")).textContent).toMatch(/battle.*could not continue/i);
  expect(vi.mocked(fetch).mock.calls[0][1]?.signal?.aborted).toBe(true);
});
