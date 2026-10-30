//! Real GPU contracts for paint and postprocessed information marks.
use native_client::{gpu, pipelines};
use renderer_core::{buffer, overlay, Targets};
use wgpu::*;

const SIZE: u32 = 64;

fn read(device: &Device, encoder: &mut CommandEncoder, texture: &TextureView) -> Buffer {
    let shader = device.create_shader_module(ShaderModuleDescriptor {
        label: Some("test-mark-read"),
        source: ShaderSource::Wgsl(
            r#"
@group(0) @binding(0) var image: texture_2d<f32>;
@group(0) @binding(1) var<storage, read_write> pixels: array<vec4f>;
@compute @workgroup_size(64) fn read(@builtin(global_invocation_id) id: vec3u) {
    let size = textureDimensions(image);
    if id.x >= size.x * size.y { return; }
    pixels[id.x] = textureLoad(image, vec2i(i32(id.x % size.x), i32(id.x / size.x)), 0);
}
"#
            .into(),
        ),
    });
    let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
        label: Some("test-mark-read"),
        layout: None,
        module: &shader,
        entry_point: Some("read"),
        compilation_options: Default::default(),
        cache: None,
    });
    let size = u64::from(SIZE * SIZE) * 16;
    let output = device.create_buffer(&BufferDescriptor {
        label: None,
        size,
        usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let group = renderer_core::bind(
        device,
        &pipeline.get_bind_group_layout(0),
        "test-mark-read",
        &[
            BindingResource::TextureView(texture),
            output.as_entire_binding(),
        ],
    );
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups((SIZE * SIZE).div_ceil(64), 1, 1);
    }
    let readback = device.create_buffer(&BufferDescriptor {
        label: None,
        size,
        usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    encoder.copy_buffer_to_buffer(&output, 0, &readback, 0, size);
    readback
}
fn finish(gpu: &gpu::Gpu, encoder: CommandEncoder, readback: Buffer) -> Vec<[f32; 4]> {
    gpu.queue.submit([encoder.finish()]);
    readback.map_async(MapMode::Read, .., |r| r.unwrap());
    gpu.device.poll(PollType::wait_indefinitely()).unwrap();
    let mapped = readback.get_mapped_range(..).unwrap();
    let out = bytemuck::cast_slice(&mapped).to_vec();
    drop(mapped);
    readback.unmap();
    out
}
fn close(actual: [f32; 4], expected: [f32; 4]) {
    assert!(
        actual
            .into_iter()
            .zip(expected)
            .all(|(a, b)| (a - b).abs() < 1.5 / 255.0),
        "expected {expected:?}, got {actual:?}"
    );
}
fn camera(
    device: &Device,
    queue: &Queue,
    pipelines: &renderer_core::pipelines::Pipelines,
) -> BindGroup {
    let record = presentation::records::Camera {
        view_proj: [
            1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
        ],
        eye: [0., 0., 2.],
        width: SIZE as f32,
        height: SIZE as f32,
        ..Default::default()
    };
    let uniform = buffer(
        device,
        queue,
        "test-camera",
        BufferUsages::UNIFORM,
        bytemuck::bytes_of(&record),
    );
    renderer_core::bind(
        device,
        pipelines.layout("cameraLayout"),
        "test-camera",
        &[uniform.as_entire_binding()],
    )
}
#[test]
fn empty_display_geometry_preserves_and_resolves_existing_silhouettes() {
    let gpu = gpu::open(&gpu::headless_instance(), None).unwrap();
    let pipelines = pipelines::build(&gpu.device).unwrap();
    let targets = Targets::new(&gpu.device, [SIZE, SIZE]);
    let overlay = overlay::new(
        &gpu.device,
        &gpu.queue,
        &pipelines,
        &targets,
        &renderer_core::records::OverlayGlow {
            radius_px: 1.0,
            sigma_px: 0.4,
            ..Default::default()
        },
    );
    let camera = camera(&gpu.device, &gpu.queue, &pipelines);
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    {
        let _pass = encoder.begin_render_pass(&RenderPassDescriptor {
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &targets.overlay_msaa,
                depth_slice: None,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color {
                        r: 0.1,
                        g: 0.2,
                        b: 0.3,
                        a: 0.5,
                    }),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: &targets.depth,
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(0.),
                    store: StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
    }
    overlay.encode_geometry(
        &mut encoder,
        &targets,
        &camera,
        overlay::Geometry::default(),
    );
    let readback = read(&gpu.device, &mut encoder, &targets.overlay);
    let pixels = finish(&gpu, encoder, readback);
    close(pixels[(SIZE * SIZE / 2) as usize], [0.1, 0.2, 0.3, 0.5]);
}

fn quad(left: f32, right: f32, z: f32, normal: [f32; 3], color: [f32; 4]) -> Vec<f32> {
    let mut vertices = Vec::new();
    for [x, y] in [
        [left, -0.8],
        [right, -0.8],
        [right, 0.8],
        [left, -0.8],
        [right, 0.8],
        [left, 0.8],
    ] {
        vertices.extend([x, y, z]);
        vertices.extend(normal);
        vertices.extend(color);
    }
    vertices
}
fn mesh(device: &Device, queue: &Queue, words: &[f32]) -> Buffer {
    buffer(
        device,
        queue,
        "test-marks",
        BufferUsages::VERTEX,
        bytemuck::cast_slice(words),
    )
}
fn identity(device: &Device, queue: &Queue) -> Buffer {
    buffer(
        device,
        queue,
        "test-identity",
        BufferUsages::VERTEX,
        bytemuck::cast_slice(&[0f32, 0., 0., 0., 1., 1., 1., 0.]),
    )
}
fn clear_depth(encoder: &mut CommandEncoder, depth: &TextureView, value: f32) {
    let _pass = encoder.begin_render_pass(&RenderPassDescriptor {
        depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
            view: depth,
            depth_ops: Some(Operations {
                load: LoadOp::Clear(value),
                store: StoreOp::Store,
            }),
            stencil_ops: None,
        }),
        ..Default::default()
    });
}
fn at(pixels: &[[f32; 4]], x: u32) -> [f32; 4] {
    pixels[(SIZE / 2 * SIZE + x) as usize]
}

#[test]
fn opaque_overlay_writes_only_pregrass_depth_and_translucent_overlay_does_not() {
    let gpu = gpu::open(&gpu::headless_instance(), None).unwrap();
    let pipelines = pipelines::build(&gpu.device).unwrap();
    let targets = Targets::new(&gpu.device, [SIZE, SIZE]);
    let overlay = overlay::new(
        &gpu.device,
        &gpu.queue,
        &pipelines,
        &targets,
        &renderer_core::records::OverlayGlow {
            radius_px: 1.0,
            sigma_px: 0.4,
            ..Default::default()
        },
    );
    let camera = camera(&gpu.device, &gpu.queue, &pipelines);
    let instances = identity(&gpu.device, &gpu.queue);
    let opaque = mesh(
        &gpu.device,
        &gpu.queue,
        &quad(-0.9, -0.1, 0.7, [0., 0., 1.], [1., 1., 0., 1.]),
    );
    let mut words = quad(-0.9, -0.1, 0.5, [0., 0., 1.], [0., 1., 1., 1.]);
    words.extend(quad(0.1, 0.9, 0.7, [0., 0., 1.], [1., 0., 0., 1.]));
    words.extend(quad(0.1, 0.9, 0.5, [0., 0., 1.], [0., 0., 1., 1.]));
    let translucent = mesh(&gpu.device, &gpu.queue, &words);
    let draw = |vertices, vertex_count| overlay::MeshDraw {
        vertices,
        instances: &instances,
        vertex_count,
    };
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    clear_depth(&mut encoder, &targets.depth, 0.9);
    clear_depth(&mut encoder, &targets.overlay_depth, 0.2);
    overlay.clear(&mut encoder, &targets);
    overlay.encode_geometry(
        &mut encoder,
        &targets,
        &camera,
        overlay::Geometry {
            opaque: Some(draw(&opaque, 6)),
            translucent: Some(draw(&translucent, 18)),
        },
    );
    let readback = read(&gpu.device, &mut encoder, &targets.overlay);
    let pixels = finish(&gpu, encoder, readback);
    let left = at(&pixels, 16);
    assert!(
        left[0] > 0.5 && left[1] > 0.5 && left[2] == 0.,
        "opaque yellow must hide later cyan, using pre-grass depth: {left:?}"
    );
    let right = at(&pixels, 48);
    assert!(
        right[0] == 0. && right[1] == 0. && right[2] > 0.5,
        "front translucent red must not hide later blue through a depth write: {right:?}"
    );
}

#[test]
fn paint_preserves_still_marching_pointer_depth_and_clear_contracts() {
    use renderer_core::paint::{PaintGeometry, PaintResources};
    let gpu = gpu::open(&gpu::headless_instance(), None).unwrap();
    let pipelines = pipelines::build(&gpu.device).unwrap();
    let targets = Targets::new(&gpu.device, [SIZE, SIZE]);
    let paint = PaintResources::new(&pipelines);
    let instances = identity(&gpu.device, &gpu.queue);
    let still = mesh(
        &gpu.device,
        &gpu.queue,
        &quad(-0.9, -0.5, 3., [0.; 3], [1., 0., 0., 1.]),
    );
    let marching = mesh(
        &gpu.device,
        &gpu.queue,
        &quad(-0.3, 0.1, 3., [0., 1., 1.], [0., 1., 1., 1.]),
    );
    let mut pointer = quad(0.3, 0.7, 3., [0.; 3], [0., 1., 0., 0.5]);
    pointer.extend(quad(0.75, 0.95, 0., [0.; 3], [1., 0., 0., 1.]));
    let pointer = mesh(&gpu.device, &gpu.queue, &pointer);
    let draw = |vertices, vertex_count| overlay::MeshDraw {
        vertices,
        instances: &instances,
        vertex_count,
    };
    let geometry = PaintGeometry {
        still: Some(draw(&still, 6)),
        marching: Some(draw(&marching, 6)),
        pointer: Some(draw(&pointer, 12)),
    };
    let render = |time, shown| {
        let record = presentation::records::Camera {
            view_proj: [
                1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 0.1, 0., 0., 0., 0., 1.,
            ],
            eye: [0., 0., 100.],
            width: SIZE as f32,
            height: SIZE as f32,
            time,
            ..Default::default()
        };
        let uniform = buffer(
            &gpu.device,
            &gpu.queue,
            "paint-camera",
            BufferUsages::UNIFORM,
            bytemuck::bytes_of(&record),
        );
        let camera = renderer_core::bind(
            &gpu.device,
            pipelines.layout("cameraLayout"),
            "paint-camera",
            &[uniform.as_entire_binding()],
        );
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        clear_depth(&mut encoder, &targets.depth, 0.3);
        paint.encode(&mut encoder, &targets, &camera, geometry, shown);
        let readback = read(&gpu.device, &mut encoder, &targets.paint);
        finish(&gpu, encoder, readback)
    };
    let pixels = render(0., true);
    close(at(&pixels, 9), [0.5, 0., 0., 1.]);
    close(at(&pixels, 29), [0., 0.5, 0.5, 1.]);
    close(at(&pixels, 48), [0., 0.25, 0., 0.5]);
    close(at(&pixels, 59), [0.; 4]);
    let pixels = render(0.5, true);
    close(at(&pixels, 9), [0.5, 0., 0., 1.]);
    close(at(&pixels, 29), [0.; 4]);
    close(at(&pixels, 48), [0., 0.25, 0., 0.5]);
    assert!(
        render(0.5, false).iter().all(|p| *p == [0.; 4]),
        "hidden paint must clear the previous frame"
    );
}
