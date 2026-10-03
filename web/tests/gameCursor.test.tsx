import { createRef } from "react";
import { act, render } from "@testing-library/react";
import { expect, test } from "vitest";
import { GameCursor, type GameCursorHandle } from "../src/battle/present/gameCursor";

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
});
