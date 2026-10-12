// The battle's biome, from its one owner, `fixtures/biomes/summer.json`.
// Every lab route draws its ground under it, as every lab reuses the
// game's look; the ground admits it in Rust (`ground::biome`).
import summer from "@fixtures/biomes/summer.json";
import type { Biome } from "@packages/battle-renderer/src/terrain/terrainSurface";

export const gameBiome: Biome = summer;
