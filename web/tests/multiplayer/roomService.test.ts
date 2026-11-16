import { inProcessBattleMachines } from "../../../apps/game-server/src/machines/inProcess";
import {
  createOnlineState,
  battleCapacity,
  BYTES_PER_BATTLE,
} from "../../../apps/game-server/src/online";
import { pendingBattle } from "./support/pendingBattle";
// @vitest-environment node
import { afterEach, expect, test, vi } from "vitest";
import {
  EMPTY_ROOM_TIMEOUT_MS,
  THROTTLE_MAX_GUESSES,
  THROTTLE_WINDOW_MS,
  createRoomService,
} from "../../../apps/game-server/src/rooms";
import { PRESENCE_GRACE_MS, type PresenceClock } from "../../../apps/game-server/src/presence";
import { helloRegistry } from "../../../apps/game-server/src/http/hello";
import { createLobbyHttp } from "../../../apps/game-server/src/http/index";
import { recordDecoder, readJson } from "../../../packages/multiplayer/src/stream/index";
import type {
  BuildIdentity,
  RoomList,
  RoomView,
  StreamRecord,
} from "../../../packages/multiplayer/src/wire";

class Clock implements PresenceClock {
  time = 0;
  tasks = new Set<{ at: number; run: () => void }>();
  now = () => this.time;
  schedule = (run: () => void, delay: number) => {
    const task = { at: this.time + delay, run };
    this.tasks.add(task);
    return () => {
      this.tasks.delete(task);
    };
  };
  advance(ms: number) {
    const until = this.time + ms;
    for (;;) {
      const next = [...this.tasks]
        .filter((task) => task.at <= until)
        .sort((a, b) => a.at - b.at)[0];
      if (!next) break;
      this.tasks.delete(next);
      this.time = next.at;
      next.run();
    }
    this.time = until;
  }
}

const cleanup: (() => void | Promise<void>)[] = [];
afterEach(async () => {
  for (const close of cleanup.splice(0).reverse()) await close();
});

function setup(capacity = 3) {
  const clock = new Clock();
  let draw = 0;
  const service = createRoomService({
    createBattle: pendingBattle,
    online: createOnlineState({
      capacity,
      clock,
      releaseCallsign: () => {},
      callsign: (who) => who.toUpperCase(),
    }),
    random: () => (draw++ % 1000) / 1000,
  });
  cleanup.push(service.dispose);
  const made = service.create("alpha", "public");
  if ("refused" in made) throw new Error(made.refused);
  const open = (who: string, code = made.code, after: number | null = null, address = "client") => {
    const records: StreamRecord[] = [];
    const opened = service.roomStream(who, code, address, after, (record) => {
      records.push(record);
      return true;
    });
    if ("refused" in opened) throw new Error(opened.refused);
    return {
      records,
      close: opened.close,
      states: () => records.filter((r) => r.kind === "state").map((r) => readJson<RoomView>(r)),
    };
  };
  const listRecords: StreamRecord[] = [];
  const closeList = service.listStream("observer", null, (record) => {
    listRecords.push(record);
    return true;
  });
  cleanup.push(closeList);
  const lists = () =>
    listRecords
      .filter((record) => record.kind === "state")
      .map((record) => readJson<RoomList>(record));
  return { service, clock, code: made.code, open, lists };
}

