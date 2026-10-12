// @vitest-environment node
// Which voice lines play: the arbitration every battle line and the workbench
// preview go through, against a fake clock, sink and bank.
import { expect, test } from "vitest";
import {
  Chatter,
  type ChatterRules,
  type LineSink,
  type LineSource,
  type VoiceLine,
  type VoiceRequest,
} from "@packages/battle-audio/src/voices/chatter";
import type { Intensity, Trigger } from "@packages/battle-audio/src/voices/voiceCatalog";

const RULES: ChatterRules = { lines: 4, wait_s: 1, group_s: 0.25 };

/** A clip of `seconds`, as the bank would hand it over. */
const clip = (seconds: number, name = "") =>
  ({ duration: seconds, name }) as unknown as AudioBuffer & { name: string };

/** One set: a leader who answers, two soldiers who shout; every take `seconds` long. */
class FakeBank implements LineSource {
  missing = new Set<string>();
  constructor(private readonly seconds = 2) {}
  voices() {
    return [
      { id: "lead", speaks: "responses" as const },
      { id: "rifle", speaks: "barks" as const },
      { id: "gunner", speaks: "barks" as const },
    ];
  }
  get(set: string, voice: string, trigger: Trigger, intensity: Intensity, take: number) {
    const id = `${set}/${voice}/${trigger}/${intensity}/${take}`;
    return this.missing.has(id) ? null : clip(this.seconds, id);
  }
}

class FakeSink implements LineSink {
  t = 0;
  readonly played: VoiceLine[] = [];
  now() {
    return this.t;
  }
  play(line: VoiceLine) {
    this.played.push(line);
  }
}

function setup(rules: Partial<ChatterRules> = {}, seconds = 2, seed = 1) {
  const sink = new FakeSink();
  const bank = new FakeBank(seconds);
  const chatter = new Chatter({ ...RULES, ...rules }, bank, sink, seed);
  /** Advances the clock to `t` and arbitrates, as a frame would. */
  const at = (t: number, listener: readonly number[] = [0, 0, 0]) => {
    sink.t = t;
    return chatter.update(listener);
  };
  return { sink, bank, chatter, at };
}

const bark = (speaker: string, at: number, extra: Partial<VoiceRequest> = {}): VoiceRequest => ({
  tier: "own_bark",
  speaker,
  set: "us_tank_crew",
  trigger: "under_fire",
  intensity: "steady",
  at,
  ...extra,
});
const speakers = (lines: readonly VoiceLine[]) => lines.map((line) => line.request.speaker);

test("at most four lines play at once; the rest wait for a free line", () => {
  const { sink, chatter, at } = setup({}, 0.6);
  for (const speaker of ["a", "b", "c", "d", "e", "f"]) chatter.request(bark(speaker, 0));
  at(0);
  expect(speakers(sink.played)).toEqual(["a", "b", "c", "d"]);
  at(0.3);
  expect(sink.played).toHaveLength(4);
  at(0.6);
  expect(speakers(sink.played)).toEqual(["a", "b", "c", "d", "e", "f"]);
  expect(sink.played.slice(4).map((line) => line.at)).toEqual([0.6, 0.6]);
});

test("responses beat own barks, which beat others' barks, whatever the distance", () => {
  const { sink, chatter, at } = setup({ lines: 1 }, 0.4);
  chatter.request(bark("enemy", 0, { tier: "other_bark", position: [5, 0, 0] }));
  chatter.request(bark("own", 0, { position: [400, 0, 0] }));
  chatter.request(bark("answer", 0, { tier: "response", trigger: "move" }));
  at(0);
  at(0.4);
  at(0.8);
  expect(speakers(sink.played)).toEqual(["answer", "own", "enemy"]);
});

test("within a tier the request nearest the camera speaks first", () => {
  const { sink, chatter, at } = setup({ lines: 1 }, 0.4);
  chatter.request(bark("far", 0, { position: [50, 0, 0] }));
  chatter.request(bark("near", 0, { position: [10, 0, 0] }));
  at(0);
  expect(speakers(sink.played)).toEqual(["near"]);

  // The camera, not the world origin, decides.
  const moved = setup({ lines: 1 }, 0.4);
  moved.chatter.request(bark("west", 0, { position: [-50, 0, 0] }));
  moved.chatter.request(bark("east", 0, { position: [50, 0, 0] }));
  moved.at(0, [60, 0, 0]);
  expect(speakers(moved.sink.played)).toEqual(["east"]);
});

test("a request that can't start within a second of its moment is dropped", () => {
  const { sink, chatter, at } = setup({ lines: 1 }, 1.5);
  chatter.request(bark("first", 0));
  chatter.request(bark("second", 0));
  expect(at(0).dropped).toEqual([]);
  expect(at(1).dropped).toEqual([]);
  const late = at(1.5);
  expect(late.dropped.map((drop) => [drop.request.speaker, drop.reason])).toEqual([
    ["second", "late"],
  ]);
  expect(speakers(sink.played)).toEqual(["first"]);

  // Arriving already more than a second old is just as late.
  chatter.request(bark("stale", 0.2));
  expect(at(2).dropped.map((drop) => [drop.request.speaker, drop.reason])).toEqual([
    ["stale", "late"],
  ]);
});

