// @vitest-environment node
import { expect, test } from "vitest";
import game from "@fixtures/game.json";
import { BattleMusic, type BattleMusicRules } from "@packages/battle-audio/src/music/battleMusic";
import type { VoiceRequest } from "@packages/battle-audio/src/voices/chatter";

const RULES = game.presentation.audio.music as BattleMusicRules;
const TRACKS = { tension: ["t1"], combat: ["c1"] };

/** A player that records what the score asks of it. */
function recorder() {
  const calls: (string | number)[][] = [];
  return {
    calls,
    player: {
      play: (track: string) => void calls.push(["play", track]),
      crossfadeTo: (track: string) => void calls.push(["crossfade", track]),
      duck: (depth: number) => void calls.push(["duck", depth]),
    } as never,
  };
}

const bark = (tier: VoiceRequest["tier"], trigger: VoiceRequest["trigger"]): VoiceRequest => ({
  tier,
  speaker: tier === "own_bark" ? "own/1" : "seen/1",
  set: "s",
  trigger,
  intensity: "stressed",
  position: [0, 0, 0],
  at: 0,
});

test("a battle opens on tension, and only the side's own fighting barks bring combat", () => {
  const { calls, player } = recorder();
  const music = new BattleMusic(RULES, TRACKS, player, () => 0);
  music.update(0, [], false);
  expect(calls).toEqual([["play", "t1"]]);
  // An enemy under fire, and an own unit's contact call, leave the mood alone.
  music.update(1, [bark("other_bark", "under_fire"), bark("other_bark", "man_down")], false);
  music.update(2, [bark("own_bark", "contact_infantry"), bark("response", "move")], false);
  expect(calls).toEqual([["play", "t1"]]);
  for (const trigger of RULES.combat_triggers) {
    const { calls, player } = recorder();
    const music = new BattleMusic(RULES, TRACKS, player, () => 0);
    music.update(0, [], false);
    music.update(3, [bark("own_bark", trigger)], false);
    expect(calls, trigger).toEqual([
      ["play", "t1"],
      ["crossfade", "c1"],
    ]);
  }
});

test("the music dips while a voice line sounds and recovers after it", () => {
  const { calls, player } = recorder();
  const music = new BattleMusic(RULES, TRACKS, player, () => 0);
  music.update(0, [], false);
  music.update(0.1, [], true);
  music.update(0.2, [], true);
  music.update(1.5, [], false);
  expect(calls.filter(([what]) => what === "duck")).toEqual([
    ["duck", RULES.duck.depth],
    ["duck", 0],
  ]);
});
