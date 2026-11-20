// Shared admission, battle allocation and presence for every online mode.
import { cpus, totalmem } from "node:os";
import type { BattleSetup, RoomList } from "../../../packages/multiplayer/src/wire.ts";
import type { BattleSeat } from "../../../packages/multiplayer/src/link.ts";
import type { HostedBattle } from "./battleHost/remote.ts";
import { hostedBattles } from "./hosting.ts";
import type { BattleMachines } from "./machines/index.ts";
import { createPresence, PRESENCE_GRACE_MS, type PresenceClock } from "./presence.ts";

/** A battle's share of this computer's memory when counting how many it may
 *  run at once. A long ten-AI battle outgrows it; rented machines are sized
 *  in the deploy settings. */
export const BYTES_PER_BATTLE = 2 * 1024 ** 3;
export function battleCapacity(cores = cpus().length, memoryBytes = totalmem()): number {
  return Math.max(0, Math.min(cores - 1, Math.floor(memoryBytes / BYTES_PER_BATTLE)));
}

type Member = {
  changed?(who: string): void;
  gone?(who: string): void;
  remembers(who: string): boolean;
};

export function createOnlineState(options: {
  machines: BattleMachines;
  clock: PresenceClock;
  callsign(who: string): string | undefined;
  releaseCallsign(who: string): void;
}) {
  const members = new Set<Member>();
  const devicesWithGame = new Set<string>();
  let frontline: RoomList["frontline"] = { battles: 0, commanders: 0 };
  const listListeners = new Set<() => void>();
  const slots = new Map<object, HostedBattle | null>();
  const releaseIfForgotten = (who: string) => {
    const absent = presence.goneFor(who);
    if (absent === null || absent < PRESENCE_GRACE_MS || [...members].some((m) => m.remembers(who)))
      return;
    options.releaseCallsign(who);
    presence.forget(who);
  };
  const presence = createPresence(
    options.clock,
    (who) => {
      for (const m of members) m.changed?.(who);
    },
    (who) => {
      for (const m of members) m.gone?.(who);
      releaseIfForgotten(who);
    },
  );
  const battles = hostedBattles(options.machines, presence);
  const capacity = options.machines.capacity;
  return {
    /** Claimed before preparation awaits; the mode releases it only when its game is removed. */
    claimGame(device: string): (() => void) | null {
      if (devicesWithGame.has(device)) return null;
      devicesWithGame.add(device);
      return () => {
        devicesWithGame.delete(device);
      };
    },
    createBattle: (
      candidates: BattleSetup[],
      id: string,
      attend?: (who: string) => () => void,
    ): HostedBattle => battles.create(candidates, id, attend),
    battleAddress: (host: HostedBattle) => battles.address(host),
    frontline: () => frontline,
    publishFrontline(next: RoomList["frontline"]) {
      if (next.battles === frontline.battles && next.commanders === frontline.commanders) return;
      frontline = next;
      for (const listener of listListeners) listener();
    },
    onFrontlineChanged(listener: () => void) {
      listListeners.add(listener);
      return () => listListeners.delete(listener);
    },
    clock: options.clock,
    callsign: options.callsign,
    presence,
    releaseIfForgotten,
    subscribe(member: Member) {
      members.add(member);
      return () => members.delete(member);
    },
    slotFree: () => slots.size < capacity,
    reserve() {
      if (slots.size >= capacity) return null;
      const key = {};
      slots.set(key, null);
      return {
        serve(value: HostedBattle) {
          slots.set(key, value);
        },
        seating(entries: [string, BattleSeat][]): Promise<void> {
          const served = slots.get(key);
          return served ? battles.seating(served, entries) : Promise.resolve();
        },
        release() {
          const served = slots.get(key);
          if (served) battles.release(served);
          slots.delete(key);
        },
      };
    },
    dispose() {
      battles.dispose();
      presence.dispose();
      slots.clear();
      devicesWithGame.clear();
      members.clear();
      listListeners.clear();
    },
  };
}
export type OnlineState = ReturnType<typeof createOnlineState>;
export type BattleSlot = NonNullable<ReturnType<OnlineState["reserve"]>>;