test("each new room opens on a map of its own", () => {
  const { service } = setup(8);
  const seeds = ["bravo", "charlie", "delta"].map((who) => {
    const made = service.create(who, "public");
    if ("refused" in made) throw new Error(made.refused);
    let view: RoomView | undefined;
    const opened = service.roomStream(who, made.code, "client", null, (record) => {
      if (record.kind === "state") view = readJson<RoomView>(record);
      return true;
    });
    if ("refused" in opened) throw new Error(opened.refused);
    opened.close();
    return view!.settings.map.seed;
  });
  expect(new Set(seeds).size).toBe(seeds.length);
});
test("room streams always open with the current state and both players receive their own view of every change", () => {
  const { service, code, open } = setup();
  const host = open("alpha");
  expect(host.records.map((record) => record.kind)).toEqual(["open", "state"]);
  const guest = open("bravo");
  expect(guest.records.map((record) => record.kind)).toEqual(["open", "state"]);
  const initial = host.states().at(-1)!;
  const guestInitial = guest.states().at(-1)!;
  expect(initial.you).not.toBe(guestInitial.you);
  expect(initial.seats).toEqual(guestInitial.seats);
  expect(service.act("bravo", code, { kind: "ready", ready: true })).toEqual({ ok: true });
  expect(
    host
      .states()
      .at(-1)
      ?.seats.find((seat) => seat.id === guestInitial.you)?.occupant,
  ).toMatchObject({ ready: true });
  expect(guest.states().at(-1)).toEqual({ ...host.states().at(-1), you: guestInitial.you });
  const last = guest.records.at(-1)!;
  guest.close();
  const resumed = open("bravo", code, last.id);
  expect(resumed.records.map((record) => record.kind)).toEqual(["open", "state"]);
  expect(resumed.states()).toEqual([guest.states().at(-1)]);
  host.close();
  resumed.close();
});

test("the list publishes a public lobby and removes it on start while a private room stays unlisted", () => {
  const { service, code, open, lists } = setup();
  expect(lists().at(-1)?.rooms).toEqual([
    expect.objectContaining({ code, host: "ALPHA", blue: ["human"], red: ["open"] }),
  ]);
  const privateRoom = service.create("private-host", "private");
  if ("refused" in privateRoom) throw new Error(privateRoom.refused);
  expect(
    lists()
      .at(-1)
      ?.rooms.map((room) => room.code),
  ).toEqual([code]);
  const host = open("alpha");
  const redSeat = host
    .states()
    .at(-1)!
    .seats.find((seat) => seat.occupant.kind === "open")!.id;
  expect(service.act("alpha", code, { kind: "set_seat", seat: redSeat, to: "ai" })).toEqual({
    ok: true,
  });
  expect(service.act("alpha", code, { kind: "ready", ready: true })).toEqual({ ok: true });
  expect(service.act("alpha", code, { kind: "start" })).toEqual({ ok: true });
  expect(host.states().at(-1)?.phase).toEqual({ kind: "starting" });
  expect(lists().at(-1)).toEqual({ rooms: [], frontline: { battles: 0, commanders: 0 } });
  const reopened = open("alpha", code, host.records.at(-1)!.id);
  expect(reopened.states().at(-1)?.you).toBe(host.states().at(-1)?.you);
  expect(reopened.states().at(-1)?.phase).toEqual({ kind: "starting" });
  expect(service.roomStream("newcomer", code, "another", null, () => true)).toEqual({
    refused: "started",
  });
  host.close();
  reopened.close();
});

test("a surviving stream and a reconnect inside the grace preserve the room, but a gone host frees its seat and passes hosting", () => {
  const { service, clock, code, open } = setup();
  const host = open("alpha");
  const otherTab = open("alpha");
  const guest = open("bravo");
  expect(service.act("alpha", code, { kind: "ready", ready: true })).toEqual({ ok: true });
  const before = guest.states().at(-1)!;
  const hostSeat = before.seats.find(
    (seat) => seat.occupant.kind === "human" && seat.occupant.host,
  )!.id;
  host.close();
  clock.advance(PRESENCE_GRACE_MS);
  expect(guest.states().at(-1)).toEqual(before);
  otherTab.close();
  clock.advance(PRESENCE_GRACE_MS / 2);
  const back = open("alpha");
  clock.advance(PRESENCE_GRACE_MS);
  expect(guest.states().at(-1)).toEqual(before);
  expect(back.states().at(-1)?.you).toBe(hostSeat);
  back.close();
  clock.advance(PRESENCE_GRACE_MS - 1);
  expect(guest.states().at(-1)).toEqual(before);
  clock.advance(1);
  const after = guest.states().at(-1)!;
  expect(after.seats.find((seat) => seat.id === hostSeat)?.occupant).toEqual({ kind: "open" });
  expect(after.seats.find((seat) => seat.id === after.you)?.occupant).toMatchObject({
    kind: "human",
    callsign: "BRAVO",
    host: true,
  });
  const replacement = open("charlie");
  expect(replacement.states().at(-1)?.you).toBe(hostSeat);
  replacement.close();
  guest.close();
});

