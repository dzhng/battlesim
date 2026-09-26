// The battle's biome, from its one owner, `fixtures/biomes/summer.json`.
// Every lab route draws its ground under it, as every lab reuses the
// village's look.
import summer from "@fixtures/biomes/summer.json";
import { validateBiome, type Biome } from "@packages/battle-renderer/src/terrain/biome";

export const villageBiome: Biome = validateBiome(summer as unknown as Biome, "summer");
