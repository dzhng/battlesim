import { createRef } from "react";
import { act, fireEvent, render } from "@testing-library/react";
import { expect, test } from "vitest";
import {
  AppCursor,
  GameCursor,
  useCursorAction,
  type CursorAction,
  type GameCursorHandle,
} from "../src/battle/present/gameCursor";

test("placing and changing an action keeps the arrow anchored, default removes its badge, and exit hides it", () => {
  const handle = createRef<GameCursorHandle>();
  const view = render(<GameCursor handle={handle} />);
  const cursor = view.getByTestId("game-cursor");
  expect(cursor.hidden).toBe(true);
  act(() => handle.current!.place({ x: 120, y: 85 }, "default"));
  const arrow = cursor.querySelector(".game-cursor-arrow");
  const anchored = cursor.style.transform;
  expect(cursor.hidden).toBe(false);
  expect(cursor.querySelector<HTMLElement>(".game-cursor-badge")!.hidden).toBe(true);
  act(() => handle.current!.place({ x: 120, y: 85 }, "garrison"));
  expect(cursor.style.transform).toBe(anchored);
  expect(cursor.querySelector(".game-cursor-arrow")).toBe(arrow);
  expect(cursor.querySelector<HTMLElement>(".game-cursor-badge")!.hidden).toBe(false);
  expect(cursor.querySelector<HTMLElement>('[data-action="garrison"]')!.hidden).toBe(false);
  act(() => handle.current!.place({ x: 200, y: 145 }, "default"));
  expect(cursor.style.transform).not.toBe(anchored);
  expect(cursor.querySelector<HTMLElement>(".game-cursor-badge")!.hidden).toBe(true);
  act(() => handle.current!.place(null, "default"));
  expect(cursor.hidden).toBe(true);
  view.unmount();
});

// jsdom has no PointerEvent; a MouseEvent carries the same client coordinates.
globalThis.PointerEvent ??= MouseEvent as typeof PointerEvent;

test("the app cursor follows the pointer over every element, takes its action from the battle, and leaves only with the pointer", () => {
  let setAction: (action: CursorAction) => void = () => {};
  function Battle() {
    setAction = useCursorAction();
    return <button>Resume</button>;
  }
  const view = render(
    <AppCursor>
      <Battle />
    </AppCursor>,
  );
  const cursor = view.getByTestId("game-cursor");
  const menuButton = view.getByRole("button", { name: "Resume" });
  fireEvent.pointerMove(menuButton, { clientX: 300, clientY: 200 });
  expect(cursor.hidden).toBe(false);
  expect(cursor.style.transform).toBe("translate(300px, 200px) scale(0.7)");
  act(() => setAction("garrison"));
  expect(cursor.dataset.action).toBe("garrison");
  fireEvent.pointerMove(document.body, { clientX: 310, clientY: 210 });
  expect(cursor.style.transform).toBe("translate(310px, 210px) scale(0.7)");
  expect(cursor.dataset.action).toBe("garrison");
  fireEvent.pointerOut(document.body, { relatedTarget: null });
  expect(cursor.hidden).toBe(true);
  fireEvent.pointerMove(menuButton, { clientX: 5, clientY: 6 });
  expect(cursor.hidden).toBe(false);
  view.unmount();
});
