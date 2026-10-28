//! The ground's and the grass's tuning: every number that sets their look
//! and is not the biome's. The values are `fixtures/game.json`'s
//! `presentation.terrain` and `presentation.grass`; the browser's shaders
//! (`frame/terrainMaterial.ts`, `frame/grassPass.ts`) splice most into their
//! WGSL as constants, and the tables read the few below. The water's and the
//! shore's are uniforms instead (`TerrainParams.waterLooks`, `shoreTuning`),
//! so they retune without rebuilding a shader. What stays in a shader is its formula: maths, Rec. 709's weights,
//! hash multipliers, decorrelating offsets, a lattice's fixed turns.
//!
//! **`presentation.terrain`**, by the formula that reads it (shares are of
//! the thing named, pixel ranges fade from the first to the second):
//! - `distant`: the patchwork fades to the distant colour over `fade_m`
//!   inside its region's edge, and plots give way to it as the smallest
//!   plot's side shrinks over `plot_pixels` of it (6 to 2 pixels).
//! - `mottle` (`mottle`, `groundTint`): the dry patches' hue weight
//!   `patch_hue`; the fine noise's brightness `fine_value` (off the plots) and
//!   hue `fine_hue`, fading over `fine_fade` of its scale; a dry patch covers
//!   its plot where its noise passes `patch_cut`, its edge no sharper than
//!   `patch_edge_m` nor a pixel, its slope read over `slope_step`; `dry` scales
//!   linear rgb toward ochre before luminance is restored, never toward blue.
//! - `grain` (`fieldTexture`): four octaves, each `octave` times finer,
//!   weighted `weights` (the third is the biome's `grain_m`), each fading over
//!   `fade` of its size, driven into its limits by `crisp`, each on a lattice
//!   turned `turns` from the rows and pushed by the octave before by `warp` of
//!   its cell (one lattice, driven hard, comes out as squares).
//! - `rows`: a row breaks over `break_grains` grains; it is a cosine cubed (a
//!   narrow furrow between broad beds) of mean `mean`, fading over `fade` of a
//!   row.
//! - `wheeling`: a pair of furrows `gauge_rows` apart, measured over a pixel
//!   of at least `least_pixel_m`.
//! - `verge`: the share of mottle it carries, its roughness.
//! - `forest_verge`: the drawn edge lies `inset` of the verge outside the
//!   shape, its patches `patch_scale` per metre, no sharper than `feather` of
//!   the verge.
//! - `forest_floor`: moss and humus drifts, two octaves each (`weights`,
//!   `fine_scale` or `scales`) eased across `cut`; roots where a second field
//!   `run_scale` finer passes `runs`, a root within `line` of its middle,
//!   fading to `mean` over `fade` of the spacing.
//! - `dapple`: two octaves `octave` apart (`weights`), an edge `edge` either
//!   side of the cut, fading over `fade` of a fleck.
//! - `shading.max_slope`: the steepest the shading normal tilts (tan 40°):
//!   past it the ground takes a grey sheen off the sky that reads as fog.
//! - `bank` (`groundBank`): its shading eases over at least `ease_m` either
//!   side of its top; stretches share it by how far inside each puts the
//!   point, one `blend_m` farther out counting 1/e as much (the nearest alone
//!   creases every inner bend); a weight stops changing `blend_reach_m` from
//!   its edge, where `exp` leaves f32.
//! - `shore`: the wet bank whole to `wet_whole` of its width, the earth over
//!   `earth_whole` of its way; the earth's line wanders by two octaves on
//!   lattices turned `turns`, the second `fine` finer with `fine_share` of the
//!   wander, fading over `fine_fade`; the earth keeps `earth_mottle` of the
//!   ground's mottle; the shore takes a field's lightness `field_ease_m`
//!   inside its edge (two fields' banks meet in one tone); the bands'
//!   roughness; the bed takes over across `bed_ease_m` inside the waterline.
//! - `wander`: lattice-free noise, three octaves turned off the grid at a
//!   point pushed `push` by an octave of scale `push_scale`.
//! - `road_grain`: an octave shows while its cells are wider than `fade`
//!   pixels; the finer octaves' scales, the lumps' and stones' weights, the
//!   stones where the finest passes `stones` either way.
//! - `shoulder`: worn whole to `whole` of its width; its wander is the noise
//!   about its middle stretched by `wander`; it fades over `pixels` of its
//!   width (a few pixels wide it reads as a halo); it carries `grain` of the
//!   road's grain; the field comes back through it in tufts `tuft_m` across
//!   where their noise is under `tufts`.
//! - `strip`: its edge wanders `wander` of its half width over `wander_m`, no
//!   sharper than `edge_m`; it fades over `pixels` of its width (a few pixels
//!   wide it reads as a road marking); `cover` toward the verge's colour, and
//!   `grass` of the road's bareness taken away.
//! - `ruts`: fade over `fade` of a rut's width; along the road a rut fades to
//!   `faint` of itself over `break_m` as its noise crosses `break_cut`.
//! - `surface`: a patch where its noise passes `patch_cut`; a carried road's
//!   start wanders in drifts `join_drift_m` across.
//! - `slabs`: a walk's or kerb's joint is `joint_m` wide, showing while a
//!   pixel is under `joint_pixels` of it; an area's are `area_joint_m`, its
//!   grid whole while a slab spans more than 1 / `area_slab_pixels` pixels;
//!   at most `area_border_m` of plain border to a walk, whose edge joint is
//!   `area_edge_depth` times as deep.
//! - `curb`: no sharper than `edge_m`; fades over `pixels` of its width.
//! - `marks`: no sharper than `edge_m`; fade over `pixels` of their width; a
//!   stroke crosses a road where their directions' cosine is under
//!   `crossing_cos`; a dash keeps `clear_m` from a crossing's bars; paint
//!   wears in patches `wear_m` across where their noise crosses `wear_cut`.
//! - `water`: it fades in from its edge over `shore_fade_m` of depth (or a
//!   few pixels, if wider); its reflection in three `lobes` of rising
//!   `roughness`, each `weights` of the light; the share of its light it
//!   keeps in shadow; its baked normal map (`normals`) on three layers of
//!   `tiles_m`, each drifting at its share (`drifts`) of the wind's drift,
//!   leaning by `strength`, gone over `fade_m` pixels; the bed seen through
//!   it shifted and blurred (`refraction`), and lit by the light its waves
//!   focus (`caustics`).
//! - `scars` (depths in full craters): read no deeper than `depth_cap`; the
//!   rim `rim_reach_cells` out, slopes from taps `slope_tap_cells` either
//!   side; relief fading over `detail_fade` cells; the bowl's lip at `lip`
//!   (depth, wander) over `lip_wander_m`; the soil ring's skirt from `ring`;
//!   a pock (a small shell's, its ring scorched earth instead) under the first
//!   of `pock`, a crater past the second, blended between; a pock's soot
//!   `pock_soot`, edge `pock_edge`, `pock_edge_dark` of its darkness there;
//!   ash flecks of two octaves crossing an edge falling over `ash.edge`, none
//!   under `ash.least`; a rut's edge `rut`; the least rise per run a bowl is
//!   read at, `grazing`.
//! - `scar_surface`: the grain's and flecks' scales; each colour scaled
//!   between its two values (`trampled`, `tracks`, soot `ash` by flecks and
//!   drifts weighted `ash_weights` at `ash_drift_scale`, `soil`); a full
//!   crater's floor is `cavity` of its soil's brightness.
//!
//! **`presentation.grass`**: grass thins and lowers over `edge_m` past a
//! road's or a wood's margin; a clump strays `placement.jitter_m` from its
//! sequence point, and a crop leaves its rows `placement.headland_m` from a
//! verge or margin; ground seen flatter than `density.least_facing` holds as
//! many clumps as at that slant; `stand`: no clump where the roads wore the
//! ground past `bare_wear`, sparse patches under `sparse_cut`, at a margin
//! `edge_density` as dense and `edge_height` as tall, a clump grows in from
//! `grow_from` of its rank's threshold and not under `least_height_m`, dry
//! patches past `parched_cut`; `mix`: drifts past `drift_cut`, a grass wholly
//! of its drifts `drift_gather` times its share in one; `tier_jitter` scales
//! the near tier's threshold per clump; a laid clump leans `laid.lean` of its
//! height and sinks `laid.sink`; grass sways from where the wind's wave passes
//! `sway.gust_from`, its flutter turning `flutter_phase` radians a metre
//! downwind and `flutter_across` of that across; a blade's roughness.
use serde::Deserialize;

