// The grass pass's tuning: every number that sets how the field grows and
// moves and is not the biome's (`biome.grass`, read per map). The browser
// shader (`frame/grassPass.ts`) splices these into its WGSL as constants;
// anything that ports the pass's formulas reads the same table.
//
// Grouped by the formula that reads them. What stays in the shader: maths
// (π, R2's generator, identities), hash multipliers and the seeds that
// decorrelate one hash or noise read from another, and culling bounds.

export const grassTuning = {
  /** Where a clump stands (the build's sequence walk and row snap). */
  placement: {
    /** How far a clump strays from its sequence point, either way, metres. */
    jitterM: 0.4,
    /** A crop leaves its rows this near its plot's verge and any bare
     *  margin: the headland, where the drill turned. */
    headlandM: 0.3,
  },

  /** How many clumps a square metre holds (`grassDensity`). */
  density: {
    /** Ground seen this flat (the eye's drop over its distance) or flatter
     *  holds as many clumps as at this slant. */
    leastFacing: 0.15,
  },

  /** Whether a clump grows, and how tall (the build). */
  stand: {
    /** No clump grows where the roads have worn the ground more than this
     *  (`groundShoulder`): a road's own surface, and the foot of its shoulder. */
    bareWear: 0.9,
    /** The sparse patches are where their noise is under these (eased). */
    sparseCut: [0.3, 0.55],
    /** At a bare margin's edge the stand is this share as dense and this
     *  share as tall as `GRASS_EDGE_M` in from it. */
    edgeDensity: 0.5,
    edgeHeight: 0.35,
    /** A clump grows in from this share of its rank's threshold, so it
     *  grows as the density passes it and shrinks away as the fade takes it. */
    growFrom: 0.7,
    /** A clump shorter than this, in metres, is not grown. */
    leastHeightM: 0.02,
    /** Dry patches are where their noise passes these (eased). */
    parchedCut: [0.55, 0.8],
  },

  /** Which of a ground's grasses a clump is (the build's mix). */
  mix: {
    /** A drifting grass's drifts are where their noise passes these (eased). */
    driftCut: [0.4, 0.65],
    /** A grass that keeps wholly to its drifts is this many times its share
     *  inside one, and absent outside. */
    driftGather: 3,
  },

  /** A clump's tier: its pixel height against the near tier's threshold,
   *  scaled per clump between these, so the tiers meet raggedly. */
  tierJitter: [0.85, 1.15],

  /** A clump laid flat (tracks, trampling) leans this far over, as a
   *  fraction of its height, and sinks by this much of it. */
  laid: {
    lean: 1.1,
    sink: 0.6,
  },

  /** The one wind (the blade vertex). */
  wind: {
    /** A gust front leans the grass from where its wave passes this. */
    gustFrom: 0.2,
    /** The flutter's phase turns this many radians a metre downwind, and
     *  this share of it sways across the wind. */
    flutterPhase: 0.35,
    flutterAcross: 0.5,
  },

  /** A blade's surface. */
  blade: {
    roughness: 0.9,
  },
} as const;
