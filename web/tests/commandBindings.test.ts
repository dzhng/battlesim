// @vitest-environment jsdom
import { expect, test } from "vitest";
import {
  CommandBindings,
  commandForKey,
  isAttackMoveClick,
} from "../src/battle/input/commandBindings";

/** A real DOM key event, dispatched at `target` so its target is set. */
function press(code: string, init: KeyboardEventInit = {}, target: EventTarget = document.body) {
  let seen: KeyboardEvent | null = null;
  const listen = (e: Event) => (seen = e as KeyboardEvent);
  target.addEventListener("keydown", listen);
  target.dispatchEvent(new KeyboardEvent("keydown", { code, bubbles: true, ...init }));
  target.removeEventListener("keydown", listen);
  return seen!;
}

test("each command key maps to its command, per the user's table", () => {
  expect(commandForKey(press("Backspace"))).toBe("stop");
  expect(commandForKey(press("KeyR"))).toBe("attack_move");
  expect(commandForKey(press("KeyF"))).toBe("toggle_fire_policy");
  expect(commandForKey(press("KeyG"))).toBe("attack_ground");
  expect(commandForKey(press("KeyT"))).toBe("toggle_deployment");
  expect(commandForKey(press("Escape"))).toBe("disarm");
});

test("the letters the camera takes are no longer commands", () => {
  for (const code of ["KeyW", "KeyA", "KeyS", "KeyD", "KeyQ", "KeyE"])
    expect(commandForKey(press(code))).toBeNull();
});

test("no two commands share a key", () => {
  const codes = Object.values(CommandBindings).map((b) => b.code);
  expect(new Set(codes).size).toBe(codes.length);
});

test("keys typed into a control, held with a modifier or auto-repeated are not commands", () => {
  for (const tag of ["input", "textarea", "select"]) {
    const control = document.body.appendChild(document.createElement(tag));
    expect(commandForKey(press("KeyG", {}, control))).toBeNull();
    control.remove();
  }
  const editable = document.body.appendChild(document.createElement("div"));
  editable.contentEditable = "true";
  // jsdom lacks isContentEditable; define it as a browser reports it.
  Object.defineProperty(editable, "isContentEditable", { value: true });
  expect(commandForKey(press("KeyG", {}, editable))).toBeNull();
  editable.remove();
  expect(commandForKey(press("KeyR", { ctrlKey: true }))).toBeNull();
  expect(commandForKey(press("KeyR", { metaKey: true }))).toBeNull();
  expect(commandForKey(press("KeyR", { altKey: true }))).toBeNull();
  expect(commandForKey(press("KeyR", { repeat: true }))).toBeNull();
  // Shift is not a guard: it queues, and never changes which command a key is.
  expect(commandForKey(press("KeyR", { shiftKey: true }))).toBe("attack_move");
});

test("Ctrl+right-click attack-moves; a plain right-click or Ctrl+left-click does not", () => {
  expect(isAttackMoveClick({ button: "right", ctrl: true })).toBe(true);
  expect(isAttackMoveClick({ button: "right", ctrl: false })).toBe(false);
  expect(isAttackMoveClick({ button: "left", ctrl: true })).toBe(false);
});
