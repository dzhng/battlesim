/** Canonical static scenery selection metadata. */
export const STUMP = 6;
export interface FelledWorkStats {
  /** Cumulative startup envelope rows. */
  envelopeRowsTotal: number;
  /** Cumulative reconciled rows; excludes snapshot equality/replacement work. */
  snapshotRowsTotal: number;
  /** Standing/fallen membership tests in the last selection. */
  membershipChecks: number;
}

/** What the presentation says of the scenery it placed and walked
 *  (`scenery_stats`). */
export interface SceneryWalkStats {
  visitedBounds: number;
  felledWork: FelledWorkStats;
  /** Instances placed: the forest's, the tree lines' shrubs', the backdrop's. */
  placed: [number, number, number];
  /** Dressing cells the last walk drew, and whether more are wanted. */
  cells: number;
  pending: boolean;
  falling: boolean;
}

export const NO_SCENERY_WALK: SceneryWalkStats = {
  visitedBounds: 0,
  felledWork: { envelopeRowsTotal: 0, snapshotRowsTotal: 0, membershipChecks: 0 },
  placed: [0, 0, 0],
  cells: 0,
  pending: false,
  falling: false,
};
