//! The water: its baked normal map, the wave field its wakes spread on, and
//! its own pass after the world's. The pass copies the lit world before the
//! water (`Targets::scene_colour`), builds that copy's mips and the guard
//! that says how seen and how painted each pixel under the water is, then
//! draws the water over the world's samples, reading depth and writing none.
//! Water behind a pane draws over it: the cost of keeping glass in the world
//! pass.
use presentation::effects::frame::WaveFrame;
use presentation::records::WaveSplat;
use presentation::wave::WAVE;
use wgpu::*;

use crate::frame::FrameInputs;
use crate::pipelines::Pipelines;
use crate::{bind, Targets, HDR_FORMAT};

/// How much of what lies under the water it shows: its bed refracted and
/// blurred by depth, or (the lowest graphics tier) its depth colour over the
/// bed straight below, which builds no mips and no guard.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum WaterDetail {
    #[default]
    Full,
    Depth,
}

/// The normal map's texels along a side.
const NORMALS_SIZE: u32 = 256;

/// The water's normal map, baked once, and the sampler that tiles it; the
/// terrain's group binds both.
pub struct WaterNormals {
    pub view: TextureView,
    pub sampler: Sampler,
}

impl WaterNormals {
    /// The map, its bake queued before this returns.
    pub fn new(device: &Device, queue: &Queue, pipelines: &Pipelines) -> Self {
        let texture = device.create_texture(&TextureDescriptor {
            label: Some("water-normals"),
            size: Extent3d {
                width: NORMALS_SIZE,
                height: NORMALS_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: HDR_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        let mut encoder = device.create_command_encoder(&Default::default());
        full_screen(
            &mut encoder,
            "water-normals-bake",
            &view,
            pipelines.render("waterNormalsBake"),
            None,
        );
        queue.submit([encoder.finish()]);
        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("water-normals"),
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..Default::default()
        });
        Self { view, sampler }
    }

    pub fn record_memory(&self, memory: &mut crate::memory::ResourceMemory) {
        memory.view(&self.view);
    }
}

/// A full-screen triangle drawn into `view` with `group` at slot 0.
fn full_screen(
    encoder: &mut CommandEncoder,
    label: &'static str,
    view: &TextureView,
    pipeline: &RenderPipeline,
    group: Option<&BindGroup>,
) {
    let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
        label: Some(label),
        color_attachments: &[Some(RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Clear(Color::TRANSPARENT),
                store: StoreOp::Store,
            },
        })],
        ..Default::default()
    });
    pass.set_pipeline(pipeline);
    if let Some(group) = group {
        pass.set_bind_group(0, group, &[]);
    }
    pass.draw(0..3, 0..1);
}

/// Splats one step takes at most; past it the rest of that step's are
/// dropped. A budget per step, not per frame, so how many steps a frame
/// covers decides nothing.
const STEP_SPLATS: usize = 256;
/// Splats a frame holds: every step's budget.
const MAX_SPLATS: usize = STEP_SPLATS * WAVE.max_steps as usize;

/// One run of `waveStep` (`WaveRun`): which of the frame's steps it is, how
/// many splats the frame has, how far the field moved (texels; the first run
/// only), and 1 when it only moves the field without stepping it.
#[repr(C)]
#[derive(Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
struct WaveRun {
    shift: [i32; 2],
    step: u32,
    splats: u32,
    still: u32,
    pad: [u32; 3],
}

/// What a frame's waves ask of a field last run in `epoch` (None: no
/// battle). A new epoch, or a battle gone, clears it first, even with no
/// step to run, so one side's waves never linger for the next. A frame's
/// first run carries its shift, and a frame with no step still moves the
/// field by it, so every texel the camera moves reaches the field.
#[derive(Default)]
struct WavePlan {
    epoch: Option<u32>,
    clear: bool,
    runs: Vec<WaveRun>,
    /// The splats kept, each step's first `STEP_SPLATS` in order.
    splats: Vec<WaveSplat>,
}

fn wave_plan(frame: Option<&WaveFrame>, epoch: Option<u32>) -> WavePlan {
    let Some(frame) = frame else {
        return WavePlan {
            clear: epoch.is_some(),
            ..Default::default()
        };
    };
    let s = &frame.step;
    let mut taken = vec![0; s.steps as usize];
    let splats: Vec<WaveSplat> = frame
        .splats
        .iter()
        .filter(|p| {
            let n = taken.get_mut(p.step as usize);
            n.is_some_and(|n| {
                *n += 1;
                *n <= STEP_SPLATS
            })
        })
        .copied()
        .collect();
    let count = splats.len() as u32;
    let mut runs: Vec<WaveRun> = (0..s.steps)
        .map(|k| WaveRun {
            shift: if k == 0 { s.shift } else { [0, 0] },
            step: k,
            splats: count,
            ..Default::default()
        })
        .collect();
    let clear = Some(s.epoch) != epoch;
    if s.steps == 0 && !clear && s.shift != [0, 0] {
        runs.push(WaveRun {
            shift: s.shift,
            still: 1,
            ..Default::default()
        });
    }
    WavePlan {
        epoch: Some(s.epoch),
        clear,
        runs,
        splats,
    }
}

