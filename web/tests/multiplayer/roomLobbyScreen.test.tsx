// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { MemoryRouter, Route, Routes } from "react-router";
import { PRESENCE_GRACE_MS } from "@apps/game-server/src/presence";
import { roomHttp, build } from "./support/roomHttp";
import { RoomPage } from "@apps/battle-lab/src/online/RoomPage";
import { MultiplayerPage } from "@apps/battle-lab/src/online/MultiplayerPage";
import { browserId } from "@apps/battle-lab/src/online/identity";
import { PreviewWorker } from "../support/previewWorker";
import { httpRecords } from "@packages/multiplayer/src/stream/http";
import { readJson } from "@packages/multiplayer/src/stream/index";
import type { RoomView, RoomAction } from "@packages/multiplayer/src/wire";
vi.mock("@apps/battle-lab/src/buildIdentity", () => ({ buildIdentity: async () => build }));
vi.mock("@apps/battle-lab/src/menu/MenuShell", () => ({ useMenuShell: () => ({ back() {} }) }));
const address = vi.hoisted(() => ({ value: "" }));
vi.mock("@apps/battle-lab/src/online/server", () => ({ serverAddress: () => address.value }));
let service: Awaited<ReturnType<typeof roomHttp>>;
let lifetime: AbortController;
beforeEach(() => {
  PreviewWorker.all = [];
  vi.stubGlobal("Worker", PreviewWorker);
  lifetime = new AbortController();
});
afterEach(async () => {
  cleanup();
  lifetime.abort();
  await service?.close();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});
const other = "c".repeat(32);
async function snapshot(code: string, who = browserId()) {
  for await (const record of httpRecords(
    `${service.address}/rooms/${code}`,
    who,
    lifetime.signal,
  )) {
    if (record.kind === "state") return readJson<RoomView>(record);
  }
  throw new Error("No room view");
}
async function open(asHost = true) {
  service = await roomHttp();
  address.value = service.address;
  await service.post("/hello", browserId(), { build, callsign: "SCOUT" });
  await service.post("/hello", other, { build, callsign: "WING" });
  const host = asHost ? browserId() : other;
  const created = await service.post("/rooms", host, { visibility: "public" });
  const code: string = created.code;
  await snapshot(code, asHost ? other : browserId());
  // Host, guest, one open seat and one AI, using the real room actions.
  await service.post(`/rooms/${code}/act`, host, { kind: "add_seat", side: "blue" });
  await service.post(`/rooms/${code}/act`, host, { kind: "set_seat", seat: "seat-3", to: "open" });
  await service.post(`/rooms/${code}/act`, host, { kind: "add_seat", side: "red" });
  const page = render(
    <MemoryRouter initialEntries={[`/room/${code}`]}>
      <Routes>
        <Route path="/room/:code" element={<RoomPage />} />
        <Route path="/multiplayer" element={<MultiplayerPage />} />
      </Routes>
    </MemoryRouter>,
  );
  await page.findByRole("region", { name: "Room lobby" });
  return { page, code, host };
}
const group = (page: ReturnType<typeof render>, side: string) =>
  within(page.getByRole("group", { name: `${side} side` }));
const waitView = async (code: string, predicate: (view: RoomView) => boolean) =>
  waitFor(async () => expect(predicate(await snapshot(code))).toBe(true));

test("a guest cannot change any setting or send a host action, while streamed settings still arrive", async () => {
  const { page, code, host } = await open(false);
  const before = await snapshot(code);
  const sent: RoomAction[] = [];
  const original = globalThis.fetch;
  vi.spyOn(globalThis, "fetch").mockImplementation((input, init) => {
    if (String(input).endsWith("/act")) sent.push(JSON.parse(String(init?.body)));
    return original(input, init);
  });
  const choicesBefore = page
    .getAllByRole("radio")
    .map((button) => button.getAttribute("aria-checked"));
  for (const button of page.getAllByRole("radio")) fireEvent.click(button);
  for (const name of ["Blue faction", "Red faction", "Reroll", "region", "lighting"]) {
    const control = page.queryByRole("button", { name });
    if (control) fireEvent.click(control);
    expect(page.queryByRole("listbox")).toBeNull();
  }
  expect(sent).toEqual([]);
  expect(page.getAllByRole("radio").map((button) => button.getAttribute("aria-checked"))).toEqual(
    choicesBefore,
  );
  expect((await snapshot(code)).settings).toEqual(before.settings);
  expect(page.queryByRole("button", { name: "Start" })).toBeNull();
  const map = within(page.getByRole("radiogroup", { name: "map" }));
  const next = map.getAllByRole("radio").find((b) => b.getAttribute("aria-checked") !== "true")!;
  const index = map.getAllByRole("radio").indexOf(next);
  const { MAP_TYPES } = await import("@web/maps/source");
  await act(async () => {
    await service.post(`/rooms/${code}/act`, host, {
      kind: "settings",
      patch: { map: { type: MAP_TYPES[index] } },
    });
  });
  await waitFor(() => expect(next.getAttribute("aria-checked")).toBe("true"));
});

