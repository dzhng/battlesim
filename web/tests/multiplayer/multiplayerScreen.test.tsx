// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, waitFor, within } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import { MemoryRouter, Route, Routes, useParams } from "react-router";
import { roomHttp, build } from "./support/roomHttp";
import { MultiplayerPage } from "@apps/battle-lab/src/online/MultiplayerPage";
import { browserId, setCallsign } from "@apps/battle-lab/src/online/identity";

vi.mock("@apps/battle-lab/src/buildIdentity", () => ({ buildIdentity: async () => build }));
vi.mock("@apps/battle-lab/src/menu/MenuShell", () => ({ useMenuShell: () => ({ back() {} }) }));
const address = vi.hoisted(() => ({ value: "" }));
vi.mock("@apps/battle-lab/src/online/server", () => ({ serverAddress: () => address.value }));
let service: Awaited<ReturnType<typeof roomHttp>>;
afterEach(async () => {
  cleanup();
  await service?.close();
  vi.restoreAllMocks();
});
async function open(capacity = 8) {
  service = await roomHttp(capacity);
  address.value = service.address;
  setCallsign("SCOUT");
  return render(
    <MemoryRouter initialEntries={["/multiplayer"]}>
      <Routes>
        <Route path="/multiplayer" element={<MultiplayerPage />} />
        <Route path="/room/:code" element={<h2>Room reached</h2>} />
      </Routes>
    </MemoryRouter>,
  );
}
const other = "b".repeat(32);
function RoomReached() {
  return <h2 data-code={useParams().code}>Room reached</h2>;
}
test("an empty list says so, then follows rooms created and changed elsewhere", async () => {
  const page = await open();
  await page.findByRole("status", { name: "No public rooms" });
  await service.post("/hello", other, { build, callsign: "WING" });
  const room = await service.post("/rooms", other, { visibility: "public" });
  const row = await page.findByRole("listitem", { name: /WING/ });
  service.cut();
  expect(within(row).getByRole("button", { name: /Join/ })).toBeTruthy();
  await service.post(`/rooms/${room.code}/act`, other, {
    kind: "set_seat",
    seat: "seat-2",
    to: "ai",
  });
  await waitFor(() => expect(within(row).queryByRole("button", { name: /Join/ })).toBeNull());
  await service.post(`/rooms/${room.code}/act`, other, {
    kind: "set_seat",
    seat: "seat-2",
    to: "open",
  });
  fireEvent.click(await within(row).findByRole("button", { name: /Join/ }));
  await page.findByRole("heading", { name: "Room reached" });
});
test("Create room asks which kind in a dialog, and backing out of it makes no room", async () => {
  const page = await open();
  await page.findByRole("status", { name: "No public rooms" });
  expect(page.queryByRole("dialog")).toBeNull();
  expect(page.queryByRole("button", { name: "Rejoin battle" })).toBeNull();
  fireEvent.click(page.getByRole("button", { name: "Create room" }));
  const dialog = page.getByRole("dialog", { name: "Create room" });
  expect(within(dialog).getByRole("button", { name: "Public room" })).toBeTruthy();
  expect(within(dialog).getByRole("button", { name: "Private room" })).toBeTruthy();
  // Escape closes the dialog, and goes no further than it.
  const escape = new KeyboardEvent("keydown", { code: "Escape", key: "Escape", cancelable: true });
  act(() => void window.dispatchEvent(escape));
  expect(escape.defaultPrevented).toBe(true);
  expect(page.queryByRole("dialog")).toBeNull();
  fireEvent.click(page.getByRole("button", { name: "Create room" }));
  fireEvent.click(page.getByRole("button", { name: "Cancel" }));
  expect(page.queryByRole("dialog")).toBeNull();
  expect(page.getByRole("status", { name: "No public rooms" })).toBeTruthy();
  // The maker's device has no game: nothing was made.
  expect(await service.post("/rooms", browserId(), { visibility: "public" })).toEqual({
    code: expect.any(String),
  });
});

test("a player whose started room is still running is offered the way back to it", async () => {
  service = await roomHttp();
  address.value = service.address;
  setCallsign("SCOUT");
  const me = browserId();
  await service.post("/hello", me, { build, callsign: "SCOUT" });
  const room = await service.post("/rooms", me, { visibility: "private" });
  for (const action of [
    { kind: "set_seat", seat: "seat-2", to: "ai" },
    { kind: "ready", ready: true },
    { kind: "start" },
  ])
    expect(await service.post(`/rooms/${room.code}/act`, me, action)).toEqual({ ok: true });
  const page = render(
    <MemoryRouter initialEntries={["/multiplayer"]}>
      <Routes>
        <Route path="/multiplayer" element={<MultiplayerPage />} />
        <Route path="/room/:code" element={<RoomReached />} />
      </Routes>
    </MemoryRouter>,
  );
  fireEvent.click(await page.findByRole("button", { name: "Rejoin battle" }));
  expect((await page.findByRole("heading", { name: "Room reached" })).dataset.code).toBe(room.code);
});

