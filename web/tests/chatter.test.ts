// @vitest-environment node
// Voice lines at the seam to Rust, where their rules live
// (`crates/presentation/tests/voices.rs` holds them to the vectors the
// TypeScript arbitration wrote): a loaded take and a request in, a start
// command on the voices bus out; a battle's lines heard from where its sound
// frame last heard, never the world origin.
import { presentationSettings } from "@web/presentationSettings";
import { audioMix } from "@web/audioMix";
import { beforeAll, expect, test } from "vitest";
import { loadPresentation } from "@web/battle/sim/presentation";
import type { AudioPresentation } from "@packages/battle-audio/src/audioPresentation";
import { gameSounds } from "@packages/battle-audio/src/shippedSounds";
import { SoundFrame, type VoiceSink } from "@packages/battle-audio/src/soundFrame";
import { Chatter, type VoiceRequest } from "@packages/battle-audio/src/voices/chatter";

beforeAll(() => loadPresentation());

const RULES = { lines: 1, wait_s: 1, group_s: 0.25, gain: 0.5 };

/** A chatter that knows one set: a leader who answers and a soldier who shouts, every take 0.5 s. */
function crew() {
  const chatter = new Chatter(RULES, 1);
  chatter.voices("tank", [
    { id: "lead", speaks: "responses" },
    { id: "rifle", speaks: "barks" },
  ]);
  for (const voice of ["lead", "rifle"])
    for (const trigger of ["move", "under_fire"] as const)
      chatter.loaded("tank", voice, trigger, "steady", 0, 0.5);
  return chatter;
}

const bark = (speaker: string, position: number[]): VoiceRequest => ({
  tier: "own_bark",
  speaker,
  set: "tank",
  trigger: "under_fire",
  intensity: "steady",
  position,
  at: 0,
});

test("a line started is a start command for its take on the voices bus", () => {
  const chatter = crew();
  chatter.request(
    {
      tier: "response",
      speaker: "own/1",
      set: "tank",
      trigger: "move",
      intensity: "steady",
      at: 0,
    },
    0,
  );
  const { commands, started, dropped } = chatter.update(0.1, [0, 0, 0]);
  expect(dropped).toEqual([]);
  expect(commands).toMatchObject([
    { op: "start", sound: "tank/lead/move/steady", variant: 0, bus: "voices", at: 0.1 },
  ]);
  expect(started).toMatchObject([{ id: commands[0].id, voice: "lead", take: 0, duration: 0.5 }]);
});

test("a battle's lines are heard from its sound frame's listener", () => {
  const sink: VoiceSink = {
    now: () => 0,
    duration: () => 0.5,
    start() {},
    set() {},
    stop() {},
    listen() {},
    duck() {},
  };
  const frame = new SoundFrame(
    {
      tickHz: 30,
      presentation: audioMix as unknown as AudioPresentation,
      catalog: gameSounds,
      smokeTimes: presentationSettings.effects.smoke,
      cookOff: presentationSettings.effects.cook_off,
    },
    sink,
  );
  const chatter = crew();
  chatter.request(bark("west", [-50, 0, 0]), 0);
  chatter.request(bark("east", [50, 0, 0]), 0);
  // The camera stands east: the east bark is nearer.
  frame.update(0, 0, { vehicles: [], soldiers: [] }, { position: [60, 0, 0], forward: [0, 1, 0] });
  const { started } = frame.hearLines(chatter);
  expect(started.map((line) => line.request.speaker)).toEqual(["east"]);
});
