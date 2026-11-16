// @vitest-environment jsdom
import { useEffect } from "react";
import { act, cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import { MemoryRouter, Route, Routes, useLocation } from "react-router";
import OnlineBattle from "@apps/battle-lab/src/routes/onlineBattle";
import { PageVisit } from "@apps/battle-lab/src/navigation";
import { MultiplayerPage } from "@apps/battle-lab/src/online/MultiplayerPage";
import { jsonRecord } from "@packages/multiplayer/src/stream/index";
import { browserId } from "@apps/battle-lab/src/online/identity";
import type { BattleConnectionSession } from "@web/battle/prepare/client";
import { followBattle, type OnlineBattleEntry } from "@apps/battle-lab/src/onlineSession";
import type { OnlineMap } from "@apps/battle-lab/src/online/useOnlineMap";
import { roomHttp, build } from "./support/roomHttp";
import { frontlineHost } from "./support/frontlineHost";

vi.mock("@apps/battle-lab/src/buildIdentity", () => ({ buildIdentity: async () => build }));
vi.mock("@apps/battle-lab/src/menu/MenuShell", () => ({ useMenuShell: () => ({ back() {} }) }));
const address = vi.hoisted(() => ({ value: "" }));
vi.mock("@apps/battle-lab/src/online/server", () => ({ serverAddress: () => address.value }));
vi.mock("@web/battle/remote/clock", async () => ({
  remoteClock: (await import("./timerClock")).timerClock,
}));
vi.mock("@web/battle/prepare/lanes", () => ({
  lanesOf: () => ({ entries: [{ holder: "commander" }] }),
}));
vi.mock("@apps/battle-lab/src/BattleView", () => ({
  // Keep the real remote connection and HTTP transport; the GPU is an outer edge.
  BattleView: ({
    prepared,
    side,
    cover,
    onFailure,
  }: {
    prepared: Extract<BattleConnectionSession, { transport: "remote" }>;
    side: "blue" | "red";
    cover: React.ReactNode;
    onFailure(message: string): void;
  }) => {
    useEffect(() => {
      const channel = prepared.connect(() => {}, onFailure);
      channel.send({ type: "connect", side });
      return () => channel.close();
    }, [prepared, side, onFailure]);
    return cover;
  },
}));
let service: Awaited<ReturnType<typeof roomHttp>>;
afterEach(async () => {
  cleanup();
  await service?.close();
  vi.restoreAllMocks();
});
function Destination() {
  const { state } = useLocation();
  return (
    <>
      <MultiplayerPage />
      <output aria-label="Notice">{state?.frontlineNotice}</output>
    </>
  );
}
test("an idle battle stream returns to Multiplayer with the reason while waiting for its first snapshot", async () => {
  let end!: () => void;
  let reading!: () => void;
  const opened = new Promise<void>((resolve) => {
    reading = resolve;
  });
  service = await roomHttp(1, (setup) => ({
    ...frontlineHost(setup),
    attach(port) {
      port.onReader((_after, deliver) => {
        deliver(jsonRecord("open", { resume: "keyframe", layout: "{}", tickHz: 30 }));
        end = () => deliver(jsonRecord("end", { reason: "idle" }));
        reading();
        return () => {};
      });
    },
  }));
  address.value = service.address;
  await service.post("/hello", browserId(), { build, callsign: "SCOUT" });
  const briefing = await service.post("/frontline/join", browserId(), {});
  const ticket = await service.post(`/frontline/${briefing.battle}/deploy`, browserId(), {});
  const entry = {
    address: service.address,
    ticket: { ...ticket, player: "commander" },
  } as unknown as OnlineBattleEntry;
  followBattle(() => {}, entry, "", {
    request: { map_source: { request: { seed: "1" } }, battle_seed: 1 },
    battle: {
      scenario: JSON.stringify({
        players: [],
        skirmish: { sites: { entries: [{ center: [0, 0], yaw: 0 }] } },
      }),
      report: {
        identity: { kind: "generated", generation: { map_hash: ticket.mapIdentity } },
        size: [100, 100],
        extents: { rendered: [0, 0, 100, 100] },
      },
    },
  } as unknown as OnlineMap);
  const page = render(
    <MemoryRouter initialEntries={[{ pathname: `/online/battle/${ticket.battle}`, state: entry }]}>
      <PageVisit>
        <Routes>
          <Route path="/online/battle/:id" element={<OnlineBattle />} />
          <Route path="/multiplayer" element={<Destination />} />
        </Routes>
      </PageVisit>
    </MemoryRouter>,
  );
  await waitFor(() => {
    if (page.queryByRole("alert")) {
      if (!page.queryByTestId("error-details"))
        fireEvent.click(page.getByRole("button", { name: "Details" }));
      throw new Error(page.getByTestId("error-details").textContent!);
    }
    expect(end).toBeTruthy();
  });
  await opened;
  expect(page.getByTestId("loading")).toBeTruthy();
  await act(async () => end());
  await page.findByRole("region", { name: "Frontline" });
  expect(page.getByLabelText("Notice").textContent).toMatch(/idle.*AI/i);
  expect((await page.findByRole("alert")).textContent).toMatch(/idle.*AI/i);
  expect(page.queryByTestId("loading")).toBeNull();
});
