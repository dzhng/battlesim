use super::{box_filter, camera, Atlas, Spec};
use crate::pipelines::Pipelines;
use crate::readback::RgbaReadback;
use wgpu::*;

pub(crate) struct BakeInput {
    pub appearance: String,
    pub bounds: [[f64; 3]; 2],
    pub vertices: Buffer,
    pub indices: Buffer,
    pub first: u32,
    pub count: u32,
    pub models: BindGroup,
    pub spec: Spec,
}

/// An already-submitted bake, independent of a renderer/catalog borrow. Queue
/// order preserves its material/pose snapshot while replacement inputs arrive.
pub struct BakeRequest {
    device: Device,
    reads: Vec<RgbaReadback>,
    atlas: Atlas,
    width: u32,
    height: u32,
}

impl BakeRequest {
    pub(crate) fn new(
        device: &Device,
        queue: &Queue,
        pipelines: &Pipelines,
        input: BakeInput,
    ) -> Result<Self, String> {
        input
            .spec
            .validate(device.limits().max_texture_dimension_2d)?;
        if input
            .bounds
            .iter()
            .flatten()
            .any(|value| !value.is_finite())
        {
            return Err("impostor appearance has no finite bounds".into());
        }
        let spec = &input.spec;
        let pipeline = pipelines.bound("modelImpostor");
        let camera_layout = pipelines.layout("cameraLayout");
        let (center, radius) = camera::frame(input.bounds);
        let px = spec.cell * spec.supersample;
        let (width, height) = (spec.yaws * px, spec.pitches.len() as u32 * px);
        let texture = |label, format, usage| {
            device.create_texture(&TextureDescriptor {
                label: Some(label),
                size: Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            })
        };
        let outputs = ["impostor-albedo", "impostor-normal", "impostor-mask"].map(|label| {
            texture(
                label,
                TextureFormat::Rgba8Unorm,
                TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
            )
        });
        let views = outputs
            .each_ref()
            .map(|t| t.create_view(&Default::default()));
        let depth = texture(
            "impostor-depth",
            TextureFormat::Depth32Float,
            TextureUsages::RENDER_ATTACHMENT,
        )
        .create_view(&Default::default());
        let mut cameras = Vec::new();
        for &pitch in &spec.pitches {
            for column in 0..spec.yaws {
                let yaw = std::f64::consts::TAU * f64::from(column) / f64::from(spec.yaws);
                let camera = camera::camera(center, radius, yaw, pitch);
                let uniform = crate::buffer(
                    device,
                    queue,
                    "impostor-camera",
                    BufferUsages::UNIFORM,
                    bytemuck::bytes_of(&camera),
                );
                cameras.push(crate::bind(
                    device,
                    camera_layout,
                    "impostor-camera",
                    &[uniform.as_entire_binding()],
                ));
            }
        }
        let instance = presentation::records::Instance {
            tint: [1.; 4],
            scale: [1., 1., 1., 0.],
            ..Default::default()
        };
        let instance = crate::buffer(
            device,
            queue,
            "impostor-model",
            BufferUsages::VERTEX,
            bytemuck::bytes_of(&instance),
        );
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("impostor-bake"),
        });
        {
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
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("impostor-atlas"),
                color_attachments: &attachments,
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &depth,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(0.),
                        store: StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            for (index, camera) in cameras.iter().enumerate() {
                let (x, y) = (index as u32 % spec.yaws * px, index as u32 / spec.yaws * px);
                pass.set_viewport(x as f32, y as f32, px as f32, px as f32, 0., 1.);
                pass.set_scissor_rect(x, y, px, px);
                pipeline.set(
                    &mut pass,
                    &[("cameraLayout", camera), ("modelLayout", &input.models)],
                );
                pass.set_vertex_buffer(0, input.vertices.slice(..));
                pass.set_vertex_buffer(1, instance.slice(..));
                pass.set_index_buffer(input.indices.slice(..), IndexFormat::Uint32);
                pass.draw_indexed(input.first..input.first + input.count, 0, 0..1);
            }
        }
        let reads = outputs
            .iter()
            .map(|texture| RgbaReadback::encode(device, &mut encoder, texture))
            .collect::<Result<Vec<_>, _>>()?;
        queue.submit([encoder.finish()]);
        let atlas = Atlas {
            appearance: input.appearance,
            spec: spec.clone(),
            width: width / spec.supersample,
            height: height / spec.supersample,
            center,
            radius,
            albedo: Vec::new(),
            normal: Vec::new(),
            mask: Vec::new(),
            hash: String::new(),
        };
        Ok(Self {
            device: device.clone(),
            reads,
            atlas,
            width,
            height,
        })
    }

    pub async fn read(self) -> Result<Atlas, String> {
        let Self {
            device,
            reads,
            mut atlas,
            width,
            height,
        } = self;
        let mut channels = Vec::new();
        for readback in reads {
            channels.push(box_filter(
                &readback.read(&device).await?,
                width,
                height,
                atlas.spec.supersample,
            ));
        }
        let mut channels = channels.into_iter();
        atlas.albedo = channels.next().expect("albedo");
        atlas.normal = channels.next().expect("normal");
        atlas.mask = channels.next().expect("mask");
        atlas.hash();
        Ok(atlas)
    }
}