test.each([false, true])(
  "a room without connected humans is deleted and releases its reserved slot, started=%s",
  (start) => {
    const { service, clock, code, open, lists } = setup(1);
    const host = open("alpha");
    if (start) {
      const seat = host
        .states()
        .at(-1)!
        .seats.find((seat) => seat.occupant.kind === "open")!.id;
      expect(service.act("alpha", code, { kind: "set_seat", seat, to: "ai" })).toEqual({
        ok: true,
      });
      expect(service.act("alpha", code, { kind: "ready", ready: true })).toEqual({ ok: true });
      expect(service.act("alpha", code, { kind: "start" })).toEqual({ ok: true });
    }
    host.close();
    clock.advance(EMPTY_ROOM_TIMEOUT_MS - 1);
    expect(service.create("another", "public")).toEqual({ refused: "full" });
    clock.advance(1);
    expect(lists().at(-1)).toEqual({ rooms: [], frontline: { battles: 0, commanders: 0 } });
    expect(service.roomStream("alpha", code, "address", null, () => true)).toEqual({
      refused: "unknown",
    });
    const next = service.create("another", "public");
    expect(next).toEqual({ code: expect.any(String) });
  },
);

test("creation reserves every available slot before any battle starts", () => {
  const { service } = setup(2);
  expect(service.create("second", "private")).toEqual({ code: expect.any(String) });
  expect(service.create("third", "public")).toEqual({ refused: "full" });
});

test("capacity is bounded by both the machine's spare cores and its battle memory budget", () => {
  expect(battleCapacity(5, BYTES_PER_BATTLE * 2.5)).toBe(2);
  expect(battleCapacity(3, BYTES_PER_BATTLE * 20)).toBe(2);
  expect(battleCapacity(1, BYTES_PER_BATTLE * 20)).toBe(0);
  expect(battleCapacity(8, BYTES_PER_BATTLE / 2)).toBe(0);
});

test("wrong code guesses throttle one address, another can still join, and the throttle expires", () => {
  const { service, clock, open } = setup();
  const made = service.create("private-host", "private");
  if ("refused" in made) throw new Error(made.refused);
  for (let i = 0; i < THROTTLE_MAX_GUESSES; i++) {
    expect(
      service.roomStream(`rotated-${i}`, "WRONG-CODE", "guessing-address", null, () => true),
    ).toEqual({ refused: "unknown" });
  }
  expect(
    service.roomStream("outsider", "WRONG-CODE", "guessing-address", null, () => true),
  ).toEqual({ refused: "throttled" });
  expect(service.roomStream("outsider", made.code, "guessing-address", null, () => true)).toEqual({
    refused: "throttled",
  });
  const owner = open("private-host", made.code);
  const other = open("other", made.code, null, "other-address");
  expect(
    other
      .states()
      .at(-1)
      ?.seats.find((seat) => seat.id === other.states().at(-1)?.you)?.occupant,
  ).toMatchObject({ callsign: "OTHER" });
  expect(service.act("other", made.code, { kind: "leave" })).toEqual({ ok: true });
  other.close();
  clock.advance(THROTTLE_WINDOW_MS);
  const allowed = open("outsider", made.code, null, "guessing-address");
  expect(
    allowed
      .states()
      .at(-1)
      ?.seats.find((seat) => seat.id === allowed.states().at(-1)?.you)?.occupant,
  ).toMatchObject({ callsign: "OUTSIDER" });
  owner.close();
  allowed.close();
});

