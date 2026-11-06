import { act } from "@testing-library/react";
import { expect, test } from "vitest";
import { building, getGpu, mount } from "./viewportHarness";

test("a failed initial scene build refuses the page GPU", async () => {
  const { view } = await mount();
  await building();
  await act(async () => getGpu().reject!(new Error("initial scene failed")));
  expect(getGpu().refused).toContainEqual(
    expect.objectContaining({ message: "initial scene failed" }),
  );
  expect(window.__lab?.error).toContain("initial scene failed");
  view.unmount();
});

test("a rebuild requested by a viewport consumer reports its terminal failure", async () => {
  const { advance, view } = await mount();
  await building();
  await act(async () => getGpu().resolve!());
  act(() => advance(1));
  const rebuilding = window.__lab!.rebuild!();
  const failure = expect(rebuilding).rejects.toThrow("consumer rebuild failed");
  await act(async () => {});
  await act(async () => getGpu().reject!(new Error("consumer rebuild failed")));
  await failure;
  expect(window.__lab?.ready).toBe(false);
  expect(window.__lab?.error).toContain("consumer rebuild failed");
  view.unmount();
});
