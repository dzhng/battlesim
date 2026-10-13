// @vitest-environment jsdom
import { afterEach, expect, test, vi } from "vitest";
import { downloadScenarioBundle } from "../src/battle/scenarioBundle";

afterEach(() => {
  vi.restoreAllMocks();
  vi.useRealTimers();
});

test("scenario download keeps its blob available after the click handoff", async () => {
  vi.useFakeTimers();
  const url = "blob:scenario";
  const create = vi.spyOn(URL, "createObjectURL").mockReturnValue(url);
  const revoke = vi.spyOn(URL, "revokeObjectURL").mockImplementation(() => {});
  const click = vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(() => {});

  downloadScenarioBundle("{}", "scenario.json");
  expect(create).toHaveBeenCalled();
  expect(click).toHaveBeenCalled();
  await Promise.resolve();
  expect(revoke).not.toHaveBeenCalled();

  vi.runAllTimers();
  expect(revoke).toHaveBeenCalledWith(url);
});
