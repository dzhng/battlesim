//! Separate high air preserves model lighting, premultiplied coverage and its
//! own depth while the single-output client still draws it with the world,
//! and keeps the world's effects in front of its models over them.
mod support;

use presentation::light::LightSettings;
use presentation::records::{
    Appearance, Batch, Camera, Instance as ModelInstance, Joint, View, FOG_UNITS, HIGH_PASS, NONE,
    VIEW_PASS,
};
use presentation::shadow::SunShadowFrame;
use presentation::view::{Camera as Eye, View as Framed};
use renderer_core::bundle::{Bundle, BundleSource, Coverage, Draw, Material, Mesh, Node, Shape};
use renderer_core::effects::{EffectSheets, EffectsResources};
use renderer_core::environment::Environment;
use renderer_core::fog::Fog;
use renderer_core::high_air::HighAir;
use renderer_core::models::{Models, Surface};
use renderer_core::pipelines;
use renderer_core::post;
use renderer_core::shadow::ShadowResources;
use renderer_core::sky::Sky;
use renderer_core::Targets;
use std::sync::Arc;
use wgpu::*;

const SIZE: [u32; 2] = [128, 64];
const IDENTITY: [f32; 16] = [
    1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
];
struct Source;
impl BundleSource for Source {
    fn bundle(&self, _: &str) -> Result<Arc<Bundle>, String> {
        Ok(Arc::new(Bundle {
            shape: Shape::Static(vec![Node {
                name: "body".into(),
                tiers: vec![Mesh {
                    positions: vec![-0.2, -0.5, 0.3, 0.2, -0.5, 0.3, 0.0, 0.5, 0.3],
                    normals: [0, 0, 32767, 0].repeat(3),
                    uvs: vec![0.; 6],
                    colors: vec![255; 12],
                    indices: vec![0, 1, 2],
                    draws: vec![Draw {
                        material: 0,
                        first: 0,
                        count: 3,
                    }],
                    ..Default::default()
                }],
                top: None,
            }]),
            materials: vec![Material {
                name: "opaque".into(),
                base_color: [0.3, 0.6, 0.9, 1.],
                metallic: 0.,
                roughness: 1.,
                tint: 1.,
                textures: Default::default(),
                colour_scale: None,
                wear: None,
                coverage: Coverage::Opaque,
                interior: None,
            }],
            textures: vec![],
        }))
    }
}
fn read(
    device: &Device,
    queue: &Queue,
    texture: &Texture,
    mut encoder: CommandEncoder,
) -> Vec<[u8; 4]> {
    let row = (SIZE[0] * 4).next_multiple_of(COPY_BYTES_PER_ROW_ALIGNMENT);
    let output = device.create_buffer(&BufferDescriptor {
        label: Some("high-air-readback"),
        size: u64::from(row * SIZE[1]),
        usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        TexelCopyBufferInfo {
            buffer: &output,
            layout: TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: None,
            },
        },
        texture.size(),
    );
    queue.submit([encoder.finish()]);
    output.map_async(MapMode::Read, .., |r| r.unwrap());
    device.poll(PollType::wait_indefinitely()).unwrap();
    let mapped = output.get_mapped_range(..).unwrap();
    let mut pixels = vec![];
    for y in 0..SIZE[1] {
        pixels.extend_from_slice(bytemuck::cast_slice(
            &mapped[(y * row) as usize..(y * row + SIZE[0] * 4) as usize],
        ));
    }
    drop(mapped);
    output.unmap();
    pixels
}
fn prepare(models: &mut Models, gpu: &support::Gpu, high: bool) {
    let joint = Joint {
        x: [1., 0., 0., 0.],
        y: [0., 1., 0., 0.],
        z: [0., 0., 1., 0.],
    };
    let instances: Vec<_> = [-0.5, 0.5]
        .into_iter()
        .map(|x| ModelInstance {
            placement: [x, 0., 0., 0.],
            data: [0.; 4],
            tint: [1.; 4],
            scale: [1.; 4],
        })
        .collect();
    let batches = [VIEW_PASS, if high { HIGH_PASS } else { VIEW_PASS }]
        .into_iter()
        .enumerate()
        .map(|(first, pass)| Batch {
            first: first as u32,
            count: 1,
            pass,
            fog: FOG_UNITS,
            caster: NONE,
            ..Default::default()
        })
        .collect::<Vec<_>>();
    models
        .prepare_records(&gpu.device, &gpu.queue, &[joint], &instances, &batches)
        .unwrap();
}

