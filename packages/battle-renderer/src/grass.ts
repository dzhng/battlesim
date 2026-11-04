/** The grass diagnostics returned by the shared runtime. */
import type { Biome } from "./terrain/terrainSurface";
/** One clump as the probe reads it back. */
export interface GrassClumpRow {
  root: [number, number, number];
  height: number;
  kind: string;
  /** Its colour, linear rgb: the ground's under it, dried where it is dry. */
  colour: [number, number, number];
  tier: 0 | 1;
  /** How far tracks or trampling laid it over, 0 upright to 1 flat. */
  laid: number;
}
export interface GrassCounts {
  /** Clumps drawn per tier, and what the build found before capacity. */
  near: number;
  far: number;
  nearFound: number;
  farFound: number;
}

export interface GrassProbes {
  suppress(on: boolean): void;
  retune(rules: Biome["grass"] | null): void;
  rules(): Biome["grass"] | null;
  counts(): Promise<GrassCounts>;
  clumps(): Promise<GrassClumpRow[]>;
}
