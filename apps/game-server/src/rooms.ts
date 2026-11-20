import { battlePlayers } from "../../../packages/skirmish-setup/src/seats.ts";
import { randomUUID } from "node:crypto";
import type { HostedBattle } from "./battleHost/remote.ts";
import type { OnlineState, BattleSlot } from "./online.ts";
import {
  createRoom,
  battleSetup,
  deserted,
  recordBattle,
  roomCode,
  normalizeRoomCode,
  roomReduce,
  roomView,
  type Room,
  type RoomRefusal,
} from "../../../packages/multiplayer/src/room.ts";
import {
  KEYFRAME,
  jsonRecord,
  readJson,
  recordSource,
  type RecordSource,
} from "../../../packages/multiplayer/src/stream/index.ts";
import type { RoomAction, RoomList } from "../../../packages/multiplayer/src/wire.ts";
import { seedFrom } from "../../../web/src/maps/source.ts";
import { createPresence, PRESENCE_GRACE_MS } from "./presence.ts";

/** Reclaim a battle slot after a minute without any connected human member. */
export const EMPTY_ROOM_TIMEOUT_MS = 60_000;

type Deliver = Parameters<RecordSource["attach"]>[1];
type StoredRoom = {
  room: Room;
  releaseGame: () => void;
  slot: BattleSlot;
  source: RecordSource;
  idle: { cancel: () => void } | null;
  battle: { host: HostedBattle; controllers: Map<string, string> } | null;
};