/// What the tables read of `presentation.terrain`.
#[derive(Clone, Deserialize)]
pub struct TerrainTuning {
    pub distant: Distant,
    pub forest_verge: ForestVerge,
    pub bank: Bank,
    pub marks: Marks,
    pub scars: Scars,
    pub water: WaterTuning,
    pub shore: ShoreTuning,
}

/// The water's surface, as every look draws it (`WaterLook`'s shared part).
#[derive(Clone, Deserialize)]
pub struct WaterTuning {
    pub shore_fade_m: f64,
    pub lobes: Lobes,
    pub shadow: f64,
    pub normals: Normals,
    pub refraction: Refraction,
    pub caustics: Caustics,
}

/// The sun's and the sky's reflection off the water (`waterPass`): three
/// lobes of rising roughness, each weighted by its share of the light.
#[derive(Clone, Deserialize)]
pub struct Lobes {
    pub roughness: [f64; 3],
    pub weights: [f64; 3],
}

/// The light the waves focus on the bed (`waterCaustics`): their pattern's
/// tile, their strength, the share of their light lost a metre down, and
/// how sharp their crests are.
#[derive(Clone, Deserialize)]
pub struct Caustics {
    pub tile_m: f64,
    pub strength: f64,
    pub fade_per_m: f64,
    pub sharpness: f64,
}

