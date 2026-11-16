import { createHash, randomBytes } from "node:crypto";
import type { BattleSeat, SeatPort } from "../../../../packages/multiplayer/src/link";
import type { BattleHost } from "../../../../apps/game-server/src/battleHost/index";
import type { RemoteBattleHost } from "../../../../apps/game-server/src/battleHost/remote";
import { httpLink } from "../../../src/battle/remote/httpLink";
import { jsonRecord } from "../../../../packages/multiplayer/src/stream/index";

const occupants = new WeakMap<RemoteBattleHost, [string, BattleSeat][]>();

/** The contract suite opens player routes, including when preparation fails. */
export function playerPort(
  host: RemoteBattleHost,
  battle: string,
  port: SeatPort,
  ended: () => string | null,
) {
  const who = randomBytes(16).toString("hex");
  const entries = occupants.get(host) ?? [];
  occupants.set(host, entries);
  entries.push([who, port.seat]);
  const connected = host.ready.then(async () => {
    await host.setSeating(entries);
    return httpLink(host.address!, battle, who);
  });
  void connected.catch(() => {});
  const request = async <T>(run: () => Promise<T>) => {
    if (ended()) throw new Error("the battle has ended");
    try {
      return await run();
    } catch (error) {
      if (ended()) throw new Error("the battle has ended");
      throw error;
    }
  };
  port.onUp((batch) => request(async () => (await connected).up(batch)));
  port.onPreview((query) => request(async () => (await connected).preview(query)));
  port.onReader((after, deliver) => {
    if (ended()) {
      deliver(jsonRecord("open", { resume: "continued" }));
      deliver(jsonRecord("end", { reason: ended() }));
      return () => {};
    }
    const lifetime = new AbortController();
    void (async () => {
      try {
        const link = await connected;
        for await (const record of link.records(after, lifetime.signal)) {
          if (!deliver(record) || record.kind === "end") break;
        }
      } catch {
        if (!lifetime.signal.aborted) deliver(jsonRecord("end", { reason: ended() ?? "crashed" }));
      }
    })();
    return () => lifetime.abort();
  });
}

const players = new WeakMap<
  BattleHost,
  { address: string; battle: string; who: Map<string, string> }
>();
export function seatedPlayers(
  host: BattleHost,
  address: string,
  battle: string,
  entries: [string, BattleSeat][],
) {
  players.set(host, {
    address,
    battle,
    who: new Map(entries.map(([who, seat]) => [seat.player, who])),
  });
}
export function playerLink(host: BattleHost, seat: BattleSeat) {
  const routes = players.get(host);
  const who = routes?.who.get(seat.player);
  return routes && who ? httpLink(routes.address, routes.battle, who) : undefined;
}
export async function seatRequest(address: string, battle: string, who: string) {
  const response = await fetch(`${address}/battles/${battle}/preview`, {
    method: "POST",
    headers: { "X-Player": who, "Content-Type": "application/json" },
    body: JSON.stringify({ kind: "purchase", variant: "unknown", destination: [0, 0] }),
  });
  return response.status;
}

/** Stable valid credentials for service-level fixtures, never public player ids. */
export const testBrowser = (name: string) => createHash("md5").update(name).digest("hex");
