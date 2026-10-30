// @vitest-environment node
// The score's decisions are Rust's (`crates/presentation/tests/music.rs`);
// this holds the page's carrier to them: each command reaches the player.
import { audioMix } from "@web/audioMix";
import { beforeAll, expect, test } from "vitest";
import { BattleMusic, type BattleMusicRules } from "@packages/battle-audio/src/music/battleMusic";
import type { VoiceRequest } from "@packages/battle-audio/src/voices/chatter";
import { loadPresentation } from "@web/battle/sim/presentation";

const RULES = audioMix.music as BattleMusicRules;
const TRACKS = { tension: ["t1"], combat: ["c1"] };

beforeAll(() => loadPresentation());

/** A player that records what the score asks of it. */
function recorder() {
  const calls: (string | number)[][] = [];
  return {
    calls,
    player: {
      play: (track: string, { fade = 0 } = {}) => void calls.push(["play", track, fade]),
      stop: (fade = 0) => void calls.push(["stop", fade]),
      duck: (depth: number, glide: number) => void calls.push(["duck", depth, glide]),
    },
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

test("the score opens on tension, an own fighting bark crossfades to combat, and lines dip it", () => {
  const { calls, player } = recorder();
  const music = new BattleMusic(RULES, TRACKS, player, 1);
  const fade = RULES.crossfade_s;
  music.update(0, [], false);
  music.update(1, [bark("other_bark", "under_fire"), bark("own_bark", "contact_infantry")], false);
  expect(calls).toEqual([["play", "t1", fade]]);
  music.update(2, [bark("own_bark", "under_fire")], true);
  music.update(3, [], false);
  expect(calls.slice(1)).toEqual([
    ["stop", fade],
    ["play", "c1", fade],
    ["duck", RULES.duck.depth, RULES.duck.attack_s],
    ["duck", 0, RULES.duck.release_s],
  ]);
});
