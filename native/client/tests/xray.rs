//! GPU occlusion regression over actual authored-model packing and exported
//! x-ray shaders. Run under the repository GPU lock.
use native_client::{gpu, pipelines};
use presentation::records::{
    Appearance, Batch, Camera, Instance as ModelInstance, Joint, FOG_FACES, FOG_UNITS, NONE,
    VIEW_PASS,
};
use renderer_core::bundle::{Bundle, BundleSource, Coverage, Draw, Material, Mesh, Node, Shape};
use renderer_core::fog::Layer;
use renderer_core::models::{Models, Surface};
use renderer_core::xray::XrayResources;
use renderer_core::{buffer, Targets};
use std::sync::Arc;
use wgpu::*;

const WIDTH: u32 = 128;
const HEIGHT: u32 = 64;

struct Source;

fn mesh(positions: Vec<f32>, indices: Vec<u32>) -> Mesh {
    let vertices = positions.len() / 3;
    Mesh {
        positions,
        normals: [0, 0, 32767, 0].repeat(vertices),
        uvs: vec![0.0; vertices * 2],
        colors: vec![255; vertices * 4],
        draws: vec![Draw {
            material: 0,
            first: 0,
            count: indices.len(),
        }],
        indices,
        ..Default::default()
    }
}

impl BundleSource for Source {
    fn bundle(&self, name: &str) -> Result<Arc<Bundle>, String> {
        let mesh = match name {
            "test_xray_unit" => mesh(
                vec![-0.09, -0.25, 0.3, 0.09, -0.25, 0.3, 0.0, 0.25, 0.3],
                vec![0, 1, 2],
            ),
            "test_xray_walls" => {
                let mut positions = Vec::new();
                let mut indices = Vec::new();
                for (left, right) in [(-0.85, -0.75), (-0.35, -0.15), (0.15, 0.35), (0.65, 0.85)] {
                    let first = positions.len() as u32 / 3;
                    positions.extend([
                        left, -0.3, 0.8, right, -0.3, 0.8, right, 0.3, 0.8, left, 0.3, 0.8,
                    ]);
                    indices.extend([first, first + 1, first + 2, first, first + 2, first + 3]);
                }
                mesh(positions, indices)
            }
            _ => return Err(format!("unexpected test appearance {name}")),
        };
        Ok(Arc::new(Bundle {
            shape: Shape::Static(vec![Node {
                name: "body".into(),
                tiers: vec![mesh],
                top: None,
            }]),
            materials: vec![Material {
                name: "opaque".into(),
                base_color: [1.0; 4],
                metallic: 0.0,
                roughness: 1.0,
                tint: 1.0,
                textures: Default::default(),
                colour_scale: None,
                wear: None,
                coverage: Coverage::Opaque,
                interior: None,
            }],
            textures: Vec::new(),
        }))
    }
}