/// The wave field: a height field over a square of water around what the
/// camera looks at, which the wakes the presentation decides
/// (`Presentation::waves`) splat and which spreads as rings and V-wakes.
/// It is RedGPU's lake wave equation (a 9-point Laplacian, damping, an
/// absorbing border), stepped in fragment passes ping-ponging two
/// `rgba16float` targets, at the fixed rate and count the presentation's
/// clock gives, so it spreads alike at any frame rate. Each step's height is
/// red, the one before green. The water reads the latest from `shown`.
pub struct WaveField {
    fields: [Texture; 2],
    shown: Texture,
    shown_view: TextureView,
    pipeline: RenderPipeline,
    splats: Buffer,
    runs: Vec<Buffer>,
    /// Each step's group reading each field.
    groups: Vec<[BindGroup; 2]>,
    current: std::cell::Cell<usize>,
    epoch: Option<u32>,
    plan: WavePlan,
}

impl WaveField {
    pub fn new(device: &Device, pipelines: &Pipelines) -> Self {
        let texture = |label, usage| {
            device.create_texture(&TextureDescriptor {
                label: Some(label),
                size: Extent3d {
                    width: WAVE.texels,
                    height: WAVE.texels,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: HDR_FORMAT,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT | usage,
                view_formats: &[],
            })
        };
        let fields = [
            texture("wave-a", TextureUsages::COPY_SRC),
            texture("wave-b", TextureUsages::COPY_SRC),
        ];
        let shown = texture("wave-shown", TextureUsages::COPY_DST);
        let splats = device.create_buffer(&BufferDescriptor {
            label: Some("wave-splats"),
            size: (MAX_SPLATS * size_of::<WaveSplat>()) as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let runs: Vec<Buffer> = (0..WAVE.max_steps)
            .map(|_| {
                device.create_buffer(&BufferDescriptor {
                    label: Some("wave-run"),
                    size: size_of::<WaveRun>() as u64,
                    usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                })
            })
            .collect();
        let layout = pipelines.layout("waveLayout");
        let groups = runs
            .iter()
            .map(|run| {
                fields.each_ref().map(|was| {
                    bind(
                        device,
                        layout,
                        "wave-step",
                        &[
                            BindingResource::TextureView(&was.create_view(&Default::default())),
                            run.as_entire_binding(),
                            splats.as_entire_binding(),
                        ],
                    )
                })
            })
            .collect();
        Self {
            shown_view: shown.create_view(&Default::default()),
            fields,
            shown,
            pipeline: pipelines.render("waveStep").clone(),
            splats,
            runs,
            groups,
            current: Default::default(),
            epoch: None,
            plan: WavePlan::default(),
        }
    }

    /// The latest step's heights, for the water's fog group.
    pub fn view(&self) -> &TextureView {
        &self.shown_view
    }

    /// Takes this frame's waves (`Presentation::waves`, once per drawn
    /// frame: it advances the field's clock) and queues their uploads.
    pub fn prepare(&mut self, queue: &Queue, frame: Option<&WaveFrame>) {
        self.plan = wave_plan(frame, self.epoch);
        self.epoch = self.plan.epoch;
        if self.plan.runs.is_empty() {
            return;
        }
        if !self.plan.splats.is_empty() {
            queue.write_buffer(&self.splats, 0, bytemuck::cast_slice(&self.plan.splats));
        }
        for (run, buffer) in self.plan.runs.iter().zip(&self.runs) {
            queue.write_buffer(buffer, 0, bytemuck::bytes_of(run));
        }
    }

    /// The prepared frame's runs.
    pub fn encode(&self, encoder: &mut CommandEncoder) {
        if self.plan.clear {
            let views = [&self.fields[0], &self.fields[1], &self.shown]
                .map(|t| t.create_view(&Default::default()));
            let attachments = views.each_ref().map(|view| {
                Some(RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::TRANSPARENT),
                        store: StoreOp::Store,
                    },
                })
            });
            encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("wave-clear"),
                color_attachments: &attachments,
                ..Default::default()
            });
        }
        if self.plan.runs.is_empty() {
            return;
        }
        let mut current = self.current.get();
        for group in &self.groups[..self.plan.runs.len()] {
            let next = self.fields[1 - current].create_view(&Default::default());
            full_screen(
                encoder,
                "wave-step",
                &next,
                &self.pipeline,
                Some(&group[current]),
            );
            current = 1 - current;
        }
        self.current.set(current);
        encoder.copy_texture_to_texture(
            self.fields[current].as_image_copy(),
            self.shown.as_image_copy(),
            self.shown.size(),
        );
    }

    pub fn record_memory(&self, memory: &mut crate::memory::ResourceMemory) {
        for texture in self.fields.iter().chain([&self.shown]) {
            memory.texture(texture);
        }
        memory.buffer(&self.splats);
        for run in &self.runs {
            memory.buffer(run);
        }
    }
}

