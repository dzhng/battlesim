//! A static prop's far mesh is the fifth mesh slot: a batch at `FAR` draws
//! that mesh's triangles and pixels, needs no card atlas, and is counted at
//! its own tier.
mod support;
use renderer_core::runtime::AdmissionWork;

use contract::identity::ContentId;
use presentation::records::{Appearance, Batch, Instance, Joint, NONE, VIEW_PASS};
use presentation::static_world::props::DRAWS_PROP;
use presentation::view::{FAR, IMPOSTOR, MESH_SLOTS};
use renderer_core::bundle::{Bundle, BundleSource, Coverage, Draw, Material, Mesh, Node, Shape};
use renderer_core::models::{ArtStore, Models, Surface};
use renderer_core::{pipelines, Targets};
use std::sync::Arc;
use wgpu::*;

const SIZE: [u32; 2] = [64, 32];
const NAME: &str = "test_far";
const STATE: &str = "default";

/// `count` copies of one triangle spanning x from `left` to `right` (clip space).
fn triangles(left: f32, right: f32, count: u32) -> Mesh {
    let positions = [
        left,
        -0.8,
        0.5,
        right,
        -0.8,
        0.5,
        (left + right) / 2.0,
        0.8,
        0.5,
    ];
    Mesh {
        positions: positions.to_vec(),
        normals: [0, 0, 32767, 0].repeat(3),
        uvs: vec![0.; 6],
        colors: vec![255; 12],
        indices: [0, 1, 2].repeat(count as usize),
        draws: vec![Draw {
            material: 0,
            first: 0,
            count: 3 * count as usize,
        }],
        ..Default::default()
    }
}

