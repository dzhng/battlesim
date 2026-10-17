import { describe, expect, it } from "vitest";
import {
  BARK_TRIGGERS,
  RESPONSE_TRIGGERS,
  barkTrigger,
  lineText,
  validateVoiceCatalog,
} from "@packages/battle-audio/src/voices/voiceCatalog";
import voiceJson from "@fixtures/voices.json";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const roster = Object.assign(
  {},
  ...["eastern", "europe", "shared", "us"].map(
    (name) =>
      JSON.parse(
        readFileSync(resolve(process.cwd(), `../fixtures/units/roster/${name}.json`), "utf8"),
      ).units,
  ),
);

// The simulation's layout, as the encoder's codec test holds it
// (`crates/sim/tests/publication.rs`): its bark tags are the contract's names.
const { layout } = JSON.parse(
  readFileSync(resolve(process.cwd(), "../fixtures/parity/publication/codec-vectors.json"), "utf8"),
) as { layout: { shoutTriggers: string[] } };

describe("bark names", () => {
  it("are the simulation's barks, B1 to B17 in order", () => {
    expect(layout.shoutTriggers.map(barkTrigger)).toEqual(BARK_TRIGGERS);
  });

  it("are scripted, both intensities and both takes, by every role", () => {
    const catalog = validateVoiceCatalog(voiceJson, roster);
    for (const [name, role] of Object.entries(catalog.roles))
      for (const tag of layout.shoutTriggers) {
        const lines = role.lines[barkTrigger(tag)];
        expect(lines?.steady, `${name} ${tag}`).toHaveLength(2);
        expect(lines?.stressed, `${name} ${tag}`).toHaveLength(2);
      }
  });

  it("refuse a tag that names no bark or the wrong number", () => {
    expect(() => barkTrigger("b18_surrender")).toThrow();
    expect(() => barkTrigger("b4_under_fire")).toThrow();
    expect(() => barkTrigger("under_fire")).toThrow();
  });
});

describe("voice catalog", () => {
  it("exports the 32 approved triggers", () => {
    expect(RESPONSE_TRIGGERS).toHaveLength(15);
    expect(BARK_TRIGGERS).toHaveLength(17);
    expect(new Set([...RESPONSE_TRIGGERS, ...BARK_TRIGGERS]).size).toBe(32);
  });

  it("validates every role's four takes for every trigger", () => {
    const catalog = validateVoiceCatalog(voiceJson, roster);
    for (const role of Object.values(catalog.roles)) {
      for (const trigger of [...RESPONSE_TRIGGERS, ...BARK_TRIGGERS]) {
        expect(role.lines[trigger].steady).toHaveLength(2);
        expect(role.lines[trigger].stressed).toHaveLength(2);
      }
    }
  });

  it("expands callsigns only on selected and arrived responses", () => {
    const catalog = validateVoiceCatalog(voiceJson, roster);
    expect(lineText(catalog, "us_tank_crew", "lead", "selected", "steady", 0)).toContain(
      catalog.sets.us_tank_crew.callsign,
    );
    expect(lineText(catalog, "us_tank_crew", "lead", "arrived", "steady", 0)).toContain(
      catalog.sets.us_tank_crew.callsign,
    );
    expect(
      lineText(catalog, "us_tank_crew", "lead", "friendly_destroyed_nearby", "steady", 0),
    ).not.toContain(catalog.sets.us_tank_crew.callsign);
  });

  it("contains the four permanently saved voice ids", () => {
    const catalog = validateVoiceCatalog(voiceJson, roster);
    expect(
      catalog.sets.us_tank_crew.voices.some((voice) => voice.voice_id === "z0wdO1PMjmqh0usMe4hA"),
    ).toBe(true);
    expect(
      catalog.sets.ru_rifle_squad.voices.some((voice) => voice.voice_id === "Jqx4K9GHzgAqONoiSwhb"),
    ).toBe(true);
    expect(
      catalog.sets.us_recon_infantry.voices.some(
        (voice) => voice.voice_id === "HDZsRSSxHYdXkV0AKe32",
      ),
    ).toBe(true);
    expect(
      catalog.sets.uk_attack_helicopter_pilot.voices.some(
        (voice) => voice.voice_id === "DqHWHfkdp2tajWvTPtM7",
      ),
    ).toBe(true);
  });

  it("keeps set callsigns and saved voice IDs unique", () => {
    const catalog = validateVoiceCatalog(voiceJson, roster);
    expect(new Set(Object.values(catalog.sets).map((set) => set.callsign)).size).toBe(
      Object.keys(catalog.sets).length,
    );
    const ids = Object.values(catalog.sets).flatMap((set) =>
      set.voices.map((voice) => voice.voice_id).filter((id): id is string => id !== null),
    );
    expect(new Set(ids).size).toBe(ids.length);
  });

  it("uses female response voices for pilots and vehicle crews", () => {
    const catalog = validateVoiceCatalog(voiceJson, roster);
    const femaleResponseSets = Object.values(catalog.sets).filter((set) =>
      set.voices.some(
        (voice) =>
          voice.gender === "female" && (voice.speaks === "responses" || voice.speaks === "all"),
      ),
    );
    expect(femaleResponseSets.length).toBeGreaterThanOrEqual(5);
  });

  it("keeps sniper and three-person infantry teams to two voices", () => {
    const catalog = validateVoiceCatalog(voiceJson, roster);
    for (const set of Object.values(catalog.sets)) {
      if (["sniper_team", "atgm_team", "manpads_team"].includes(set.role))
        expect(set.voices).toHaveLength(2);
    }
  });

  it("resolves the FN-6 team to a Chinese MANPADS set", () => {
    const catalog = validateVoiceCatalog(voiceJson, roster);
    expect(catalog.assignments.eastern_air_defense_team_fn_6).toBe("cn_manpads_team");
    expect(catalog.sets.cn_manpads_team.country).toBe("cn");
  });

  it("does not resolve test or menu units", () => {
    const catalog = validateVoiceCatalog(voiceJson, {
      test_tank: { roles: ["mbt"], roster: { factions: ["us"] } },
      menu_us_tank: { roles: ["mbt"], roster: { factions: ["us"] } },
    });
    expect(catalog.assignments.test_tank).toBeUndefined();
    expect(catalog.assignments.menu_us_tank).toBeUndefined();
  });
});
