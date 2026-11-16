// Shared admission, battle allocation and presence for every online mode.
import { cpus, totalmem } from "node:os";
import type { BattleSetup, RoomList } from "../../../packages/multiplayer/src/wire.ts";
import type { Seat } from "../../../packages/multiplayer/src/link.ts";
import type { BattleHost } from "./battleHost/index.ts";
import { hostedBattles } from "./battles.ts";
import type { BattleMachines } from "./machines/index.ts";
import { createPresence, PRESENCE_GRACE_MS, type PresenceClock } from "./presence.ts";

/** Slice 01's memory budget leaves room for a battle to grow toward an hour. */
export const BYTES_PER_BATTLE = 2 * 1024 ** 3;
export function battleCapacity(cores = cpus().length, memoryBytes = totalmem()): number {
  return Math.max(0, Math.min(cores - 1, Math.floor(memoryBytes / BYTES_PER_BATTLE)));
}

type ServedBattle = { id: string; host: BattleHost; seatOf(who: string): Seat | undefined };
type Member = {
  changed(who: string): void;
  gone(who: string): void;
  remembers(who: string): boolean;
};

export function createOnlineState(
  options: ({ capacity: number } | { machines: BattleMachines }) & {
    clock: PresenceClock;
    callsign(who: string): string | undefined;
    releaseCallsign(who: string): void;
  },
) {
  const members = new Set<Member>();
  let frontline: RoomList["frontline"] = { battles: 0, commanders: 0 };
  const listListeners = new Set<() => void>();
  const slots = new Map<object, ServedBattle | null>();
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
      for (const m of members) m.changed(who);
    },
    (who) => {
      for (const m of members) m.gone(who);
      releaseIfForgotten(who);
    },
  );
  const battles = "machines" in options ? hostedBattles(options.machines, presence) : undefined;
  const capacity = "machines" in options ? options.machines.capacity : options.capacity;
  return {
    createBattle: (setup: BattleSetup, id: string): BattleHost => {
      if (!battles) throw new Error("battle machines missing");
      return battles.create(setup, id);
    },
    battleAddress: (host: BattleHost) => battles?.address(host),
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
        serve(value: ServedBattle) {
          slots.set(key, value);
        },
        seating(entries: [string, Seat][]): Promise<void> {
          const served = slots.get(key);
          return served && battles ? battles.seating(served.host, entries) : Promise.resolve();
        },
        release() {
          const served = slots.get(key);
          if (served) battles?.release(served.host);
          slots.delete(key);
        },
      };
    },
    battle(id: string) {
      return [...slots.values()].find((b) => b?.id === id) ?? undefined;
    },
    dispose() {
      battles?.dispose();
      presence.dispose();
      slots.clear();
      members.clear();
      listListeners.clear();
    },
  };
}
export type OnlineState = ReturnType<typeof createOnlineState>;
export type BattleSlot = NonNullable<ReturnType<OnlineState["reserve"]>>;