/// Tiers 0 to 3 cover the right half with 5, 4, 3 and 2 triangles; the far
/// mesh is one triangle over the left half.
struct Source;
impl BundleSource for Source {
    fn bundle(&self, _: &str) -> Result<Arc<Bundle>, String> {
        let mut tiers: Vec<Mesh> = (0..4).map(|t| triangles(0.1, 0.9, 5 - t)).collect();
        tiers.push(triangles(-0.9, -0.1, 1));
        Ok(Arc::new(Bundle {
            shape: Shape::Static(vec![Node {
                name: STATE.into(),
                tiers,
                top: None,
            }]),
            materials: vec![Material {
                name: "opaque".into(),
                base_color: [0.5, 0.5, 0.5, 1.],
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

    fn content_id(&self, _: &str) -> Result<ContentId, String> {
        ContentId::bundle(contract::identity::bytes_hash(include_bytes!(
            "far_mesh.rs"
        )))
    }
}

fn models(gpu: &support::Gpu, pipelines: &pipelines::Pipelines) -> Models {
    let prop = Appearance {
        name_len: NAME.len() as u32,
        state: NAME.len() as u32,
        state_len: STATE.len() as u32,
        lies: NONE,
        draws: DRAWS_PROP,
        ..Default::default()
    };
    Models::from_world(
        &gpu.device,
        &gpu.queue,
        pipelines,
        &Source,
        &[prop],
        &format!("{NAME}{STATE}"),
        |_| None,
        |_, _| None,
        ArtStore::shared(),
        &mut AdmissionWork::default(),
    )
    .unwrap()
}

fn prepare(models: &mut Models, gpu: &support::Gpu, tier: i32) -> Result<(), String> {
    let joint = Joint {
        x: [1., 0., 0., 0.],
        y: [0., 1., 0., 0.],
        z: [0., 0., 1., 0.],
    };
    let instances = [Instance {
        tint: [1.; 4],
        scale: [1.; 4],
        ..Default::default()
    }; 2];
    let batch = Batch {
        count: 2,
        tier: tier as u32,
        pass: VIEW_PASS,
        caster: NONE,
        ..Default::default()
    };
    models.prepare_records(&gpu.device, &gpu.queue, &[joint], &instances, &[batch])
}

/// Which halves of the frame a flat pipeline over `models`' opaque draws
/// covers: (left, right).
fn covered(gpu: &support::Gpu, targets: &Targets, models: &Models) -> (bool, bool) {
    let shader = gpu.device.create_shader_module(ShaderModuleDescriptor {
        label: Some("far-mesh"),
        source: ShaderSource::Wgsl(
            r#"
@vertex fn vertex(@location(0) point: vec3f, @location(1) placement: vec4f) -> @builtin(position) vec4f {
    return vec4f(point.xy + placement.xy, point.z, 1.0);
}
@fragment fn fragment() -> @location(0) vec4f { return vec4f(1.0); }
"#
            .into(),
        ),
    });
    let buffers = [
        Some(VertexBufferLayout {
            array_stride: renderer_core::models::VERTEX_BYTES as u64,
            step_mode: VertexStepMode::Vertex,
            attributes: &vertex_attr_array![0 => Float32x3],
        }),
        Some(VertexBufferLayout {
            array_stride: size_of::<Instance>() as u64,
            step_mode: VertexStepMode::Instance,
            attributes: &vertex_attr_array![1 => Float32x4],
        }),
    ];
    let pipeline = gpu
        .device
        .create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("far-mesh"),
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
    let row = (SIZE[0] * 4).next_multiple_of(COPY_BYTES_PER_ROW_ALIGNMENT);
    let output = gpu.device.create_buffer(&BufferDescriptor {
        label: Some("far-mesh-readback"),
        size: u64::from(row * SIZE[1]),
        usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        targets.display_texture.as_image_copy(),
        TexelCopyBufferInfo {
            buffer: &output,
            layout: TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: None,
            },
        },
        targets.display_texture.size(),
    );
    gpu.queue.submit([encoder.finish()]);
    output.map_async(MapMode::Read, .., |r| r.unwrap());
    gpu.device.poll(PollType::wait_indefinitely()).unwrap();
    let mapped = output.get_mapped_range(..).unwrap();
    let opaque = |x: u32, y: u32| mapped[(y * row + x * 4 + 3) as usize] == 255;
    let half = |xs: std::ops::Range<u32>| (0..SIZE[1]).any(|y| xs.clone().any(|x| opaque(x, y)));
    (half(0..SIZE[0] / 2), half(SIZE[0] / 2..SIZE[0]))
}

#[test]
fn a_batch_at_the_far_tier_draws_the_far_mesh_without_a_card_atlas() {
    let gpu = support::gpu().expect("a real GPU");
    let pipelines = pipelines::build_builtin(&gpu.device).unwrap();
    let targets = Targets::new(&gpu.device, SIZE);
    let mut models = models(&gpu, &pipelines);

    // The far mesh: exactly its one triangle per instance, only its pixels.
    prepare(&mut models, &gpu, FAR).expect("a far batch needs no card atlas");
    models.submissions.reset();
    assert_eq!(covered(&gpu, &targets, &models), (true, false));
    let submitted = models.submissions.snapshot().color.models_including_props;
    assert_eq!((submitted.triangles, submitted.instances), (2, 2));
    // Counted at its own tier, beside the four every appearance has.
    let tiers = models.diagnostics()["tiers"].clone();
    assert_eq!(tiers.as_array().map(Vec::len), Some(MESH_SLOTS));
    assert_eq!(tiers[FAR as usize], 2);
    let props = models.drawn_instances()["props"].clone();
    let mut levels = vec![0; MESH_SLOTS];
    levels[FAR as usize] = 2;
    assert_eq!(
        props,
        serde_json::json!([{"appearance": NAME, "levels": levels}])
    );

    // Tier 3 draws its own two triangles where the far mesh does not.
    prepare(&mut models, &gpu, 3).unwrap();
    models.submissions.reset();
    assert_eq!(covered(&gpu, &targets, &models), (false, true));
    let submitted = models.submissions.snapshot().color.models_including_props;
    assert_eq!(submitted.triangles, 4);

    // A card is still only ever drawn from a baked atlas.
    let card = prepare(&mut models, &gpu, IMPOSTOR).unwrap_err();
    assert!(card.contains("atlas"), "{card}");
}
