//! Explicit diagnostics must read the real submitted GPU data.
use native_client::{gpu, pipelines};
use renderer_core::probes::FogProbes;

#[test]
fn fog_shape_probe_executes_the_shared_shader_and_preserves_request_order() {
    let gpu = gpu::open(&gpu::headless_instance(), None).unwrap();
    let pipelines = pipelines::build(&gpu.device).unwrap();
    let probes = FogProbes::new(&pipelines);
    let rows = [
        [3.0, 2.0, 1.0, 0.0],
        [3.0, 2.0, 1.0, std::f32::consts::FRAC_PI_2],
        [3.0, 2.0, 1.0, std::f32::consts::PI],
        [3.0, 2.0, 1.0, std::f32::consts::FRAC_PI_4],
        [3.0, 2.0, 1.0, 3.0 * std::f32::consts::FRAC_PI_4],
    ];
    let values = pollster::block_on(
        probes
            .shape(&gpu.device, &gpu.queue, &rows)
            .unwrap()
            .read_f32(&gpu.device),
    )
    .unwrap();
    for (actual, expected) in values.iter().zip([3.0, 2.0, 1.0, 2.5, 1.5]) {
        assert!(
            (actual - expected).abs() < 1e-5,
            "expected {expected}, read {actual}"
        );
    }
    assert_eq!(values.len(), rows.len());
    assert!(pollster::block_on(
        probes
            .shape(&gpu.device, &gpu.queue, &[])
            .unwrap()
            .read_f32(&gpu.device)
    )
    .unwrap()
    .is_empty());
}