const build: BuildIdentity = { data: "test-commit", wire: "test-commit", sim: "test-sim" };
const ids = ["1".repeat(32), "2".repeat(32), "3".repeat(32), "4".repeat(32)];
async function serving(clientAddressHeader: string | null = null) {
  const clock = new Clock();
  const server = createLobbyHttp(build, {
    machines: inProcessBattleMachines({
      capacity: 3,
      createBattle: pendingBattle,
      ordersPerSecond: 100,
    }),
    mapAttempts: 1,
    clock,
    random: Math.random,
    clientAddressHeader,
  });
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  const address = server.address();
  if (!address || typeof address === "string") throw new Error("no TCP address");
  cleanup.push(async () => {
    server.closeAllConnections();
    await new Promise<void>((resolve) => server.close(() => resolve()));
  });
  const replies: string[] = [];
  const request = (
    path: string,
    value?: unknown,
    id = ids[0],
    headers: Record<string, string> = {},
  ) =>
    fetch(`http://127.0.0.1:${address.port}${path}`, {
      method: value === undefined ? "GET" : "POST",
      headers: { "X-Player": id, "Content-Type": "application/json", ...headers },
      body: value === undefined ? undefined : JSON.stringify(value),
    });
  const reply = async (
    path: string,
    value?: unknown,
    id = ids[0],
    headers: Record<string, string> = {},
  ) => {
    const response = await request(path, value, id, headers);
    const text = await response.text();
    replies.push(text);
    return JSON.parse(text);
  };
  const hello = (id: string, callsign: string) => reply("/hello", { build, callsign }, id);
  const open = async (path: string, id = ids[0], headers: Record<string, string> = {}) => {
    const response = await request(path, undefined, id, headers);
    expect(response.status).toBe(200);
    expect(response.headers.get("content-type")).toBe("application/octet-stream");
    expect(response.headers.get("x-accel-buffering")).toBe("no");
    expect(response.headers.get("cache-control")).toBe("no-store, no-transform");
    const reader = response.body!.getReader();
    cleanup.push(async () => {
      await reader.cancel();
    });
    const decoder = recordDecoder();
    const records: StreamRecord[] = [];
    const pending: StreamRecord[] = [];
    return {
      records,
      async state<T>(): Promise<T> {
        for (;;) {
          while (pending.length) {
            const record = pending.shift()!;
            if (record.kind === "state") return readJson<T>(record);
            if (record.kind === "end") throw new Error("stream ended before state");
          }
          const chunk = await reader.read();
          if (chunk.done) throw new Error("connection ended before state");
          const decoded = decoder.push(chunk.value);
          records.push(...decoded);
          pending.push(...decoded);
        }
      },
    };
  };
  return { clock, hello, reply, open, request, replies };
}

