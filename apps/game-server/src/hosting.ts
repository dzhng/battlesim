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
import { seatDigest, type BattleArchive } from "./archive.ts";

type ReplayLookup = { replay: string } | { refused: "unknown" | "not_seated" | "unavailable" };

export function hostedBattles(
  machines: BattleMachines,
  presence: ReturnType<typeof createPresence>,
  archive: BattleArchive,
) {
  const active = new Map<
    HostedBattle,
    {
      id: string;
      remote: RemoteBattleHost;
      seating: Map<string, BattleSeat>;
      /** Everyone who held a seat at any time; they may export it once it is over. */
      seated: Set<string>;
      connections: Map<string, () => void>;
      pending: Promise<void>;
    }
  >();
  /** Records still being written, so an export right after the end waits for its file. */
  const archiving = new Map<string, Promise<void>>();
  return {
    /** `attend` is the mode's own count of who is in this battle, opened and
     *  closed with each of a browser's battle streams, as presence is. */
    create(
      candidates: BattleSetup[],
      id: string,
      attend?: (who: string) => () => void,
    ): HostedBattle {
      const key = randomBytes(16).toString("hex");
      const seated = new Set<string>();
      const remote = createRemoteBattleHost(
        { candidates },
        id,
        machines.start(key),
        key,
        (outcome, replay) => {
          const endedAt = new Date().toISOString();
          // A battle that never started has nothing to keep.
          const kept = Promise.all([started, replay])
            .then(
              ([{ ready, at }, replay]) =>
                archive.save({
                  id,
                  setup: candidates[ready.candidate],
                  mapIdentity: ready.mapIdentity,
                  startedAt: at,
                  endedAt,
                  outcome,
                  seated,
                  replay,
                }),
              () => {},
            )
            .catch((error: Error) => console.error(`A battle was not archived: ${error.message}`))
            .finally(() => archiving.delete(id));
          archiving.set(id, kept);
        },
      );
      const started = remote.ready.then((ready) => ({ ready, at: new Date().toISOString() }));
      started.catch(() => {});
      const entry = {
        id,
        remote,
        seating: new Map<string, BattleSeat>(),
        seated,
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
      for (const [who] of entries) entry.seated.add(who);
      entry.pending = entry.remote.setSeating(entries);
      await entry.pending;
    },
    /** A seated browser's export: from the running battle, or its record once it is over. */
    async replay(id: string, who: string): Promise<ReplayLookup> {
      const live = [...active.values()].find((entry) => entry.id === id);
      if (live) {
        if (!live.seating.has(who)) return { refused: "not_seated" };
        return { replay: await live.remote.replay() };
      }
      await archiving.get(id);
      const record = await archive.find(id);
      if (!record) return { refused: "unknown" };
      if (!record.seated.includes(seatDigest(who))) return { refused: "not_seated" };
      return record.replay === null ? { refused: "unavailable" } : { replay: record.replay };
    },
    /** Settles when the battle's machine is gone and its capacity is free. */
    release(host: HostedBattle): Promise<void> {
      const entry = active.get(host);
      if (!entry) return Promise.resolve();
      for (const close of entry.connections.values()) close();
      active.delete(host);
      host.dispose();
      return entry.remote.stopped;
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
