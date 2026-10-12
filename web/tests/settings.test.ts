import { afterEach, expect, test } from "vitest";
import { soundSettings } from "@packages/battle-audio/src/settings";

afterEach(() => soundSettings.set({ muted: false, volume: 0.8, music: 1, voices: 1 }));

test("music and voices levels persist and clamp like master volume", () => {
  soundSettings.set({ music: 2, voices: -1 });
  expect(soundSettings.get()).toMatchObject({ music: 1, voices: 0 });
  soundSettings.set({ music: 0.35, voices: 0.7 });
  expect(soundSettings.get()).toMatchObject({ music: 0.35, voices: 0.7 });
});