/// The water's pass over the targets: the world copy's mips, the guard
/// seeded from the fog mask and the ground paint and reduced by mips, then
/// the water itself.
pub struct WaterPass {
    mip: RenderPipeline,
    seed: RenderPipeline,
    guard: RenderPipeline,
    /// Each level's view and the group that reads the one before it: the
    /// copy's, then the guard's; and the guard's first level and its group.
    mips: Vec<(TextureView, BindGroup)>,
    guards: Vec<(TextureView, BindGroup)>,
    seeded: (TextureView, BindGroup),
}

impl WaterPass {
    /// The pass over `targets`, its guard seeded from their fog mask and
    /// `paint` (the fog's ground paint). Rebuild it with the targets.
    pub fn new(
        device: &Device,
        pipelines: &Pipelines,
        targets: &Targets,
        paint: &TextureView,
    ) -> Self {
        let taps = device.create_sampler(&SamplerDescriptor {
            label: Some("scene-colour-mips"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..Default::default()
        });
        let level = |texture: &Texture, mip| {
            texture.create_view(&TextureViewDescriptor {
                base_mip_level: mip,
                mip_level_count: Some(1),
                ..Default::default()
            })
        };
        let chain = |texture: &Texture, layout, label, sampled: bool| {
            (1..texture.mip_level_count())
                .map(|mip| {
                    let source = BindingResource::TextureView(&level(texture, mip - 1));
                    let sampler = BindingResource::Sampler(&taps);
                    let resources = if sampled {
                        vec![source, sampler]
                    } else {
                        vec![source]
                    };
                    let group = bind(device, pipelines.layout(layout), label, &resources);
                    (level(texture, mip), group)
                })
                .collect()
        };
        let guard = &targets.fog_mask_under_texture;
        let seeded = (
            level(guard, 0),
            bind(
                device,
                pipelines.layout("guardSeedLayout"),
                "water-guard-seed",
                &[
                    BindingResource::TextureView(&targets.fog_mask),
                    BindingResource::TextureView(paint),
                ],
            ),
        );
        Self {
            mip: pipelines.render("sceneColourMip").clone(),
            seed: pipelines.render("waterGuardSeed").clone(),
            guard: pipelines.render("waterGuardMip").clone(),
            mips: chain(
                &targets.scene_colour_texture,
                "sceneMipLayout",
                "scene-colour-mip",
                true,
            ),
            guards: chain(guard, "guardMipLayout", "water-guard-mip", false),
            seeded,
        }
    }

    /// The water over the world's samples, resolved again into `lit` and
    /// the fog mask, over the copy of `lit` as the world left it. With
    /// `refracted` the copy's mips and the guard are built; without, the
    /// water reads the copy at its own pixel alone.
    pub fn encode(&self, inputs: &FrameInputs<'_>, encoder: &mut CommandEncoder, refracted: bool) {
        let t = inputs.targets;
        encoder.copy_texture_to_texture(
            t.lit_texture.as_image_copy(),
            t.scene_colour_texture.as_image_copy(),
            Extent3d {
                width: t.size[0],
                height: t.size[1],
                depth_or_array_layers: 1,
            },
        );
        if refracted {
            let draws = self
                .mips
                .iter()
                .map(|m| (&self.mip, m))
                .chain([(&self.seed, &self.seeded)])
                .chain(self.guards.iter().map(|m| (&self.guard, m)));
            for (pipeline, (view, group)) in draws {
                full_screen(encoder, "water-under-mip", view, pipeline, Some(group));
            }
        }
        let resolved = |view, resolve_target| {
            Some(RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: Some(resolve_target),
                ops: Operations {
                    load: LoadOp::Load,
                    store: StoreOp::Discard,
                },
            })
        };
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("water"),
            color_attachments: &[
                resolved(&t.hdr_msaa, &t.lit),
                resolved(&t.fog_mask_msaa, &t.fog_mask),
            ],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: &t.depth,
                depth_ops: None,
                stencil_ops: None,
            }),
            ..Default::default()
        });
        let mut groups = inputs.environment.groups().to_vec();
        groups.push(inputs.fog.water_group());
        inputs.ground.draw_water(&mut pass, &groups);
    }
}

#[cfg(test)]
#[path = "water_tests.rs"]
mod tests;
