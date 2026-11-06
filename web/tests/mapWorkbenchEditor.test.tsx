import { MemoryRouter } from "react-router";
// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import { MapWorkbench } from "../../apps/map-workbench/src/MapWorkbench";
import { Measurements } from "../../apps/map-workbench/src/Measurements";
import type { Report, Snapshot, WorkbenchAPI } from "../../apps/map-workbench/src/protocol";

afterEach(cleanup);

test("measurement summaries stay readable while exposing exact native values", () => {
  render(
    <Measurements
      report={{
        fingerprint: "measured",
        choice: { type: "mixed", size: "small", seed: "1" },
        status: "ok",
        diagnostics: [],
        counts: { buildings: 3050 },
        metrics: { tree_line_m: 591.3250791863752, tiny_fraction: 0.00000123456789 },
      }}
    />,
  );
  expect(screen.getByText("3050")).toBeTruthy();
  expect(screen.getByText("591.325").getAttribute("title")).toBe("591.3250791863752");
  expect(screen.getByText("0.00000123457").getAttribute("title")).toBe("0.00000123456789");
});

test("encounter positions identify their axes and units", () => {
  render(
    <Measurements
      report={{
        fingerprint: "placed",
        choice: { type: "mixed", size: "small", seed: "1" },
        status: "ok",
        diagnostics: [],
        encounter: { placement: { head: [3734.07, 384.17] } },
      }}
    />,
  );
  expect(screen.getByText("Map positions are [x, y] in metres.")).toBeTruthy();
  expect(screen.getByText("[3734.07, 384.17]")).toBeTruthy();
});

function fixture() {
  const snapshot: Snapshot = {
    revision: "saved",
    documents: {
      presets: { districts: { apartments: { block_depth_m: 130 } }, transit: { max_s: 215 } },
      defaults: {},
    },
    receipts: {},
    fields: [
      {
        id: "depth",
        document: "presets",
        path: ["districts", "apartments", "block_depth_m"],
        group: "districts.apartments",
        label: "Block depth",
        description: "Spacing between apartment streets.",
        unit: "m",
        role: "construction",
        editable: true,
      },
      {
        id: "transit",
        document: "presets",
        path: ["transit", "max_s"],
        group: "transit",
        label: "Maximum transit",
        description: "Maximum estimated road journey.",
        unit: "s",
        role: "validation",
        editable: true,
      },
    ],
  };
  const generate = vi.fn<WorkbenchAPI["generate"]>(
    async (request) =>
      ({
        status: "ok",
        fingerprint: "tested",
        choice: request.choice,
        diagnostics: [],
        artifactId: "artifact",
        svg: '<svg viewBox="0 0 100 100"><path data-rule-group="districts.apartments" data-feature-id="district-1" d="M10 10H90V90H10Z"/></svg>',
        features: [
          {
            id: "district-1",
            group: "districts.apartments",
            label: "Apartment district",
            crop: "district-1",
          },
        ],
        counts: { buildings: 42 },
      }) satisfies Report,
  );
  const api: WorkbenchAPI = {
    snapshot: async () => snapshot,
    generate,
    inspect: async () => ({ svg: "", features: [] }),
    sight: async () => ({
      status: "unavailable",
      samples: 0,
      target: 0.5,
      step_m: 300,
      bearings: 64,
      elapsed_ms: 0,
    }),
    preview: async () => ({
      candidateId: "candidate",
      revision: "saved",
      files: [],
      diagnostics: [],
    }),
    save: async () => snapshot,
    export: async () => ({}),
  };
  return { api, generate };
}