test("localhost routes create, list, join and act; both streams update from their transport identity without exposing credentials", async () => {
  const spies = (["log", "info", "warn", "error", "debug"] as const).map((method) =>
    vi.spyOn(console, method).mockImplementation(() => {}),
  );
  cleanup.push(() => {
    for (const spy of spies) spy.mockRestore();
  });
  const http = await serving();
  expect(await http.reply("/rooms", { visibility: "public" })).toEqual({ refused: "callsign" });
  expect(await http.hello(ids[0], "SCOUT")).toEqual({ callsign: "SCOUT" });
  expect(await http.hello(ids[1], "SCOUT")).toEqual({ callsign: "SCOUT#2" });
  expect(await http.hello(ids[2], "STRANGER")).toEqual({ callsign: "STRANGER" });
  const list = await http.open("/rooms");
  expect(await list.state()).toEqual({ rooms: [], frontline: { battles: 0, commanders: 0 } });
  const made = await http.reply("/rooms", { visibility: "public" });
  expect((await list.state<RoomList>()).rooms).toEqual([
    expect.objectContaining({ code: made.code, host: "SCOUT" }),
  ]);
  const host = await http.open(`/rooms/${made.code}`);
  const hostInitial = await host.state<RoomView>();
  const guest = await http.open(`/rooms/${made.code}`, ids[1]);
  const guestInitial = await guest.state<RoomView>();
  expect(guestInitial.you).not.toBe(hostInitial.you);
  expect(guestInitial.seats.find((seat) => seat.id === guestInitial.you)?.occupant).toMatchObject({
    callsign: "SCOUT#2",
  });
  await host.state();
  // The guessed actor in the body cannot turn a guest's Ready into the host's Ready.
  expect(
    await http.reply(
      `/rooms/${made.code}/act`,
      { kind: "ready", ready: true, who: ids[0], player: ids[0] },
      ids[1],
    ),
  ).toEqual({ ok: true });
  const hostChanged = await host.state<RoomView>();
  const guestChanged = await guest.state<RoomView>();
  expect(hostChanged.seats.find((seat) => seat.id === guestInitial.you)?.occupant).toMatchObject({
    ready: true,
  });
  expect(hostChanged.seats.find((seat) => seat.id === hostInitial.you)?.occupant).toMatchObject({
    ready: false,
  });
  expect(guestChanged).toEqual({ ...hostChanged, you: guestInitial.you });
  expect(
    await http.reply(
      `/rooms/${made.code}/act`,
      { kind: "ready", ready: true, who: ids[0] },
      ids[2],
    ),
  ).toEqual({ refused: "you are not in this room" });
  for (const bad of [
    null,
    { kind: "ready", ready: "yes" },
    { kind: "sit" },
    { kind: "settings", patch: null },
  ]) {
    expect(await http.reply(`/rooms/${made.code}/act`, bad)).toEqual({ refused: "invalid" });
  }
  expect(
    await http.reply(`/rooms/${made.code}/act`, { kind: "settings", patch: { lighting: ids[0] } }),
  ).toEqual({ refused: "unknown lighting" });
  expect(await http.reply(`/rooms/${made.code}?after=bad`)).toEqual({ refused: "invalid" });
  expect(await http.reply(`/rooms/${made.code}`, undefined, "invalid-credential")).toEqual({
    refused: "invalid",
  });
  const reopened = await http.open(`/rooms/${made.code}?after=${guest.records.at(-1)!.id}`, ids[1]);
  expect(await reopened.state()).toEqual(guestChanged);
  const sent = [
    http.replies,
    [list, host, guest, reopened].flatMap((stream) =>
      stream.records.map((record) => new TextDecoder().decode(record.payload)),
    ),
    spies.flatMap((spy) => spy.mock.calls),
  ];
  for (const id of ids) expect(JSON.stringify(sent)).not.toContain(id);
  expect(spies.flatMap((spy) => spy.mock.calls)).toEqual([]);
});

