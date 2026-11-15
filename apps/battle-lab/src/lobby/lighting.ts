// Match lighting: a lobby choice of Random or a fixture look, resolved
// to one named preset so every client and a later replay draw the same sky.
// Presentation only — nothing here enters the sim or its hashes.
import { light_sun_azimuth, map_seed_index } from "@wasm/renderer_browser";
import { lightPresetLabels, lightPresets } from "@web/presentationSettings";
import { DEFAULT_LIGHTING } from "@packages/skirmish-setup/src/lobby";

/** The lobby's Random choice, not a fixture look. */
export const LIGHTING_RANDOM = "random";

/**
 * Default match lighting: Daylight (`day`), the shipped mid-morning look.
 * Golden hour and Random stay one click away; a new match should not
 * silently change every existing battle's light.
 */
export { DEFAULT_LIGHTING };

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
  return names[map_seed_index(mapSeed, names.length)];
}

/**
 * A look's light for the map seed: one of its `sun_azimuths`, picked by
 * `presentation::light`'s rule (the native client's too), else as authored.
 * Presentation only; nothing here enters the sim or its hashes.
 */
export function lightWithMapSeed<
  T extends { sun_azimuth: number; sun_azimuths?: readonly number[] },
>(light: T, mapSeed: string): T {
  if (!light.sun_azimuths?.length) return light;
  return { ...light, sun_azimuth: light_sun_azimuth(JSON.stringify(light), mapSeed) };
}
