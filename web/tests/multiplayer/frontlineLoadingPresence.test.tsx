// @vitest-environment jsdom
import { useEffect } from "react";
import { act, cleanup, render, waitFor } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import { MemoryRouter, Route, Routes } from "react-router";
import OnlineBattle from "@apps/battle-lab/src/routes/onlineBattle";
import { PageVisit } from "@apps/battle-lab/src/navigation";
import { browserId } from "@apps/battle-lab/src/online/identity";
import { jsonRecord } from "@packages/multiplayer/src/stream/index";
import { PRESENCE_GRACE_MS } from "@apps/game-server/src/presence";
import { roomHttp, build } from "./support/roomHttp";
import { frontlineHost } from "./support/frontlineHost";
import { PageClock } from "./pageClock";
import { keepBuiltMap, type OnlineBattleEntry } from "@apps/battle-lab/src/onlineSession";
import type { OnlineMap } from "@apps/battle-lab/src/online/useOnlineMap";
import type { BattleConnectionSession } from "@web/battle/prepare/client";
const state = vi.hoisted(() => ({ described: false, detached: false }));
vi.mock("@web/battle/remote/clock", async () => ({
  remoteClock: (await import("./timerClock")).timerClock,
}));
vi.mock("@web/battle/prepare/lanes", () => ({
  lanesOf: () => ({ entries: [{ holder: "commander" }] }),
}));
vi.mock("@apps/battle-lab/src/BattleView", () => ({
  // A viewport held before its first frame: the real description connection
  // closes, and starting the running stream must wait for graphics resources.
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
      const channel = prepared.connect((reply) => {
        if (reply.type === "ready") state.described = true;
      }, onFailure);
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
  state.described = state.detached = false;
});

test("Frontline remains present after its description stream closes while the first frame is still loading", async () => {
  const clock = new PageClock();
  service = await roomHttp(
    1,
    (setup) => ({
      ...frontlineHost(setup),
      attach(port) {
        port.onReader((_after, deliver) => {
          deliver(jsonRecord("open", { resume: "keyframe", layout: "{}", tickHz: 30 }));
          deliver({ id: 1, kind: "publication", flags: 1, payload: new Uint8Array(24) });
          return () => {
            state.detached = true;
          };
        });
      },
    }),
    clock,
  );
  await service.post("/hello", browserId(), { build, callsign: "SCOUT" });
  const briefing = await service.post("/frontline/join", browserId(), {});
  const ticket = await service.post(`/frontline/${briefing.battle}/deploy`, browserId(), {});
  const entry = {
    address: service.address,
    ticket: { ...ticket, player: "commander" },
  } as unknown as OnlineBattleEntry;
  keepBuiltMap(ticket.battle, {
    request: { map_source: { request: { seed: "1" } }, battle_seed: 1 },
    battle: {
      scenario: JSON.stringify({
        players: [],
        skirmish: { sites: { entries: [{ center: [0, 0], yaw: 0 }] } },
      }),
      report: { size: [100, 100], extents: { rendered: [0, 0, 100, 100] } },
    },
  } as unknown as OnlineMap);
  const page = render(
    <MemoryRouter initialEntries={[{ pathname: `/online/battle/${ticket.battle}`, state: entry }]}>
      <PageVisit>
        <Routes>
          <Route path="/online/battle/:id" element={<OnlineBattle />} />
        </Routes>
      </PageVisit>
    </MemoryRouter>,
  );
  await waitFor(() => expect(state.described && state.detached).toBe(true));
  expect(page.getByTestId("loading")).toBeTruthy();
  await act(async () => clock.advance(PRESENCE_GRACE_MS + 1));
  const resumed = await service.post("/frontline/join", browserId(), {});
  expect(resumed.battle).toBe(ticket.battle);
});
