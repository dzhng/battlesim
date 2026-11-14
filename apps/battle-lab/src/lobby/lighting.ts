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

/**
 * Pick a candidate sun azimuth with the map seed: the values sorted
 * numerically, then `mapSeed % n`, matching `presentation::light::pick_sun_azimuth`.
 * Presentation only — nothing here enters the sim or its hashes. The side
 * is derivable from the seed and the fixture list, so `ReplayFile.light_preset`
 * does not need to store it.
 */
export function pickSunAzimuth(azimuths: readonly number[], mapSeed: string): number {
  if (azimuths.length === 0) throw new Error("no sun azimuths");
  const sorted = [...azimuths].sort((a, b) => a - b);
  const index = Number(BigInt(mapSeed) % BigInt(sorted.length));
  const picked = sorted[index];
  if (picked === undefined) throw new Error("no sun azimuths");
  return picked;
}

/** Apply a seed-picked azimuth when the look lists candidates; day is a no-op. */
export function lightWithMapSeed<
  T extends { sun_azimuth: number; sun_azimuths?: readonly number[] },
>(light: T, mapSeed: string): T {
  if (light.sun_azimuths === undefined || light.sun_azimuths.length === 0) return light;
  return { ...light, sun_azimuth: pickSunAzimuth(light.sun_azimuths, mapSeed) };
}
