// The terrain material's tuning: every number that sets the ground's look
// and is not the biome's (`biome.ts`, read per map). The browser shader
// (`frame/terrainMaterial.ts`) splices these into its WGSL as constants;
// anything that ports the material's formulas reads the same table.
//
// Grouped by the formula that reads them. What stays in the shader: maths
// (π, a mix's 0.5, identities), the Rec. 709 luminance weights, hash
// multipliers, the offsets that decorrelate one noise read from another, and
// the fixed turns of a noise lattice. Those are the formula, not its look.

export const terrainTuning = {
  /** Past the patchwork, the distant colour (`groundColour`). */
  distant: {
    /** The patchwork fades to the distant colour over this far inside its
     *  region's edge, metres. */
    fadeM: 600,
    /** Plots give way to the distant colour as the smallest plot's side
     *  shrinks from 6 to 2 pixels (as fractions of it, the pixel footprint). */
    plotPixels: [1 / 6, 1 / 2],
  },

  /** The painterly variation of the ground (`mottle`, `groundTint`). */
  mottle: {
    /** The dry patches' weight (times each plot kind's `mottle`): they shift
     *  hue only. */
    patchHue: 0.6,
    /** The fine noise's weights: brightness, and hue where it is high. The
     *  brightness is for what is not a plot (verge, forest floor, shore, road). */
    fineValue: 0.45,
    fineHue: 0.4,
    /** The fine noise fades as a pixel grows from the first to the second of
     *  these shares of its scale. */
    fineFade: [0.25, 1],
    /** A dry patch covers the share of its plot where its noise passes this. */
    patchCut: 0.58,
    /** A patch's edge is never sharper than this many metres, nor than a
     *  pixel; its slope is read over this step of the noise's lattice. */
    patchEdgeM: 0.3,
    slopeStep: 0.05,
    /** How a hue shift scales linear rgb before its luminance is restored:
     *  toward ochre, as drier grass or crop. Never toward blue, as a shadow
     *  under the sky is. */
    dry: [0.7, 0, -0.9],
  },

  /** A plot's grain (`fieldTexture`). */
  grain: {
    /** Four octaves of noise, the coarsest first: each this many times finer
     *  than the last and weighted so. The third is the biome's `grain_m`, a
     *  lump; the last is the lump's own surface, which only the near ground
     *  shows (without it a field seen from 25 m is soft ripples). */
    octave: 2.7,
    weights: [0.15, 0.25, 0.35, 0.25],
    /** An octave fades to its mean as a pixel grows from the first of these
     *  shares of its size to the second. */
    fade: [0.3, 0.9],
    /** How hard the noise is driven into its limits: past 1 a clod or a
     *  tussock has an edge, where plain noise is a soft blob. */
    crisp: 2,
    /** Each octave lies on a lattice of its own, turned this far (radians)
     *  from the plot's rows, and is pushed about by the octave before it, by
     *  this share of its own cell: value noise on one lattice, driven into
     *  its limits, comes out as squares. */
    turns: [0.55, 1.9, 2.9, 4.3],
    warp: 0.45,
  },

  /** A plot's rows (`fieldTexture`). */
  rows: {
    /** A row breaks along its length over this many grains. */
    breakGrains: 2,
    /** A row is the cube of a cosine across the plot: a narrow dark furrow
     *  between broad beds (the cosine alone reads as ripples on water), of
     *  this mean. */
    mean: 5 / 16,
    /** It fades to that mean as a pixel grows from the first of these shares
     *  of a row to the second: sooner than the cosine did, for its furrow is
     *  a third as wide. */
    fade: [0.15, 0.45],
  },

  /** A drilled crop's wheelings (`groundWheeling`). */
  wheeling: {
    /** A wheeling is one of two furrows this many rows apart, a pair in every
     *  `tram_rows`: the tractor's track through a drilled crop. */
    gaugeRows: 2,
    /** The narrowest pixel a wheeling's coverage is measured over, metres. */
    leastPixelM: 0.02,
  },

  /** The verge along every plot edge (`groundColour`). */
  verge: {
    /** How much of the ground's mottle the verge carries. */
    mottle: 0.18,
    roughness: 0.95,
  },

  /** The forest floor's drawn edge (`forestVergeInside`, `forestFloorWeight`). */
  forestVerge: {
    /** The drawn edge lies this share of the verge outside the simulation's
     *  shape, before its wander and patches. */
    inset: 0.5,
    /** The verge's patches, per metre. */
    patchScale: 0.7,
    /** The floor's edge is never sharper than this share of its verge. */
    feather: 0.1,
  },

  /** The forest floor's litter, moss, humus and roots (`forestFloor`). Each
   *  drift is two octaves of noise, weighted so, eased across its cut. */
  forestFloor: {
    moss: { cut: [0.38, 0.72], weights: [0.65, 0.35], fineScale: 2.1 },
    humus: { cut: [0.42, 0.78], weights: [0.6, 0.4], scales: [1.3, 3.1] },
    roots: {
      /** Roots run where a second field passes from the first to the
       *  second of these, read this much finer than the roots. */
      runs: [0.55, 0.75],
      runScale: 1.7,
      /** A root is where the field lies within these of its middle. */
      line: [0.015, 0.035],
      /** Roots fade to `mean` as a pixel grows from the first share of a
       *  root's spacing to the second. */
      fade: [0.02, 0.08],
      mean: 0.02,
    },
  },

  /** Sun flecks under the crowns (`groundDapple`): two octaves, the second
   *  `octave` times finer, weighted so; a fleck's edge is `edge` either side
   *  of its cut, and the flecks fade to their mean as a pixel grows from the
   *  first share of a fleck to the second. */
  dapple: {
    weights: [0.65, 0.35],
    octave: 2.7,
    edge: 0.06,
    fade: [0.1, 0.5],
  },

  /** The ground's shading normal (`groundBank`, `scarredNormal`). */
  shading: {
    /** The steepest the shading normal is ever tilted (tan of 40°): past it
     *  the ground takes a grey sheen off the sky that reads as fog. */
    maxSlope: 0.84,
  },

  /** A river's cut bank, as shading reads it (`groundBank`). */
  bank: {
    /** The bank's shading ends over at least this far either side of its
     *  top, and gives way to the bed's this far inside the waterline, so
     *  neither is a ruled crease. */
    easeM: 0.75,
    /** Stretches share the shading by how far inside each puts the point:
     *  one that puts it this many metres farther out counts 1/e as much. The
     *  nearest stretch alone would turn the bank's face at a stroke along the
     *  line where two stretches are as near, on the inside of every bend. */
    blendM: 0.7,
    /** A stretch's weight stops changing this far inside or outside its
     *  water's edge: beyond it `exp` leaves what a 32-bit float holds, on a
     *  river wider than 120 m or a bank cut longer than 70 m. */
    blendReachM: 50,
  },

  /** The shore's bands (`groundShore`, `groundColour`). */
  shore: {
    /** The wet bank is whole out to this share of its width and fades over
     *  the rest; the bare earth's colour is whole over this share of the way
     *  from the wet bank's end to its own outer line. */
    wetWhole: 0.4,
    earthWhole: 0.65,
    /** The earth's outer line wanders by two octaves of noise: the second
     *  this many times finer than the biome's scale, with this share of the
     *  wander. Each lies on its own turned lattice (radians from the map's
     *  axes): one octave on the map's own shows as flat runs and regular
     *  lumps beside a round river. */
    fine: 3.1,
    fineShare: 0.4,
    turns: [0.55, 1.9],
    /** The fine octave fades to its mean as a pixel grows from the first to
     *  the second of these shares of its scale. */
    fineFade: [0.3, 1],
    /** How much of the ground's own mottle the bare earth keeps. */
    earthMottle: 0.6,
    /** The shore takes a field's own lightness this far inside the field's
     *  edge, and the verge's at the edge itself: two fields' banks meet in
     *  one tone, with no seam across the bank where their plots do. */
    fieldEaseM: 10,
    earthRoughness: 0.92,
    wetRoughness: 0.8,
    /** The bed takes over from the wet bank's silt across this far inside
     *  the waterline. */
    bedEaseM: 0.3,
  },

  /** Noise with no lattice to see (`wanderNoise`): three octaves, each turned
   *  off the grid, read at a point a broader octave has pushed about. */
  wander: {
    /** The push: its octave's scale, and how far it moves the point. */
    pushScale: 0.61,
    push: 1.4,
    /** The second and third octaves' scales, and all three weights. */
    octaves: [2.03, 4.01],
    weights: [0.5, 0.3, 0.2],
  },

  /** A road surface's grain (`roadGrain`): lumps at three scales and, finest,
   *  stones as crisp specks. */
  roadGrain: {
    /** An octave shows while its cells are wider than a pixel or two, as
     *  cells over the pixel. */
    fade: [0.6, 1.2],
    /** The finer octaves' scales (the first is 1), the last the stones'. */
    octaves: [2.03, 4.01, 8.3],
    /** The three lumps' weights, then the stones'. */
    weights: [0.25, 0.35, 0.4, 0.5],
    /** The stones: specks lighter and darker than the surface, where the
     *  finest octave passes these levels either way. */
    stones: [0.7, 0.76],
  },

  /** The worn ground beside a road (`groundShoulder`, `groundRoads`). */
  shoulder: {
    /** A shoulder is worn whole out to this share of its width there, and
     *  fades from it to its edge. */
    whole: 0.35,
    /** The wander is the noise about its middle, stretched this much: the
     *  noise seldom leaves the middle of its range. */
    wander: 5,
    /** A shoulder fades out as a pixel grows from the first share of its
     *  width to the second: whole at the default camera, gone by the tactical
     *  one. A few pixels wide it is a fringe that follows the road, and reads
     *  as a halo or a contact shadow round it. */
    pixels: [0.03, 0.08],
    /** A shoulder carries this share of its road's grain. */
    grain: 0.5,
    /** Toward its outer edge the field comes back through a shoulder in
     *  tufts this many metres across, where their noise is under these. */
    tuftM: 1.1,
    tufts: [0.44, 0.52],
  },

  /** A track's grass centre strip (`groundStrip`, `groundRoads`). */
  strip: {
    /** The strip's edge wanders by this share of its half width over this
     *  many metres, and is never sharper than this. (The strip is never
     *  broken along its length: from the default camera its pieces read as a
     *  dashed line painted down the track.) */
    wander: 3,
    wanderM: 2.5,
    edgeM: 0.3,
    /** A strip fades out as a pixel grows from the first share of its width
     *  to the second, with the ruts beside it: a few pixels wide it is a line
     *  down the middle of the track, and reads as a road marking. */
    pixels: [0.2, 0.4],
    /** How far the strip's ground goes to the verge's colour, and how much
     *  of the road's bareness the strip takes away for the grass. */
    cover: 0.6,
    grass: 0.75,
  },

  /** A road's ruts (`groundRuts`). */
  ruts: {
    /** Ruts show while one is wider than a few pixels: they fade to the
     *  surface's mean as a pixel grows from the first share of a rut's width
     *  to the second (gone under 2.5 pixels a rut). */
    fade: [0.2, 0.4],
    /** Along the road a rut comes and goes: down to this share of itself,
     *  over this many metres, as its noise crosses `breakCut`. */
    faint: 0.4,
    breakM: 6,
    breakCut: [0.4, 0.6],
  },

  /** A road's own surface (`groundRoads`). */
  surface: {
    /** A patch covers the share of a surface where its noise passes this. */
    patchCut: [0.52, 0.66],
    /** Where one road is carried onto another (`join_m`), the line the upper
     *  road's surface starts from wanders in drifts this many metres across. */
    joinDriftM: 2.5,
  },

  /** A walk's slabs, kerbstones and an area's slab grid (`strokeJoint`,
   *  `walkJoint`, `groundRoads`). */
  slabs: {
    /** The joint between two of a walk's slabs, or two kerbstones, is this
     *  wide, and shows while a pixel is under the first of `jointPixels`
     *  shares of it, gone by the second. */
    jointM: 0.03,
    jointPixels: [0.7, 2.5],
    /** An area's slabs are larger, their joints wider: they read from
     *  farther off. */
    areaJointM: 0.1,
    /** An area's slab grid is whole while a slab spans more than 1 / the
     *  first of these pixels, gone by 1 / the second: a long fade, since its
     *  joints already thin with distance (`areaJoint`), and at a grazing view
     *  the footprint grows fast. */
    areaSlabPixels: [1 / 24, 1 / 2.5],
    /** The widest plain border between a walk's edge and an area's slab grid. */
    areaBorderM: 2,
    /** How much deeper the joint along the walk's outer edge is than the
     *  grid's own, so the walk reads as its own strip. */
    areaEdgeDepth: 1.5,
  },

  /** A street's curb (`groundCurb`). */
  curb: {
    /** Its stones and its face are never sharper than this, in metres. */
    edgeM: 0.03,
    /** A curb fades out as a pixel grows from the first share of its stones'
     *  width to the second: a pixel or two wide it is a line that crawls, and
     *  shows on streets that run one way and not the other. */
    pixels: [0.25, 0.6],
  },

  /** A street's painted lines (`groundMarks`, `groundRoads`). */
  marks: {
    /** Painted lines are never sharper than this, in metres. */
    edgeM: 0.03,
    /** A line fades out as a pixel grows from the first share of its width
     *  to the second (the centre line by its own width, a crossing's bars by
     *  theirs): a pixel wide, a dashed line crawls. */
    pixels: [0.35, 0.85],
    /** A stroke crosses a road where their directions differ by more than
     *  this (the cosine of the angle between them): a stretch round the
     *  road's own bend does not. */
    crossingCos: 0.8,
    /** A dash of the centre line comes no nearer than this to a crossing's bars. */
    clearM: 0.3,
    /** Paint wears in patches this many metres across, where their noise
     *  crosses `wearCut`. */
    wearM: 1.3,
    wearCut: [0.4, 0.6],
  },

  /** The water surface (`waterSurface`, `waterNormal`; presentation, not
   *  rules: the biome's `water` row holds its colours). */
  water: {
    /** Smooth enough that sky and sun reflect in it. */
    roughness: 0.14,
    /** How much of its light water keeps in a shadow: the murk in it is lit
     *  by the sun, so a bridge or a tree shades it though the sky it reflects
     *  does not. */
    shadow: 0.55,
    /** Two octaves of ripples, metres across, the short at `shortWeight`;
     *  the slope they give the normal, read over `stepM` either way. */
    ripple: {
      longM: 3.2,
      shortM: 0.9,
      shortWeight: 0.5,
      slope: 0.09,
      stepM: 0.25,
      /** Ripples fade out as a pixel grows from the first of these widths to
       *  the second, in metres: far water lies flat and calm. */
      fadeM: [0.15, 1.2],
    },
    /** The light on the water lies in lanes `m` wide across the stream, each
     *  holding its distance from the bank, so the lanes run with the channel
     *  round every bend. They drift across the stream by `drift` lanes over
     *  `driftM`, and are broken along it into streaks about `streakM` long. */
    lanes: {
      m: 0.5,
      drift: 2.5,
      driftM: 11,
      streakM: 3.5,
      /** Lane and streak's weights in what lights a streak. */
      weights: [0.6, 0.4],
      /** A streak is lit where lane and streak together pass from the first
       *  of these to the second; about `litMean` of the water is. */
      lit: [0.55, 0.8],
      litMean: 0.1,
    },
  },

  /** The side's learned scars, cut into iso-lines (`groundScars`,
   *  `groundScarsSeen`). Depths are in full craters. */
  scars: {
    /** Crater depth never reads past this many full craters (bytes saturate). */
    depthCap: 1.6,
    /** The rim is read from the crater field this many cells out,
     *  diagonally; the slopes from taps this many cells either side. */
    rimReachCells: 1.5,
    slopeTapCells: 0.75,
    /** The relief fades out as a pixel grows from the first of these many
     *  cells to the second. */
    detailFade: [0.5, 2],
    /** The bowl's lip: the depth its edge crosses, give or take half the
     *  wander, by noise over `lipWanderM`. */
    lip: [0.3, 0.14],
    lipWanderM: 0.45,
    /** The thrown-soil ring: the cubic crater field's skirt from this depth
     *  out to the lip, its outer edge wandering by the same share. */
    ring: [0.035, 0.03],
    /** A crater whose deepest point nearby stays under the first depth threw
     *  up no soil: a small shell's pock, its ring drawn as scorched earth
     *  instead. Past the second it is a crater with its soil ring; between,
     *  one blends into the other, so a pock dug deeper by more hits grows a
     *  ring without a step. Every shell's crater reaches past the second
     *  within the taps that read its ring. */
    pock: [0.05, 0.09],
    /** How dark a pock's scorched earth is, as a share of full soot. */
    pockSoot: 0.85,
    /** A pock's edge: the depth it crosses, give or take half the wander.
     *  Shallower than the soil ring's, as a pock is all scorched earth. */
    pockEdge: [0.013, 0.006],
    /** A pock is this share of its darkness at its edge, all of it at its
     *  heart (where its depth reaches the ring's): browner toward its edge,
     *  as a burn is. */
    pockEdgeDark: 0.45,
    /** Ash flecks: two octaves of noise, the first at `scale` per metre, the
     *  second `fine` times finer, weighted so, crossing an edge that falls
     *  from the first of `edge` at the scorch's reach to the second at its
     *  heart; none where the scorch is under `least`. */
    ash: { scale: 4.5, fine: 3.1, weights: [0.75, 0.25], edge: [0.97, 0.72], least: 0.45 },
    /** A rut's edge: the eased track weight it crosses, give or take by noise. */
    rut: [0.35, 0.2],
    /** The view's least rise per unit run when a bowl is looked into: at a
     *  grazing view the read point steps no farther than this allows. */
    grazing: 0.25,
  },

  /** How the scars colour the ground (`scarredSurface`). Each colour is
   *  scaled between two values by a fine grain or fleck noise. */
  scarSurface: {
    /** The grain's and the flecks' scales, per metre. */
    grainScale: 1.7,
    fleckScale: 6.3,
    trampled: [0.9, 1.1],
    tracks: [0.85, 1.1],
    /** Soot from ash grey to char black, by flecks and drifts weighted so,
     *  the drifts at `ashDriftScale` per metre. */
    ash: [0.6, 2.4],
    ashWeights: [0.6, 0.4],
    ashDriftScale: 0.7,
    /** Thrown soil and a crater's floor. */
    soil: [0.8, 1.15],
    /** A full crater's floor is this much of its soil's brightness. */
    cavity: 0.7,
  },
} as const;
