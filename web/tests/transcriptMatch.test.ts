// A take's transcript against its script: what the generator flags for review.
import { describe, expect, it } from "vitest";
import { matches } from "@packages/battle-audio/tools/generate/transcript";

describe("transcript match", () => {
  it("reads spoken numbers and ignores audio tags and punctuation", () => {
    const callsign = "Oscar 21";
    expect(
      matches("[shouting] Oscar 21, moving!", "Oscar two-one moving", { callsign }).flagged,
    ).toBe(false);
    expect(
      matches("Oscar 55, on the way.", "oscar fifty five on the way", { callsign: "Oscar 55" }),
    ).toMatchObject({ flagged: false, score: 1 });
  });

  it("reads a spelling-only difference as the same words", () => {
    const heard = (script: string, transcript: string) =>
      matches(script, transcript, { callsign: "Oscar 1" });
    for (const [script, transcript] of [
      ["Enemy in the treeline.", "Enemy in the tree line"],
      ["Armour, front.", "Armor front"],
      ["Reloading, stand by.", "Reloading. Standby"],
      ["[shouting] Bail out! Bail out, God damn it!", "Bail out. Bail out, goddammit"],
      ["Co-pilot wounded.", "Copilot wounded"],
      ["[shouting] Backblast clear!", "Back blast clear!"],
      ["Troops, four hundred metres.", "Troops, 400 meters"],
    ])
      expect(heard(script, transcript), transcript).toMatchObject({ flagged: false, score: 1 });
  });

  it("still flags a misread word or a dropped keyword once spelling is set aside", () => {
    const callsign = "Oscar 1";
    expect(matches("Rocket out.", "Rock it out", { callsign }).flagged).toBe(true);
    expect(matches("Armour, front.", "Armor hell", { callsign }).flagged).toBe(true);
    expect(
      matches("[shouting] Tank! Gunner, sabot!", "Tank, gun, sabot", { callsign }),
    ).toMatchObject({ flagged: true, missing: ["gunner"] });
    // Overlong is a flag however well the words match.
    expect(
      matches("Enemy in the treeline.", "Enemy in the tree line", {
        callsign,
        length: { durationS: 5, talkS: 3 },
      }),
    ).toMatchObject({ flagged: true, overlong: true });
  });

  it("flags a misheard keyword even when the rest of the line is right", () => {
    const result = matches("Gunner, sabot, tank!", "Gutter, sabot, tank!", { callsign: "Oscar 1" });
    expect(result).toMatchObject({ flagged: true, missing: ["gunner"] });
  });

  it("flags a take longer than its trigger's talk time", () => {
    const line = "Roger, moving out.";
    expect(matches(line, line, { callsign: "", length: { durationS: 2, talkS: 3 } }).flagged).toBe(
      false,
    );
    expect(matches(line, line, { callsign: "", length: { durationS: 4, talkS: 3 } })).toMatchObject(
      {
        flagged: true,
        overlong: true,
      },
    );
  });
});
