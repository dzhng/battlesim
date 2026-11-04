//! Canonical per-frame fog uploads consumed unchanged by GPU renderer shells.
use crate::records::{FogBox, FogEye, FogParams};

/// The structures pass of one frame (`fogWhole`): clear the flags, flag each
/// row's structure seen or not, copy the flags into the `wholes` texture's
/// rows from `flags_row`.
#[derive(Clone, Debug, Default)]
pub struct WholePass {
    /// Rows to flag (`whole_records`), and their words.
    pub count: u32,
    pub items: Vec<u32>,
    /// `(whole_step_m, count, 0, 0)`: the pass's uniform.
    pub uniform: [f32; 4],
    /// Workgroups: x, y.
    pub dispatch: [u32; 2],
    pub flags_row: u32,
    pub flag_rows: u32,
}

/// One drawn frame's fog: what to upload, then which passes to run, in order
/// (rebuild, structures, cull). A renderer sizes its buffers to what it is
/// handed and holds the rest from earlier frames.
#[derive(Clone, Debug, Default)]
pub struct FogUpload {
    pub params: FogParams,
    /// Whether fog is drawn: off, the passes run nothing and `params` says so.
    pub active: bool,
    /// Every eye, in map slot order of the latest sight.
    pub eyes: Vec<FogEye>,
    /// The maps' and the terrain maps' words: a map slot per eye that fits,
    /// `azimuth_bins × radial_bins` and `terrain_azimuth_bins × radial_bins`
    /// words each. A change rebuilds every map.
    pub map_words: [u32; 2],
    /// The rebuilt eyes' records (`horizon_records`) and the terrain march's
    /// and merge's workgroups; empty when none rebuild.
    pub rebuild: Vec<u32>,
    pub rebuild_dispatch: [u32; 2],
    /// The side's foliage cells (`[column, row, canopy_m, depth_per_m]`
    /// past the grid's header), when they changed.
    pub foliage: Option<Vec<f32>>,
    /// The occluders and the `wholes` texture's words
    /// (`WHOLE_TEXTURE_WIDTH` a row), when the side's knowledge changed.
    pub occluders: Option<Vec<FogBox>>,
    pub wholes: Option<Vec<u32>>,
    pub whole: Option<WholePass>,
    /// The cull: tiles across and down, and each tile's list length.
    pub tiles: [u32; 3],
}