/// How the bed shows displaced through the surface's slope (`waterUnder`):
/// its shift as a share of the depth it is seen through, over at most
/// `depth_m`, never past `most` of the screen's height; the metres of water
/// per mip it blurs by.
#[derive(Clone, Deserialize)]
pub struct Refraction {
    pub share: f64,
    pub depth_m: f64,
    pub most: f64,
    pub blur_m: f64,
}

/// The baked normal map's three layers (`waterNormal`): their tiles in
/// metres (primary, macro, detail), each's share of the wind's drift, how
/// far the map's slope leans the normal, and the pixel footprints the
/// ripples fade over.
#[derive(Clone, Deserialize)]
pub struct Normals {
    pub tiles_m: [f64; 3],
    pub drifts: [f64; 3],
    pub strength: f64,
    pub fade_m: [f64; 2],
}

/// The bank's bands (`groundShore`) and its ground (`ShoreTuning` in
/// `TerrainParams`).
#[derive(Clone, Deserialize)]
pub struct ShoreTuning {
    pub wet_whole: f64,
    pub earth_whole: f64,
    pub fine: f64,
    pub fine_share: f64,
    pub turns: [f64; 2],
    pub fine_fade: [f64; 2],
    pub earth_mottle: f64,
    pub field_ease_m: f64,
    pub earth_roughness: f64,
    pub wet_roughness: f64,
    pub bed_ease_m: f64,
}

#[derive(Clone, Deserialize)]
pub struct Scars {
    pub depth_cap: f64,
}

#[derive(Clone, Deserialize)]
pub struct Distant {
    pub plot_pixels: [f64; 2],
}

#[derive(Clone, Deserialize)]
pub struct ForestVerge {
    pub feather: f64,
}

#[derive(Clone, Deserialize)]
pub struct Bank {
    pub ease_m: f64,
}

#[derive(Clone, Deserialize)]
pub struct Marks {
    pub clear_m: f64,
}

/// What the tables read of `presentation.grass`: grass thins and lowers
/// over `edge_m` past a road's or a wood's margin, which the ground's
/// surface field reads out to.
#[derive(Clone, Deserialize)]
pub struct GrassTuning {
    pub edge_m: f64,
}
