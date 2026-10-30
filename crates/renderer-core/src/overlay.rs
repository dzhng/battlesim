//! Shared display-space overlay geometry and halo composition.

use crate::pipelines::Pipelines;
use crate::Targets;
use wgpu::*;

/// A caller-owned mesh in the world's ten-float vertex layout, with the
/// shared placement/tint instance layout. Pass owners select its pipeline.
#[derive(Clone, Copy)]
pub struct MeshDraw<'a> {
    pub vertices: &'a Buffer,
    pub instances: &'a Buffer,
    pub vertex_count: u32,
}

impl MeshDraw<'_> {
    pub fn draw(&self, pass: &mut RenderPass) {
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.set_vertex_buffer(1, self.instances.slice(..));
        pass.draw(0..self.vertex_count, 0..1);
    }
}

#[derive(Clone, Copy, Default)]
pub struct Geometry<'a> {
    pub opaque: Option<MeshDraw<'a>>,
    pub translucent: Option<MeshDraw<'a>>,
}

struct Screen {
    pipeline: RenderPipeline,
    group: BindGroup,
}

/// Owns the display-space geometry and halo passes. Geometry loads the
/// silhouettes left by x-ray and always resolves, including an empty UI frame.
/// Opaque marks write the pre-grass depth copy; translucent marks only read it.
pub struct OverlayResources {
    glow: Buffer,
    opaque: RenderPipeline,
    translucent: RenderPipeline,
    rows: Screen,
    columns: Screen,
    composite: Screen,
    paint: Screen,
    no_glow: Buffer,
}

fn bind(device: &Device, layout: &BindGroupLayout, resources: &[BindingResource<'_>]) -> BindGroup {
    crate::bind(device, layout, "shared-overlay", resources)
}

pub fn new(
    device: &Device,
    queue: &Queue,
    pipelines: &Pipelines,
    targets: &Targets,
    glow: &[u8],
) -> OverlayResources {
    let glow = crate::buffer(
        device,
        queue,
        "overlay-glow",
        BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        glow,
    );
    let opaque = pipelines.render("overlayOpaque").clone();
    let translucent = pipelines.render("overlayTranslucent").clone();
    let glow_binding = glow.as_entire_binding();
    let rows = Screen {
        pipeline: pipelines.render("glowRows").clone(),
        group: bind(
            device,
            pipelines.layout("glowRowsLayout"),
            &[
                glow_binding.clone(),
                BindingResource::TextureView(&targets.overlay),
            ],
        ),
    };
    let columns = Screen {
        pipeline: pipelines.render("glowColumns").clone(),
        group: bind(
            device,
            pipelines.layout("glowColumnsLayout"),
            &[
                glow_binding.clone(),
                BindingResource::TextureView(&targets.overlay_glow_rows),
            ],
        ),
    };
    let composite = Screen {
        pipeline: pipelines.render("overlayComposite").clone(),
        group: bind(
            device,
            pipelines.layout("overlaySource"),
            &[
                glow_binding,
                BindingResource::TextureView(&targets.overlay),
                BindingResource::TextureView(&targets.overlay_glow),
            ],
        ),
    };
    // Paint inspection reads the stored mark itself, with no halo or lighting.
    let no_glow = crate::buffer(
        device,
        queue,
        "paint-no-glow",
        BufferUsages::UNIFORM,
        bytemuck::cast_slice(&[1.0f32, 0.0, 0.0, 0.0]),
    );
    let paint = Screen {
        pipeline: pipelines.render("overlayComposite").clone(),
        group: bind(
            device,
            pipelines.layout("overlaySource"),
            &[
                no_glow.as_entire_binding(),
                BindingResource::TextureView(&targets.paint),
                BindingResource::TextureView(&targets.overlay_glow),
            ],
        ),
    };
    OverlayResources {
        glow,
        opaque,
        translucent,
        rows,
        columns,
        composite,
        paint,
        no_glow,
    }
}

impl OverlayResources {
    /// Start an overlay frame when no earlier silhouette pass clears it.
    pub fn clear(&self, encoder: &mut CommandEncoder, targets: &Targets) {
        let _pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("overlay-clear"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &targets.overlay_msaa,
                depth_slice: None,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::TRANSPARENT),
                    store: StoreOp::Store,
                },
            })],
            ..Default::default()
        });
    }

    pub fn write_glow(&self, queue: &Queue, bytes: &[u8]) {
        queue.write_buffer(&self.glow, 0, bytes);
    }

    pub fn encode_geometry(
        &self,
        encoder: &mut CommandEncoder,
        targets: &Targets,
        camera: &BindGroup,
        geometry: Geometry<'_>,
    ) {
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("shared-overlay"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &targets.overlay_msaa,
                depth_slice: None,
                resolve_target: Some(&targets.overlay),
                ops: Operations {
                    load: LoadOp::Load,
                    store: StoreOp::Discard,
                },
            })],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: &targets.overlay_depth,
                depth_ops: Some(Operations {
                    load: LoadOp::Load,
                    store: StoreOp::Discard,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
        for (pipeline, mesh) in [
            (&self.opaque, geometry.opaque),
            (&self.translucent, geometry.translucent),
        ] {
            if let Some(mesh) = mesh {
                pass.set_pipeline(pipeline);
                pass.set_bind_group(0, camera, &[]);
                mesh.draw(&mut pass);
            }
        }
    }

    fn screen(
        encoder: &mut CommandEncoder,
        screen: &Screen,
        target: &TextureView,
        load: LoadOp<Color>,
    ) {
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("shared-overlay-screen"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: Operations {
                    load,
                    store: StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        pass.set_pipeline(&screen.pipeline);
        pass.set_bind_group(0, &screen.group, &[]);
        pass.draw(0..3, 0..1);
    }

    pub fn encode_composite(
        &self,
        encoder: &mut CommandEncoder,
        targets: &Targets,
        output: &TextureView,
    ) {
        Self::screen(
            encoder,
            &self.rows,
            &targets.overlay_glow_rows,
            LoadOp::Clear(Color::TRANSPARENT),
        );
        Self::screen(
            encoder,
            &self.columns,
            &targets.overlay_glow,
            LoadOp::Clear(Color::TRANSPARENT),
        );
        Self::screen(encoder, &self.composite, output, LoadOp::Load);
    }

    pub fn encode_paint(&self, encoder: &mut CommandEncoder, output: &TextureView) {
        Self::screen(encoder, &self.paint, output, LoadOp::Load);
    }
}

impl OverlayResources {
    pub fn record_memory(&self, memory: &mut crate::memory::ResourceMemory) {
        memory.buffer(&self.glow);
        memory.buffer(&self.no_glow);
    }
}
