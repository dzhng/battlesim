import { testBrowser } from "./support/player";
// @vitest-environment node
import { afterEach, beforeAll, expect, test } from "vitest";
import { inProcessBattleMachines } from "../../../apps/game-server/src/machines/inProcess";
import { createFrontlineService } from "../../../apps/game-server/src/frontline";
import { createOnlineState } from "../../../apps/game-server/src/online";
import { createBattleHost, type BattleHost } from "../../../apps/game-server/src/battleHost/index";
import { MAP_ATTEMPTS } from "../../../apps/game-server/src/serverData";
import type { BattleSetup } from "../../../packages/multiplayer/src/wire";
import { PageClock } from "./pageClock";
import { loadBattleSupport, data, generator } from "./support/battle";

beforeAll(loadBattleSupport, 60_000);
const cleanups: (() => void)[] = [];
afterEach(() => {
  for (const close of cleanups.splice(0).reverse()) close();
});
const unmakeable = (setup: BattleSetup): BattleSetup => ({
  ...setup,
  map: { ...setup.map, type: "no such map type" as never },
});

test.each([false, true])(
  "one new game's draws start one machine, and refusal leaves none running (all refused: %s)",
  async (allRefused) => {
    const clock = new PageClock();
    let starts = 0,
      running = 0,
      attempts = 0;
    const hosts: BattleHost[] = [];
    const provider = inProcessBattleMachines({
      capacity: 1,
      ordersPerSecond: JSON.parse(data.rules).commands.orders_per_s,
      createBattle(spec) {
        const candidates = spec.candidates.map((setup) => {
          attempts++;
          return attempts <= MAP_ATTEMPTS - (allRefused ? 0 : 1) ? unmakeable(setup) : setup;
        });
        const host = createBattleHost({ candidates }, { clock, generator }, data);
        hosts.push(host);
        return host;
      },
    });
    const online = createOnlineState({
      clock,
      callsign: () => "SCOUT",
      releaseCallsign: () => {},
      machines: {
        capacity: provider.capacity,
        async start(key) {
          starts++;
          const machine = await provider.start(key);
          running++;
          let stopped = false;
          return {
            ...machine,
            stop() {
              if (!stopped) {
                stopped = true;
                running--;
              }
              machine.stop();
            },
          };
        },
      },
    });
    let draw = 0;
    const frontline = createFrontlineService({
      online,
      mapAttempts: MAP_ATTEMPTS,
      random: () => [0, 0, 0.5, 0, 1 / 2 ** 32, 0, 0][draw++ % 7],
    });
    cleanups.push(online.dispose, frontline.dispose);
    const result = await frontline.join(testBrowser("alpha"));
    expect(starts).toBe(1);
    expect(attempts).toBe(MAP_ATTEMPTS);
    if (allRefused) {
      expect(result).toEqual({ refused: "unavailable" });
      const { vi } = await import("vitest");
      await vi.waitFor(() => expect(running).toBe(0));
      expect(online.slotFree()).toBe(true);
    } else {
      if ("refused" in result) throw new Error(result.refused);
      const ready = await hosts[0].ready;
      expect(ready.candidate).toBe(MAP_ATTEMPTS - 1);
      expect(result.mapIdentity).toBe(ready.mapIdentity);
      const ticket = await frontline.deploy(testBrowser("alpha"), result.battle);
      if ("refused" in ticket) throw new Error(ticket.refused);
      expect(ticket.setup).toEqual(result.setup);
      expect(ticket.mapIdentity).toBe(result.mapIdentity);
      expect(result.setup.seats.find((s) => s.controller === "human")?.side).toBe(result.side);
      expect(result.roster.filter((s) => s.callsign !== null)).toEqual([
        { side: result.side, callsign: "SCOUT" },
      ]);
      expect(running).toBe(1);
    }
  },
  120_000,
);

test("a machine allocation refusal answers full and releases the game's slot", async () => {
  let starts = 0;
  const online = createOnlineState({
    clock: new PageClock(),
    callsign: () => "SCOUT",
    releaseCallsign: () => {},
    machines: {
      capacity: 1,
      async start() {
        starts++;
        throw new Error("allocation refused");
      },
    },
  });
  const frontline = createFrontlineService({
    online,
    mapAttempts: MAP_ATTEMPTS,
    random: () => 0.25,
  });
  cleanups.push(online.dispose, frontline.dispose);
  expect(await frontline.join(testBrowser("alpha"))).toEqual({ refused: "full" });
  expect(starts).toBe(1);
  expect(online.slotFree()).toBe(true);
});

test("eager candidates keep each map draw followed by its initial seat draw", async () => {
  const { newGame } = await import("../../../packages/multiplayer/src/frontline");
  const { frontlineHost } = await import("./support/frontlineHost");
  const values = [0.12, 0.84, 0.41, 0.67, 0.23, 0.91, 0.36, 0.58, 0.05];
  let sourceAt = 0,
    expectedAt = 0,
    game = 0;
  const expectedRandom = () => values[expectedAt++ % values.length];
  const selected = [0, MAP_ATTEMPTS - 1, 0];
  const online = createOnlineState({
    machines: inProcessBattleMachines({
      capacity: 1,
      ordersPerSecond: 10000,
      createBattle(spec) {
        const candidate = selected[game++];
        const host = frontlineHost(spec.candidates[candidate]);
        return { ...host, ready: host.ready.then((ready) => ({ ...ready, candidate })) };
      },
    }),
    clock: new PageClock(),
    callsign: () => "SCOUT",
    releaseCallsign: () => {},
  });
  const frontline = createFrontlineService({
    online,
    mapAttempts: MAP_ATTEMPTS,
    random: () => values[sourceAt++ % values.length],
  });
  cleanups.push(online.dispose, frontline.dispose);
  for (const candidate of selected) {
    let expected!: BattleSetup;
    let side!: "blue" | "red";
    for (let attempt = 0; attempt < MAP_ATTEMPTS; attempt++) {
      const setup = newGame(expectedRandom);
      const drawnSide = expectedRandom() < 0.5 ? "blue" : "red";
      setup.seats.find((s) => s.side === drawnSide)!.controller = "human";
      if (attempt === candidate) {
        expected = setup;
        side = drawnSide;
      }
    }
    const result = await frontline.join(testBrowser("alpha"));
    if ("refused" in result) throw new Error(result.refused);
    expect(result.setup).toEqual(expected);
    expect(result.side).toBe(side);
    expect(result.roster.filter((s) => s.callsign)).toEqual([{ side, callsign: "SCOUT" }]);
    frontline.leave(testBrowser("alpha"), result.battle);
    expect(online.slotFree()).toBe(true);
  }
});
