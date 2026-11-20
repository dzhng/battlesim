// Full screen, as a video player has it: the whole page, the browser's own
// bars hidden. F toggles it anywhere in the app (`FullscreenBinding`), and the
// main menu and the pause menu each carry a button. Esc stays the game's while
// full screen (Chromium's keyboard lock), so cancelling an order or opening the
// pause menu never drops the player out; holding Esc leaves, as the browser's
// own banner says. A browser without the lock leaves on the first Esc.
import { useEffect, useSyncExternalStore } from "react";
import { FullscreenBinding } from "@web/battle/input/commandBindings";
import { isGameKey } from "@web/battle/input/keyGuard";

/** Chromium's Keyboard Lock, which the DOM's types don't carry yet. */
interface KeyboardLock {
  lock(codes?: string[]): Promise<void>;
  unlock(): void;
}
const keyboard = () => (navigator as Navigator & { keyboard?: KeyboardLock }).keyboard;

function subscribe(change: () => void) {
  document.addEventListener("fullscreenchange", change);
  return () => document.removeEventListener("fullscreenchange", change);
}
const isFullscreen = () => document.fullscreenElement !== null;

/** Enter full screen, or leave it. Called from a click or key press: the
 *  browser grants it only to the player's own gesture. */
export async function toggleFullscreen(): Promise<void> {
  if (isFullscreen()) {
    keyboard()?.unlock();
    await document.exitFullscreen();
    return;
  }
  await document.documentElement.requestFullscreen({ navigationUI: "hide" });
  await keyboard()
    ?.lock(["Escape"])
    .catch(() => {});
}

/** Whether the page is full screen now. */
export function useFullscreen(): boolean {
  return useSyncExternalStore(subscribe, isFullscreen);
}

/** F toggles full screen while this is mounted (once, at the app's root). */
export function useFullscreenKey(): void {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.code !== FullscreenBinding.code || !isGameKey(e) || e.defaultPrevented) return;
      e.preventDefault();
      void toggleFullscreen().catch(() => {});
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);
}

/** The control's words for the state it would change. */
export const fullscreenLabel = (full: boolean) => (full ? "Exit full screen" : "Full screen");