export function createRoomService(options: { online: OnlineState; random: () => number }) {
  const { online } = options;
  const { clock, presence, releaseIfForgotten } = online;
  const rooms = new Map<string, StoredRoom>();
  // A member is in a room while one of that room's own streams is open: its
  // state, or a seat's in its battle. The browser's presence cannot say so:
  // the room list keeps it too, and a member reading the list has left their
  // room. Same grace, counted for each room.
  const seatKey = (code: string, who: string) => `${code}/${who}`;
  const seatOf = (key: string) => {
    const at = key.indexOf("/");
    return { stored: rooms.get(key.slice(0, at)), who: key.slice(at + 1) };
  };
  const attendance = createPresence(
    clock,
    (key) => {
      const { stored, who } = seatOf(key);
      if (!stored) return;
      updateIdle(stored);
      updateCommander(stored, who);
    },
    (key) => {
      const { stored, who } = seatOf(key);
      if (!stored) return;
      if (stored.room.phase === "lobby") transition(stored, who, { kind: "leave" });
      else updateCommander(stored, who);
    },
  );

  const roomList = (): RoomList => ({
    frontline: online.frontline(),
    rooms: [...rooms.values()]
      .map(({ room }) => room)
      .filter((room) => room.visibility === "public" && room.phase === "lobby")
      .map((room) => ({
        code: room.code,
        host: room.members.find((member) => member.who === room.host)?.callsign ?? "",
        map: room.lobby.map,
        blue: room.lobby.seats
          .filter((seat) => seat.side === "blue")
          .map((seat) => seat.occupant.kind),
        red: room.lobby.seats
          .filter((seat) => seat.side === "red")
          .map((seat) => seat.occupant.kind),
      })),
  });

  const append = (source: RecordSource, value: unknown) =>
    source.append("state", KEYFRAME, jsonRecord("state", value).payload);
  // A single source numbers each room/list and retains only its current snapshot.
  const listSource = recordSource(
    null,
    () => ({}),
    () => publishList(),
  );
  let lastList: string | undefined;
  const publishList = () => {
    const next = roomList();
    const encoded = JSON.stringify(next);
    if (encoded === lastList) return;
    lastList = encoded;
    append(listSource, next);
  };
  const unlistenList = online.onFrontlineChanged(publishList);

  const remove = (code: string, reason = "room_deleted") => {
    const stored = rooms.get(normalizeRoomCode(code));
    if (!stored) return;
    stored.idle?.cancel();
    rooms.delete(stored.room.code);
    stored.releaseGame();
    stored.slot.release();
    stored.battle?.host.dispose();
    stored.source.end(reason);
    for (const member of stored.room.members) {
      attendance.forget(seatKey(stored.room.code, member.who));
      releaseIfForgotten(member.who);
    }
    publishList();
  };
  const updateIdle = (stored: StoredRoom) => {
    const absences = stored.room.members.map((member) =>
      attendance.goneFor(seatKey(stored.room.code, member.who)),
    );
    if (absences.includes(null)) {
      stored.idle?.cancel();
      stored.idle = null;
      return;
    }
    if (stored.idle) return;
    const absentFor = absences.length ? Math.min(...(absences as number[])) : 0;
    stored.idle = {
      cancel: clock.schedule(
        () => remove(stored.room.code),
        Math.max(0, EMPTY_ROOM_TIMEOUT_MS - absentFor),
      ),
    };
  };
  const publishRoom = (stored: StoredRoom) => {
    // The source carries only a public view; each reader projects its own `you` and ticket.
    append(stored.source, roomView(stored.room, ""));
    publishList();
    updateIdle(stored);
  };
  /** Who commands `who`'s army: its human while they are in the room, its AI
   *  once they have left it by choice or been gone past the grace. Inside the
   *  grace it stays as it was. */
  const updateCommander = (stored: StoredRoom, who: string) => {
    const battle = stored.battle;
    if (!battle) return;
    // The model owns the seat-to-player projection, including dropped open seats.
    const member = stored.room.members.find((m) => m.who === who);
    if (!member) return;
    const absent = attendance.goneFor(seatKey(stored.room.code, who));
    const kind = member.away
      ? "active_ai"
      : absent === null
        ? "human"
        : absent >= PRESENCE_GRACE_MS
          ? "active_ai"
          : null;
    if (kind === null) return;
    const player = battlePlayers(stored.room.lobby.seats).playerOf[member.seat]!;
    if (battle.controllers.get(player) === kind) return;
    battle.controllers.set(player, kind);
    void battle.host
      .setController(player, kind)
      .catch(() => remove(stored.room.code, "battle_failed"));
  };
  const startBattle = (stored: StoredRoom) => {
    try {
      const setup = battleSetup(stored.room);
      const id = randomUUID();
      const host = online.createBattle([setup], id, (who) =>
        attendance.open(seatKey(stored.room.code, who)),
      );
      stored.battle = {
        host,
        controllers: new Map(setup.seats.map((s) => [s.player, s.controller])),
      };
      const ended = (reason: string) => {
        if (rooms.get(stored.room.code) === stored) remove(stored.room.code, reason);
      };
      host.onEnd((end) => ended(end.kind === "finished" ? "battle_finished" : "battle_failed"));
      for (const member of stored.room.members) updateCommander(stored, member.who);
      void host.ready
        .then(async ({ mapIdentity }) => {
          if (rooms.get(stored.room.code) !== stored) return;
          const address = online.battleAddress(host);
          stored.room = recordBattle(stored.room, { id, mapIdentity, address });
          stored.slot.serve(host);
          await stored.slot.seating(
            stored.room.members.map((member) => {
              const { phase } = roomView(stored.room, member.who);
              if (phase.kind !== "battle") throw new Error("battle ticket missing");
              return [member.who, { side: phase.ticket.side, player: phase.ticket.player }];
            }),
          );
          if (rooms.get(stored.room.code) === stored) publishRoom(stored);
        })
        .catch(() => ended("battle_failed"));
    } catch {
      remove(stored.room.code, "battle_failed");
    }
  };
  const transition = (
    stored: StoredRoom,
    who: string,
    action: Parameters<typeof roomReduce>[2],
  ) => {
    const next = roomReduce(stored.room, who, action, clock.now());
    if ("refused" in next) return next;
    if (next !== stored.room) {
      const previous = stored.room;
      stored.room = next;
      // A lobby nobody is left in has no host to start it and nobody to
      // come back to it: it goes with its last member.
      if (next.phase === "lobby" && next.members.length === 0) {
        remove(next.code);
        for (const member of previous.members) releaseIfForgotten(member.who);
        return { ok: true } as const;
      }
      // Nor is there anyone to play a battle every human has chosen to leave.
      if (deserted(next)) {
        remove(next.code);
        return { ok: true } as const;
      }
      publishRoom(stored);
      for (const member of previous.members) {
        if (!next.members.some((m) => m.who === member.who))
          attendance.forget(seatKey(next.code, member.who));
        releaseIfForgotten(member.who);
      }
      updateCommander(stored, who);
      if (previous.phase === "lobby" && next.phase === "starting") startBattle(stored);
    }
    return { ok: true } as const;
  };
  /** The started room `who` holds a seat in, which opening it returns them to. */
  const returnable = (who: string) =>
    [...rooms.values()].find(
      ({ room }) => room.phase !== "lobby" && room.members.some((m) => m.who === who),
    )?.room.code;
  // A room's members come and go by its own streams (`attendance`); the
  // lobby is told only whose callsign a room still holds.
  const unsubscribe = online.subscribe({
    remembers: (who) =>
      [...rooms.values()].some(({ room }) => room.members.some((m) => m.who === who)),
  });

  return {
    create(
      who: string,
      visibility: Room["visibility"],
      device = who,
    ): { code: string } | RoomRefusal {
      const callsign = online.callsign(who);
      if (!callsign) return { refused: "callsign" };
      const releaseGame = online.claimGame(device);
      if (!releaseGame) return { refused: "game_running" };
      if (!online.slotFree()) {
        releaseGame();
        return { refused: "full" };
      }
      presence.track(who);
      let code: string;
      do {
        code = roomCode(options.random);
      } while (rooms.has(code));
      attendance.track(seatKey(code, who));
      const source = recordSource(
        null,
        () => ({}),
        () => append(source, roomView(stored.room, "")),
      );
      const stored: StoredRoom = {
        releaseGame,
        slot: online.reserve()!,
        room: createRoom(
          { who, callsign },
          visibility,
          code,
          seedFrom(options.random),
          clock.now(),
        ),
        source,
        idle: null,
        battle: null,
      };
      rooms.set(code, stored);
      updateIdle(stored);
      publishList();
      return { code };
    },
    listStream(who: string, after: number | null, deliver: Deliver) {
      const closePresence = presence.open(who);
      // The list is everyone's; which room is the reader's own is added for
      // them alone, as of each record they are sent.
      const detach = listSource.attach(after, (record) => {
        const yours = record.kind === "state" ? returnable(who) : undefined;
        return deliver(
          yours
            ? {
                ...record,
                payload: jsonRecord("state", { ...readJson<RoomList>(record), yours }).payload,
              }
            : record,
        );
      });
      return () => {
        detach();
        closePresence();
      };
    },
    roomStream(
      who: string,
      code: string,
      after: number | null,
      deliver: Deliver,
    ): { close: () => void } | RoomRefusal {
      const stored = rooms.get(normalizeRoomCode(code));
      const returning = stored?.room.members.some((member) => member.who === who && !member.away);
      // A resume keeps an existing membership; it must never silently join
      // again, nor bring back a member who left the battle: that page is gone.
      if (after !== null && !returning) return { refused: "removed" };
      if (!stored) return { refused: "unknown" };
      const callsign = online.callsign(who);
      if (!callsign) return { refused: "callsign" };
      const joined = transition(stored, who, { kind: "join", callsign });
      if ("refused" in joined) return joined;
      const closePresence = presence.open(who);
      const closeAttendance = attendance.open(seatKey(stored.room.code, who));
      const detach = stored.source.attach(after, (record) =>
        deliver(
          record.kind === "state"
            ? { ...record, payload: jsonRecord("state", roomView(stored.room, who)).payload }
            : record,
        ),
      );
      return {
        close: () => {
          detach();
          closeAttendance();
          closePresence();
        },
      };
    },
    act(who: string, code: string, action: RoomAction): { ok: true } | RoomRefusal {
      const stored = rooms.get(normalizeRoomCode(code));
      return stored ? transition(stored, who, action) : { refused: "unknown" };
    },
    dispose() {
      for (const stored of rooms.values()) {
        stored.releaseGame();
        stored.idle?.cancel();
        stored.battle?.host.dispose();
        stored.slot.release();
        stored.source.end("disposed");
      }
      rooms.clear();
      attendance.dispose();
      listSource.end("disposed");
      unsubscribe();
      unlistenList();
    },
  };
}
export type RoomService = ReturnType<typeof createRoomService>;