test("ready, sit, add, remove and set-seat send actions and wait for the stream", async () => {
  const { page } = await open();
  expect(page.getByTestId("lobby-vs").textContent).toBe("1 VS 2");
  const sent: RoomAction[] = [];
  let release!: () => void;
  const original = globalThis.fetch;
  vi.spyOn(globalThis, "fetch").mockImplementation(async (input, init) => {
    if (String(input).endsWith("/act")) {
      sent.push(JSON.parse(String(init?.body)));
      await new Promise<void>((resolve) => {
        release = resolve;
      });
    }
    return original(input, init);
  });
  async function request(
    button: HTMLElement,
    action: RoomAction,
    unchanged: () => void,
    changed: () => void,
  ) {
    fireEvent.click(button);
    expect(sent.at(-1)).toEqual(action);
    unchanged();
    await act(async () => {
      release();
    });
    await waitFor(changed);
  }
  await request(
    page.getByRole("button", { name: "Ready" }),
    { kind: "ready", ready: true },
    () => expect(page.queryByRole("status", { name: "SCOUT ready" })).toBeNull(),
    () => expect(page.getByRole("status", { name: "SCOUT ready" })).toBeTruthy(),
  );
  await request(
    group(page, "Blue").getByRole("button", { name: "Add seat" }),
    { kind: "add_seat", side: "blue" },
    () => expect(group(page, "Blue").getAllByRole("listitem")).toHaveLength(2),
    () => expect(group(page, "Blue").getAllByRole("listitem")).toHaveLength(3),
  );
  const openSeat = within(page.getByRole("listitem", { name: "Blue 2 seat" }));
  const picker = openSeat.getByRole("button", { name: "Blue 2 seat" });
  expect(openSeat.queryByRole("button", { name: "Move here, Blue 2" })).toBeNull();
  fireEvent.click(picker);
  expect(page.getByRole("option", { name: "move here" })).toBeTruthy();
  await request(
    page.getByRole("option", { name: "ai" }),
    { kind: "set_seat", seat: "seat-3", to: "ai" },
    () => expect(picker.textContent).toBe("Open"),
    () => expect(picker.textContent).toBe("AI"),
  );
  expect(page.getByTestId("lobby-vs").textContent).toBe("3 VS 2");
  fireEvent.click(picker);
  expect(page.queryByRole("option", { name: "move here" })).toBeNull();
  await request(
    page.getByRole("option", { name: "open" }),
    { kind: "set_seat", seat: "seat-3", to: "open" },
    () => expect(picker.textContent).toBe("AI"),
    () => expect(picker.textContent).toBe("Open"),
  );
  expect(page.getByTestId("lobby-vs").textContent).toBe("2 VS 2");
  fireEvent.click(picker);
  await request(
    page.getByRole("option", { name: "move here" }),
    { kind: "sit", seat: "seat-3" },
    () => expect(picker.textContent).toBe("Open"),
    () => expect(openSeat.getByText("SCOUT · you")).toBeTruthy(),
  );
  await request(
    page.getByRole("button", { name: "Remove Blue 3" }),
    { kind: "remove_seat", seat: "seat-5" },
    () => expect(group(page, "Blue").getAllByRole("listitem")).toHaveLength(3),
    () => expect(group(page, "Blue").getAllByRole("listitem")).toHaveLength(2),
  );
});