test("map selection and search edit the same district rule and regenerate the edited snapshot", async () => {
  const { api } = fixture();
  const preview = vi.fn<WorkbenchAPI["preview"]>(async (draft) => ({
    candidateId: "candidate",
    revision: draft.revision,
    files: [],
    diagnostics: [],
  }));
  api.preview = preview;
  render(<MapWorkbench api={api} />, { wrapper: MemoryRouter });
  await waitFor(() => expect(screen.getByTestId("map-plan").querySelector("path")).not.toBeNull());
  fireEvent.click(screen.getByTestId("map-plan").querySelector("path")!);
  expect(screen.queryByLabelText("Maximum transit")).toBeNull();
  fireEvent.change(screen.getByLabelText("Block depth"), { target: { value: "-" } });
  expect(screen.getByLabelText("Block depth").getAttribute("aria-invalid")).toBe("true");
  expect(screen.getByText(/OLDER RESULT/)).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "Undo edit" }));
  expect((screen.getByLabelText("Block depth") as HTMLInputElement).value).toBe("130");
  expect(screen.getByLabelText("Block depth").getAttribute("aria-invalid")).toBe("false");
  fireEvent.change(screen.getByLabelText("Block depth"), { target: { value: "160" } });
  fireEvent.change(screen.getByLabelText("Search rules"), { target: { value: "Block depth" } });
  expect((screen.getByLabelText("Block depth") as HTMLInputElement).value).toBe("160");
  fireEvent.click(screen.getByRole("button", { name: "Review save" }));
  await waitFor(() =>
    expect(preview.mock.calls.at(-1)?.[0].documents.presets).toEqual({
      districts: { apartments: { block_depth_m: 160 } },
      transit: { max_s: 215 },
    }),
  );
  expect(screen.getByText(/wherever this preset is used/)).toBeTruthy();
});

test("a formatting-only edit keeps the admitted input and does not regenerate", async () => {
  const { api, generate } = fixture();
  render(<MapWorkbench api={api} />, { wrapper: MemoryRouter });
  await waitFor(() => expect(screen.getByTestId("map-plan").querySelector("path")).not.toBeNull());
  fireEvent.change(screen.getByLabelText("Block depth"), { target: { value: "130.0" } });
  await new Promise((resolve) => setTimeout(resolve, 450));
  expect(generate).toHaveBeenCalledTimes(1);
  expect(screen.getByTestId("map-plan").querySelector("path")).not.toBeNull();
});

test("selection remains scoped to the displayed artifact when a newer seed is refused", async () => {
  const { api, generate } = fixture();
  const inspect = vi.fn<WorkbenchAPI["inspect"]>(async () => ({ svg: "<svg />", features: [] }));
  api.inspect = inspect;
  generate.mockImplementation(async (request) =>
    request.choice.seed === "2"
      ? {
          fingerprint: "refused",
          choice: request.choice,
          status: "refused",
          diagnostics: [],
          artifactId: "refusal",
        }
      : {
          fingerprint: "admitted",
          choice: request.choice,
          status: "ok",
          diagnostics: [],
          artifactId: "artifact",
          svg: '<svg><path data-rule-group="districts.apartments" data-feature-id="district-1" /></svg>',
          features: [
            {
              id: "district-1",
              group: "districts.apartments",
              label: "Apartment",
              crop: "district-1",
            },
          ],
        },
  );
  render(<MapWorkbench api={api} />, { wrapper: MemoryRouter });
  await waitFor(() => expect(screen.getByTestId("map-plan").querySelector("path")).not.toBeNull());
  fireEvent.click(screen.getByTestId("map-plan").querySelector("path")!);
  fireEvent.click(screen.getByRole("button", { name: "Inspect selection" }));
  await waitFor(() =>
    expect(inspect).toHaveBeenCalledWith("artifact", "district-1", expect.anything()),
  );
  fireEvent.change(screen.getByLabelText("Map seed"), { target: { value: "2" } });
  await waitFor(() => expect(screen.getByRole("status").textContent).toBe("Current draft refused"));
  fireEvent.click(screen.getByRole("button", { name: "Inspect selection" }));
  await waitFor(() => expect(inspect).toHaveBeenCalledTimes(2));
  expect(inspect.mock.calls.at(-1)?.slice(0, 2)).toEqual(["artifact", "district-1"]);
});

test("cancelling a sample closes it without restarting generation", async () => {
  const { api, generate } = fixture();
  render(<MapWorkbench api={api} />, { wrapper: MemoryRouter });
  await waitFor(() => expect(screen.getByTestId("map-plan").querySelector("path")).not.toBeNull());
  generate.mockImplementation(
    (_request, signal) =>
      new Promise((_resolve, reject) => {
        signal?.addEventListener("abort", () => reject(new Error("cancelled")), { once: true });
      }),
  );
  fireEvent.click(screen.getByRole("button", { name: "Run seed sample" }));
  await waitFor(() => expect(generate).toHaveBeenCalledTimes(2));
  fireEvent.click(screen.getByRole("button", { name: "Cancel sample" }));
  await waitFor(() =>
    expect(screen.getByText(/cancelled; remaining seeds unattempted/)).toBeTruthy(),
  );
  await new Promise((resolve) => setTimeout(resolve, 450));
  expect(generate).toHaveBeenCalledTimes(2);
});

