// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import type { LoadedModel } from "@apps/battle-lab/src/workbench/sources";

// The route needs the browser's complete HMR API; disable Vitest's partial stub.
const viteClientPath = "/@vite/client";
const viteClient = await import(viteClientPath);
vi.spyOn(viteClient, "createHotContext").mockReturnValue(undefined);
const { default: Workbench } = await import("@apps/battle-lab/src/routes/workbench");

const sources = vi.hoisted(() => ({ loadDropped: vi.fn(), catalogModel: vi.fn() }));
const catalog = {
  generation: 1,
  sides: { blue: [1, 1, 1], red: [1, 1, 1] },
  skeletons: new Map(),
  appearances: new Map(),
  onRequest: new Map(),
};
vi.mock("@apps/battle-lab/src/LabViewport", () => ({ LabViewport: () => null }));
vi.mock("@web/battle/catalog/context", () => ({ useSessionCatalog: () => session }));
const session = { units: { cards: [], ids: [] }, rules: {} };
vi.mock("@apps/battle-lab/src/workbench/AtlasView", () => ({ AtlasView: () => null }));
vi.mock("@apps/battle-lab/src/workbench/benchWorld", () => ({
  benchWorld: () => ({}),
  benchOverlay: () => ({}),
  drawnPose: () => null,
  posedSockets: () => [],
}));
vi.mock("@apps/battle-lab/src/workbench/sheet", () => ({
  framingBounds: () => null,
  figureSpot: () => null,
  sheetPose: () => null,
  renderReferenceSheet: vi.fn(),
  renderSheet: vi.fn(),
  textureSheet: vi.fn(),
}));
vi.mock("@apps/battle-lab/src/workbench/sources", () => ({
  loadDropped: sources.loadDropped,
  catalogModel: sources.catalogModel,
  loadCatalog: async () => catalog,
  sideTint: () => undefined,
  catalogNames: () => [],
  INFANTRY_LOOPS: [],
}));
function model(name: string): LoadedModel {
  return {
    name,
    unit: "vehicle",
    type: null,
    scenery: null,
    body: { edges: [], label: "test body" },
    installed: { ...catalog, sides: { blue: [1, 1, 1], red: [1, 1, 1] } },
    source: name,
    findings: [],
    stats: null,
    clipStats: null,
    loadMs: 0,
  };
}
function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: Error) => void;
  const promise = new Promise<T>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}
function choose(name: string, bytes: Promise<ArrayBuffer> = Promise.resolve(new ArrayBuffer(0))) {
  const file = new File([], name);
  Object.defineProperty(file, "arrayBuffer", { value: () => bytes });
  fireEvent.change(screen.getByTestId("workbench-file"), { target: { files: [file] } });
}
afterEach(() => {
  cleanup();
  vi.resetAllMocks();
  window.history.replaceState(null, "", "/");
});

test("a slow older file read cannot replace a more recently selected source", async () => {
  sources.loadDropped.mockImplementation(async (_session, name) => model(name));
  render(<Workbench />);
  const olderRead = deferred<ArrayBuffer>();
  choose("older.glb", olderRead.promise);
  choose("latest.glb");
  await waitFor(() =>
    expect(screen.getByTestId("workbench-model").textContent).toContain("latest.glb"),
  );
  await act(async () => {
    olderRead.resolve(new ArrayBuffer(0));
  });
  expect(screen.getByTestId("workbench-model").textContent).toContain("latest.glb");
});

test("an older validation failure cannot replace the newest error or finish its loading state", async () => {
  const older = deferred<LoadedModel>();
  const latest = deferred<LoadedModel>();
  sources.loadDropped.mockImplementation((_session, name) =>
    name === "older.glb" ? older.promise : latest.promise,
  );
  render(<Workbench />);
  choose("older.glb");
  await waitFor(() => expect(sources.loadDropped).toHaveBeenCalledTimes(1));
  choose("latest.glb");
  await waitFor(() => expect(sources.loadDropped).toHaveBeenCalledTimes(2));
  await act(async () => {
    older.reject(new Error("obsolete source failure"));
  });
  expect(screen.queryByText("obsolete source failure")).toBeNull();
  expect(screen.getByText("validating…")).toBeDefined();
  await act(async () => {
    latest.reject(new Error("latest source failure"));
  });
  expect(screen.getByText("latest source failure")).toBeDefined();
  expect(screen.queryByText("validating…")).toBeNull();
});

test("a pending catalog selection cannot replace a newer dropped file", async () => {
  window.history.replaceState(null, "", "?bundle=catalog-older");
  const older = deferred<LoadedModel>();
  sources.catalogModel.mockReturnValue(older.promise);
  sources.loadDropped.mockImplementation(async (_session, name) => model(name));
  render(<Workbench />);
  await waitFor(() => expect(sources.catalogModel).toHaveBeenCalled());
  choose("latest.glb");
  await waitFor(() =>
    expect(screen.getByTestId("workbench-model").textContent).toContain("latest.glb"),
  );
  await act(async () => {
    older.resolve(model("catalog-older"));
  });
  expect(screen.getByTestId("workbench-model").textContent).toContain("latest.glb");
});

test("a failed current source rejects the automation request and shows its error", async () => {
  sources.loadDropped.mockRejectedValue(new Error("current validation failure"));
  render(<Workbench />);
  await act(async () => {
    await expect(window.__workbench!.drop("current.glb", new Uint8Array())).rejects.toThrow(
      "current validation failure",
    );
  });
  expect(screen.getByText("current validation failure")).toBeDefined();
});
