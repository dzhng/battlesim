import { act, renderHook } from "@testing-library/react";
import { expect, test } from "vitest";
import { usePauseMenu } from "@apps/battle-lab/src/PauseMenu";
import type { SimClient } from "../src/battle/sim/client";

test("the menu pauses a stalled battle until closed, and preserves an existing pause", () => {
  let paused = false;
  const client = {
    status: "waiting-consumer",
    get paused() {
      return paused;
    },
    pause: () => {
      paused = true;
    },
    resume: () => {
      paused = false;
    },
  } as unknown as SimClient;
  const hook = renderHook(() => usePauseMenu(client));
  act(() => hook.result.current.show(true));
  expect(paused).toBe(true);
  act(() => hook.result.current.show(true));
  act(() => hook.result.current.show(false));
  expect(paused).toBe(false);
  client.pause();
  act(() => hook.result.current.show(true));
  act(() => hook.result.current.show(false));
  expect(paused).toBe(true);
  hook.unmount();
});
