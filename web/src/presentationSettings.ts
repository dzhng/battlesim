import presentation from "../../fixtures/presentation.json";
import { check_presentation } from "./wasm/renderer_browser/renderer_browser.js";
import { loadPresentationModule } from "./presentationModule";
import type { LightPresentation } from "@packages/battle-renderer/src/light/sceneLight";
import type { MapBorderStyle } from "@packages/battle-renderer/src/playAreaOverlay";
import type { FogStyle } from "@packages/battle-renderer/src/fog";

await loadPresentationModule();
const admitted = JSON.parse(check_presentation(JSON.stringify(presentation))) as {
  fog_style: FogStyle;
  ally_xray: readonly [number, number, number, number];
  ruler_reach: readonly [number, number, number, number];
  light: LightPresentation;
  light_preset: string;
  light_presets: Record<string, LightPresentation>;
};
export const selectedFogStyle = admitted.fog_style;

/** Shared authored values, admitted once in Rust before any page reads them.
 * Display colours derive from the HUD while retaining overlay-owned alpha.
 * `light` is the resolved selected preset, so web and native stay in parity. */
export const presentationSettings = {
  ...presentation,
  light: admitted.light,
  map_border: presentation.map_border as unknown as MapBorderStyle,
};

/** The fixture's named lighting looks, including `day`. */
export const lightPresets = admitted.light_presets;

/** The look `presentationSettings.light` currently holds. */
export const lightPreset = admitted.light_preset;

/** A named lighting look from the fixture table. */
export function lightForPreset(name: string): LightPresentation {
  const light = admitted.light_presets[name];
  if (light === undefined) {
    throw new Error(`unknown light preset: ${name}`);
  }
  return light;
}

/** UI projection of the authored overlay, with shared colours resolved in Rust. */
export const overlaySettings = {
  ...presentation.overlay,
  xray: { ...presentation.overlay.xray, ally: admitted.ally_xray },
  ruler: { ...presentation.overlay.ruler, reach: admitted.ruler_reach },
};
