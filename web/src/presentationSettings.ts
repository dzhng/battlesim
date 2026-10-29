import presentation from "../../fixtures/presentation.json";
import {
  validateLight,
  type LightPresentation,
} from "@packages/battle-renderer/src/light/sceneLight";
import {
  validateMapBorder,
  type MapBorderStyle,
} from "@packages/battle-renderer/src/playAreaOverlay";

/** Shared visual settings. Section readers admit their own shapes; Rust
 * admits the ground, fog, pose and effect records when they are installed. */
export const presentationSettings = {
  ...presentation,
  light: validateLight(presentation.light as unknown as LightPresentation),
  map_border: validateMapBorder(presentation.map_border as unknown as MapBorderStyle),
};