test("Start follows canStart from the server and stops at starting", async () => {
  const { page, code } = await open();
  const start = page.getByRole("button", { name: "Start" }) as HTMLButtonElement;
  expect(start.disabled).toBe(true);
  fireEvent.click(page.getByRole("button", { name: "Ready" }));
  await waitView(
    code,
    (view) => view.seats[0].occupant.kind === "human" && view.seats[0].occupant.ready,
  );
  expect(start.disabled).toBe(true);
  await act(async () => {
    await service.post(`/rooms/${code}/act`, other, { kind: "ready", ready: true });
  });
  await waitFor(() => expect(start.disabled).toBe(false));
  fireEvent.click(start);
  await page.findByRole("status", { name: "Preparing the battlefield" });
  expect((await snapshot(code)).phase.kind).toBe("starting");
  expect(page.queryByRole("button", { name: "Start" })).toBeNull();
});

test("a removed player returns to Multiplayer with a reason", async () => {
  const { page, code, host } = await open(false);
  await act(async () => {
    await service.post(`/rooms/${code}/act`, host, { kind: "add_seat", side: "red" });
    await service.post(`/rooms/${code}/act`, host, { kind: "remove_seat", seat: "seat-2" });
  });
  await page.findByRole("heading", { name: "Multiplayer" });
  expect((await page.findByRole("alert")).textContent).toMatch(/no longer.*room/i);
});

test("a player gone past the presence grace returns to Multiplayer instead of silently joining again", async () => {
  const { page, code, host } = await open(false);
  await fetch(`${service.address}/rooms/${code}`, {
    headers: { "X-Player": host },
    signal: lifetime.signal,
  });
  const original = globalThis.fetch;
  let release!: () => void;
  let cutSeen!: () => void;
  const waiting = new Promise<void>((resolve) => {
    cutSeen = resolve;
  });
  const resume = new Promise<void>((resolve) => {
    release = resolve;
  });
  vi.spyOn(globalThis, "fetch").mockImplementation(async (input, init) => {
    if (String(input).includes(`/rooms/${code}?after=`)) {
      cutSeen();
      await resume;
    }
    return original(input, init);
  });
  service.cut();
  await waiting;
  // The service's real presence grace is the behaviour under test.
  await new Promise((resolve) => setTimeout(resolve, PRESENCE_GRACE_MS + 200));
  await service.post("/hello", browserId(), { build, callsign: "SCOUT" });
  await act(async () => {
    release();
  });
  await page.findByRole("heading", { name: "Multiplayer" });
  expect((await page.findByRole("alert")).textContent).toMatch(/no longer.*room/i);
});

test("the room previews the server's exact seed and never substitutes one on refusal", async () => {
  const { page, code } = await open();
  const view = await snapshot(code);
  await waitFor(() => expect(PreviewWorker.all.length).toBeGreaterThan(0));
  const worker = PreviewWorker.all.at(-1)!;
  expect(worker.request.map_source.request.seed).toBe(view.settings.map.seed);
  await worker.admit();
  expect(page.getByTestId("lobby-map").querySelector("svg")?.dataset.seed).toBe(
    view.settings.map.seed,
  );
  const count = PreviewWorker.all.length;
  await act(async () => {
    await service.post(`/rooms/${code}/act`, browserId(), {
      kind: "settings",
      patch: { map: { seed: "5" } },
    });
  });
  await waitFor(() => expect(PreviewWorker.all.length).toBe(count + 1));
  await PreviewWorker.all.at(-1)!.refuse();
  await waitFor(() => expect(page.getByTestId("lobby-map").dataset.status).toBe("failed"));
  expect(PreviewWorker.all).toHaveLength(count + 1);
  expect((await snapshot(code)).settings.map.seed).toBe("5");
});

test("Copy link shares only the room code, and Leave returns to the room list", async () => {
  const writeText = vi.fn(async (_value: string) => {});
  vi.stubGlobal("navigator", { clipboard: { writeText } });
  const { page, code } = await open();
  fireEvent.click(page.getByRole("button", { name: "Copy link" }));
  await page.findByRole("button", { name: "Copied" });
  const link = new URL(writeText.mock.calls[0][0]);
  expect(link.pathname).toBe(`/room/${code}`);
  expect(link.search).toBe("");
  expect(link.href).not.toContain(browserId());
  fireEvent.click(page.getByRole("button", { name: "Leave" }));
  await page.findByRole("heading", { name: "Multiplayer" });
});