/// A flat output isolates batch routing from the lighting asserted below.
fn world_pixels(gpu: &support::Gpu, targets: &Targets, models: &Models) -> Vec<[u8; 4]> {
    let shader = gpu.device.create_shader_module(ShaderModuleDescriptor {
        label: Some("world-routing"),
        source: ShaderSource::Wgsl(r#"
@vertex fn vertex(@location(0) point: vec3f, @location(1) placement: vec4f) -> @builtin(position) vec4f {
    return vec4f(point.xy + placement.xy, point.z, 1.0);
}
@fragment fn fragment() -> @location(0) vec4f { return vec4f(1.0); }
"#.into()),
    });
    let buffers = [
        Some(VertexBufferLayout {
            array_stride: renderer_core::models::VERTEX_BYTES as u64,
            step_mode: VertexStepMode::Vertex,
            attributes: &vertex_attr_array![0 => Float32x3],
        }),
        Some(VertexBufferLayout {
            array_stride: size_of::<ModelInstance>() as u64,
            step_mode: VertexStepMode::Instance,
            attributes: &vertex_attr_array![1 => Float32x4],
        }),
    ];
    let pipeline = gpu
        .device
        .create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("world-routing"),
            layout: None,
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vertex"),
                compilation_options: Default::default(),
                buffers: &buffers,
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fragment"),
                compilation_options: Default::default(),
                targets: &[Some(renderer_core::DISPLAY_FORMAT.into())],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &targets.display,
                depth_slice: None,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::TRANSPARENT),
                    store: StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        pass.set_pipeline(&pipeline);
        models.draw(&mut pass, Surface::Opaque);
    }
    read(&gpu.device, &gpu.queue, &targets.display_texture, encoder)
}

/// What a high-air frame draws with, seen as `view` and `camera`.
struct Stage {
    pipelines: pipelines::Pipelines,
    targets: Targets,
    environment: Environment,
    fog: Fog,
    post: post::PostResources,
    models: Models,
    effects: EffectsResources,
}

fn stage(gpu: &support::Gpu, view: &View, camera: &Camera) -> Stage {
    let pipelines = pipelines::build_builtin(&gpu.device).unwrap();
    let targets = Targets::new(&gpu.device, SIZE);
    let game: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/presentation.json")).unwrap();
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
    let fog = Fog::new(&gpu.device, &gpu.queue, &pipelines, &targets);
    let post = post::new(
        &gpu.device,
        &gpu.queue,
        &pipelines,
        &targets,
        bytemuck::bytes_of(&light.bloom()),
        bytemuck::bytes_of(&light.grade()),
        false,
    );
    let appearances = [Appearance {
        name_len: 4,
        lies: NONE,
        ..Default::default()
    }];
    let models = Models::from_world(
        &gpu.device,
        &gpu.queue,
        &pipelines,
        &Source,
        &appearances,
        "body",
        |_| None,
        |_, _| None,
    )
    .unwrap();
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
    Stage {
        pipelines,
        targets,
        environment,
        fog,
        post,
        models,
        effects,
    }
}

