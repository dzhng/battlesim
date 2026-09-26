// The battle's light, from the one fixture owner's presentation block. Every
// lab route draws under it, as every lab reuses the village's numbers.
import village from "@fixtures/village.json";
import {
  validateLight,
  type LightPresentation,
} from "@packages/battle-renderer/src/light/sceneLight";

export const villageLight: LightPresentation = validateLight(
  village.presentation.light as unknown as LightPresentation,
);