test("a successful action reply cannot change a banner while its streamed view is held back", async () => {
  const { page, code } = await open();
  const { recordDecoder, encodeRecord } = await import("@packages/multiplayer/src/stream/index");
  const original = globalThis.fetch;
  let answered!: () => void;
  const actionReply = new Promise<void>((resolve) => {
    answered = resolve;
  });
  let holding = false;
  let release = () => {};
  let connected!: () => void;
  const reconnected = new Promise<void>((resolve) => {
    connected = resolve;
  });
  let queued!: () => void;
  const held = new Promise<void>((resolve) => {
    queued = resolve;
  });
  vi.spyOn(globalThis, "fetch").mockImplementation(async (input, init) => {
    const response = await original(input, init);
    if (String(input).endsWith("/act")) {
      await response.clone().json();
      answered();
    }
    if (!String(input).includes(`/rooms/${code}?after=`)) return response;
    const decoder = recordDecoder();
    const reader = response.body!.getReader();
    const pending: Uint8Array[] = [];
    return new Response(
      new ReadableStream<Uint8Array>({
        async start(controller) {
          release = () => {
            holding = false;
            for (const bytes of pending.splice(0)) controller.enqueue(bytes);
          };
          try {
            for (;;) {
              const chunk = await reader.read();
              if (chunk.done) {
                controller.close();
                return;
              }
              for (const record of decoder.push(chunk.value)) {
                const bytes = encodeRecord(record);
                if (holding && record.kind === "state") {
                  pending.push(bytes);
                  queued();
                } else {
                  controller.enqueue(bytes);
                  if (record.kind === "state") connected();
                }
              }
            }
          } catch (error) {
            controller.error(error);
          }
        },
        cancel: () => reader.cancel(),
      }),
      { status: response.status, headers: response.headers },
    );
  });
  service.cut();
  await reconnected;
  holding = true;
  fireEvent.click(page.getByRole("button", { name: "Ready" }));
  await held;
  await act(async () => {
    await actionReply;
  });
  await waitView(
    code,
    (view) => view.seats[0].occupant.kind === "human" && view.seats[0].occupant.ready,
  );
  expect(page.queryByRole("status", { name: "SCOUT ready" })).toBeNull();
  expect(page.getByRole("button", { name: "Ready" }).getAttribute("aria-pressed")).toBe("false");
  await act(async () => {
    release();
  });
  await page.findByRole("status", { name: "SCOUT ready" });
});

test("room rows keep seat order but number only fighting lanes after a guest moves", async () => {
  const { page } = await open(false);
  await waitFor(() => expect(PreviewWorker.all.length).toBeGreaterThan(0));
  const entries = (["blue", "red"] as const).flatMap((side) =>
    [2400, 600].map((x) => ({ side, center: [x, side === "blue" ? 12 : 2988], yaw: 0 })),
  );
  await PreviewWorker.all.at(-1)!.admit({
    sites: JSON.stringify({ entries, objectives: [] }),
    svg: `<svg>${["blue", "red"].flatMap((side) => [1, 2].map((lane) => `<g data-entry="${side}" data-lane="${lane}"></g>`)).join("")}</svg>`,
  });
  expect(page.getByRole("img", { name: "Red lane 1: you" })).toBeTruthy();
  expect(page.getByRole("img", { name: "Blue lane 2: unused" })).toBeTruthy();
  fireEvent.click(page.getByRole("button", { name: "Move here, Blue 2" }));
  await waitFor(() => expect(group(page, "Blue").getByText("SCOUT · you")).toBeTruthy());
  expect(page.getByRole("img", { name: "Blue lane 2: you" })).toBeTruthy();
  expect(page.getByRole("img", { name: "Red lane 1: occupied" })).toBeTruthy();
  expect(page.getByRole("img", { name: "Red lane 2: unused" })).toBeTruthy();
  const red = group(page, "Red").getAllByRole("listitem");
  expect(within(red[0]).getByRole("button", { name: "Move here, Red 1" })).toBeTruthy();
  expect(within(red[0]).queryByRole("img", { name: /lane/ })).toBeNull();
  expect(within(red[1]).getByRole("img", { name: "Red lane 1" }).textContent).toBe("1");
  expect(group(page, "Red").queryByRole("img", { name: "Red lane 2" })).toBeNull();
  expect(
    group(page, "Blue")
      .getAllByRole("img")
      .map((lane) => lane.textContent),
  ).toEqual(["1", "2"]);
});

