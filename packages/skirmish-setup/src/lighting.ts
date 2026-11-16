// Lobby lighting choices come from the authored presentation presets.
import presentation from "../../../fixtures/presentation.json";

export const LIGHTING_RANDOM = "random";
export function lightingPresetNames(): string[] {
  return [
    ...new Set([presentation.light_preset, ...Object.keys(presentation.light_presets)]),
  ].sort();
}
export function lightingChoices(): string[] {
  return [LIGHTING_RANDOM, ...lightingPresetNames()];
}
export function isKnownLighting(value: unknown): value is string {
  return typeof value === "string" && lightingChoices().includes(value);
}
