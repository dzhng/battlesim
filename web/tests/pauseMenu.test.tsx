import { renderInRouter as render } from "./support/router";
import { act, cleanup, fireEvent, renderHook } from "@testing-library/react";
import { afterEach, expect, test } from "vitest";
import { MenuButton, PauseMenu, usePauseMenu } from "@apps/battle-lab/src/PauseMenu";
import { useState } from "react";
import { useUnitControl } from "../src/battle/input/useUnitControl";
import type { ObservationView } from "../src/battle/sim/observation";
import type { Order } from "../src/battle/sim/protocol";

import type { SimClient } from "../src/battle/sim/client";

test("the paused battle offers a return to the game's main menu", () => {
  const view = render(<PauseMenu onClose={() => {}} />);
  expect(view.getByRole("link", { name: "Main menu" }).getAttribute("href")).toBe("/");
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
      return { seq: sent.length, applied_tick: 0, error: null };
    },
  } as unknown as SimClient;
  const observation = {
    own: [{ id: 1, kind: "rifle", position: [0, 0, 0] }],
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
  const view = render(<Battle />);
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