test("only guests see the host ownership note beside read-only settings", async () => {
  const guest = await open(false);
  expect(guest.page.getByRole("note", { name: "Set by the host" }).textContent).toBe(
    "Set by the host",
  );
  for (const name of ["map", "size", "region", "lighting"])
    for (const radio of within(guest.page.getByRole("radiogroup", { name })).getAllByRole("radio"))
      expect((radio as HTMLButtonElement).disabled).toBe(true);
  guest.page.unmount();
  await service.close();
  const host = await open();
  expect(host.page.queryByRole("note", { name: "Set by the host" })).toBeNull();
});

test("the map status names the map while the player's ready state stays independent", async () => {
  const { page } = await open();
  const map = page.getByRole("status", { name: "Map preview" });
  expect(map.textContent).toBe("Map generating");
  await waitFor(() => expect(PreviewWorker.all.length).toBeGreaterThan(0));
  await PreviewWorker.all.at(-1)!.admit();
  expect(map.textContent).toBe("Map ready");
  expect(page.queryByRole("status", { name: "SCOUT ready" })).toBeNull();
  fireEvent.click(page.getByRole("button", { name: "Reroll" }));
  await waitFor(() => expect(map.textContent).toBe("Map generating"));
  await PreviewWorker.all.at(-1)!.refuse();
  await waitFor(() => expect(map.textContent).toBe("Map unavailable"));
  expect(page.getByRole("button", { name: "Ready" })).toBeTruthy();
});

test("both sides expose open seats as sit targets without a lane until occupied", async () => {
  const { page, code, host } = await open(false);
  await act(async () => {
    await service.post(`/rooms/${code}/act`, host, { kind: "add_seat", side: "red" });
    await service.post(`/rooms/${code}/act`, host, {
      kind: "set_seat",
      seat: "seat-5",
      to: "open",
    });
  });
  for (const name of ["Blue 2", "Red 3"]) {
    const row = await page.findByRole("listitem", { name: `${name} seat` });
    expect(within(row).getByText("Open", { exact: true })).toBeTruthy();
    expect(within(row).getByRole("button", { name: `Move here, ${name}` }).textContent).toBe(
      "Move here",
    );
    expect(within(row).queryByRole("img", { name: /lane/ })).toBeNull();
  }
  fireEvent.click(page.getByRole("button", { name: "Move here, Red 3" }));
  const row = page.getByRole("listitem", { name: "Red 3 seat" });
  await waitFor(() => expect(within(row).getByText("SCOUT · you")).toBeTruthy());
  expect(within(row).getByRole("img", { name: "Red lane 2" }).textContent).toBe("2");
});

test("the host's player removal says who leaves the room and removes that player", async () => {
  const { page, code } = await open();
  fireEvent.click(page.getByRole("button", { name: "Remove WING from room" }));
  await waitFor(() => expect(page.queryByText("WING", { exact: true })).toBeNull());
  const view = await snapshot(code);
  expect(
    view.seats.some((seat) => seat.occupant.kind === "human" && seat.occupant.callsign === "WING"),
  ).toBe(false);
  expect(group(page, "Red").getAllByRole("listitem")).toHaveLength(1);
});

test("an unfinished room numbers its real fighters without inventing a lane on the empty side", async () => {
  const { page } = await open();
  fireEvent.click(page.getByRole("button", { name: "Remove WING from room" }));
  fireEvent.click(await page.findByRole("button", { name: "Red 1 seat" }));
  fireEvent.click(page.getByRole("option", { name: "open" }));
  await group(page, "Red").findByText("Open", { exact: true });
  expect(group(page, "Blue").getByRole("img", { name: "Blue lane 1" }).textContent).toBe("1");
  expect(group(page, "Red").queryByRole("img", { name: /lane/ })).toBeNull();
  await waitFor(() =>
    expect(page.getByRole("status", { name: "Map preview" }).textContent).toBe("Map unavailable"),
  );
});