#[test]
fn display_readback_removes_row_padding_swizzles_bgra_and_keeps_the_encoded_frame() {
    use renderer_core::readback::{Readback, RgbaReadback};
    use wgpu::*;
    let gpu = gpu::open(&gpu::headless_instance(), None).unwrap();
    let texture = gpu.device.create_texture(&TextureDescriptor {
        label: Some("probe-display"),
        size: Extent3d {
            width: 3,
            height: 2,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Bgra8Unorm,
        usage: TextureUsages::COPY_SRC | TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let original = [
        3u8, 2, 1, 4, 7, 6, 5, 8, 11, 10, 9, 12, 15, 14, 13, 16, 19, 18, 17, 20, 23, 22, 21, 24,
    ];
    let write = |bytes: &[u8]| {
        gpu.queue.write_texture(
            texture.as_image_copy(),
            bytes,
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(12),
                rows_per_image: Some(2),
            },
            texture.size(),
        )
    };
    write(&original);
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    let read = RgbaReadback::encode(&gpu.device, &mut encoder, &texture).unwrap();
    gpu.queue.submit([encoder.finish()]);
    write(&[0; 24]);
    assert_eq!(
        pollster::block_on(read.read(&gpu.device)).unwrap(),
        (1u8..=24).collect::<Vec<_>>()
    );
    let source = renderer_core::buffer(
        &gpu.device,
        &gpu.queue,
        "probe-bounds",
        BufferUsages::COPY_SRC,
        &[0; 16],
    );
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    assert!(Readback::buffer(&gpu.device, &mut encoder, &source, 0..20).is_err());
    assert!(Readback::buffer(&gpu.device, &mut encoder, &source, 1..9).is_err());
}

#[test]
fn fog_point_probe_rebuilds_the_same_horizons_that_hide_a_wall_and_enforce_reach() {
    use presentation::fog::sight::{Eye, FogSight, Geometry, Occluder, Sensors, Shape, SightWorld};
    use renderer_core::{fog::Fog, probes::FogPoint, Targets};
    let gpu = gpu::open(&gpu::headless_instance(), None).unwrap();
    let pipelines = pipelines::build(&gpu.device).unwrap();
    let settings: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/presentation.json")).unwrap();
    let geometry: Geometry = serde_json::from_value(settings["fog_geometry"].clone()).unwrap();
    let mut sight = FogSight::new(geometry).unwrap();
    sight
        .set_world(
            SightWorld {
                nx: 2,
                ny: 2,
                spacing: 256.0,
                playable: [0.0, 0.0, 256.0, 256.0],
                sensors: Sensors {
                    fog_target_height_m: 0.0,
                    foliage_full_block: 1.0,
                    min_sight_gap_m: 0.0,
                },
                map: vec![Occluder {
                    x: 48.0,
                    y: 32.0,
                    yaw: 0.0,
                    hx: 1.0,
                    hy: 10.0,
                    base: 0.0,
                    top: 10.0,
                }],
                ids: vec![1.0],
                occluding: Default::default(),
            },
            vec![],
        )
        .unwrap();
    let eye = Eye {
        key: "test-eye".into(),
        position: [32.0, 32.0, 2.0],
        forward: 0.0,
        shape: Shape {
            front: 1.0,
            side: 1.0,
            rear: 1.0,
        },
        range: 80.0,
    };
    sight.set_sight(&[], Some(&[])).unwrap();
    sight.clear_sight();
    let targets = Targets::new(&gpu.device, [32, 32]);
    let mut fog = Fog::new(&gpu.device, &gpu.queue, &pipelines, &targets);
    let probes = FogProbes::new(&pipelines);
    let points: [[f32; 3]; 5] = [
        [40.0, 32.0, 0.0],
        [64.0, 32.0, 0.0],
        [32.0, 64.0, 0.0],
        [200.0, 200.0, 0.0],
        [300.0, 32.0, 0.0],
    ];
    let points: Vec<_> = points
        .into_iter()
        .map(|position| FogPoint {
            position,
            normal: None,
        })
        .collect();
    // Scenes with no fog source pass the canonical default upload, whose
    // point count is intentionally unset: every requested point is clear.
    assert_eq!(
        pollster::block_on(
            probes
                .probe(
                    &gpu.device,
                    &gpu.queue,
                    &mut fog,
                    &presentation::fog::upload::FogUpload::default(),
                    &points
                )
                .unwrap()
                .read_u8(&gpu.device)
        )
        .unwrap(),
        vec![1; points.len()]
    );
    let inactive = sight.prepare(None, 0, 0, points.len() as u32).unwrap();
    assert_eq!(
        pollster::block_on(
            probes
                .probe(&gpu.device, &gpu.queue, &mut fog, &inactive, &points)
                .unwrap()
                .read_u8(&gpu.device)
        )
        .unwrap(),
        vec![1; points.len()]
    );
    sight.set_sight(std::slice::from_ref(&eye), None).unwrap();
    let upload = sight.prepare(None, 0, 0, points.len() as u32).unwrap();
    assert!(
        !upload.rebuild.is_empty(),
        "first probe must build actual GPU maps"
    );
    let seen = pollster::block_on(
        probes
            .probe(&gpu.device, &gpu.queue, &mut fog, &upload, &points)
            .unwrap()
            .read_u8(&gpu.device),
    )
    .unwrap();
    assert_eq!(seen, [1, 0, 1, 0, 0]);
    let upload = sight.prepare(None, 0, 0, points.len() as u32).unwrap();
    assert!(upload.rebuild.is_empty(), "held eye uses its existing map");
    assert_eq!(
        pollster::block_on(
            probes
                .probe(&gpu.device, &gpu.queue, &mut fog, &upload, &points)
                .unwrap()
                .read_u8(&gpu.device)
        )
        .unwrap(),
        seen
    );

    let camera = presentation::view::Camera {
        eye: [32.0, -16.0, 64.0],
        target: [32.0, 32.0, 0.0],
        up: [0.0, 0.0, 1.0],
        fov_y: 1.0,
        near: 0.1,
        size: [32, 32],
    };
    let camera = presentation::view::View::new(&camera).camera(&camera, 0.0);
    let upload = sight.prepare(Some(&camera), 32, 32, 0).unwrap();
    let tiles = (upload.tiles[0] * upload.tiles[1]) as usize;
    fog.upload(&gpu.device, &gpu.queue, &upload);
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    {
        let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &targets.depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(0.5),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
    }
    fog.encode(&mut encoder);
    gpu.queue.submit([encoder.finish()]);
    let prior_counts = pollster::block_on(
        FogProbes::tile_counts(&gpu.device, &gpu.queue, &fog)
            .unwrap()
            .read_u32(&gpu.device),
    )
    .unwrap();
    assert_eq!(prior_counts.len(), tiles);
    assert_eq!(
        pollster::block_on(
            FogProbes::whole_flags(&gpu.device, &gpu.queue, &fog, 1)
                .unwrap()
                .read_u8(&gpu.device)
        )
        .unwrap(),
        [1]
    );
    sight.invalidate();
    let upload = sight.prepare(None, 0, 0, points.len() as u32).unwrap();
    let pending = probes
        .probe(&gpu.device, &gpu.queue, &mut fog, &upload, &points)
        .unwrap();
    assert_eq!(
        pollster::block_on(pending.read_u8(&gpu.device)).unwrap(),
        seen
    );
    assert_eq!(
        pollster::block_on(
            FogProbes::tile_counts(&gpu.device, &gpu.queue, &fog)
                .unwrap()
                .read_u32(&gpu.device)
        )
        .unwrap(),
        prior_counts,
        "explicit point probes retain last frame's screen-tile data"
    );
    assert_eq!(
        pollster::block_on(
            FogProbes::whole_flags(&gpu.device, &gpu.queue, &fog, 1)
                .unwrap()
                .read_u8(&gpu.device)
        )
        .unwrap(),
        [1]
    );
    // Crossing the wall reverses its occlusion direction. An empty probe must
    // still rebuild that changed map before the next point request or frame.
    sight
        .set_sight(
            &[Eye {
                position: [80.0, 32.0, 2.0],
                ..eye.clone()
            }],
            None,
        )
        .unwrap();
    let empty_upload = sight.prepare(None, 0, 0, 0).unwrap();
    assert!(!empty_upload.rebuild.is_empty());
    assert!(pollster::block_on(
        probes
            .probe(&gpu.device, &gpu.queue, &mut fog, &empty_upload, &[])
            .unwrap()
            .read_u8(&gpu.device)
    )
    .unwrap()
    .is_empty());
    let upload = sight.prepare(None, 0, 0, points.len() as u32).unwrap();
    assert!(upload.rebuild.is_empty());
    let mirrored = pollster::block_on(
        probes
            .probe(&gpu.device, &gpu.queue, &mut fog, &upload, &points)
            .unwrap()
            .read_u8(&gpu.device),
    )
    .unwrap();
    assert_eq!(
        &mirrored[..2],
        &[0, 1],
        "the empty probe must have rebuilt the reversed wall horizon"
    );
    // Another sight publication moves the eye out of reach of the wall. Point
    // probing consumes maps; the next actual view must still clear whole flags.
    sight
        .set_sight(
            &[Eye {
                position: [240.0, 240.0, 2.0],
                range: 20.0,
                ..eye
            }],
            None,
        )
        .unwrap();
    let upload = sight.prepare(None, 0, 0, points.len() as u32).unwrap();
    assert_eq!(
        pollster::block_on(
            probes
                .probe(&gpu.device, &gpu.queue, &mut fog, &upload, &points)
                .unwrap()
                .read_u8(&gpu.device)
        )
        .unwrap(),
        [0; 5]
    );
    let frame_upload = sight.prepare(Some(&camera), 32, 32, 0).unwrap();
    fog.upload(&gpu.device, &gpu.queue, &frame_upload);
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    fog.encode(&mut encoder);
    gpu.queue.submit([encoder.finish()]);
    assert_eq!(
        pollster::block_on(
            FogProbes::whole_flags(&gpu.device, &gpu.queue, &fog, 1)
                .unwrap()
                .read_u8(&gpu.device)
        )
        .unwrap(),
        [0],
        "the next actual frame must still run whole-structure visibility after a probe"
    );
}

#[test]
fn grass_probes_decode_actual_tier_buffers_and_snapshot_counts_before_a_later_frame() {
    use presentation::static_world::ground::grass::Window;
    use renderer_core::{
        grass::{Grass, GrassData},
        probes::grass::{queue_clumps, queue_counts},
    };
    use wgpu::*;
    let gpu = gpu::open(&gpu::headless_instance(), None).unwrap();
    let pipelines = pipelines::build(&gpu.device).unwrap();
    let heights = renderer_core::buffer(
        &gpu.device,
        &gpu.queue,
        "probe-flat-ground",
        BufferUsages::STORAGE,
        &[0; 16],
    );
    let mut grass = Grass::from_data(
        &gpu.device,
        &gpu.queue,
        &pipelines,
        &heights,
        GrassData {
            params: &[0; 1024],
            props: &[0; 4],
            shapes: &[0.0; 16],
            max_blades: 1,
            capacity: [2, 2],
        },
    );
    assert_eq!(
        pollster::block_on(
            queue_counts(&gpu.device, &gpu.queue, &grass)
                .unwrap()
                .read(&gpu.device)
        )
        .unwrap()
        .near,
        0
    );
    grass.prepare_records(
        &gpu.device,
        &gpu.queue,
        Some((
            Window {
                x0: 0.0,
                y0: 0.0,
                tiles_x: 1,
                tiles_y: 1,
                reach: 4.0,
            },
            1.0,
        )),
        &presentation::records::Camera::default(),
        |_, _| {},
    );
    let shader = gpu.device.create_shader_module(ShaderModuleDescriptor {
        label: Some("controlled-grass-output"),
        source: ShaderSource::Wgsl(
            r#"
@group(0) @binding(0) var<storage,read_write> args: array<u32>;
@group(0) @binding(1) var<storage,read_write> clumps: array<u32>;
@compute @workgroup_size(1) fn fill() {
  args[1]=1u; args[6]=1u; args[10]=7u; args[11]=9u;
  clumps[0]=bitcast<u32>(1.0); clumps[1]=bitcast<u32>(2.0); clumps[2]=bitcast<u32>(3.0);
  clumps[3]=bitcast<u32>(0.4); clumps[4]=0x80402010u; clumps[5]=0u;
  clumps[24]=bitcast<u32>(7.0); clumps[25]=bitcast<u32>(8.0); clumps[26]=bitcast<u32>(9.0);
  clumps[27]=bitcast<u32>(0.8); clumps[28]=0xffa08060u; clumps[29]=1u;
}"#
            .into(),
        ),
    });
    let pipeline = gpu
        .device
        .create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("controlled-grass-output"),
            layout: None,
            module: &shader,
            entry_point: Some("fill"),
            compilation_options: Default::default(),
            cache: None,
        });
    let source = grass.probe_source().unwrap();
    let args = source.args.clone();
    let group = renderer_core::bind(
        &gpu.device,
        &pipeline.get_bind_group_layout(0),
        "controlled-grass-output",
        &[
            source.args.as_entire_binding(),
            source.clumps.as_entire_binding(),
        ],
    );
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }
    gpu.queue.submit([encoder.finish()]);
    let counts = queue_counts(&gpu.device, &gpu.queue, &grass).unwrap();
    let clumps = queue_clumps(
        &gpu.device,
        &gpu.queue,
        &grass,
        &["short".into(), "tall".into()],
    )
    .unwrap();
    // Snapshot ownership releases grass before mapping and preserves both tier
    // counts even if the next submitted frame immediately resets arguments.
    grass.regrow();
    gpu.queue
        .write_buffer(&args, 0, bytemuck::cast_slice(&[0u32; 12]));
    let counts = pollster::block_on(counts.read(&gpu.device)).unwrap();
    assert_eq!(
        (counts.near, counts.far, counts.near_found, counts.far_found),
        (1, 1, 7, 9)
    );
    let clumps = pollster::block_on(clumps.read(&gpu.device)).unwrap();
    assert_eq!(clumps.len(), 2);
    assert_eq!(clumps[0].root, [1.0, 2.0, 3.0]);
    assert_eq!(clumps[1].root, [7.0, 8.0, 9.0]);
    assert_eq!((clumps[0].tier, clumps[1].tier), (0, 1));
    assert_eq!((&*clumps[0].kind, &*clumps[1].kind), ("short", "tall"));
    assert_eq!(clumps[0].height, 0.4);
    assert_eq!(
        clumps[0].colour,
        [
            (16.0f64 / 255.0).powi(2),
            (32.0f64 / 255.0).powi(2),
            (64.0f64 / 255.0).powi(2)
        ]
    );
    assert_eq!(clumps[0].laid, 1.0 - 128.0 / 255.0);
    assert_eq!(clumps[1].laid, 0.0);
}