#[test]
fn high_air_has_separate_depth_premultiplied_grade_and_clears_stale_output() {
    let gpu = support::gpu().expect("real GPU");
    let Stage {
        pipelines,
        targets,
        environment,
        fog,
        post,
        mut models,
        effects,
    } = stage(
        &gpu,
        &View {
            view: IDENTITY,
            projection: IDENTITY,
            eye: [0., 0., 1., 0.],
            ..Default::default()
        },
        &Camera {
            view_proj: IDENTITY,
            width: SIZE[0] as f32,
            height: SIZE[1] as f32,
            ..Default::default()
        },
    );
    models.set_high_apart(true);
    prepare(&mut models, &gpu, true);
    assert_eq!(
        models.xray_instance_count(),
        1,
        "high air is excluded from x-ray candidates"
    );
    let world = world_pixels(&gpu, &targets, &models);
    let submitted = models.submissions.snapshot();
    assert_eq!(submitted.color.models_including_props.triangles, 1);
    assert_eq!(submitted.color.models_including_props.instances, 1);
    assert_eq!(submitted.color.models_including_props.draws, 1);
    let world_at = (SIZE[1] / 2 * SIZE[0] + SIZE[0] / 4) as usize;
    let high_at = (SIZE[1] / 2 * SIZE[0] + SIZE[0] * 3 / 4) as usize;
    assert_eq!(
        world[world_at], [255; 4],
        "ordinary model vanished from world"
    );
    assert_eq!(
        world[high_at], [0; 4],
        "separate high air leaked into world"
    );
    let high = HighAir::new(&gpu.device, &pipelines, &targets);
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    // The world has already finished with a near-depth clear. High air must
    // replace this depth, rather than disappearing behind that world.
    {
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: &targets.depth,
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(1.),
                    store: StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
        pipelines.bound("modelPrepass").set(
            &mut pass,
            &[("cameraLayout", &environment.camera), models.group()],
        );
        models.draw(&mut pass, Surface::Solid);
    }
    post.encode(&mut encoder, post::PostMode::Look);
    high.encode(&mut encoder, &environment, &models, &fog, &effects, &post);
    let pixels = read(&gpu.device, &gpu.queue, high.texture(), encoder);
    let submitted = models.submissions.snapshot();
    assert_eq!(submitted.depth.models_including_props.triangles, 1);
    assert_eq!(submitted.high_depth.models_including_props.triangles, 1);
    assert_eq!(submitted.high_color.models_including_props.triangles, 1);
    assert_eq!(submitted.high_color.models_including_props.draws, 1);
    let world = pixels[(SIZE[1] / 2 * SIZE[0] + SIZE[0] / 4) as usize];
    let airborne = pixels[(SIZE[1] / 2 * SIZE[0] + SIZE[0] * 3 / 4) as usize];
    assert_eq!(world, [0; 4], "world model leaked into high-air layer");
    assert_eq!(
        airborne[3], 255,
        "high-air model vanished behind world depth"
    );
    assert!(
        airborne[..3].iter().any(|v| *v > 10),
        "high-air model is unlit"
    );
    assert!(
        pixels.iter().any(|p| p[3] > 0 && p[3] < 255),
        "MSAA edge must retain partial coverage"
    );
    assert!(
        pixels.iter().all(|p| p[..3].iter().all(|v| *v <= p[3])),
        "output must be premultiplied"
    );
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    post.encode(&mut encoder, post::PostMode::Look);
    let graded = read(&gpu.device, &gpu.queue, &targets.display_texture, encoder);
    let graded = graded[(SIZE[1] / 2 * SIZE[0] + SIZE[0] * 3 / 4) as usize];
    assert!(
        airborne[..3]
            .iter()
            .zip(&graded[..3])
            .all(|(a, b)| a.abs_diff(*b) <= 1),
        "high air must share world grading: {airborne:?} vs {graded:?}"
    );
    prepare(&mut models, &gpu, false);
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    high.encode(&mut encoder, &environment, &models, &fog, &effects, &post);
    let pixels = read(&gpu.device, &gpu.queue, high.texture(), encoder);
    assert!(
        pixels.iter().all(|pixel| *pixel == [0; 4]),
        "high-air layer retained a stale model"
    );
    models.set_high_apart(false);
    prepare(&mut models, &gpu, true);
    assert_eq!(
        models.xray_instance_count(),
        2,
        "single-output native mode retains combined view routing"
    );
    models.submissions.reset();
    let combined = world_pixels(&gpu, &targets, &models);
    let submitted = models.submissions.snapshot();
    assert_eq!(submitted.color.models_including_props.triangles, 2);
    assert_eq!(submitted.color.models_including_props.instances, 2);
    assert_eq!(submitted.color.models_including_props.draws, 2);
    assert_eq!(submitted.high_color.models_including_props.draws, 0);
    assert_eq!(combined[world_at], [255; 4]);
    assert_eq!(
        combined[high_at], [255; 4],
        "single-output shell lost high air"
    );
    models.set_surface_shown(Surface::Opaque, false);
    models.submissions.reset();
    let hidden = world_pixels(&gpu, &targets, &models);
    assert!(hidden.iter().all(|pixel| *pixel == [0; 4]));
    assert_eq!(
        models
            .submissions
            .snapshot()
            .color
            .models_including_props
            .draws,
        0
    );
}

