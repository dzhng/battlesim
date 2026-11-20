// Full screen, as a video player has it: the whole page, the browser's own
// bars hidden. F toggles it anywhere in the app (`FullscreenBinding`), and the
// main menu and the pause menu each carry a button. Esc stays the game's while
// full screen (Chromium's keyboard lock), so cancelling an order or opening the
// pause menu never drops the player out; holding Esc leaves, as the browser's
// own banner says.
import { useEffect, useSyncExternalStore } from "react";
import { FullscreenBinding } from "@web/battle/input/commandBindings";
import { isGameKey } from "@web/battle/input/keyGuard";

/** Chromium's Keyboard Lock, which the DOM's types don't carry yet. */
const keyboard = () =>
  (navigator as Navigator & { keyboard: { lock(codes: string[]): Promise<void>; unlock(): void } })
    .keyboard;

function subscribe(change: () => void) {
  document.addEventListener("fullscreenchange", change);
  return () => document.removeEventListener("fullscreenchange", change);
}
const isFullscreen = () => document.fullscreenElement !== null;

async function setFullscreen(on: boolean) {
  if (!on) {
    keyboard().unlock();
    return document.exitFullscreen();
  }
  await document.documentElement.requestFullscreen({ navigationUI: "hide" });
  await keyboard().lock(["Escape"]);
}

/** Enter full screen, or leave it. Called from a click or key press: the
 *  browser grants it only to the player's own gesture, and a refusal leaves
 *  the page as it was. */
export function toggleFullscreen(): void {
  setFullscreen(!isFullscreen()).catch(() => {});
}

/** Whether the page is full screen now. */
export function useFullscreen(): boolean {
  return useSyncExternalStore(subscribe, isFullscreen);
}

/** F toggles full screen while this is mounted (once, at the app's root). */
export function useFullscreenKey(): void {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.code === FullscreenBinding.code && isGameKey(e)) toggleFullscreen();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);
}

/** The control's words for the state it would change. */
export const fullscreenLabel = (full: boolean) => (full ? "Exit full screen" : "Full screen");
