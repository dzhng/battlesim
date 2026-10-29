//! Authored bake pixels and actual relit cards through the shared model owner.
use native_client::{gpu, pipelines};
use presentation::records::{
    Appearance, Batch, Camera, Instance as ModelInstance, Joint, View, FOG_UNITS, NONE, VIEW_PASS,
};
use renderer_core::bundle::{Bundle, BundleSource, Coverage, Draw, Material, Mesh, Shape};
use renderer_core::impostor::Spec;
use renderer_core::models::Models;
use std::sync::Arc;
use wgpu::*;

fn identity() -> Joint {
    Joint {
        x: [1., 0., 0., 0.],
        y: [0., 1., 0., 0.],
        z: [0., 0., 1., 0.],
    }
}
struct Source;
impl BundleSource for Source {
    fn bundle(&self, _: &str) -> Result<Arc<Bundle>, String> {
        let positions = vec![
            -0.35, -0.2, 0., 0.3, -0.2, 0., -0.1, 0.25, 0., -0.1, 0., 0.8,
        ];
        Ok(Arc::new(Bundle {
            shape: Shape::Skinned(vec![Mesh {
                positions,
                normals: [0, 0, 32767, 0].repeat(4),
                uvs: vec![0.; 8],
                colors: vec![255; 16],
                joints: Some(vec![0; 16]),
                weights: Some([65535, 0, 0, 0].repeat(4)),
                indices: vec![0, 1, 2, 0, 1, 3, 1, 2, 3, 2, 0, 3],
                draws: vec![Draw {
                    material: 0,
                    first: 0,
                    count: 12,
                }],
                ..Default::default()
            }]),
            materials: vec![Material {
                name: "cloth".into(),
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
struct RigidSource(bool);
impl BundleSource for RigidSource {
    fn bundle(&self, name: &str) -> Result<Arc<Bundle>, String> {
        let mut bundle = Arc::unwrap_or_clone(Source.bundle(name)?);
        let Shape::Skinned(tiers) = bundle.shape else {
            unreachable!()
        };
        let node = renderer_core::bundle::Node {
            name: "body".into(),
            tiers,
            top: None,
        };
        bundle.shape = if self.0 {
            Shape::Articulated(vec![node])
        } else {
            Shape::Static(vec![node])
        };
        Ok(Arc::new(bundle))
    }
}

fn make_models(gpu: &gpu::Gpu, pipelines: &renderer_core::pipelines::Pipelines) -> Models {
    Models::from_world(
        &gpu.device,
        &gpu.queue,
        pipelines,
        &Source,
        &[Appearance {
            name_len: 4,
            lies: NONE,
            ..Default::default()
        }],
        "body",
        |_| Some(vec![identity()]),
    )
    .unwrap()
}
const MATRIX: [f32; 16] = [
    1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
];

fn read(gpu: &gpu::Gpu, mut encoder: CommandEncoder, image: &TextureView) -> Vec<[f32; 4]> {
    let shader = gpu.device.create_shader_module(ShaderModuleDescriptor {
        label: Some("impostor-pixels"),
        source: ShaderSource::Wgsl(
            r#"
@group(0) @binding(0) var image:texture_2d<f32>;
@group(0) @binding(1) var<storage,read_write> pixels:array<vec4f>;
@compute @workgroup_size(64) fn read(@builtin(global_invocation_id) id:vec3u){
let size=textureDimensions(image);if id.x>=size.x*size.y{return;}
pixels[id.x]=textureLoad(image,vec2i(i32(id.x%size.x),i32(id.x/size.x)),0);
}
"#
            .into(),
        ),
    });
    let pipeline = gpu
        .device
        .create_compute_pipeline(&ComputePipelineDescriptor {
            label: None,
            layout: None,
            module: &shader,
            entry_point: Some("read"),
            compilation_options: Default::default(),
            cache: None,
        });
    let output = gpu.device.create_buffer(&BufferDescriptor {
        label: None,
        size: 128 * 64 * 16,
        usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let group = renderer_core::bind(
        &gpu.device,
        &pipeline.get_bind_group_layout(0),
        "read",
        &[
            BindingResource::TextureView(image),
            output.as_entire_binding(),
        ],
    );
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(128, 1, 1);
    }
    let readback = gpu.device.create_buffer(&BufferDescriptor {
        label: None,
        size: 128 * 64 * 16,
        usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_buffer_to_buffer(&output, 0, &readback, 0, 128 * 64 * 16);
    gpu.queue.submit([encoder.finish()]);
    readback.map_async(MapMode::Read, .., |r| r.unwrap());
    gpu.device.poll(PollType::wait_indefinitely()).unwrap();
    let map = readback.get_mapped_range(..).unwrap();
    let pixels = bytemuck::cast_slice(&map).to_vec();
    drop(map);
    readback.unmap();
    pixels
}
#[test]
fn authored_bake_and_runtime_cards_preserve_coverage_normals_tint_and_real_geometry() {
    let gpu = gpu::open(&gpu::headless_instance(), None).expect("a real GPU");
    let pipelines = pipelines::build(&gpu.device).unwrap();
    let mut models = make_models(&gpu, &pipelines);
    let atlas = pollster::block_on(
        models
            .impostor_request(&gpu.device, &gpu.queue, &pipelines, 0, &Spec::runtime())
            .unwrap()
            .read(),
    )
    .unwrap();
    let second = pollster::block_on(
        models
            .impostor_request(&gpu.device, &gpu.queue, &pipelines, 0, &Spec::runtime())
            .unwrap()
            .read(),
    )
    .unwrap();
    assert_eq!(
        atlas.hash, second.hash,
        "identical GPU bake inputs must have the same identity"
    );
    let detached = {
        let old = make_models(&gpu, &pipelines);
        old.impostor_request(&gpu.device, &gpu.queue, &pipelines, 0, &Spec::runtime())
            .unwrap()
    };
    let detached = pollster::block_on(detached.read()).unwrap();
    assert_eq!(
        atlas.hash, detached.hash,
        "an owned bake must finish after its originating model catalog is dropped"
    );
    for articulated in [false, true] {
        let rigid = Models::from_world(
            &gpu.device,
            &gpu.queue,
            &pipelines,
            &RigidSource(articulated),
            &[Appearance {
                name_len: 4,
                lies: NONE,
                ..Default::default()
            }],
            "body",
            |_| Some(vec![identity()]),
        )
        .unwrap();
        let snapshot = rigid
            .impostor_request(&gpu.device, &gpu.queue, &pipelines, 0, &Spec::runtime())
            .unwrap();
        let rigid_atlas = pollster::block_on(snapshot.read()).unwrap();
        assert_eq!(atlas.hash,rigid_atlas.hash,"the same rigid authored body must bake identically through static/articulated review inputs");
    }
    let covered = atlas
        .albedo
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| p[3] > 0)
        .count();
    assert!(
        covered > 100 && covered < atlas.width as usize * atlas.height as usize / 2,
        "atlas must contain a bounded actual model: {covered}"
    );
    assert_eq!(
        &atlas.albedo[..4],
        &[0; 4],
        "atlas needs transparent framing margin"
    );
    let index = atlas
        .albedo
        .as_chunks::<4>()
        .0
        .iter()
        .position(|p| p[3] == 255)
        .expect("solid interior");
    let albedo = &atlas.albedo[index * 4..index * 4 + 3];
    let expected = [0.3f64, 0.6, 0.9].map(|v| (v.powf(1. / 2.2) * 255.).round() as u8);
    assert!(
        albedo.iter().zip(expected).all(|(a, b)| a.abs_diff(b) <= 1),
        "bake must retain unlit untinted albedo {albedo:?} vs {expected:?}"
    );
    assert_eq!(
        &atlas.normal[index * 4..index * 4 + 4],
        &[128, 128, 255, 255]
    );
    assert_eq!(&atlas.mask[index * 4..index * 4 + 4], &[255, 0, 0, 255]);
    if let Ok(directory) = std::env::var("IMPOSTOR_EVIDENCE_DIR") {
        std::fs::create_dir_all(&directory).unwrap();
        for (name, bytes) in [
            ("albedo", &atlas.albedo),
            ("normal", &atlas.normal),
            ("mask", &atlas.mask),
        ] {
            std::fs::write(format!("{directory}/{name}.rgba"), bytes).unwrap();
        }
        std::fs::write(format!("{directory}/atlas.json"),serde_json::to_vec(&serde_json::json!({"appearance":atlas.appearance,"spec":atlas.spec,"width":atlas.width,"height":atlas.height,"center":atlas.center,"radius":atlas.radius,"hash":atlas.hash})).unwrap()).unwrap();
    }
    pollster::block_on(models.bake_cards(&gpu.device, &gpu.queue, &pipelines)).unwrap();
    let instances =
        [(-0.5, [1., 0., 0., 1.]), (0.5, [0., 0., 1., 1.])].map(|(x, tint)| ModelInstance {
            placement: [x, 0., 0., 0.],
            scale: [0.8, 0.8, 0.8, 0.],
            tint,
            ..Default::default()
        });
    let batch = Batch {
        count: 2,
        tier: 4,
        fog: FOG_UNITS,
        pass: VIEW_PASS,
        caster: NONE,
        ..Default::default()
    };
    models
        .prepare_records(&gpu.device, &gpu.queue, &[identity()], &instances, &[batch])
        .unwrap();
    assert_eq!(models.drawn().impostors, 2);
    assert_eq!(
        models.drawn().triangles,
        4,
        "two actual cards, never coarsest posed meshes"
    );
    let targets = renderer_core::Targets::new(&gpu.device, [128, 64]);
    let game: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/game.json")).unwrap();
    let light: presentation::light::LightSettings =
        serde_json::from_value(game["presentation"]["light"].clone()).unwrap();
    let cold = presentation::shadow::SunShadowFrame::cold(&light);
    let shadow = renderer_core::shadow::ShadowResources::new(
        &gpu.device,
        &gpu.queue,
        &pipelines,
        16,
        bytemuck::bytes_of(&cold.receiver),
    );
    let sky = renderer_core::sky::Sky::new(&gpu.device, &gpu.queue, &pipelines, &light.sky());
    let mut environment = renderer_core::environment::Environment::new(
        &gpu.device,
        &gpu.queue,
        &pipelines,
        &sky.lut,
        &light.sun(),
        &light.aerial(),
        &shadow,
    );
    environment.prepare(
        &gpu.queue,
        &View {
            view: MATRIX,
            projection: MATRIX,
            eye: [0., 0., 3., 0.],
            ..Default::default()
        },
        &Camera {
            view_proj: MATRIX,
            eye: [0., 0., 3.],
            width: 128.,
            height: 64.,
            ..Default::default()
        },
    );
    let fog = renderer_core::fog::Fog::new(&gpu.device, &gpu.queue, &pipelines, &targets.depth);
    let render = |models: &Models, high: bool| {
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        {
            let attachments = [
                (&targets.hdr_msaa, &targets.lit),
                (&targets.fog_mask_msaa, &targets.fog_mask),
            ]
            .map(|(view, resolve)| {
                Some(RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: Some(resolve),
                    ops: Operations {
                        load: LoadOp::Clear(Color::TRANSPARENT),
                        store: StoreOp::Discard,
                    },
                })
            });
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                color_attachments: &attachments,
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
            let mut groups = environment.groups().to_vec();
            groups.push(fog.group(renderer_core::fog::Layer::Units));
            groups.push(models.card_group().unwrap());
            pipelines.bound("modelCards").set(&mut pass, &groups);
            if high {
                models.draw_high_air_cards(&mut pass, renderer_core::fog::Layer::Units);
            } else {
                models.draw_cards(&mut pass, renderer_core::fog::Layer::Units);
            }
        }
        read(&gpu, encoder, &targets.lit)
    };
    let pixels = render(&models, false);
    for (range, channel) in [(0..64, 0), (64..128, 2)] {
        let colors: Vec<_> = pixels
            .iter()
            .enumerate()
            .filter(|(i, p)| range.contains(&(i % 128)) && p[3] > 0.9)
            .map(|(_, p)| p)
            .collect();
        assert!(colors.len() > 20, "actual shaded card must cover pixels");
        let sum = colors.iter().fold([0f64; 3], |mut sum, pixel| {
            for c in 0..3 {
                sum[c] += f64::from(pixel[c]);
            }
            sum
        });
        assert!(sum[channel] > 3.0 * sum[1-channel/2],
            "masked cloth hue must dominate its card despite physical specular and filtered edges: {sum:?}");
    }
    let high_batch = Batch {
        pass: presentation::records::HIGH_PASS,
        ..batch
    };
    models.set_high_apart(true);
    models
        .prepare_records(
            &gpu.device,
            &gpu.queue,
            &[identity()],
            &instances,
            &[high_batch],
        )
        .unwrap();
    assert!(
        models.has_high_meshes(),
        "a card-only high frame must keep its output live"
    );
    assert!(
        render(&models, false).iter().all(|p| *p == [0.; 4]),
        "separate high cards must not leak into the ground view"
    );
    assert!(
        render(&models, true).iter().any(|p| p[3] > 0.9),
        "high output must draw actual cards"
    );
    models.set_surface_shown(renderer_core::models::Surface::Opaque, false);
    assert!(
        !models.has_high_meshes(),
        "opaque suppression hides the high card-only frame"
    );
    assert!(
        render(&models, true).iter().all(|p| *p == [0.; 4]),
        "surface suppression must hide cards too"
    );
}
