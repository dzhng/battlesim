import { act, cleanup, fireEvent } from "@testing-library/react";
import { renderInRouter as render } from "./support/router";
import { afterEach, expect, test, vi } from "vitest";
import { GraphicsSettingsControls } from "@apps/battle-lab/src/GraphicsSettingsControls";
import { graphicsSettings } from "@apps/battle-lab/src/graphicsSettings";
import { PauseMenu } from "@apps/battle-lab/src/PauseMenu";

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
  fireEvent.click(view.getByRole("button", { name: "Settings" }));
  expect(view.getByRole("dialog", { name: "Settings" })).toBeTruthy();
  fireEvent.keyDown(window, { code: "Escape", cancelable: true });
  expect(view.queryByRole("dialog", { name: "Settings" })).toBeNull();
  view.unmount();
});