test("the way back goes when its battle ends, without the page asking", async () => {
  service = await roomHttp();
  address.value = service.address;
  setCallsign("SCOUT");
  const me = browserId();
  await service.post("/hello", me, { build, callsign: "SCOUT" });
  const room = await service.post("/rooms", me, { visibility: "private" });
  const act = (action: unknown) => service.post(`/rooms/${room.code}/act`, me, action);
  await act({ kind: "set_seat", seat: "seat-2", to: "ai" });
  await act({ kind: "ready", ready: true });
  await act({ kind: "start" });
  const page = render(
    <MemoryRouter initialEntries={["/multiplayer"]}>
      <Routes>
        <Route path="/multiplayer" element={<MultiplayerPage />} />
      </Routes>
    </MemoryRouter>,
  );
  await page.findByRole("button", { name: "Rejoin battle" });
  // Its last human leaves from elsewhere, and the battle ends.
  expect(await act({ kind: "leave" })).toEqual({ ok: true });
  await waitFor(() => expect(page.queryByRole("button", { name: "Rejoin battle" })).toBeNull());
});

test("creation reports full capacity, and public and private creation both enter the room", async () => {
  const page = await open(0);
  await page.findByRole("status", { name: "No public rooms" });
  fireEvent.click(page.getByRole("button", { name: "Create room" }));
  fireEvent.click(page.getByRole("button", { name: "Public room" }));
  expect((await page.findByRole("alert")).textContent).toMatch(/servers are full/i);
  page.unmount();
  await service.close();
  for (const visibility of ["Public", "Private"]) {
    const next = await open();
    await next.findByRole("status", { name: "No public rooms" });
    fireEvent.click(next.getByRole("button", { name: "Create room" }));
    fireEvent.click(next.getByRole("button", { name: `${visibility} room` }));
    await next.findByRole("heading", { name: "Room reached" });
    next.unmount();
    await service.close();
  }
});
test("a callsign edit re-sends hello and displays the assigned clash suffix", async () => {
  const original = globalThis.fetch;
  const sent = vi.spyOn(globalThis, "fetch").mockImplementation(original);
  const page = await open();
  await page.findByRole("status", { name: "No public rooms" });
  await service.post("/hello", other, { build, callsign: "FALCON" });
  const field = page.getByRole("textbox", { name: "Callsign" }) as HTMLInputElement;
  fireEvent.change(field, { target: { value: "FALCON" } });
  fireEvent.submit(page.getByRole("form", { name: "Callsign" }));
  await waitFor(() => expect(field.value).toBe("FALCON#2"));
  const requests = sent.mock.calls.length;
  await act(async () => {
    fireEvent.blur(field);
  });
  expect(sent.mock.calls).toHaveLength(requests);
  expect((await service.post("/hello", browserId(), { build, callsign: "FALCON" })).callsign).toBe(
    "FALCON#2",
  );
});
test("a refused build and a missing address offer no online action or list request", async () => {
  const page = await open();
  await page.findByRole("status", { name: "No public rooms" });
  page.unmount();
  const original = globalThis.fetch;
  const requests: string[] = [];
  vi.spyOn(globalThis, "fetch").mockImplementation((input, init) => {
    requests.push(String(input));
    if (String(input).endsWith("/hello"))
      return original(input, {
        ...init,
        body: JSON.stringify({
          build: { data: "wrong", wire: "wrong", sim: "wrong" },
          callsign: "X",
        }),
      });
    return original(input, init);
  });
  const refused = render(
    <MemoryRouter>
      <MultiplayerPage />
    </MemoryRouter>,
  );
  await refused.findByRole("status", { name: "Online unavailable" });
  expect(refused.queryByRole("button", { name: "Create room" })).toBeNull();
  expect(requests.filter((path) => path.endsWith("/rooms"))).toEqual([]);
  refused.unmount();
  requests.length = 0;
  address.value = "";
  const missing = render(
    <MemoryRouter>
      <MultiplayerPage />
    </MemoryRouter>,
  );
  await missing.findByRole("status", { name: "Online unavailable" });
  expect(missing.queryByRole("button", { name: "Join private room" })).toBeNull();
  expect(requests).toEqual([]);
});

test("a valid private code enters its room without disclosing the credential", async () => {
  const page = await open();
  await page.findByRole("status", { name: "No public rooms" });
  await service.post("/hello", other, { build, callsign: "WING" });
  const room = await service.post("/rooms", other, { visibility: "private" });
  fireEvent.click(page.getByRole("button", { name: "Join private room" }));
  fireEvent.change(page.getByRole("textbox", { name: "Room code" }), {
    target: {
      value: ` ${room.code.slice(0, 2).toLowerCase()} - ${room.code.slice(2).toLowerCase()} `,
    },
  });
  fireEvent.click(page.getByRole("button", { name: "Enter room" }));
  await page.findByRole("heading", { name: "Room reached" });
  expect(page.container.textContent).not.toContain(browserId());
});

test("a build refusal while editing the callsign withdraws all online actions", async () => {
  const page = await open();
  await page.findByRole("status", { name: "No public rooms" });
  const original = globalThis.fetch;
  vi.spyOn(globalThis, "fetch").mockImplementation((input, init) => {
    if (String(input).endsWith("/hello"))
      return original(input, {
        ...init,
        body: JSON.stringify({
          build: { data: "changed-build", wire: "changed-build", sim: "changed-sim" },
          callsign: "SCOUT",
        }),
      });
    return original(input, init);
  });
  fireEvent.change(page.getByRole("textbox", { name: "Callsign" }), { target: { value: "WING" } });
  fireEvent.submit(page.getByRole("form", { name: "Callsign" }));
  await page.findByRole("status", { name: "Online unavailable" });
  expect(page.queryByRole("button", { name: "Create room" })).toBeNull();
  expect(page.queryByRole("button", { name: "Join private room" })).toBeNull();
});
