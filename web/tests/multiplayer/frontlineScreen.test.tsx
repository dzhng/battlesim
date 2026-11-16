// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, within } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import { MemoryRouter, Route, Routes, useLocation } from "react-router";
import { MultiplayerPage } from "@apps/battle-lab/src/online/MultiplayerPage";
import { Suspense } from "react";
import { PreviewWorker } from "../support/previewWorker";
import { roomHttp, build } from "./support/roomHttp";

vi.mock("@apps/battle-lab/src/buildIdentity", () => ({ buildIdentity: async () => build }));
vi.mock("@apps/battle-lab/src/menu/MenuShell", () => ({ useMenuShell: () => ({ back() {} }) }));
const address = vi.hoisted(() => ({ value: "" }));
vi.mock("@apps/battle-lab/src/online/server", () => ({ serverAddress: () => address.value }));
let service: Awaited<ReturnType<typeof roomHttp>>;
afterEach(async () => {
  cleanup();
  await service?.close();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  PreviewWorker.all = [];
});
async function open(
  capacity = 0,
  createBattle?: Parameters<typeof roomHttp>[1],
  clock?: Parameters<typeof roomHttp>[2],
) {
  service = await roomHttp(capacity, createBattle, clock);
  address.value = service.address;
  const page = render(
    <Suspense fallback={null}>
      <MemoryRouter initialEntries={["/multiplayer"]}>
        <Routes>
          <Route path="/multiplayer" element={<MultiplayerPage />} />
          <Route path="/online/battle/:id" element={<Arrival />} />
        </Routes>
      </MemoryRouter>
    </Suspense>,
  );
  await page.findByRole("status", { name: "No public rooms" });
  return page;
}

test("Play refused at capacity stays in Frontline and explains that the servers are full", async () => {
  const page = await open();
  const band = within(page.getByRole("region", { name: "Frontline" }));
  fireEvent.click(band.getByRole("button", { name: "Play" }));
  expect((await band.findByRole("alert")).textContent).toMatch(/servers are full.*try again/i);
  expect(band.getByRole("button", { name: "Play" }).hasAttribute("disabled")).toBe(false);
});

test("Play distinguishes an unmakeable battlefield from full capacity", async () => {
  const { pendingBattle } = await import("./support/pendingBattle");
  const page = await open(1, () => ({
    ...pendingBattle(),
    ready: Promise.reject(new Error("map admission refused")),
  }));
  const band = within(page.getByRole("region", { name: "Frontline" }));
  fireEvent.click(band.getByRole("button", { name: "Play" }));
  expect((await band.findByRole("alert")).textContent).toMatch(/no battlefield.*try again/i);
  expect(band.getByRole("button", { name: "Play" }).hasAttribute("disabled")).toBe(false);
});

test("the band's live numbers follow Frontline joins, deployments and departures on the room stream", async () => {
  const { frontlineHost } = await import("./support/frontlineHost");
  const { browserId } = await import("@apps/battle-lab/src/online/identity");
  const page = await open(1, frontlineHost);
  const band = within(page.getByRole("region", { name: "Frontline" }));
  const activity = band.getByRole("status", { name: "Frontline activity" });
  await vi.waitFor(() => expect(activity.textContent).toMatch(/0 battles.*0 commanders/));
  const briefing = await service.post("/frontline/join", browserId(), {});
  await vi.waitFor(() => expect(activity.textContent).toMatch(/1 battle.*1 commander/));
  await service.post(`/frontline/${briefing.battle}/deploy`, browserId(), {});
  const other = "d".repeat(32);
  await service.post("/hello", other, { build, callsign: "WING" });
  const next = await service.post("/frontline/join", other, {});
  await service.post(`/frontline/${next.battle}/deploy`, other, {});
  await vi.waitFor(() => expect(activity.textContent).toMatch(/1 battle.*2 commanders/));
  await service.post(`/frontline/${briefing.battle}/leave`, other, {});
  await vi.waitFor(() => expect(activity.textContent).toMatch(/1 battle.*1 commander/));
  await service.post(`/frontline/${briefing.battle}/leave`, browserId(), {});
  await vi.waitFor(() => expect(activity.textContent).toMatch(/0 battles.*0 commanders/));
});

function Arrival() {
  const { state } = useLocation();
  return <output aria-label="Battle ticket">{JSON.stringify(state)}</output>;
}
async function briefingPage(clock?: Parameters<typeof roomHttp>[2]) {
  const { frontlineHost } = await import("./support/frontlineHost");
  vi.stubGlobal("Worker", PreviewWorker);
  const page = await open(1, frontlineHost, clock);
  fireEvent.click(page.getByRole("button", { name: "Play" }));
  const card = within(await page.findByRole("region", { name: "Battle briefing" }));
  await vi.waitFor(() => expect(PreviewWorker.all.at(-1)?.message).toBeTruthy());
  return { page, card, worker: PreviewWorker.all.at(-1)! };
}
async function admit(worker: PreviewWorker, hash = "frontline-map") {
  await act(async () =>
    worker.onmessage?.({
      data: {
        type: "prepared",
        battle: {
          scenario: "{}",
          report: {
            request: worker.request,
            identity: { kind: "generated", generation: { map_hash: hash } },
          },
        },
      },
    } as never),
  );
}
test("Deploy stays unavailable until preparation returns the server's exact map, then carries the ticket to the shared battle route", async () => {
  const { page, card, worker } = await briefingPage();
  expect(card.getByRole("button", { name: "Deploy" }).hasAttribute("disabled")).toBe(true);
  await admit(worker);
  await vi.waitFor(() =>
    expect(card.getByRole("button", { name: "Deploy" }).hasAttribute("disabled")).toBe(false),
  );
  fireEvent.click(card.getByRole("button", { name: "Deploy" }));
  const entry = JSON.parse((await page.findByLabelText("Battle ticket")).textContent!);
  expect(entry.ticket.mapIdentity).toBe("frontline-map");
  // The battlefield the card built goes to the battle page in memory, not
  // in the history entry: it is megabytes.
  expect(entry.prepared).toBeUndefined();
  const { builtMap } = await import("@apps/battle-lab/src/onlineSession");
  const built = builtMap(entry.ticket.battle)!.battle.report.identity;
  expect(built.kind === "generated" && built.generation.map_hash).toBe(entry.ticket.mapIdentity);
  const { browserId } = await import("@apps/battle-lab/src/online/identity");
  expect(JSON.stringify(entry).includes(browserId())).toBe(false);
});

