//! The shared lighting and effect pass a test draws sprites with, seen as
//! `view` and `camera`, bloom off.
use presentation::light::LightSettings;
use presentation::records::{Camera, View};
use presentation::shadow::SunShadowFrame;
use renderer_core::effects::{EffectSheets, EffectsResources};
use renderer_core::environment::Environment;
use renderer_core::pipelines::{self, Pipelines};
use renderer_core::shadow::ShadowResources;
use renderer_core::sky::Sky;
use renderer_core::Targets;
use std::sync::Arc;

pub fn stage(
    gpu: &crate::support::Gpu,
    size: [u32; 2],
    view: &View,
    camera: &Camera,
) -> (
    Pipelines,
    Targets,
    Environment,
    EffectsResources,
    LightSettings,
) {
    let pipelines = pipelines::build_builtin(&gpu.device).unwrap();
    let targets = Targets::new(&gpu.device, size);
    let game: serde_json::Value =
        serde_json::from_str(include_str!("../../../../fixtures/presentation.json")).unwrap();
    let mut light: LightSettings = serde_json::from_value(game["light"].clone()).unwrap();
    light.bloom.strength = 0.;
    let cold = SunShadowFrame::cold(&light);
    let shadow = ShadowResources::new(
        &gpu.device,
        &gpu.queue,
        &pipelines,
        16,
        bytemuck::bytes_of(&cold.receiver),
    );
    let sky = Sky::new(&gpu.device, &gpu.queue, &pipelines, &light.sky());
    let mut environment = Environment::new(
        &gpu.device,
        &gpu.queue,
        &pipelines,
        &sky.lut,
        &light.sun(),
        &light.aerial(),
        &shadow,
    );
    environment.prepare(&gpu.queue, view, camera);
    // No sheet shape is drawn here; the atlas only has to exist.
    let mut png = vec![];
    {
        let mut encoder = ::png::Encoder::new(&mut png, 4, 4);
        encoder.set_color(::png::ColorType::Rgba);
        encoder.set_depth(::png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&[0; 64]).unwrap();
    }
    let sheets = Arc::new(EffectSheets::decode(4, &[&png]).unwrap());
    let effects = EffectsResources::new(
        &gpu.device,
        &gpu.queue,
        &pipelines,
        &targets,
        &environment,
        sheets,
    );
    (pipelines, targets, environment, effects, light)
}
