// Adapted from ~/dev/game game-renderer/src/environment/postParameters.ts
// (reuse manifest): renderer-independent battle post policy. Local change: the
// URL-parameter overrides are dropped; slice 13 reads `presentation.light.grade`.
import type { CivsimEnvironmentId } from "./environment";

/** Physically-restrained bloom. Threshold is LINEAR-HDR luminance (pre-tone-map,
 *  because the scene passes render with NoToneMapping): 1.02 sits just above a
 *  fully sunlit diffuse surface, so only super-white specular/emissive — the
 *  sky sun disc and the disciplined GGX sea glint — spills. Strength/radius and
 *  highpass softness lean toward the pen's five-level glow without turning
 *  bright grass into bloom. */
export const BLOOM_STRENGTH = 0.085;
export const BLOOM_RADIUS = 0.56;
export const BLOOM_THRESHOLD = 1.02;
export const BLOOM_SMOOTH_WIDTH = 0.75;
export const BLOOM_LEVELS = 5;

/** Pen print-grade constants applied as post-chain policy rather than material
 *  albedo. Shadow tint follows the #5C6E9E violet family; highlight tint follows
 *  the pen's warm cream push. */
// This saturation keeps close-crop grass at the hero band low end without
// losing its green undertone.
export const GRADE_SATURATION_BOOST = 1.15;
export const GRADE_CONTRAST = 0.16;
export const GRADE_SPLIT_TONE = 0.85;
export const GRADE_SHADOW_LIFT = 1.0;
export const GRADE_LUMA = [0.2126, 0.7152, 0.0722] as const;
export const GRADE_SHADOW_TINT = [0.9, 0.95, 1.16] as const;
export const GRADE_HIGHLIGHT_TINT = [1.055, 1.012, 0.925] as const;
export const GRADE_LIFT = [0.017, 0.021, 0.036] as const;

const PRESET_GRADE_STRENGTH: Record<CivsimEnvironmentId, number> = {
  golden: 1.0,
  dusk: 0.45,
  noon: 0.3,
  "overcast-highland": 0.06,
};

export interface BattlePostGradeUniforms {
  strength: number;
  saturationBoost: number;
  contrast: number;
  splitTone: number;
  shadowLift: number;
}

export function gradeStrengthForPreset(environmentId: CivsimEnvironmentId): number {
  return PRESET_GRADE_STRENGTH[environmentId];
}

function finiteClamped(value: number, minValue: number, maxValue: number): number {
  if (!Number.isFinite(value)) return minValue;
  return Math.min(maxValue, Math.max(minValue, value));
}

export function clampBattlePostGrade(
  uniforms: Partial<BattlePostGradeUniforms>,
): Partial<BattlePostGradeUniforms> {
  const out: Partial<BattlePostGradeUniforms> = {};
  if (uniforms.strength !== undefined) out.strength = finiteClamped(uniforms.strength, 0, 1.5);
  if (uniforms.saturationBoost !== undefined)
    out.saturationBoost = finiteClamped(uniforms.saturationBoost, 0, 4);
  if (uniforms.contrast !== undefined) out.contrast = finiteClamped(uniforms.contrast, 0, 0.6);
  if (uniforms.splitTone !== undefined) out.splitTone = finiteClamped(uniforms.splitTone, 0, 1.5);
  if (uniforms.shadowLift !== undefined)
    out.shadowLift = finiteClamped(uniforms.shadowLift, 0, 1.5);
  return out;
}
export function battlePostGrade(
  environmentId: CivsimEnvironmentId,
  overrides: Partial<BattlePostGradeUniforms> = {},
): BattlePostGradeUniforms {
  return {
    strength: gradeStrengthForPreset(environmentId),
    saturationBoost: GRADE_SATURATION_BOOST,
    contrast: GRADE_CONTRAST,
    splitTone: GRADE_SPLIT_TONE,
    shadowLift: GRADE_SHADOW_LIFT,
    ...clampBattlePostGrade(overrides),
  };
}
