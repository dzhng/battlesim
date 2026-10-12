// A scripted 30 s firefight, heard from a camera at the origin: an own tank
// platoon takes fire, a recon squad is pinned, a seen enemy squad shouts, a
// helicopter joins, and the player keeps ordering groups. It is busier than a
// quiet minute on purpose, so the line cap, the late drop, self-overlap and
// one-unit-answers all happen within it.
import type { VoiceRequest } from "../../../../packages/battle-audio/src/voices/chatter";

/** One line a unit asks to say, `t` seconds into the preview. */
export interface ScriptLine extends Omit<VoiceRequest, "at"> {
  t: number;
}

/** Where the camera listens from; distances rank lines within a tier. */
export const LISTENER = [0, 0, 0] as const;
export const SCRIPT_S = 30;

const TANK = "us_tank_crew";
const RECON = "us_recon_infantry";
const HELI = "uk_attack_helicopter_pilot";
const ENEMY = "ru_rifle_squad";

const stands: Record<string, readonly number[]> = {
  "Abrams 1": [60, 20, 0],
  "Abrams 2": [85, -15, 0],
  "Abrams 3": [120, 35, 0],
  "Recon 1": [160, -70, 0],
  "Apache 1": [-40, 90, 60],
  "Enemy squad": [330, 40, 0],
};
const set: Record<string, string> = {
  "Abrams 1": TANK,
  "Abrams 2": TANK,
  "Abrams 3": TANK,
  "Recon 1": RECON,
  "Apache 1": HELI,
  "Enemy squad": ENEMY,
};

/** A line of `speaker`'s, placed where it stands. */
const line = (
  t: number,
  speaker: string,
  tier: VoiceRequest["tier"],
  trigger: VoiceRequest["trigger"],
  intensity: VoiceRequest["intensity"] = "steady",
  groupKey?: string,
): ScriptLine => ({
  t,
  tier,
  speaker,
  set: set[speaker],
  trigger,
  intensity,
  // An owner's answer is heard over the radio, wherever the camera is.
  ...(tier === "response" ? {} : { position: stands[speaker] }),
  ...(groupKey === undefined ? {} : { groupKey }),
});
/** The player orders a group: the clicked unit asks first, the rest just after. */
const order = (
  t: number,
  key: string,
  trigger: VoiceRequest["trigger"],
  units: readonly string[],
) => units.map((unit, i) => line(t + i * 0.05, unit, "response", trigger, "steady", key));

export const CHATTER_SCRIPT: readonly ScriptLine[] = [
  line(0.2, "Abrams 1", "response", "selected"),
  ...order(0.9, "order-1", "move", ["Abrams 1", "Abrams 2", "Abrams 3"]),
  line(3.0, "Enemy squad", "other_bark", "contact_vehicle"),
  line(3.3, "Abrams 1", "own_bark", "contact_infantry"),
  line(4.1, "Abrams 1", "own_bark", "opens_fire"),
  line(4.2, "Enemy squad", "other_bark", "opens_fire"),
  line(5.0, "Abrams 1", "own_bark", "under_fire", "stressed"),
  line(5.1, "Abrams 2", "own_bark", "under_fire"),
  line(6.0, "Abrams 1", "own_bark", "hit", "stressed"),
  line(7.0, "Recon 1", "own_bark", "suppressed"),
  line(8.4, "Recon 1", "own_bark", "pinned", "stressed"),
  line(9.0, "Apache 1", "response", "selected"),
  ...order(9.6, "order-2", "attack", ["Apache 1", "Abrams 2"]),
  // Everything at once: more than four lines want to start.
  line(11.0, "Abrams 1", "own_bark", "badly_damaged", "stressed"),
  line(11.0, "Abrams 2", "own_bark", "kill"),
  line(11.05, "Abrams 3", "own_bark", "reloading"),
  line(11.1, "Recon 1", "own_bark", "man_down", "stressed"),
  line(11.1, "Apache 1", "own_bark", "missile_away"),
  line(11.15, "Enemy squad", "other_bark", "under_fire", "stressed"),
  line(11.2, "Enemy squad", "other_bark", "hit", "stressed"),
  ...order(14.0, "order-3", "move", ["Recon 1", "Abrams 1"]),
  line(15.2, "Abrams 3", "own_bark", "friendly_destroyed_nearby", "stressed"),
  line(17.5, "Enemy squad", "other_bark", "pinned", "stressed"),
  line(19.5, "Apache 1", "own_bark", "incoming_missile", "stressed"),
  line(20.0, "Apache 1", "response", "move"),
  line(21.0, "Abrams 3", "response", "move"),
  line(22.5, "Enemy squad", "other_bark", "contact_air"),
  line(23.5, "Recon 1", "response", "arrived"),
  line(25.0, "Abrams 2", "own_bark", "out_of_ammo", "stressed"),
  line(26.5, "Abrams 3", "response", "selected"),
  ...order(27.0, "order-4", "move", ["Abrams 3", "Abrams 1", "Abrams 2"]),
  line(28.5, "Apache 1", "own_bark", "kill"),
];

/** Every speaker in the script, in first-appearance order: the lane view's rows. */
export const SCRIPT_SPEAKERS = [...new Set(CHATTER_SCRIPT.map((entry) => entry.speaker))];
/** Every set the script speaks with: the packs the preview loads. */
export const SCRIPT_SETS = [...new Set(CHATTER_SCRIPT.map((entry) => entry.set))];
