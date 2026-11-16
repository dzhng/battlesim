// @vitest-environment node
import { afterEach, expect, test, vi } from "vitest";
import { frontlineHost } from "./support/frontlineHost";
import { PageClock } from "./pageClock";
import { EMPTY_ROOM_TIMEOUT_MS } from "../../../apps/game-server/src/rooms";
import { PRESENCE_GRACE_MS } from "../../../apps/game-server/src/presence";
import { GONE_TIMEOUT_MS, IDLE_TIMEOUT_MS } from "../../../packages/multiplayer/src/frontline";
import { recordDecoder, readJson } from "../../../packages/multiplayer/src/stream/index";
import type { RoomView } from "../../../packages/multiplayer/src/wire";
import type { BattleEnd } from "../../../apps/game-server/src/battleHost/index";
import { roomHttp, build } from "./support/roomHttp";

let lobby: Awaited<ReturnType<typeof roomHttp>>;
afterEach(async () => {
  await lobby?.close();
});
const ids = Array.from({ length: 4 }, (_, i) => (i + 1).toString().repeat(32));
test.each([null, "X-Trusted-Client"])(
  "HTTP one device may create one room, but may join another (%s)",
  async (header) => {
    lobby = await roomHttp(8, undefined, undefined, header);
    const from = (device: string) => ({ "X-Trusted-Client": device });
    for (const [i, id] of ids.entries())
      await lobby.post("/hello", id, { build, callsign: `SCOUT-${i}` });
    const a = await lobby.post("/rooms", ids[0], { visibility: "public" }, from("a"));
    expect(a).toEqual({ code: expect.any(String) });
    expect(await lobby.post("/rooms", ids[0], { visibility: "private" }, from("a"))).toEqual({
      refused: "game_running",
    });
    if (header) {
      expect(await lobby.post("/rooms", ids[1], { visibility: "private" }, from("a"))).toEqual({
        refused: "game_running",
      });
      expect(await lobby.post("/rooms", ids[1], { visibility: "private" })).toEqual({
        refused: "address",
      });
    }
    const b = await lobby.post("/rooms", ids[2], { visibility: "public" }, from("b"));
    expect(b).toEqual({ code: expect.any(String) });
    const response = await fetch(`${lobby.address}/rooms/${b.code}`, {
      headers: { "X-Player": ids[0], ...from("a") },
    });
    expect(response.status).toBe(200);
    await response.body!.cancel();
  },
);

test.each([null, "X-Trusted-Client"])(
  "HTTP ownership spans rooms and Frontline but never limits joining (%s)",
  async (header) => {
    lobby = await roomHttp(8, (setup) => frontlineHost(setup), undefined, header);
    const from = (device: string) => ({ "X-Trusted-Client": device });
    for (const [i, id] of ids.entries())
      await lobby.post("/hello", id, { build, callsign: `SCOUT-${i}` });
    const room = await lobby.post("/rooms", ids[0], { visibility: "private" }, from("a"));
    expect(room).toEqual({ code: expect.any(String) });
    expect(await lobby.post("/frontline/join", ids[0], {}, from("a"))).toEqual({
      refused: "game_running",
    });
    const game = await lobby.post("/frontline/join", ids[2], {}, from("b"));
    expect(game.battle).toEqual(expect.any(String));
    expect(await lobby.post("/rooms", ids[2], { visibility: "private" }, from("b"))).toEqual({
      refused: "game_running",
    });
    // Being a room's maker does not limit joining an existing Frontline game.
    const joined = await lobby.post("/frontline/join", ids[0], {}, from("a"));
    expect(joined.battle).toBe(game.battle);
    expect(
      (await lobby.post(`/frontline/${game.battle}/deploy`, ids[0], {}, from("a"))).battle,
    ).toBe(game.battle);
    expect(
      (await lobby.post(`/frontline/${game.battle}/deploy`, ids[2], {}, from("b"))).battle,
    ).toBe(game.battle);
  },
);

