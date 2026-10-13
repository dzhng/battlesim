import { act, cleanup, fireEvent, waitFor } from "@testing-library/react";
import { renderInRouter as render } from "./support/router";
import { afterEach, expect, test, vi } from "vitest";
import { GraphicsSettingsControls } from "@apps/battle-lab/src/GraphicsSettingsControls";
import { graphicsSettings } from "@apps/battle-lab/src/graphicsSettings";
import { PauseMenu } from "@apps/battle-lab/src/PauseMenu";
import { GraphicsDetectionHost } from "@apps/battle-lab/src/GraphicsDetectionHost";
import { LoadingScreen } from "@apps/battle-lab/src/LoadingScreen";
import {
  cancelAutoDetect,
  registerGraphicsDetectionRunner,
  runAutoDetect,
} from "@apps/battle-lab/src/graphicsDetection";

afterEach(() => {
  cleanup();
  graphicsSettings.set({ mode: "auto", detectedMode: null, lastResult: null });
});

test("shared graphics controls expose exactly the four preset choices and persist selection", () => {
  const view = render(<GraphicsSettingsControls />);
  expect(view.getAllByRole("radio")).toHaveLength(4);
  fireEvent.click(view.getByRole("radio", { name: "Medium" }));
  expect(graphicsSettings.get().mode).toBe("medium");
  expect(view.getByRole("radio", { name: "Medium" }).getAttribute("aria-checked")).toBe("true");
  view.unmount();
});

test("rerun action is shared and selecting a tier waits for the rebuild", async () => {
  let resolve!: () => void;
  const apply = vi.fn(() => new Promise<void>((r) => (resolve = r)));
  const rerun = vi.fn();
  const view = render(<GraphicsSettingsControls onApply={apply} onRunAutoDetect={rerun} />);
  fireEvent.click(view.getByRole("button", { name: "Run auto-detect again" }));
  expect(rerun).toHaveBeenCalledOnce();
  await act(async () => fireEvent.click(view.getByRole("radio", { name: "Low" })));
  expect(view.getByRole("radio", { name: "Low" }).hasAttribute("disabled")).toBe(true);
  expect(view.getByText("Applying settings…")).toBeTruthy();
  await act(async () => resolve());
  expect(view.getByRole("radio", { name: "Low" }).hasAttribute("disabled")).toBe(false);
  view.unmount();
});

test("rerun failures stay actionable in the settings surface", async () => {
  const view = render(
    <GraphicsSettingsControls
      onRunAutoDetect={() => Promise.reject(new Error("no preset met the target"))}
    />,
  );
  await act(async () =>
    fireEvent.click(view.getByRole("button", { name: "Run auto-detect again" })),
  );
  expect(view.getByRole("alert").textContent).toContain("no preset met the target");
  expect(view.getByRole("button", { name: "Reload" })).toBeTruthy();
  view.unmount();
});

test("pause menu opens settings as a nested modal and Escape closes it", () => {
  const view = render(<PauseMenu onClose={() => {}} />);
  expect(view.queryByLabelText("Master volume")).toBeNull();
  expect(view.queryByRole("button", { name: "Copy diagnostics" })).toBeNull();
  fireEvent.click(view.getByRole("button", { name: "Settings" }));
  expect(view.getByRole("dialog", { name: "Settings" })).toBeTruthy();
  expect(view.queryByRole("dialog", { name: "Paused" })).toBeNull();
  expect(view.getByLabelText("Master volume")).toBeTruthy();
  expect(view.getByRole("button", { name: "Copy diagnostics" })).toBeTruthy();
  fireEvent.keyDown(window, { code: "Escape", cancelable: true });
  expect(view.queryByRole("dialog", { name: "Settings" })).toBeNull();
  expect(view.getByRole("dialog", { name: "Paused" })).toBeTruthy();
  expect(view.queryByLabelText("Master volume")).toBeNull();
  view.unmount();
});

test("graphics detection keeps the mounted menu covered by the shared loading card", async () => {
  Object.defineProperty(navigator, "gpu", { configurable: true, value: {} });
  const dispose = registerGraphicsDetectionRunner(
    (_, signal) =>
      new Promise((_, reject) =>
        signal.addEventListener(
          "abort",
          () => reject(new DOMException("Detection cancelled", "AbortError")),
          { once: true },
        ),
      ),
  );
  const view = render(
    <GraphicsDetectionHost runner={null}>
      <div data-testid="menu-reel">menu reel</div>
    </GraphicsDetectionHost>,
  );
  const pending = runAutoDetect();
  await waitFor(() => expect(view.getByTestId("menu-reel")).toBeTruthy());
  expect(view.getByTestId("menu-reel")).toBeTruthy();
  expect(view.queryByTestId("graphics-detection-progress")).toBeNull();
  cancelAutoDetect();
  await expect(pending).rejects.toThrow("cancelled");
  dispose();
  view.unmount();
});

test("the shared loading card shows graphics detection progress and the selected tier", () => {
  const view = render(
    <LoadingScreen
      title="Deploying"
      stages={[
        { id: "preparation", label: "Preparing the battle" },
        { id: "graphics", label: "Detecting Graphics" },
      ]}
      current="graphics"
      stageDetail="Testing HIGH graphics · 1 of 3 presets"
      back={null}
    />,
  );
  expect(view.getByTestId("loading-stage").textContent).toContain("Detecting Graphics");
  expect(view.getByRole("listitem", { name: "Detecting Graphics" }).getAttribute("data-state")).toBe(
    "active",
  );
  expect(view.getByTestId("loading-stage-detail").textContent).toContain("Testing HIGH graphics");
  view.rerender(
    <LoadingScreen
      title="Deploying"
      stages={[
        { id: "preparation", label: "Preparing the battle" },
        { id: "graphics", label: "Detecting Graphics" },
      ]}
      current="graphics"
      stageDetail="Graphics set to HIGH"
      back={null}
    />,
  );
  expect(view.getByTestId("loading-stage-detail").textContent).toBe("Graphics set to HIGH");
  view.unmount();
});
