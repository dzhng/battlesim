// Lobby-side hosting: machines, their control links and reported stream presence.
// Rooms and Frontline hold the hosted battle's control operations.
import { randomBytes } from "node:crypto";
import type { BattleSeat } from "../../../packages/multiplayer/src/link.ts";
import type { BattleSetup } from "../../../packages/multiplayer/src/wire.ts";
import {
  createRemoteBattleHost,
  type RemoteBattleHost,
  type HostedBattle,
} from "./battleHost/remote.ts";
import type { BattleMachines } from "./machines/index.ts";
import type { createPresence } from "./presence.ts";

export function hostedBattles(
  machines: BattleMachines,
  presence: ReturnType<typeof createPresence>,
) {
  const active = new Map<
    HostedBattle,
    {
      remote: RemoteBattleHost;
      seating: Map<string, BattleSeat>;
      connections: Map<string, () => void>;
      pending: Promise<void>;
    }
  >();
  return {
    /** `attend` is the mode's own count of who is in this battle, opened and
     *  closed with each of a browser's battle streams, as presence is. */
    create(
      candidates: BattleSetup[],
      id: string,
      attend?: (who: string) => () => void,
    ): HostedBattle {
      const key = randomBytes(16).toString("hex");
      const remote = createRemoteBattleHost({ candidates }, id, machines.start(key), key);
      const entry = {
        remote,
        seating: new Map<string, BattleSeat>(),
        connections: new Map<string, () => void>(),
        pending: Promise.resolve(),
      };
      active.set(remote, entry);
      remote.onStream((streams) => {
        const live = new Set(streams.map((s) => s.connection));
        for (const [connection, close] of entry.connections) {
          if (live.has(connection)) continue;
          close();
          entry.connections.delete(connection);
        }
        for (const stream of streams) {
          if (entry.connections.has(stream.connection)) continue;
          const who = [...entry.seating].find(([, s]) => s.player === stream.player)?.[0];
          if (!who) continue;
          const closes = [presence.open(who), attend?.(who)];
          entry.connections.set(stream.connection, () => {
            for (const close of closes) close?.();
          });
        }
      });
      remote.onEnd(() => {
        for (const close of entry.connections.values()) close();
        active.delete(remote);
      });
      return remote;
    },
    address(host: HostedBattle) {
      const address = active.get(host)?.remote.address;
      if (!address) throw new Error("battle address unavailable");
      return address;
    },
    async seating(host: HostedBattle, entries: [string, BattleSeat][]) {
      const entry = active.get(host);
      if (!entry) return;
      if (
        entries.length === entry.seating.size &&
        entries.every(([who, seat]) => {
          const previous = entry.seating.get(who);
          return previous?.player === seat.player && previous.side === seat.side;
        })
      )
        return entry.pending;
      entry.seating = new Map(entries);
      entry.pending = entry.remote.setSeating(entries);
      await entry.pending;
    },
    release(host: HostedBattle) {
      const entry = active.get(host);
      if (!entry) return;
      for (const close of entry.connections.values()) close();
      active.delete(host);
      host.dispose();
    },
    dispose() {
      for (const [host, entry] of active) {
        for (const close of entry.connections.values()) close();
        host.dispose();
      }
      active.clear();
    },
  };
}
