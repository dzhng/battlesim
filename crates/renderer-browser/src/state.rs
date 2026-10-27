#[derive(Debug, Default, Clone, Copy)]
pub(super) struct Stats {
    pub(super) frames: u32,
    pub(super) draw_calls: u32,
    pub(super) model_vertices: u32,
    pub(super) resizes: u32,
    pub(super) shared_passes: u32,
    pub(super) target_generation: u64,
    pub(super) shared_materials: bool,
    pub(super) shared_shadow: bool,
    pub(super) shadow_caster_batches: [u32; 4],
    pub(super) shadow_passes: u32,
    pub(super) shared_environment: bool,
    pub(super) shared_fog_sight: bool,
    pub(super) shared_model_prepass: bool,
    pub(super) surface_recoveries: u32,
    pub(super) device_recoveries: u32,
    pub(super) presentation_uploads: u32,
    pub(super) fog_sight_uploads: u32,
    pub(super) fog_height_words: u32,
    pub(super) presentation_bound: bool,
    pub(super) presentation_camera_words: u32,
    pub(super) presentation_environment_words: u32,
    pub(super) presentation_fog_words: u32,
    pub(super) presentation_overlay_words: u32,
}

#[derive(Clone)]
pub(super) struct PresentationSnapshot {
    pub(super) camera: Vec<f32>,
    pub(super) environment: Vec<f32>,
    pub(super) fog: Vec<f32>,
    pub(super) overlay: Vec<f32>,
    pub(super) shadow: Vec<f32>,
    pub(super) casters: Vec<f32>,
    pub(super) batches: Vec<u32>,
}
