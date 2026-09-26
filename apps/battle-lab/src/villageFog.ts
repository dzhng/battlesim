// The sight lights' resolution and budgets, from the one fixture owner's
// presentation block. Every lab route draws fog with them.
import village from "@fixtures/village.json";
import {
  validateFogGeometry,
  type FogGeometryPresentation,
} from "@packages/battle-renderer/src/frame/fogInputs";

export const villageFogGeometry: FogGeometryPresentation = validateFogGeometry(
  village.presentation.fog_geometry as unknown as FogGeometryPresentation,
);