test("explicit cancel does not restart the background draft", async () => {
  const { api, generate } = fixture();
  render(<MapWorkbench api={api} />, { wrapper: MemoryRouter });
  await waitFor(() => expect(screen.getByTestId("map-plan").querySelector("path")).not.toBeNull());
  generate.mockImplementation(
    (_request, signal) =>
      new Promise((_resolve, reject) => {
        signal?.addEventListener("abort", () => reject(new Error("cancelled")), { once: true });
      }),
  );
  fireEvent.click(screen.getByRole("button", { name: "Run seed sample" }));
  await waitFor(() => expect(generate).toHaveBeenCalledTimes(2));
  fireEvent.click(screen.getByRole("button", { name: "Cancel work" }));
  await waitFor(() =>
    expect(screen.getByText(/cancelled; remaining seeds unattempted/)).toBeTruthy(),
  );
  await new Promise((resolve) => setTimeout(resolve, 450));
  expect(generate).toHaveBeenCalledTimes(2);
});

test("explicit cancel after an edit does not resume the pending draft", async () => {
  const { api, generate } = fixture();
  const inspect = vi.fn<WorkbenchAPI["inspect"]>(
    (_artifactId, _crop, signal) =>
      new Promise((_resolve, reject) => {
        signal?.addEventListener("abort", () => reject(new Error("cancelled")), { once: true });
      }),
  );
  api.inspect = inspect;
  render(<MapWorkbench api={api} />, { wrapper: MemoryRouter });
  await waitFor(() => expect(screen.getByTestId("map-plan").querySelector("path")).not.toBeNull());
  fireEvent.change(screen.getByLabelText("Block depth"), { target: { value: "140" } });
  fireEvent.click(screen.getByRole("button", { name: "Whole map" }));
  await waitFor(() => expect(inspect).toHaveBeenCalledTimes(1));
  fireEvent.click(screen.getByRole("button", { name: "Cancel work" }));
  expect(screen.getByRole("status").textContent).toBe("Cancelled; previous result retained");
  await new Promise((resolve) => setTimeout(resolve, 450));
  expect(generate).toHaveBeenCalledTimes(1);
});

test("a new manual operation waits for the cancelled operation to settle", async () => {
  const { api } = fixture();
  const finishes: ((value: { svg: string; features: [] }) => void)[] = [];
  const inspect = vi.fn<WorkbenchAPI["inspect"]>(
    () => new Promise((resolve) => finishes.push(resolve)),
  );
  api.inspect = inspect;
  render(<MapWorkbench api={api} />, { wrapper: MemoryRouter });
  await waitFor(() => expect(screen.getByTestId("map-plan").querySelector("path")).not.toBeNull());
  fireEvent.click(screen.getByRole("button", { name: "Whole map" }));
  await waitFor(() => expect(inspect).toHaveBeenCalledTimes(1));
  fireEvent.click(screen.getByRole("button", { name: "Whole map" }));
  await new Promise((resolve) => setTimeout(resolve, 0));
  expect(inspect).toHaveBeenCalledTimes(1);
  finishes[0]!({ svg: "<svg>first</svg>", features: [] });
  await waitFor(() => expect(inspect).toHaveBeenCalledTimes(2));
  finishes[1]!({ svg: "<svg>second</svg>", features: [] });
  await waitFor(() => expect(screen.getByTestId("map-plan").innerHTML).toContain("second"));
  expect(screen.getByTestId("map-plan").innerHTML).not.toContain("first");
});

test("cancelling a queued manual operation prevents it from starting", async () => {
  const { api } = fixture();
  const finishes: ((value: { svg: string; features: [] }) => void)[] = [];
  const inspect = vi.fn<WorkbenchAPI["inspect"]>(
    () => new Promise((resolve) => finishes.push(resolve)),
  );
  api.inspect = inspect;
  render(<MapWorkbench api={api} />, { wrapper: MemoryRouter });
  await waitFor(() => expect(screen.getByTestId("map-plan").querySelector("path")).not.toBeNull());
  fireEvent.click(screen.getByRole("button", { name: "Whole map" }));
  await waitFor(() => expect(inspect).toHaveBeenCalledTimes(1));
  fireEvent.click(screen.getByRole("button", { name: "Whole map" }));
  fireEvent.click(screen.getByRole("button", { name: "Cancel work" }));
  finishes[0]!({ svg: "<svg>late</svg>", features: [] });
  await new Promise((resolve) => setTimeout(resolve, 0));
  expect(inspect).toHaveBeenCalledTimes(1);
  expect(screen.getByTestId("map-plan").innerHTML).not.toContain("late");
});