test("a speaker never talks over itself; others speak meanwhile", () => {
  const { sink, chatter, at } = setup({}, 0.5);
  chatter.request(bark("tank", 0));
  at(0);
  chatter.request(bark("tank", 0.2, { trigger: "hit" }));
  chatter.request(bark("squad", 0.2));
  at(0.2);
  expect(speakers(sink.played)).toEqual(["tank", "squad"]);
  at(0.5);
  expect(sink.played.map((line) => [line.request.speaker, line.at])).toEqual([
    ["tank", 0],
    ["squad", 0.2],
    ["tank", 0.5],
  ]);
});

test("a group answers once: same group key within 250 ms collapses to the first", () => {
  const { sink, chatter, at } = setup({}, 0.5);
  const order = (speaker: string, t: number) =>
    bark(speaker, t, { tier: "response", trigger: "move", groupKey: "order-7" });
  chatter.request(order("clicked", 0));
  chatter.request(order("second", 0.1));
  chatter.request(order("third", 0.25));
  const first = at(0.25);
  expect(speakers(sink.played)).toEqual(["clicked"]);
  expect(first.dropped.map((drop) => [drop.request.speaker, drop.reason])).toEqual([
    ["second", "group"],
    ["third", "group"],
  ]);
  // Later than 250 ms after the first, the same key is a new answer.
  chatter.request(order("again", 0.3));
  at(0.6);
  expect(speakers(sink.played)).toEqual(["clicked", "again"]);
});

test("a line with no loaded take is dropped and frees its place at once", () => {
  const { sink, bank, chatter, at } = setup({ lines: 1 }, 0.5);
  for (const take of [0, 1])
    bank.missing
      .add(`us_tank_crew/rifle/under_fire/steady/${take}`)
      .add(`us_tank_crew/gunner/under_fire/steady/${take}`);
  chatter.request(bark("silent", 0));
  chatter.request(bark("heard", 0, { trigger: "hit", position: [100, 0, 0] }));
  const result = at(0);
  expect(result.dropped.map((drop) => [drop.request.speaker, drop.reason])).toEqual([
    ["silent", "missing"],
  ]);
  expect(speakers(sink.played)).toEqual(["heard"]);
});

test("responses come from the answering voice; a speaker always barks in the same voice", () => {
  const { sink, chatter, at } = setup({}, 0.1);
  const names = Array.from({ length: 12 }, (_, i) => `unit-${i}`);
  for (const [i, speaker] of names.entries()) {
    chatter.request(bark(speaker, i, { tier: "response", trigger: "move" }));
    chatter.request(bark(speaker, i + 0.5));
  }
  for (let i = 0; i < 12; i++) {
    at(i);
    at(i + 0.5);
  }
  const responses = sink.played.filter((line) => line.request.tier === "response");
  expect(new Set(responses.map((line) => line.voice))).toEqual(new Set(["lead"]));
  const barks = sink.played.filter((line) => line.request.tier === "own_bark");
  expect(new Set(barks.map((line) => line.voice))).toEqual(new Set(["rifle", "gunner"]));

  // The same speaker in another battle keeps its voice.
  const again = setup({}, 0.1, 99);
  for (const [i, speaker] of names.entries()) again.chatter.request(bark(speaker, i));
  for (let i = 0; i < 12; i++) again.at(i);
  expect(again.sink.played.map((line) => line.voice)).toEqual(barks.map((line) => line.voice));
});

/** The takes `speaker` says over `count` requests, with `others` talking in between. */
function takes(seed: number, count: number, others: boolean) {
  const { sink, chatter, at } = setup({}, 0.1, seed);
  for (let i = 0; i < count; i++) {
    chatter.request(bark("tank", i));
    if (others) chatter.request(bark(`other-${i}`, i));
    at(i);
  }
  return sink.played.filter((line) => line.request.speaker === "tank").map((line) => line.take);
}

test("take choice is deterministic for a seed and unaffected by other speakers", () => {
  const alone = takes(7, 24, false);
  expect(takes(7, 24, false)).toEqual(alone);
  expect(takes(7, 24, true)).toEqual(alone);
  // Both takes are heard; another seed tells a different sequence.
  expect(new Set(alone)).toEqual(new Set([0, 1]));
  expect(takes(8, 24, false)).not.toEqual(alone);
});

test("only a loaded take is chosen", () => {
  const { sink, bank, chatter, at } = setup({}, 0.1);
  for (const voice of ["rifle", "gunner"])
    bank.missing.add(`us_tank_crew/${voice}/under_fire/steady/0`);
  for (let i = 0; i < 10; i++) {
    chatter.request(bark("tank", i));
    at(i);
  }
  expect(new Set(sink.played.map((line) => line.take))).toEqual(new Set([1]));
});