#[test]
fn model_suppression_preserves_static_props_and_buildings() {
    let gpu = support::gpu().expect("real GPU");
    let pipelines = pipelines::build_builtin(&gpu.device).unwrap();
    let targets = Targets::new(&gpu.device, SIZE);
    let appearances = [
        Appearance {
            name_len: 4,
            lies: NONE,
            ..Default::default()
        },
        Appearance {
            name_len: 4,
            lies: NONE,
            placed: 1,
            draws: 7,
            ..Default::default()
        },
        Appearance {
            name_len: 4,
            lies: NONE,
            placed: 1,
            ..Default::default()
        },
    ];
    let mut models = Models::from_world(
        &gpu.device,
        &gpu.queue,
        &pipelines,
        &Source,
        &appearances,
        "body",
        |_| None,
        |_, _| None,
    )
    .unwrap();
    let instances: Vec<_> = [-0.5, 0., 0.5]
        .into_iter()
        .map(|x| ModelInstance {
            placement: [x, 0., 0., 0.],
            data: [0.; 4],
            tint: [1.; 4],
            scale: [1.; 4],
        })
        .collect();
    let batches: Vec<_> = (0..3)
        .map(|i| Batch {
            key: i,
            appearance: i,
            first: i,
            count: 1,
            pass: VIEW_PASS,
            caster: NONE,
            fog: FOG_UNITS,
            ..Default::default()
        })
        .collect();
    models
        .prepare_records(&gpu.device, &gpu.queue, &[], &instances, &batches)
        .unwrap();
    let visible = world_pixels(&gpu, &targets, &models);
    let points =
        [SIZE[0] / 4, SIZE[0] / 2, SIZE[0] * 3 / 4].map(|x| (SIZE[1] / 2 * SIZE[0] + x) as usize);
    assert!(points.iter().all(|&at| visible[at] == [255; 4]));
    models.set_models_shown(false);
    models.submissions.reset();
    let hidden = world_pixels(&gpu, &targets, &models);
    assert_eq!(
        hidden[points[0]], [0; 4],
        "dynamic model survived suppression"
    );
    assert_eq!(hidden[points[1]], [255; 4], "static prop disappeared");
    assert_eq!(hidden[points[2]], [255; 4], "building disappeared");
    let work = models.submissions.snapshot();
    assert_eq!(work.color.models_including_props.triangles, 1);
    assert_eq!(work.color.models_including_props.instances, 1);
    assert_eq!(work.color.buildings.triangles, 1);
    models.set_models_shown(true);
    models.submissions.reset();
    assert_eq!(world_pixels(&gpu, &targets, &models), visible);
    assert_eq!(
        models
            .submissions
            .snapshot()
            .color
            .models_including_props
            .triangles,
        2
    );
}

/// A jet's nozzle glow in front of the jet stays over it in the separate
/// high-air output, as it does when one output draws both; a glow behind the
/// jet stays hidden, and none shows where the plane has no model.
#[test]
fn effects_in_front_of_high_air_stay_over_it() {
    let gpu = support::gpu().expect("real GPU");
    // Looking straight down on the high-air model (+x of the world one).
    let eye = Eye {
        eye: [0.5, 0., 4.],
        target: [0.5, 0., 0.],
        up: [0., 1., 0.],
        fov_y: 0.6,
        near: 0.1,
        size: SIZE,
    };
    let framed = Framed::new(&eye);
    let Stage {
        pipelines,
        targets,
        environment,
        fog,
        post,
        mut models,
        mut effects,
    } = stage(&gpu, &framed.record(&eye), &framed.camera(&eye, 0.));
    models.set_high_apart(true);
    prepare(&mut models, &gpu, true);
    let high = HighAir::new(&gpu.device, &pipelines, &targets);
    // Glows (shape 1): middle and radius, turn and least size, light.
    let glow = |at: [f32; 3]| {
        [
            at[0], at[1], at[2], 0.15, 0., 0., 0., 0., 6., 3., 1., 0., 1., 0., 0., 0.,
        ]
    };
    let mut frame = |glows: &[[f32; 3]]| {
        let records: Vec<f32> = glows.iter().flat_map(|&at| glow(at)).collect();
        effects.prepare(&gpu.device, &gpu.queue, bytemuck::cast_slice(&records));
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        post.encode(&mut encoder, post::PostMode::Look);
        high.encode(&mut encoder, &environment, &models, &fog, &effects, &post);
        read(&gpu.device, &gpu.queue, high.texture(), encoder)
    };
    let middle = (SIZE[1] / 2 * SIZE[0] + SIZE[0] / 2) as usize;
    // 0.8 m to the side at the front glow's distance: off every model.
    let aside = (SIZE[1] / 2 * SIZE[0] + SIZE[0] / 2 + 27) as usize;
    let bare = frame(&[]);
    assert_eq!(bare[middle][3], 255, "the high-air model is in the middle");
    assert_eq!(bare[aside], [0; 4], "nothing beside it");
    let front = frame(&[[0.5, 0., 1.], [1.3, 0., 1.]]);
    let light = |p: [u8; 4]| p[..3].iter().map(|&v| u32::from(v)).sum::<u32>();
    assert!(
        light(front[middle]) > light(bare[middle]) + 60,
        "a glow in front of the model is covered by it: {:?} vs {:?}",
        front[middle],
        bare[middle]
    );
    assert_eq!(front[middle][3], 255);
    assert_eq!(
        front[aside], [0; 4],
        "a glow off the model drew into the plane"
    );
    let behind = frame(&[[0.5, 0., -0.5]]);
    assert!(
        behind[middle]
            .iter()
            .zip(&bare[middle])
            .all(|(a, b)| a.abs_diff(*b) <= 1),
        "a glow behind the model shows through it: {:?} vs {:?}",
        behind[middle],
        bare[middle]
    );
}