test("a late comparison completion cannot replace the edited draft view", async () => {
  const { api, generate } = fixture();
  render(<MapWorkbench api={api} />, { wrapper: MemoryRouter });
  await waitFor(() => expect(screen.getByTestId("map-plan").querySelector("path")).not.toBeNull());
  fireEvent.change(screen.getByLabelText("Block depth"), { target: { value: "140" } });
  await waitFor(() =>
    expect(screen.getByRole("status").textContent).toBe("Current draft admitted"),
  );

  let finish!: (report: Report) => void;
  generate.mockImplementationOnce(
    () =>
      new Promise((resolve) => {
        finish = resolve;
      }),
  );
  fireEvent.click(screen.getByRole("button", { name: "Saved baseline · same seed" }));
  await waitFor(() => expect(generate).toHaveBeenCalledTimes(3));
  fireEvent.change(screen.getByLabelText("Block depth"), { target: { value: "150" } });
  finish({
    fingerprint: "late-baseline",
    choice: { type: "mixed", size: "small", seed: "1" },
    status: "ok",
    diagnostics: [],
    artifactId: "late-baseline",
    svg: "<svg>late</svg>",
  });
  await waitFor(() => expect(generate).toHaveBeenCalledTimes(4));

  await waitFor(() =>
    expect(screen.getByRole("button", { name: "Draft plan" }).getAttribute("aria-pressed")).toBe(
      "true",
    ),
  );
  expect(
    screen.getByRole("button", { name: "Saved baseline · same seed" }).getAttribute("aria-pressed"),
  ).toBe("false");
  expect(screen.getByTestId("map-plan").innerHTML).not.toContain("late");
});

test("reapplying an unchanged formatted value displays the outside source change", async () => {
  const { api } = fixture();
  const saved = await api.snapshot();
  render(<MapWorkbench api={api} />, { wrapper: MemoryRouter });
  await waitFor(() => expect(screen.getByTestId("map-plan").querySelector("path")).not.toBeNull());
  fireEvent.change(screen.getByLabelText("Block depth"), { target: { value: "130.0" } });
  const outside = structuredClone(saved);
  outside.revision = "outside edit";
  outside.documents.presets = {
    districts: { apartments: { block_depth_m: 170 } },
    transit: { max_s: 215 },
  };
  api.snapshot = async () => outside;
  fireEvent.click(screen.getByRole("button", { name: "Reload sources" }));
  await waitFor(() =>
    expect(screen.getByRole("button", { name: "Reapply my edits to new sources" })).toBeTruthy(),
  );
  fireEvent.click(screen.getByRole("button", { name: "Reapply my edits to new sources" }));
  expect((screen.getByLabelText("Block depth") as HTMLInputElement).value).toBe("170");
});

test("an unchanged saved comparison reuses the admitted preview", async () => {
  const { api, generate } = fixture();
  render(<MapWorkbench api={api} />, { wrapper: MemoryRouter });
  await waitFor(() => expect(screen.getByTestId("map-plan").querySelector("path")).not.toBeNull());
  fireEvent.click(screen.getByRole("button", { name: "Saved baseline · same seed" }));
  await waitFor(() =>
    expect(
      screen
        .getByRole("button", { name: "Saved baseline · same seed" })
        .getAttribute("aria-pressed"),
    ).toBe("true"),
  );
  expect(generate).toHaveBeenCalledTimes(1);
  fireEvent.click(screen.getByRole("button", { name: "Draft plan" }));
  expect(screen.getByTestId("map-plan").querySelector("path")).not.toBeNull();
});

