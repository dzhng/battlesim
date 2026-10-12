// @vitest-environment jsdom
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { MusicPlayer } from "@packages/battle-audio/src/music/musicPlayer";

/** An AudioParam that holds its last scheduled target. */
class Param {
  events: Array<[string, ...number[]]> = [];
  constructor(public value = 1) {}
  setValueAtTime(value: number, at: number) {
    this.value = value;
    this.events.push(["set", value, at]);
  }
  linearRampToValueAtTime(value: number, at: number) {
    this.value = value;
    this.events.push(["ramp", value, at]);
  }
  setTargetAtTime(value: number, at: number, time: number) {
    this.value = value;
    this.events.push(["target", value, at, time]);
  }
  cancelScheduledValues() {}
}
class Node {
  outputs = new Set<Node>();
  connect(node: Node) {
    this.outputs.add(node);
    return node;
  }
  disconnect() {
    this.outputs.clear();
  }
}
class Gain extends Node {
  gain: Param;
  constructor(_context: unknown, options: { gain?: number } = {}) {
    super();
    this.gain = new Param(options.gain ?? 1);
  }
}
class Source extends Node {
  constructor(
    _context: unknown,
    readonly options: { mediaElement: Media },
  ) {
    super();
  }
}
class Media {
  loop = false;
  preload = "";
  src = "";
  paused = true;
  play = vi.fn(async () => {
    this.paused = false;
  });
  pause = vi.fn(() => {
    this.paused = true;
  });
  load = vi.fn();
  removeAttribute(name: string) {
    if (name === "src") this.src = "";
  }
  constructor() {
    media.push(this);
  }
}
const media: Media[] = [];
const destination = new Node();
const context = { currentTime: 10 };
const tracks = {
  menu: { source: "bed", sha256: "ab".repeat(32), gain: 1 },
  combat: { source: "bed", sha256: "cd".repeat(32), gain: 0.8 },
};
const player = () => new MusicPlayer(context as never, tracks, destination as never);
/** The gain a playing element reaches the destination with. */
function reach(from: Node): number {
  if (from === destination) return 1;
  for (const next of from.outputs) {
    const rest = reach(next);
    if (rest >= 0) return (from instanceof Gain ? from.gain.value : 1) * rest;
  }
  return -1;
}
const sources: Source[] = [];
const sourceOf = (m: Media) => sources.find((s) => s.options.mediaElement === m)!;

beforeEach(() => {
  vi.useFakeTimers();
  vi.stubGlobal("GainNode", Gain);
  vi.stubGlobal(
    "MediaElementAudioSourceNode",
    class extends Source {
      constructor(c: unknown, o: { mediaElement: Media }) {
        super(c, o);
        sources.push(this);
      }
    },
  );
  vi.stubGlobal("Audio", Media);
});
afterEach(() => {
  media.length = 0;
  sources.length = 0;
  destination.disconnect();
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

test("a track streams its hashed file, looping, into the destination at its catalog gain", () => {
  player().play("combat");
  expect(media[0]).toMatchObject({ src: "/audio/music/combat.cdcdcdcd.mp3", loop: true });
  expect(media[0].play).toHaveBeenCalled();
  expect(reach(sourceOf(media[0]))).toBeCloseTo(0.8);
});

test("a crossfade ramps the new track in and the old one out over the same span, then frees the old element", () => {
  const music = player();
  music.play("menu");
  const next = music.crossfadeTo("combat", 4);
  expect((next.gain.gain as unknown as Param).events).toEqual([
    ["set", 0, 10],
    ["ramp", 0.8, 14],
  ]);
  const old = media[0];
  expect(reach(sourceOf(old))).toBe(0);
  vi.advanceTimersByTime(4000);
  expect(old.pause).toHaveBeenCalled();
  expect(old.src).toBe("");
  expect(sourceOf(old).outputs.size).toBe(0);
});

test("a long session of plays and stops leaves nothing connected but the playing track", () => {
  const music = player();
  for (let i = 0; i < 50; i++) {
    music.crossfadeTo(i % 2 ? "menu" : "combat", 1);
    vi.advanceTimersByTime(1000);
  }
  const connected = sources.filter((s) => s.outputs.size > 0);
  expect(connected).toHaveLength(1);
  expect(media.filter((m) => m.src)).toHaveLength(1);
  music.stop();
  expect(sources.filter((s) => s.outputs.size > 0)).toHaveLength(0);
});

test("ducking dips over the attack and restores over the release without touching the level", () => {
  const music = player();
  music.play("menu");
  music.setLevel(0.5);
  music.duck(0.35, 0.05, 0.8);
  expect((music.ducking.gain as unknown as Param).events.at(-1)).toEqual([
    "target",
    0.65,
    10,
    0.05,
  ]);
  expect(music.level.gain.value).toBe(0.5);
  expect(reach(sourceOf(media[0]))).toBeCloseTo(0.5 * 0.65);
  music.setLevel(0.2);
  expect(music.ducking.gain.value).toBeCloseTo(0.65);
  music.duck(0, 0.05, 0.8);
  expect((music.ducking.gain as unknown as Param).events.at(-1)).toEqual(["target", 1, 10, 0.8]);
  expect(reach(sourceOf(media[0]))).toBeCloseTo(0.2);
});

test("playback autoplay refused retries on resume", () => {
  const music = player();
  vi.stubGlobal(
    "Audio",
    class extends Media {
      play = vi.fn(() => Promise.reject(new Error("NotAllowedError")));
    },
  );
  music.play("menu");
  const refused = media[0];
  refused.play = vi.fn(async () => {
    refused.paused = false;
  });
  music.resume();
  expect(refused.play).toHaveBeenCalledTimes(1);
  expect(refused.paused).toBe(false);
});
