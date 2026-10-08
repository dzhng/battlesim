import { createRef } from "react";
import { act, render } from "@testing-library/react";
import { expect, test } from "vitest";
import { FrameRate, type FrameRateHandle } from "../src/battle/present/frameRate";

test("the readout counts drawn frames over elapsed time, including idle frames", () => {
  const handle = createRef<FrameRateHandle>();
  const view = render(<FrameRate handle={handle} />);
  const readout = view.container.querySelector(".frame-rate")!;
  act(() => {
    handle.current!.frame(0, false);
    for (let i = 1; i <= 60; i++) handle.current!.frame(i * (1000 / 60), i % 2 === 0);
  });
  expect(readout.textContent).toBe("30 FPS");
  act(() => {
    for (let i = 1; i <= 30; i++) handle.current!.frame(1000 + i * (1000 / 30), true);
  });
  expect(readout.textContent).toBe("30 FPS");
  view.unmount();
});
