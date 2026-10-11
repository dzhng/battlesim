import { act, render } from "@testing-library/react";
import { expect, test } from "vitest";
import {
  AppCursor,
  CURSOR_ACTIONS,
  cursorImage,
  cursorStyle,
  useCursorAction,
  type CursorAction,
} from "../src/battle/present/gameCursor";

const svgOf = (url: string) =>
  new DOMParser().parseFromString(
    decodeURIComponent(url.slice("data:image/svg+xml,".length)),
    "image/svg+xml",
  );

test("the system draws each action's own image, tip on the pointer, with a plain arrow fallback", () => {
  const styles = CURSOR_ACTIONS.map(cursorStyle);
  expect(new Set(styles).size).toBe(CURSOR_ACTIONS.length);
  for (const style of styles) expect(style).toMatch(/ 1x, url\(".*"\) 2x\) 2 2, default$/);
  // The default arrow has no badge; every other action adds exactly one.
  for (const action of CURSOR_ACTIONS) {
    const nestedIcons = svgOf(cursorImage(action)).querySelectorAll("svg svg").length;
    expect(nestedIcons).toBe(action === "default" ? 1 : 2);
  }
  // The 2x image is the same picture at twice the pixels.
  const sharp = svgOf(cursorImage("garrison", 2)).documentElement;
  expect([sharp.getAttribute("width"), sharp.getAttribute("viewBox")]).toEqual(["64", "0 0 32 32"]);
});

test("the app cursor is the game's arrow everywhere and takes its action from the page", () => {
  let setAction: (action: CursorAction) => void = () => {};
  function Battle() {
    setAction = useCursorAction();
    return <button>Resume</button>;
  }
  const root = document.documentElement;
  const view = render(
    <AppCursor>
      <Battle />
    </AppCursor>,
  );
  expect(root.dataset.cursor).toBe("default");
  expect(root.style.getPropertyValue("--game-cursor")).toBe(cursorStyle("default"));
  act(() => setAction("garrison"));
  expect(root.dataset.cursor).toBe("garrison");
  expect(root.style.getPropertyValue("--game-cursor")).toBe(cursorStyle("garrison"));
  view.unmount();
  expect(root.dataset.cursor).toBeUndefined();
  expect(root.style.getPropertyValue("--game-cursor")).toBe("");
});
