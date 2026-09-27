// @vitest-environment jsdom
import { afterEach, expect, test } from "vitest";
import { trackHeldKeys, type HeldKeys } from "../src/battle/input/heldKeys";

let keys: HeldKeys | null = null;
afterEach(() => {
  keys?.detach();
  keys = null;
  document.body.replaceChildren();
});

const key = (type: "keydown" | "keyup", code: string, init: KeyboardEventInit = {}, at?: Element) =>
  (at ?? window).dispatchEvent(
    new KeyboardEvent(type, { code, bubbles: true, cancelable: true, ...init }),
  );

test("a key is held from keydown to keyup, and several are held at once", () => {
  keys = trackHeldKeys(window, () => true);
  key("keydown", "KeyW");
  key("keydown", "KeyD");
  expect([...keys.held].sort()).toEqual(["KeyD", "KeyW"]);
  key("keyup", "KeyW");
  expect([...keys.held]).toEqual(["KeyD"]);
});

test("losing focus releases every held key", () => {
  keys = trackHeldKeys(window, () => true);
  key("keydown", "KeyW");
  key("keydown", "KeyQ");
  window.dispatchEvent(new Event("blur"));
  expect(keys.held.size).toBe(0);
});

test("keys typed into a control or pressed with Ctrl, Cmd or Alt are not held", () => {
  keys = trackHeldKeys(window, () => true);
  const input = document.body.appendChild(document.createElement("input"));
  key("keydown", "KeyW", {}, input);
  key("keydown", "KeyA", { ctrlKey: true });
  key("keydown", "KeyS", { metaKey: true });
  key("keydown", "KeyD", { altKey: true });
  expect(keys.held.size).toBe(0);
});

test("claimed keys lose their browser default (arrows don't scroll); others keep it", () => {
  keys = trackHeldKeys(window, (code) => code === "ArrowUp");
  const arrow = new KeyboardEvent("keydown", { code: "ArrowUp", cancelable: true });
  const other = new KeyboardEvent("keydown", { code: "KeyX", cancelable: true });
  window.dispatchEvent(arrow);
  window.dispatchEvent(other);
  expect(arrow.defaultPrevented).toBe(true);
  expect(other.defaultPrevented).toBe(false);
  // Only claimed keys are held.
  expect([...keys.held]).toEqual(["ArrowUp"]);
});

test("after detach nothing is tracked", () => {
  keys = trackHeldKeys(window, () => true);
  keys.detach();
  key("keydown", "KeyW");
  expect(keys.held.size).toBe(0);
});

test("the change callback hears each press and release that changes the held set, once", () => {
  const seen: boolean[] = [];
  keys = trackHeldKeys(
    window,
    (c) => c === "Space",
    (h) => seen.push(h.has("Space")),
  );
  key("keydown", "Space");
  key("keydown", "Space", { repeat: true });
  key("keydown", "KeyW");
  key("keyup", "Space");
  expect(seen).toEqual([true, false]);
});
