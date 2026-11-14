import { renderInRouter as render } from "./support/router";
import { act, cleanup, fireEvent, renderHook } from "@testing-library/react";
import { afterEach, expect, test } from "vitest";
import { MenuButton, PauseMenu, usePauseMenu } from "@apps/battle-lab/src/PauseMenu";
import { useState } from "react";
import { useUnitControl } from "../src/battle/input/useUnitControl";
import type { ObservationView } from "../src/battle/sim/observationView";
import type { Order } from "../src/battle/sim/protocol";

import type { SimClient } from "../src/battle/sim/client";
import { WithTestCatalog } from "./catalog";

test("leaving for the main menu asks first, and staying keeps the battle", async () => {
  const view = render(<PauseMenu onClose={() => {}} />);
  // The first press only asks; the menu's other items step aside.
  await act(async () => view.getByRole("button", { name: "Main menu" }).click());
  expect(view.getByRole("dialog", { name: "Leave the battle?" })).toBeTruthy();
  expect(view.queryByRole("button", { name: "Resume" })).toBeNull();
  await act(async () => view.getByRole("button", { name: "Stay" }).click());
  expect(view.getByRole("button", { name: "Resume" })).toBeTruthy();
  // Escape backs out of the question, not out of the pause menu.
  await act(async () => view.getByRole("button", { name: "Main menu" }).click());
  fireEvent.keyDown(window, { code: "Escape", cancelable: true });
  expect(view.getByRole("button", { name: "Resume" })).toBeTruthy();
  await act(async () => view.getByRole("button", { name: "Main menu" }).click());
  expect(view.getByRole("link", { name: "Leave battle" }).getAttribute("href")).toBe("/");
  view.unmount();
});

test("restart asks first, and only confirming restarts", async () => {
  let restarts = 0;
  const view = render(<PauseMenu onClose={() => {}} onRestart={() => restarts++} />);
  await act(async () => view.getByRole("button", { name: "Restart" }).click());
  expect(restarts).toBe(0);
  expect(view.getByRole("dialog", { name: "Restart the battle?" })).toBeTruthy();
  expect(view.queryByRole("button", { name: "Resume" })).toBeNull();
  await act(async () => view.getByRole("button", { name: "Cancel" }).click());
  expect(view.getByRole("button", { name: "Resume" })).toBeTruthy();
  await act(async () => view.getByRole("button", { name: "Restart" }).click());
  fireEvent.keyDown(window, { code: "Escape", cancelable: true });
  expect(view.getByRole("button", { name: "Resume" })).toBeTruthy();
  expect(restarts).toBe(0);
  await act(async () => view.getByRole("button", { name: "Restart" }).click());
  await act(async () => view.getByRole("button", { name: "Restart battle" }).click());
  expect(restarts).toBe(1);
  view.unmount();
});

test("the pause menu opens the tutorial above it and closes back to the menu", async () => {
  const view = render(<PauseMenu onClose={() => {}} />);
  await act(async () => view.getByRole("button", { name: "Tutorial" }).click());
  expect(view.getByRole("dialog", { name: "Tutorial" })).toBeTruthy();
  expect(view.getByRole("button", { name: "Close tutorial" })).toBeTruthy();
  fireEvent.keyDown(window, { code: "Escape", cancelable: true });
  expect(view.queryByRole("dialog", { name: "Tutorial" })).toBeNull();
  await act(async () => view.getByRole("button", { name: "Tutorial" }).click());
  await act(async () => view.getByRole("button", { name: "Close tutorial" }).click());
  expect(view.queryByRole("dialog", { name: "Tutorial" })).toBeNull();
  expect(view.getByRole("dialog", { name: "Paused" })).toBeTruthy();
  view.unmount();
});

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
  const hook = renderHook(() => {
    const [open, setOpen] = useState(false);
    return usePauseMenu(client, open, setOpen);
  });
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

afterEach(cleanup);

test("pause controls own shortcuts while armed Escape cancels before opening pause", async () => {
  let paused = false;
  const sent: Order[] = [];
  const client = {
    get paused() {
      return paused;
    },
    pause() {
      paused = true;
    },
    resume() {
      paused = false;
    },
    async command(order: Order) {
      sent.push(order);
      return { seq: sent.length, applied_tick: 0, error: null, refused: [] };
    },
  } as unknown as SimClient;
  const observation = {
    own: [{ id: 1, kind: "test_rifle", position: [0, 0, 0] }],
    contacts: [],
  } as unknown as ObservationView;
  function Battle() {
    const [open, setOpen] = useState(false);
    const control = useUnitControl(client, observation, undefined, !open);
    const pause = usePauseMenu(client, open, setOpen);
    return (
      <>
        <button onClick={() => control.setSelected([1])}>Select</button>
        <span data-testid="mode">{control.mode}</span>
        <MenuButton onOpen={() => pause.show(true)} />
        {pause.open && <PauseMenu onClose={() => pause.show(false)} />}
      </>
    );
  }
  const view = render(
    <WithTestCatalog>
      <Battle />
    </WithTestCatalog>,
  );
  fireEvent.click(view.getByRole("button", { name: "Select" }));
  fireEvent.keyDown(window, { code: "KeyG", cancelable: true });
  expect(view.getByTestId("mode").textContent).toBe("attack_ground");
  fireEvent.keyDown(window, { code: "Escape", cancelable: true });
  expect(view.queryByRole("dialog")).toBeNull();
  expect(view.getByTestId("mode").textContent).toBe("move");
  fireEvent.keyDown(window, { code: "Escape", cancelable: true });
  const resume = view.getByRole("button", { name: "Resume" });
  expect(paused).toBe(true);
  await act(async () => fireEvent.keyDown(resume, { code: "Backspace", cancelable: true }));
  expect(sent).toEqual([]);
  fireEvent.keyDown(resume, { code: "Escape", cancelable: true });
  expect(view.queryByRole("dialog")).toBeNull();
  expect(paused).toBe(false);
  await act(async () => fireEvent.keyDown(window, { code: "Backspace", cancelable: true }));
  expect(sent).toEqual([{ kind: "stop", units: [1] }]);
});
