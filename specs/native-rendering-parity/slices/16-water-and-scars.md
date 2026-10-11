# 16 — Water surface and ground scars

**Contract:** native draws the browser's water surface and the side's learned scars (craters, scorch, tracks, trample), and leaves grass out of the places the browser does.

**Seam:**
- Port into `terrain/ground.gdshaderinc` and the water draw:
  - `waterSurface` and `waterNormal` from `packages/battle-renderer/src/frame/terrainMaterial.ts`, plus the world pass's water branch (`worldPass.ts`);
  - `groundScars` and `scarredSurface` with the scar texture (`frame/scarTexture.ts`), fed by `capture/learned_ground.gd`.
- Grass reads the same scars.
- The grass count gate's `scar-omission` allowance is deleted.

**Runnable artifact:**
- Plate pairs over tank tracks and craters on market-town.
- Water: neither reel map has any, so use a pair from a capture of a map with water. Run the capture scene against such a map, for example `river`, if its encounter suits the menu reel. Otherwise judge water by a TS-exported vector test of the surface formulas and say so.

**Focused gates:**
- The grass count gate holds at 10% in every cell with no scar allowance.
- `compare-screenshots`, then an unprimed `screenshot-critique`.
- Pass cost reported.

**Visual variable:** scars and water.

**Delegated:** the scar texture's upload path.