test("a mismatched map leaves Deploy shut and offers the way back", async () => {
  const { page, card, worker } = await briefingPage();
  await admit(worker, "a-different-map");
  expect((await card.findByRole("alert")).textContent).toMatch(/map.*server/i);
  expect(card.getByRole("button", { name: "Deploy" }).hasAttribute("disabled")).toBe(true);
  fireEvent.click(card.getByRole("button", { name: "Back to Multiplayer" }));
  await page.findByRole("button", { name: "Play" });
  await vi.waitFor(() =>
    expect(page.getByRole("status", { name: "Frontline activity" }).textContent).toMatch(
      /0 battles.*0 commanders/,
    ),
  );
});
test("a failed map leaves Deploy shut and can return its reservation", async () => {
  const { page, card, worker } = await briefingPage();
  const warning = vi.spyOn(console, "warn").mockImplementation(() => {});
  await worker.refuse();
  expect((await card.findByRole("alert")).textContent).toMatch(/could not be prepared/i);
  expect(card.getByRole("button", { name: "Deploy" }).hasAttribute("disabled")).toBe(true);
  fireEvent.click(card.getByRole("button", { name: "Back to Multiplayer" }));
  await page.findByRole("button", { name: "Play" });
  warning.mockRestore();
});
test("a lapsed reservation returns to Play and says the seat was given away", async () => {
  const { PageClock } = await import("./pageClock");
  const { GONE_TIMEOUT_MS } = await import("@packages/multiplayer/src/frontline");
  const clock = new PageClock();
  const { page, card, worker } = await briefingPage(clock);
  await admit(worker);
  await act(async () => clock.advance(GONE_TIMEOUT_MS));
  fireEvent.click(card.getByRole("button", { name: "Deploy" }));
  await page.findByRole("button", { name: "Play" });
  expect((await page.findByRole("alert")).textContent).toMatch(/seat.*given away/i);
});

test("a slow Play cannot submit twice, and a forgotten callsign is repaired then retried once", async () => {
  const { frontlineHost } = await import("./support/frontlineHost");
  let release!: () => void;
  const ready = new Promise<void>((resolve) => {
    release = resolve;
  });
  const original = globalThis.fetch;
  let joins = 0;
  vi.spyOn(globalThis, "fetch").mockImplementation((input, init) => {
    if (String(input).endsWith("/frontline/join") && ++joins === 1)
      return original(input, {
        ...init,
        headers: { ...init?.headers, "X-Player": "e".repeat(32) },
      });
    return original(input, init);
  });
  const page = await open(1, (setup) => {
    const host = frontlineHost(setup);
    return { ...host, ready: ready.then(() => host.ready) };
  });
  vi.stubGlobal("Worker", PreviewWorker);
  const play = page.getByRole("button", { name: "Play" });
  fireEvent.click(play);
  fireEvent.click(play);
  await vi.waitFor(() => expect(joins).toBe(2));
  expect(play.hasAttribute("disabled")).toBe(true);
  await act(async () => release());
  await page.findByRole("region", { name: "Battle briefing" });
  expect(joins).toBe(2);
});

test("a briefing shows departed commanders as AI while their seat is held for return", async () => {
  const { frontlineHost } = await import("./support/frontlineHost");
  const { browserId } = await import("@apps/battle-lab/src/online/identity");
  vi.stubGlobal("Worker", PreviewWorker);
  const page = await open(1, frontlineHost);
  const first = await service.post("/frontline/join", browserId(), {});
  await service.post(`/frontline/${first.battle}/deploy`, browserId(), {});
  const other = "f".repeat(32);
  await service.post("/hello", other, { build, callsign: "WING" });
  const second = await service.post("/frontline/join", other, {});
  await service.post(`/frontline/${second.battle}/deploy`, other, {});
  await service.post(`/frontline/${second.battle}/leave`, other, {});
  fireEvent.click(page.getByRole("button", { name: "Play" }));
  const card = within(await page.findByRole("region", { name: "Battle briefing" }));
  const commanders = [...card.getAllByRole("list")].flatMap((list) =>
    within(list)
      .queryAllByRole("listitem")
      .map((item) => item.textContent),
  );
  expect(commanders.includes("WING")).toBe(false);
});