test("saved comparison stays visible across draft edits and resets when the seed changes", async () => {
  const { api, generate } = fixture();
  api.inspect = async () => ({
    svg: '<svg viewBox="10 10 80 80"><path data-rule-group="districts.apartments" data-feature-id="district-1" d="M10 10H90V90H10Z"/></svg>',
    features: [],
  });
  render(<MapWorkbench api={api} />, { wrapper: MemoryRouter });
  await waitFor(() => expect(screen.getByTestId("map-plan").querySelector("path")).not.toBeNull());
  fireEvent.click(screen.getByRole("button", { name: "Saved baseline · same seed" }));
  await waitFor(() =>
    expect(
      screen
        .getByRole("button", { name: "Saved baseline · same seed" })
        .getAttribute("aria-pressed"),
    ).toBe("true"),
  );
  fireEvent.click(screen.getByTestId("map-plan").querySelector("path")!);
  fireEvent.click(screen.getByRole("button", { name: "Inspect selection" }));
  await waitFor(() =>
    expect(screen.getByTestId("map-plan").querySelector("svg")?.getAttribute("viewBox")).toBe(
      "10 10 80 80",
    ),
  );
  const baseline = screen.getByTestId("map-plan").innerHTML;
  const report = await generate.mock.results[0].value;
  generate.mockResolvedValue({ ...report, artifactId: "edited", fingerprint: "edited" });
  fireEvent.change(screen.getByLabelText("Block depth"), { target: { value: "140" } });
  await waitFor(() =>
    expect(screen.getByRole("status").textContent).toBe("Current draft admitted"),
  );
  expect(screen.getByTestId("map-plan").innerHTML).toBe(baseline);
  expect(
    screen.getByRole("button", { name: "Saved baseline · same seed" }).getAttribute("aria-pressed"),
  ).toBe("true");
  expect(screen.getByText("· input tested")).toBeTruthy();
  fireEvent.change(screen.getByLabelText("Map seed"), { target: { value: "2" } });
  expect(screen.getByRole("button", { name: "Draft plan" }).getAttribute("aria-pressed")).toBe(
    "true",
  );
  expect(screen.getByTestId("map-plan").querySelector("path")).not.toBeNull();
});

test("superseded successes keep the displayed admitted artifact pinned when the latest draft refuses", async () => {
  const { api, generate } = fixture();
  render(<MapWorkbench api={api} />, { wrapper: MemoryRouter });
  await waitFor(() => expect(screen.getByTestId("map-plan").querySelector("path")).not.toBeNull());
  let finish!: (report: Report) => void;
  generate.mockImplementation((request) => {
    const depth = (
      request.draft.documents.presets.districts as { apartments: { block_depth_m: number } }
    ).apartments.block_depth_m;
    return depth === 140
      ? new Promise((resolve) => {
          finish = resolve;
        })
      : Promise.resolve({
          fingerprint: "refused",
          choice: request.choice,
          status: "refused",
          diagnostics: [],
          artifactId: "refusal",
        });
  });
  fireEvent.change(screen.getByLabelText("Block depth"), { target: { value: "140" } });
  await waitFor(() => expect(finish).toBeTypeOf("function"));
  fireEvent.change(screen.getByLabelText("Block depth"), { target: { value: "150" } });
  await new Promise((resolve) => setTimeout(resolve, 400));
  finish({
    fingerprint: "superseded",
    choice: { type: "mixed", size: "small", seed: "1" },
    status: "ok",
    diagnostics: [],
    artifactId: "superseded",
    svg: "<svg/>",
  });
  await waitFor(() => expect(screen.getByRole("status").textContent).toBe("Current draft refused"));
  expect(generate.mock.calls.at(-1)?.[0].retainedArtifactIds).toEqual(["artifact"]);
  const inspect = vi.fn<WorkbenchAPI["inspect"]>(async () => ({ svg: "<svg/>", features: [] }));
  api.inspect = inspect;
  fireEvent.click(screen.getByRole("button", { name: "Whole map" }));
  await waitFor(() => expect(inspect.mock.calls[0]?.[0]).toBe("artifact"));
});

test("reapply preserves a visible way to undo unfinished input whose source field disappeared", async () => {
  const { api, generate } = fixture();
  const saved = await api.snapshot();
  render(<MapWorkbench api={api} />, { wrapper: MemoryRouter });
  await waitFor(() => expect(screen.getByTestId("map-plan").querySelector("path")).not.toBeNull());
  fireEvent.change(screen.getByLabelText("Block depth"), { target: { value: "-" } });
  const outside = structuredClone(saved);
  outside.revision = "removed field";
  outside.fields = outside.fields.filter((field) => field.id !== "depth");
  outside.documents.presets = { transit: { max_s: 215 } };
  api.snapshot = async () => outside;
  fireEvent.click(screen.getByRole("button", { name: "Reload sources" }));
  await waitFor(() =>
    expect(screen.getByRole("button", { name: "Reapply my edits to new sources" })).toBeTruthy(),
  );
  fireEvent.click(screen.getByRole("button", { name: "Reapply my edits to new sources" }));
  expect((screen.getByLabelText("Block depth") as HTMLInputElement).value).toBe("-");
  expect(screen.getByText(/Block depth no longer exists/)).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "Undo edit" }));
  fireEvent.click(screen.getByRole("button", { name: "Reapply my edits to new sources" }));
  expect(screen.queryByLabelText("Block depth")).toBeNull();
  await waitFor(() => expect(generate).toHaveBeenCalledTimes(2));
  expect(screen.getByRole("button", { name: "Review save" }).hasAttribute("disabled")).toBe(false);
});

