// Match lighting: a lobby choice of Random or a fixture look, resolved
// to one named preset so every client and a later replay draw the same sky.
// Presentation only — nothing here enters the sim or its hashes.
import { lightPreset, lightPresetLabels, lightPresets } from "@web/presentationSettings";

/** The lobby's Random choice, not a fixture look. */
export const LIGHTING_RANDOM = "random";

/**
 * Default match lighting: Daylight (`day`), the shipped mid-morning look.
 * Golden hour and Random stay one click away; a new match should not
 * silently change every existing battle's light.
 */
export const DEFAULT_LIGHTING = lightPreset;

/** Fixture lighting looks in a stable order, including `day`. */
export function lightingPresetNames(): string[] {
  return Object.keys(lightPresets).sort();
}

/** Lobby Lighting options: Random, then every fixture look. */
export function lightingChoices(): string[] {
  return [LIGHTING_RANDOM, ...lightingPresetNames()];
}

/** A look's display name from `presentation.light_preset_labels`. */
export function lightingLabel(name: string): string {
  if (name === LIGHTING_RANDOM) return "Random";
  const label = lightPresetLabels[name];
  if (label === undefined || label.trim() === "") {
    throw new Error(`missing light preset label: ${name}`);
  }
  return label;
}

/** Read a `light=` query value, or why it is not a lobby lighting choice. */
export function parseLighting(value: string | null | undefined): string | { error: string } {
  if (value === null || value === undefined || value === "") return DEFAULT_LIGHTING;
  if (value === LIGHTING_RANDOM) return LIGHTING_RANDOM;
  if (lightingPresetNames().includes(value)) return value;
  return {
    error: `light must be random or one of ${lightingPresetNames().join(", ")}`,
  };
}

/**
 * Resolve a lobby lighting choice to a fixture preset. Random is
 * `mapSeed % n` over the sorted preset names, so every client agrees.
 * Store this resolved name on the replay: adding a look later must not
 * restage old matches.
 */
export function resolveLighting(choice: string, mapSeed: string): string {
  const names = lightingPresetNames();
  if (choice !== LIGHTING_RANDOM) {
    if (!names.includes(choice)) throw new Error(`unknown lighting: ${choice}`);
    return choice;
  }
  const index = Number(BigInt(mapSeed) % BigInt(names.length));
  const resolved = names[index];
  if (resolved === undefined) throw new Error("no lighting presets");
  return resolved;
}
