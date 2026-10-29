// Small authored test looks: reports stay synthetic unless a test selects a recording.
import { presentationSettings } from "@web/presentationSettings";
import { shippedRules } from "@web/shippedRules";
import type { NearFar } from "@packages/battle-audio/src/audioMix";

type Choice = Pick<NearFar, "near" | "far" | "gain">;
export function testWeaponLooks(
  reports: Record<string, Choice> = {},
  contacts: Record<string, Record<string, string>> = {},
) {
  const library = structuredClone(presentationSettings.weapon_looks) as unknown as Record<
    string,
    { sound: { shot: NearFar; impact_scale: number; impacts: Record<string, string> } }
  >;
  const weapons: Record<string, object> = { ...shippedRules.weapons };
  const bases: Record<string, string> = {
    rifle: "rifle",
    carbine: "rifle",
    hmg: "hmg",
    tank_ap: "cannon",
    tank_he: "cannon",
    atgm: "launch",
  };
  for (const [kind, baseline] of Object.entries(bases)) {
    const source = shippedRules.weapons[kind as keyof typeof shippedRules.weapons].appearance;
    const look = library[source];
    look.sound.shot.near = baseline;
    look.sound.shot.far = baseline === "launch" ? baseline : `${baseline}_far`;
    look.sound.impacts = {};
  }
  for (const kind of new Set([
    ...Object.keys(reports),
    ...Object.values(contacts).flatMap(Object.keys),
  ])) {
    const row = shippedRules.weapons[kind as keyof typeof shippedRules.weapons];
    const name = `test_${kind}`;
    const look = structuredClone(library[row?.appearance ?? "rifle"]);
    if (reports[kind]) Object.assign(look.sound.shot, reports[kind]);
    for (const [surface, choices] of Object.entries(contacts))
      if (choices[kind])
        look.sound.impacts[surface as keyof typeof look.sound.impacts] = choices[kind];
    library[name] = look;
    weapons[kind] = { extends: row ? kind : "rifle", name: "Test gun", appearance: name };
    // A shipped row is already complete; replacing only its appearance avoids self inheritance.
    if (row) weapons[kind] = { ...row, appearance: name };
  }
  return { looks: { ...presentationSettings, weapon_looks: library }, weapons };
}
