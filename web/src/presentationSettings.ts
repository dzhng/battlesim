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
};
export const selectedFogStyle = admitted.fog_style;

/** Shared authored values, admitted once in Rust before any page reads them.
 * Display colours derive from the HUD while retaining overlay-owned alpha. */
export const presentationSettings = {
  ...presentation,
  light: presentation.light as unknown as LightPresentation,
  map_border: presentation.map_border as unknown as MapBorderStyle,
};

/** UI projection of the authored overlay, with shared colours resolved in Rust. */
export const overlaySettings = {
  ...presentation.overlay,
  xray: { ...presentation.overlay.xray, ally: admitted.ally_xray },
  ruler: { ...presentation.overlay.ruler, reach: admitted.ruler_reach },
};
