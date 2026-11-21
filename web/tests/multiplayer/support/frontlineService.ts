// Real battle machines and clocks for Frontline lifecycle scenarios.
import { expect, vi } from "vitest";
import { testArchive } from "./archive";
import { inProcessBattleMachines } from "../../../../apps/game-server/src/machines/inProcess";
import { createFrontlineService } from "../../../../apps/game-server/src/frontline";
import { createOnlineState } from "../../../../apps/game-server/src/online";
import {
  createBattleHost,
  type BattleHost,
} from "../../../../apps/game-server/src/battleHost/index";
import { MAP_ATTEMPTS } from "../../../../apps/game-server/src/serverData";
import type {
  BattleSetup,
  BattleTicket,
  Briefing,
} from "../../../../packages/multiplayer/src/wire";
import { PageClock } from "../pageClock";
import { data, generator, seat, recordHost, FRONTLINE_DRAWS } from "./battle";
import { seatedPlayers, testBrowser } from "./player";
export const cleanups: (() => void)[] = [];

/** The same game on a map the generator will not make. */
export const unmakeable = (setup: BattleSetup): BattleSetup => ({
  ...setup,
  map: { ...setup.map, type: "no such map type" as never },
});

/** `refusedMaps`: how many of the first drawn maps the generator refuses. */
export async function service(capacity = 2, admissionMs = 0, refusedMaps = 0) {
  const clock = new PageClock();
  const battleClock = new PageClock();
  const hosts: BattleHost[] = [];
  const failures: string[] = [];
  const records: ReturnType<typeof recordHost>[] = [];
  const forgotten = new Set<string>();
  const online = createOnlineState({
    archive: testArchive(),
    machines: inProcessBattleMachines({
      capacity: capacity,
      ordersPerSecond: 10000,
      createBattle: (spec) => {
        const candidates = spec.candidates;
        const host = createBattleHost(
          { candidates: candidates.map((setup) => (refusals-- > 0 ? unmakeable(setup) : setup)) },
          { clock: battleClock, generator },
          data,
        );
        clock.suspend(admissionMs);
        void clock.advance(0);
        hosts.push(host);
        host.onEnd((end) => {
          if (end.kind === "crashed") failures.push(end.message);
        });
        records.push(recordHost(host, candidates[0].seats[0]));
        return host;
      },
    }),
    clock,
    callsign: (who) => (forgotten.has(who) ? undefined : "SCOUT"),
    releaseCallsign: (who) => {
      forgotten.add(who);
    },
  });
  online.presence.track(testBrowser("alpha"));
  let draw = 0;
  let refusals = refusedMaps;
  const frontline = createFrontlineService({
    online,
    mapAttempts: MAP_ATTEMPTS,
    random: () => FRONTLINE_DRAWS[draw++ % FRONTLINE_DRAWS.length],
  });
  cleanups.push(online.dispose, frontline.dispose);
  const join = async (who: string) => {
    const result = await frontline.join(who);
    if ("refused" in result) throw new Error(failures.join("; ") || result.refused);
    return result;
  };
  const a = await join(testBrowser("alpha"));
  const host = hosts.at(-1)!;
  const ready = await host.ready;
  const ended: Parameters<Parameters<BattleHost["onEnd"]>[0]>[0][] = [];
  host.onEnd((end) => ended.push(end));
  const run = async (ticks: number) => {
    for (let i = 0; i < ticks; i++) await battleClock.advance(1000 / ready.tickHz);
  };
  const b = { host, ready, ended, run };
  const deployed = new Map<string, string>();
  const deploy = async (who: string, briefing: Briefing): Promise<BattleTicket> => {
    let done = false;
    const pending = frontline.deploy(who, briefing.battle).finally(() => {
      done = true;
    });
    // A join queues its seat events and controller in one control request.
    // Hold time while HTTP arrives, advance exactly its publication boundary,
    // then hold again while the acknowledgement travels back to the lobby.
    const player = briefing.setup.seats.filter((s) => s.side === briefing.side)[briefing.seat - 1]
      .player;
    await vi.waitFor(async () => {
      if (done) return;
      expect((await host.referee()).seats.find((s) => s.player === player)!.controller).toBe(
        "human",
      );
    });
    if (!done) await run(1);
    const result = await pending;
    if ("refused" in result) throw new Error(result.refused);
    deployed.set(result.player, who);
    const entries = result.setup.seats.flatMap((s) => {
      const browser = deployed.get(s.player);
      return browser ? [[browser, s] as [string, typeof s]] : [];
    });
    seatedPlayers(host, result.address, result.battle, entries);
    return result;
  };
  const present = (who: string) => {
    const close = online.presence.open(who);
    cleanups.push(close);
    return close;
  };
  const closeAlpha = present(testBrowser("alpha"));
  const ticket = await deploy(testBrowser("alpha"), a);
  const alpha = seat(b, ticket),
    seen = alpha.read();
  for (let i = 0; i < ready.tickHz * 3 && !seen.latest; i++) await run(1);
  if (!seen.latest) throw new Error("the initial reader never received its snapshot");
  return {
    clock,
    battleClock,
    hosts,
    records,
    online,
    frontline,
    join,
    a,
    ticket,
    alpha,
    seen,
    present,
    closeAlpha,
    nextMap: () => {
      draw = 6;
    },
    deploy,
    b,
    run,
  };
}
