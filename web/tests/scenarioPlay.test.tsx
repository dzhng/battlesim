// @vitest-environment jsdom
import { fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import { BrowserVisit } from "./support/browserVisit";

const mocks = vi.hoisted(() => {
  let resolve!: (value: unknown) => void;
  const battle = new Promise((next) => (resolve = next));
  return {
    resolve,
    prepare: vi.fn(() => ({ battle, cancel: vi.fn() })),
    catalog: vi.fn(() => null),
    read: vi.fn(async () =>
      JSON.stringify({
        scenario: { players: [{ id: "blue-human", side: "blue", human_capable: true }] },
      }),
    ),
  };
});

vi.mock("@apps/battle-lab/src/AppAudio", () => ({
  useAppAudio: () => ({ playEmbeddedAudio: vi.fn() }),
}));
vi.mock("@web/battle/sim/module", () => ({ loadWasm: vi.fn(async () => ({})) }));
vi.mock("@web/battle/prepare/client", () => ({
  prepareUploadedScenarioBundle: mocks.prepare,
  scenarioBundleCatalog: mocks.catalog,
}));
vi.mock("@web/battle/scenarioBundle", () => ({ readScenarioBundle: mocks.read }));

import ScenarioPlay from "@apps/battle-lab/src/routes/scenarioPlay";

afterEach(() => {
  vi.clearAllMocks();
});

test("uploaded route carries the package through controller choice and deployment", async () => {
  const view = render(<ScenarioPlay />, { wrapper: BrowserVisit });
  const file = new File(["uploaded"], "scenario.json", { type: "application/json" });
  fireEvent.change(view.container.querySelector('input[type="file"]')!, {
    target: { files: [file] },
  });
  await waitFor(() =>
    expect(view.getByRole("heading", { name: "Choose controller" })).toBeTruthy(),
  );

  fireEvent.click(view.getByRole("button", { name: "Deploy scenario" }));
  await waitFor(() => expect(view.getByRole("heading", { name: "Deploying" })).toBeTruthy());
  expect(mocks.prepare).toHaveBeenCalledWith(
    expect.any(String),
    expect.any(Function),
    "blue-human",
  );
  const preparation = mocks.prepare.mock.results[0].value as { cancel: ReturnType<typeof vi.fn> };
  mocks.resolve({ report: {}, session: { scenario: "{}" } });
  await waitFor(() => expect(mocks.catalog).toHaveBeenCalled());
  await waitFor(() => expect(preparation.cancel).not.toHaveBeenCalled());
  view.unmount();
  expect(preparation.cancel).toHaveBeenCalledOnce();
});
