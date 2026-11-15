//! A model's lit colour never outruns its own vertex colours at an MSAA edge.
//! Vertex colour scales a material's albedo and is at most white, so a model
//! painted part black can be no brighter anywhere than the same model painted
//! all white. A thin sliver (a slat, a grille bar) is where this breaks: a
//! pixel it only partly covers shades once, and if its attributes are taken at
//! the pixel's centre, outside the triangle, they extrapolate past the
//! vertices into a colour no vertex has, a lone white speck once lit.
#[path = "support/effects.rs"]
mod effects;
#[path = "support/readback.rs"]
mod readback;
mod support;
use contract::identity::ContentId;
use renderer_core::runtime::AdmissionWork;

use presentation::records::{
    Appearance, Batch, Camera, Instance as ModelInstance, Joint, View, FOG_UNITS, HIGH_PASS, NONE,
};
use renderer_core::bundle::{Bundle, BundleSource, Coverage, Draw, Material, Mesh, Node, Shape};
use renderer_core::fog::Fog;
use renderer_core::high_air::HighAir;
use renderer_core::models::Models;
use renderer_core::post;
use std::sync::Arc;
use wgpu::*;

const SIZE: [u32; 2] = [128, 64];
const IDENTITY: [f32; 16] = [
    1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
];
const WHITE: [u8; 4] = [255; 4];
const BLACK: [u8; 4] = [0, 0, 0, 255];

/// One long sliver facing the camera, at most two pixels thick, painted with
/// these three vertex colours. Pixels just past its long top edge lie
/// opposite the second vertex, so painting that one black and taking colour
/// outside the triangle overshoots past white.
struct Sliver([[u8; 4]; 3]);

impl BundleSource for Sliver {
    fn content_id(&self, _: &str) -> Result<ContentId, String> {
        ContentId::bundle(contract::identity::bytes_hash(&self.0.concat()))
    }

    fn bundle(&self, _: &str) -> Result<Arc<Bundle>, String> {
        Ok(Arc::new(Bundle {
            shape: Shape::Static(vec![Node {
                name: "body".into(),
                tiers: vec![Mesh {
                    positions: vec![-0.9, -0.06, 0.3, 0.9, 0.0, 0.3, 0.9, 0.06, 0.3],
                    normals: [0, 0, 32767, 0].repeat(3),
                    uvs: vec![0.; 6],
                    colors: self.0.concat(),
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
                name: "paint".into(),
                // Dark enough that twice its light stays below white.
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
}

/// The sliver drawn alone through the model pass, graded, premultiplied.
fn drawn(gpu: &support::Gpu, sliver: &Sliver) -> Vec<[u8; 4]> {
    let view = View {
        view: IDENTITY,
        projection: IDENTITY,
        eye: [0., 0., 1., 0.],
        ..Default::default()
    };
    let camera = Camera {
        view_proj: IDENTITY,
        width: SIZE[0] as f32,
        height: SIZE[1] as f32,
        ..Default::default()
    };
    let (pipelines, targets, environment, effects, light, _shadow) =
        effects::stage(gpu, SIZE, &view, &camera);
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
    let mut models = Models::from_world(
        &gpu.device,
        &gpu.queue,
        &pipelines,
        sliver,
        &appearances,
        "body",
        |_| None,
        |_, _| None,
        &mut AdmissionWork::default(),
    )
    .unwrap();
    models.set_high_apart(true);
    let joint = Joint {
        x: [1., 0., 0., 0.],
        y: [0., 1., 0., 0.],
        z: [0., 0., 1., 0.],
    };
    let instance = ModelInstance {
        placement: [0.; 4],
        data: [0.; 4],
        tint: [1.; 4],
        scale: [1.; 4],
    };
    let batch = Batch {
        first: 0,
        count: 1,
        pass: HIGH_PASS,
        fog: FOG_UNITS,
        caster: NONE,
        ..Default::default()
    };
    models
        .prepare_records(&gpu.device, &gpu.queue, &[joint], &[instance], &[batch])
        .unwrap();
    let high = HighAir::new(&gpu.device, &pipelines, &targets);
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    post.encode(&mut encoder, post::PostMode::Look);
    high.encode(&mut encoder, &environment, &models, &fog, &effects, &post);
    read(gpu, high.texture(), encoder)
}

fn read(gpu: &support::Gpu, texture: &Texture, mut encoder: CommandEncoder) -> Vec<[u8; 4]> {
    // A 128-pixel RGBA8 row is already copy-aligned, so rows need no stripping.
    let row = SIZE[0] * 4;
    let buffer = gpu.device.create_buffer(&BufferDescriptor {
        label: Some("model-edges-readback"),
        size: u64::from(row * SIZE[1]),
        usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        TexelCopyBufferInfo {
            buffer: &buffer,
            layout: TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: None,
            },
        },
        texture.size(),
    );
    readback::pixels(&gpu.device, &gpu.queue, encoder, &buffer)
}

#[test]
fn a_sliver_is_never_brighter_than_its_brightest_vertex_colour() {
    let gpu = support::gpu().expect("real GPU");
    let white = drawn(&gpu, &Sliver([WHITE; 3]));
    let painted = drawn(&gpu, &Sliver([WHITE, BLACK, WHITE]));
    assert!(
        white.iter().any(|p| p[..3].iter().any(|v| *v > 40)),
        "the white sliver must be drawn and lit: brightest {:?}",
        white.iter().max_by_key(|p| p[0])
    );
    assert!(
        white.iter().any(|p| p[3] > 0 && p[3] < 255),
        "the sliver must have MSAA edge pixels"
    );
    assert!(
        white.iter().all(|p| p[..3].iter().all(|v| *v < 250)),
        "the white sliver must stay below white, or a brighter one can't show"
    );
    let brighter: Vec<_> = (0..white.len())
        .filter(|&i| (0..3).any(|c| painted[i][c] > white[i][c].saturating_add(1)))
        .map(|i| (i as u32 % SIZE[0], i as u32 / SIZE[0], painted[i], white[i]))
        .collect();
    assert!(
        brighter.is_empty(),
        "{} pixels of the part-black sliver outshine the white one, e.g. (x, y, painted, white) {:?}",
        brighter.len(),
        &brighter[..brighter.len().min(4)]
    );
}
