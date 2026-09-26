// The frame's provisional light: spike 01's warm afternoon on the ported golden
// preset, with the aerial range pushed out to our 1.6 km map (the source tunes
// it for miniature metres). Battle-look slice 13 owns the look and moves these
// values into `presentation.light`; until then they live here, once.
import { CIVSIM_ENVIRONMENTS, type CivsimEnvironment } from "../light/environment";
import { battlePostGrade, type BattlePostGradeUniforms } from "../light/postParameters";
import { CSM_MAP_SIZE } from "../light/shadowPolicy";
import type { ShadowTuning } from "../shadowData";

const golden = CIVSIM_ENVIRONMENTS.golden;

export const FRAME_LIGHT: CivsimEnvironment = {
  ...golden,
  sunAzimuth: -0.35,
  sunElevation: 0.62,
  physical: {
    ...golden.physical,
    sunIntensity: 5.0,
    environmentIntensity: 0.45,
    aerial: {
      ...golden.physical.aerial,
      clearRadiusM: 300,
      rangeFogNearM: 500,
      rangeFogFarM: 5000,
      rangeFogStrength: 0.25,
      valleyMistDistanceStartM: 2000,
      valleyMistDistanceFullM: 5000,
    },
  },
};

export const FRAME_GRADE: BattlePostGradeUniforms = battlePostGrade("golden");

export const FRAME_SHADOWS: ShadowTuning = {
  mapSize: CSM_MAP_SIZE,
  normalBiasScale: 1,
  depthBiasScale: 1,
};
