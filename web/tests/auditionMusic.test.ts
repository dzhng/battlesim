// @vitest-environment node
import { afterEach, expect, test, vi } from "vitest";

const loadPresentation = vi.hoisted(() => vi.fn(async () => {}));
const MusicPlayer = vi.hoisted(() => vi.fn());
vi.mock("@web/battle/sim/presentation", () => ({ loadPresentation }));
vi.mock("@packages/battle-audio/src/music/musicPlayer", () => ({ MusicPlayer }));

import { SoundAuditioner } from "../../apps/sound-workbench/src/audition";

afterEach(() => {
  vi.unstubAllGlobals();
  loadPresentation.mockReset().mockResolvedValue(undefined);
  MusicPlayer.mockReset();
});

test("a player created as its run is replaced is disposed and rejected", async () => {
  const dispose = vi.fn();
  const player = { crossfadeTo: vi.fn(), stop: vi.fn(), dispose };
  let replace: (() => void) | undefined;
  MusicPlayer.mockImplementation(function MusicPlayerMock() {
    replace?.();
    return player;
  });
  class Context {
    destination = {};
    resume = async () => {};
    close = vi.fn(async () => {});
  }
  class Node {
    connect(node: Node) {
      return node;
    }
  }
  vi.stubGlobal("AudioContext", Context);
  for (const name of ["WaveShaperNode", "StereoPannerNode"]) vi.stubGlobal(name, Node);
  const audition = new SoundAuditioner();
  replace = () => audition.stop();

  await expect(audition.music({})).rejects.toMatchObject({ name: "AbortError" });
  expect(dispose).toHaveBeenCalledOnce();
});