test.each([null, "X-Trusted-Client"])(
  "HTTP code throttling uses the configured address boundary (%s)",
  async (header) => {
    const http = await serving(header);
    await http.hello(ids[0], "HOST");
    await http.hello(ids[1], "GUEST");
    const guestList = await http.open("/rooms", ids[1]);
    await guestList.state();
    const { code } = await http.reply("/rooms", { visibility: "private" });
    const from = (address: string) => ({ "X-Trusted-Client": address });
    const owner = await http.open(`/rooms/${code}`, ids[0], from("owner"));
    await owner.state();
    for (let i = 0; i < THROTTLE_MAX_GUESSES; i++) {
      expect(
        await http.reply(
          "/rooms/WRONG-CODE",
          undefined,
          i.toString(16).padStart(32, "0"),
          from("guesser"),
        ),
      ).toEqual({ refused: "unknown" });
    }
    const blocked = await http.request("/rooms/WRONG-CODE", undefined, ids[1], from("guesser"));
    expect(blocked.status).toBe(429);
    expect(await blocked.json()).toEqual({ refused: "throttled" });
    if (header) {
      expect(await http.reply(`/rooms/${code}`, undefined, ids[1])).toEqual({ refused: "address" });
      const other = await http.open(`/rooms/${code}`, ids[1], from("other-address"));
      expect(
        (await other.state<RoomView>()).seats.some(
          (seat) => seat.occupant.kind === "human" && seat.occupant.callsign === "GUEST",
        ),
      ).toBe(true);
    } else {
      // An unconfigured, caller-supplied header cannot split the socket's allowance.
      expect(
        await http.reply(`/rooms/${code}`, undefined, ids[1], from("spoofed-address")),
      ).toEqual({ refused: "throttled" });
      http.clock.advance(THROTTLE_WINDOW_MS);
      const allowed = await http.open(`/rooms/${code}`, ids[1], from("spoofed-address"));
      expect(
        (await allowed.state<RoomView>()).seats.some(
          (seat) => seat.occupant.kind === "human" && seat.occupant.callsign === "GUEST",
        ),
      ).toBe(true);
    }
  },
);

test("hello names are released when a browser without any stream has been forgotten by its lobby", async () => {
  const http = await serving();
  expect(await http.hello(ids[0], "SCOUT")).toEqual({ callsign: "SCOUT" });
  const { code } = await http.reply("/rooms", { visibility: "public" });
  http.clock.advance(PRESENCE_GRACE_MS);
  expect(await http.hello(ids[1], "SCOUT")).toEqual({ callsign: "SCOUT" });
  http.clock.advance(EMPTY_ROOM_TIMEOUT_MS - PRESENCE_GRACE_MS);
  expect(await http.reply(`/rooms/${code}`, undefined, ids[1])).toEqual({ refused: "unknown" });
});

test("a callsign stays reserved while a started room remembers its gone member, then is released when all rooms forget them", () => {
  const clock = new Clock();
  const hello = helloRegistry(build);
  let draw = 0;
  const service = createRoomService({
    createBattle: pendingBattle,
    online: createOnlineState({
      capacity: 2,
      clock: clock,
      callsign: hello.callsignOf,
      releaseCallsign: hello.release,
    }),
    random: () => draw++ / 100,
  });
  cleanup.push(service.dispose);
  expect(hello(ids[0], { build, callsign: "SCOUT" })).toEqual({ callsign: "SCOUT" });
  const first = service.create(ids[0], "public");
  const second = service.create(ids[0], "private");
  if ("refused" in first || "refused" in second) throw new Error("creation refused");
  const view: RoomView[] = [];
  const open = service.roomStream(ids[0], second.code, "address", null, (record) => {
    if (record.kind === "state") view.push(readJson<RoomView>(record));
    return true;
  });
  if ("refused" in open) throw new Error(open.refused);
  const emptySeat = view.at(-1)!.seats.find((seat) => seat.occupant.kind === "open")!.id;
  expect(service.act(ids[0], second.code, { kind: "set_seat", seat: emptySeat, to: "ai" })).toEqual(
    { ok: true },
  );
  expect(service.act(ids[0], second.code, { kind: "ready", ready: true })).toEqual({ ok: true });
  expect(service.act(ids[0], second.code, { kind: "start" })).toEqual({ ok: true });
  open.close();
  clock.advance(PRESENCE_GRACE_MS);
  expect(hello(ids[1], { build, callsign: "SCOUT" })).toEqual({ callsign: "SCOUT#2" });
  clock.advance(EMPTY_ROOM_TIMEOUT_MS - PRESENCE_GRACE_MS);
  expect(hello(ids[2], { build, callsign: "SCOUT" })).toEqual({ callsign: "SCOUT" });
});