fn targets(device: &Device) -> Targets {
    let mut targets = Targets::new(device, [WIDTH, HEIGHT]);
    let texture = |label, format, usage| {
        device.create_texture(&TextureDescriptor {
            label: Some(label),
            size: Extent3d {
                width: WIDTH,
                height: HEIGHT,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 4,
            dimension: TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        })
    };
    // Multisample texture sampling is only needed for this test's pixel readback.
    targets.overlay_msaa = texture(
        "test-overlay",
        TextureFormat::Rgba8Unorm,
        TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
    )
    .create_view(&Default::default());
    targets
}

fn read_overlay(device: &Device, encoder: &mut CommandEncoder, overlay: &TextureView) -> Buffer {
    let shader = device.create_shader_module(ShaderModuleDescriptor { label: Some("test-overlay-read"), source: ShaderSource::Wgsl(r#"
@group(0) @binding(0) var image: texture_multisampled_2d<f32>;
@group(0) @binding(1) var<storage, read_write> pixels: array<vec4f>;
@compute @workgroup_size(64) fn read(@builtin(global_invocation_id) id: vec3u) {
    let size = textureDimensions(image);
    if (id.x >= size.x * size.y) { return; }
    let p = vec2i(i32(id.x % size.x), i32(id.x / size.x));
    var color = vec4f(0.0);
    for (var sample = 0; sample < i32(textureNumSamples(image)); sample++) { color += textureLoad(image, p, sample); }
    pixels[id.x] = color / f32(textureNumSamples(image));
}
"#.into()) });
    let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
        label: Some("test-overlay-read"),
        layout: None,
        module: &shader,
        entry_point: Some("read"),
        compilation_options: Default::default(),
        cache: None,
    });
    let size = u64::from(WIDTH * HEIGHT) * 16;
    let output = device.create_buffer(&BufferDescriptor {
        label: Some("test-overlay-pixels"),
        size,
        usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let group = renderer_core::bind(
        device,
        &pipeline.get_bind_group_layout(0),
        "test-overlay-read",
        &[
            BindingResource::TextureView(overlay),
            output.as_entire_binding(),
        ],
    );
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups((WIDTH * HEIGHT).div_ceil(64), 1, 1);
    }
    let readback = device.create_buffer(&BufferDescriptor {
        label: Some("test-overlay-readback"),
        size,
        usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_buffer_to_buffer(&output, 0, &readback, 0, size);
    readback
}

fn draw(
    gpu: &gpu::Gpu,
    pipelines: &renderer_core::pipelines::Pipelines,
    targets: &Targets,
    models: &Models,
    xray: &XrayResources,
    camera: &BindGroup,
) -> Vec<[f32; 4]> {
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("test-world-only-depth"),
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: &targets.depth,
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(0.0),
                    store: StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
        pipelines
            .bound("modelPrepass")
            .set(&mut pass, &[("cameraLayout", camera), models.group()]);
        models.draw_fogged(&mut pass, Surface::Solid, Layer::Faces);
    }
    let world_submissions = models.submissions.snapshot();
    xray.encode(&mut encoder, targets, camera, models);
    assert_eq!(
        models.submissions.snapshot(),
        world_submissions,
        "partial-world counters exclude both x-ray coverage and silhouette draws"
    );
    let readback = read_overlay(&gpu.device, &mut encoder, &targets.overlay_msaa);
    gpu.queue.submit([encoder.finish()]);
    readback.map_async(MapMode::Read, .., |result| result.expect("overlay mapped"));
    gpu.device
        .poll(PollType::wait_indefinitely())
        .expect("GPU completed");
    let mapped = readback.get_mapped_range(..).expect("mapped overlay");
    let pixels = bytemuck::cast_slice(&mapped).to_vec();
    drop(mapped);
    readback.unmap();
    pixels
}

fn color(pixels: &[[f32; 4]], x: u32) -> [f32; 4] {
    pixels[(HEIGHT / 2 * WIDTH + x) as usize]
}
fn close(actual: [f32; 4], expected: [f32; 4], subject: &str) {
    assert!(
        actual
            .into_iter()
            .zip(expected)
            .all(|(a, b)| (a - b).abs() <= 1.5 / 255.0),
        "{subject}: expected {expected:?}, got {actual:?}"
    );
}

#[test]
fn xray_preserves_occlusion_gate_information_colors_and_frame_reset() {
    let gpu = gpu::open(&gpu::headless_instance(), None).expect("a real GPU");
    let pipelines = pipelines::build(&gpu.device).expect("exported shaders");
    let targets = targets(&gpu.device);
    let names = "test_xray_unittest_xray_walls";
    let appearances = [
        Appearance {
            name_len: 14,
            lies: NONE,
            ..Default::default()
        },
        Appearance {
            name: 14,
            name_len: 15,
            lies: NONE,
            ..Default::default()
        },
    ];
    let mut models = Models::from_world(
        &gpu.device,
        &gpu.queue,
        &pipelines,
        &Source,
        &appearances,
        names,
        |_| None,
    )
    .unwrap();
    let identity = Joint {
        x: [1.0, 0.0, 0.0, 0.0],
        y: [0.0, 1.0, 0.0, 0.0],
        z: [0.0, 0.0, 1.0, 0.0],
    };
    let mut instances = vec![ModelInstance::default(); 36];
    instances[0].scale = [1.0; 4];
    for (index, (x, packed, alpha)) in [
        (-0.75, 65535.0, 0.5),
        (-0.25, 16763904.0, 0.6),
        (0.25, 1736959.0, 0.4),
        (0.75, 16777215.0, 0.0),
    ]
    .into_iter()
    .enumerate()
    {
        instances[32 + index] = ModelInstance {
            placement: [x, 0.0, 0.0, 0.0],
            data: [0.0, 0.0, 0.0, packed],
            tint: [1.0; 4],
            scale: [1.0, 1.0, 1.0, alpha],
        };
    }
    let batches = [
        Batch {
            appearance: 1,
            count: 1,
            caster: NONE,
            fog: FOG_FACES,
            pass: VIEW_PASS,
            ..Default::default()
        },
        Batch {
            first: 32,
            count: 4,
            caster: NONE,
            fog: FOG_UNITS,
            pass: VIEW_PASS,
            ..Default::default()
        },
    ];
    models
        .prepare_records(&gpu.device, &gpu.queue, &[identity], &instances, &batches)
        .unwrap();
    let camera = Camera {
        view_proj: [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ],
        width: WIDTH as f32,
        height: HEIGHT as f32,
        ..Default::default()
    };
    let uniform = buffer(
        &gpu.device,
        &gpu.queue,
        "test-camera",
        BufferUsages::UNIFORM,
        bytemuck::bytes_of(&camera),
    );
    let camera = renderer_core::bind(
        &gpu.device,
        pipelines.layout("cameraLayout"),
        "test-camera",
        &[uniform.as_entire_binding()],
    );
    let mut xray = XrayResources::new(&gpu.device, &gpu.queue, &pipelines, &targets, 0.75).unwrap();
    xray.prepare(&gpu.device, &models);
    let pixels = draw(&gpu, &pipelines, &targets, &models, &xray, &camera);
    close(
        color(&pixels, 15),
        [0.0; 4],
        "half-hidden own unit is below threshold",
    );
    close(
        color(&pixels, 48),
        [0.6, 0.48, 0.0, 0.6],
        "selected hidden unit",
    );
    close(
        color(&pixels, 80),
        [26.0 / 255.0 * 0.4, 128.0 / 255.0 * 0.4, 0.4, 0.4],
        "ally hidden unit",
    );
    close(color(&pixels, 112), [0.0; 4], "unit without x-ray cue");
    xray.set_coverage_enabled(&gpu.queue, false);
    let pixels = draw(&gpu, &pipelines, &targets, &models, &xray, &camera);
    close(
        color(&pixels, 15),
        [0.0, 0.5, 0.5, 0.5],
        "coverage-disabled own unit",
    );
    for instance in &mut instances[32..] {
        instance.scale[3] = 0.0;
    }
    models
        .prepare_records(&gpu.device, &gpu.queue, &[identity], &instances, &batches)
        .unwrap();
    xray.set_coverage_enabled(&gpu.queue, true);
    let pixels = draw(&gpu, &pipelines, &targets, &models, &xray, &camera);
    assert!(
        pixels.iter().all(|pixel| *pixel == [0.0; 4]),
        "stale silhouettes survive a cleared frame"
    );
}