test("regenerating after a refused saved comparison retains only admitted geometry", async () => {
  const { api, generate } = fixture();
  render(<MapWorkbench api={api} />, { wrapper: MemoryRouter });
  await waitFor(() => expect(screen.getByTestId("map-plan").querySelector("path")).not.toBeNull());
  fireEvent.change(screen.getByLabelText("Block depth"), { target: { value: "140" } });
  await waitFor(() => expect(generate).toHaveBeenCalledTimes(2));
  generate.mockImplementation(async (request) => ({
    artifactId: "refused-baseline",
    fingerprint: "refused",
    choice: request.choice,
    status: "refused",
    stage: "generation",
    diagnostics: [],
  }));
  fireEvent.click(screen.getByRole("button", { name: "Saved baseline · same seed" }));
  await waitFor(() => expect(screen.getByText(/Saved baseline refused:/)).toBeTruthy());
  expect(screen.getByText(/Choose Draft plan to inspect the last admitted map/)).toBeTruthy();
  expect(screen.getByRole("button", { name: "Whole map" }).hasAttribute("disabled")).toBe(true);
  expect(screen.getByRole("button", { name: "Zoom in" }).hasAttribute("disabled")).toBe(true);
  expect(screen.getByRole("button", { name: "Zoom out" }).hasAttribute("disabled")).toBe(true);
  expect(screen.getByRole("button", { name: "Measure openness" }).hasAttribute("disabled")).toBe(
    true,
  );
  fireEvent.click(screen.getByRole("button", { name: "Regenerate" }));
  await waitFor(() => expect(generate).toHaveBeenCalledTimes(4));
  expect(generate.mock.calls.at(-1)?.[0].retainedArtifactIds).toEqual(["artifact"]);
});

test("feature selection belongs to its artifact when generation reuses district IDs", async () => {
  const { api, generate } = fixture();
  const first = await generate({
    purpose: "preview",
    retainedArtifactIds: [],
    draft: await api.snapshot(),
    choice: { type: "mixed", size: "small", seed: "1" },
  });
  first.features![0].crop = "district-1";
  generate
    .mockReset()
    .mockResolvedValueOnce(first)
    .mockResolvedValue({
      ...first,
      artifactId: "next-artifact",
      svg: '<svg viewBox="0 0 100 100"><path data-rule-group="districts.detached" data-feature-id="district-1" d="M10 10H90V90H10Z"/></svg>',
      features: [
        {
          id: "district-1",
          group: "districts.detached",
          label: "Detached district",
          crop: "district-1",
        },
      ],
    });
  render(<MapWorkbench api={api} />, { wrapper: MemoryRouter });
  await waitFor(() => expect(screen.getByTestId("map-plan").querySelector("path")).not.toBeNull());
  fireEvent.click(screen.getByTestId("map-plan").querySelector("path")!);
  expect(screen.getByRole("button", { name: "Inspect selection" }).hasAttribute("disabled")).toBe(
    false,
  );
  fireEvent.change(screen.getByLabelText("Block depth"), { target: { value: "140" } });
  await waitFor(() =>
    expect(
      screen.getByTestId("map-plan").querySelector("path")?.getAttribute("data-rule-group"),
    ).toBe("districts.detached"),
  );
  expect(screen.getByTestId("map-plan").querySelector("path")?.getAttribute("data-selected")).toBe(
    "false",
  );
  expect((screen.getByLabelText("Inspect feature") as HTMLSelectElement).value).toBe("");
  expect(screen.getByRole("button", { name: "Inspect selection" }).hasAttribute("disabled")).toBe(
    true,
  );
  expect((screen.getByLabelText("Block depth") as HTMLInputElement).value).toBe("140");
});