test.each([
  { mode: "room", header: null },
  { mode: "frontline", header: null },
  { mode: "room", header: "X-Trusted-Client" },
  { mode: "frontline", header: "X-Trusted-Client" },
])(
  "HTTP $mode maker's place frees on every end path (header=$header)",
  async ({ mode, header }) => {
    const clock = new PageClock();
    let end: (reason: BattleEnd) => void = () => {};
    lobby = await roomHttp(
      8,
      (setup) => ({
        ...frontlineHost(setup),
        onEnd(listener) {
          end = listener;
        },
      }),
      clock,
      header,
    );
    const headers = { "X-Trusted-Client": "maker-device" };
    const post = (path: string, who: string, value: unknown) =>
      lobby.post(path, who, value, headers);
    const who = ids[0];
    const make = async () => {
      await post("/hello", who, { build, callsign: "MAKER" });
      if (mode === "frontline") {
        const briefing = await post("/frontline/join", who, {});
        expect(briefing.battle).toEqual(expect.any(String));
        return briefing.battle as string;
      }
      const room = await post("/rooms", who, { visibility: "public" });
      expect(room.code).toEqual(expect.any(String));
      await post(`/rooms/${room.code}/act`, who, {
        kind: "set_seat",
        seat: "seat-2",
        to: "ai",
      });
      await post(`/rooms/${room.code}/act`, who, { kind: "ready", ready: true });
      await post(`/rooms/${room.code}/act`, who, { kind: "start" });
      // Starting the room is async; a ticket proves the lifecycle listener exists.
      const response = await fetch(`${lobby.address}/rooms/${room.code}`, {
        headers: { "X-Player": who, ...headers },
      });
      const reader = response.body!.getReader();
      const decoder = recordDecoder();
      for (;;) {
        const chunk = await reader.read();
        if (chunk.done) throw new Error("room ended before battle");
        if (
          decoder
            .push(chunk.value)
            .some((r) => r.kind === "state" && readJson<RoomView>(r).phase.kind === "battle")
        )
          break;
      }
      await reader.cancel();
      await post("/hello", who, { build, callsign: "MAKER" });
      return room.code as string;
    };
    for (const reason of [
      { kind: "finished" },
      { kind: "crashed", message: "test crash" },
    ] as BattleEnd[]) {
      await make();
      expect(await post("/rooms", who, { visibility: "private" })).toEqual({
        refused: "game_running",
      });
      end(reason);
      await vi.waitFor(async () => {
        const next = await post("/rooms", who, { visibility: "private" });
        expect(next.code).toEqual(expect.any(String));
      });
      await clock.advance(EMPTY_ROOM_TIMEOUT_MS + PRESENCE_GRACE_MS);
    }
    const id = await make();
    if (mode === "frontline") await post(`/frontline/${id}/leave`, who, {});
    else await clock.advance(EMPTY_ROOM_TIMEOUT_MS + PRESENCE_GRACE_MS);
    await post("/hello", who, { build, callsign: "MAKER" });
    expect((await post("/rooms", who, { visibility: "private" })).code).toEqual(expect.any(String));
    await clock.advance(EMPTY_ROOM_TIMEOUT_MS + PRESENCE_GRACE_MS);
    await make();
    // An un-deployed reservation expires; an abandoned room times out.
    await clock.advance(Math.max(GONE_TIMEOUT_MS, EMPTY_ROOM_TIMEOUT_MS) + PRESENCE_GRACE_MS);
    await post("/hello", who, { build, callsign: "MAKER" });
    expect((await post("/rooms", who, { visibility: "private" })).code).toEqual(expect.any(String));
    if (mode === "frontline") {
      await clock.advance(EMPTY_ROOM_TIMEOUT_MS + PRESENCE_GRACE_MS);
      const playing = await make();
      const presence = await fetch(`${lobby.address}/rooms`, {
        headers: { "X-Player": who, ...headers },
      });
      await post(`/frontline/${playing}/deploy`, who, {});
      await clock.advance(IDLE_TIMEOUT_MS);
      expect((await post("/rooms", who, { visibility: "private" })).code).toEqual(
        expect.any(String),
      );
      await presence.body!.cancel();
    }
  },
);

test("the maker gets the one-game refusal even when their game fills the lobby", async () => {
  lobby = await roomHttp(1);
  await lobby.post("/hello", ids[0], { build, callsign: "HOST" });
  expect((await lobby.post("/rooms", ids[0], { visibility: "public" })).code).toEqual(
    expect.any(String),
  );
  expect(await lobby.post("/rooms", ids[0], { visibility: "private" })).toEqual({
    refused: "game_running",
  });
  expect(await lobby.post("/frontline/join", ids[0], {})).toEqual({ refused: "game_running" });
});

test("a failed Frontline preparation frees the device and a pending one already owns it", async () => {
  let reject = (_error: Error) => {};
  const ready = new Promise<never>((_resolve, no) => {
    reject = no;
  });
  let started = false;
  lobby = await roomHttp(8, (setup) => {
    started = true;
    return { ...frontlineHost(setup), ready };
  });
  await lobby.post("/hello", ids[0], { build, callsign: "HOST" });
  const playing = lobby.post("/frontline/join", ids[0], {});
  void playing.catch(() => {});
  // The pending host has been allocated only once the local control server receives start.
  await vi.waitFor(() => expect(started).toBe(true));
  expect(await lobby.post("/rooms", ids[0], { visibility: "private" })).toEqual({
    refused: "game_running",
  });
  reject(new Error("test map refused"));
  expect(await playing).toEqual({ refused: "unavailable" });
  expect((await lobby.post("/rooms", ids[0], { visibility: "private" })).code).toEqual(
    expect.any(String),
  );
});
