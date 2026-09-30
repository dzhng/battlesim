// The public sampled surface. Absent vertex pages are exactly flat at zero;
// local samples keep the simulation's south-west → north-east triangle rule.
export interface TerrainGrid {
  nx: number;
  ny: number;
  spacing: number;
  pageSize: number;
  minHeight: number;
  /** Sorted IDs in the ceil(nx/pageSize)-wide vertex-page directory. */
  pageIds: Uint32Array;
  /** pageSize² row-major heights per present page. */
  heights: Float32Array;
}

export function groundSample(grid: TerrainGrid, i: number, j: number): number {
  const size = grid.pageSize;
  const id = Math.floor(j / size) * Math.ceil(grid.nx / size) + Math.floor(i / size);
  let lo = 0,
    hi = grid.pageIds.length;
  while (lo < hi) {
    const mid = (lo + hi) >>> 1;
    if (grid.pageIds[mid] < id) lo = mid + 1;
    else hi = mid;
  }
  if (grid.pageIds[lo] !== id) return 0;
  return grid.heights[lo * size * size + (j % size) * size + (i % size)];
}

/** Ground height at (x, y), clamped to the grid's edge. */
export function groundHeight(grid: TerrainGrid, x: number, y: number): number {
  const { nx, ny, spacing } = grid;
  const fx = Math.min(Math.max(x / spacing, 0), nx - 1);
  const fy = Math.min(Math.max(y / spacing, 0), ny - 1);
  const i = Math.min(Math.floor(fx), nx - 2);
  const j = Math.min(Math.floor(fy), ny - 2);
  const u = fx - i;
  const v = fy - j;
  const h00 = groundSample(grid, i, j);
  const h11 = groundSample(grid, i + 1, j + 1);
  if (u >= v) {
    const h10 = groundSample(grid, i + 1, j);
    return h00 + u * (h10 - h00) + v * (h11 - h10);
  }
  const h01 = groundSample(grid, i, j + 1);
  return h00 + v * (h01 - h00) + u * (h11 - h01);
}
