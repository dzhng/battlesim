//! Shared display-space overlay geometry and halo composition.

use crate::pipelines::Pipelines;
use crate::Targets;
use wgpu::*;

struct Screen {
    pipeline: RenderPipeline,
    group: BindGroup,
}

/// Owns the production overlay targets, depth-tested geometry bindings and
/// display-space halo passes. Mesh packing and presentation policy stay in the
/// platform shell; pass order and target ownership stay here.
pub struct OverlayResources {
    glow: Buffer,
    geometry: RenderPipeline,
    rows: Option<Screen>,
    columns: Option<Screen>,
    composite: Screen,
}

fn bind(device: &Device, layout: &BindGroupLayout, resources: &[BindingResource<'_>]) -> BindGroup {
    crate::bind(device, layout, "shared-overlay", resources)
}

pub fn new(
    device: &Device,
    pipelines: &Pipelines,
    targets: &Targets,
    glow: &[u8],
    halo: bool,
) -> OverlayResources {
    let glow = crate::buffer(
        device,
        "overlay-glow",
        BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        glow,
    );
    let geometry = pipelines.render("overlayTranslucent").clone();
    let glow_binding = glow.as_entire_binding();
    let rows = halo.then(|| Screen {
        pipeline: pipelines.render("glowRows").clone(),
        group: bind(
            device,
            pipelines.layout("glowRowsLayout"),
            &[
                glow_binding.clone(),
                BindingResource::TextureView(&targets.overlay),
            ],
        ),
    });
    let columns = halo.then(|| Screen {
        pipeline: pipelines.render("glowColumns").clone(),
        group: bind(
            device,
            pipelines.layout("glowColumnsLayout"),
            &[
                glow_binding.clone(),
                BindingResource::TextureView(&targets.overlay_glow_rows),
            ],
        ),
    });
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
    OverlayResources {
        glow,
        geometry,
        rows,
        columns,
        composite,
    }
}

impl OverlayResources {
    pub fn write_glow(&self, queue: &Queue, bytes: &[u8]) {
        queue.write_buffer(&self.glow, 0, bytes);
    }

    pub fn encode_geometry(
        &self,
        encoder: &mut CommandEncoder,
        targets: &Targets,
        camera: &BindGroup,
        vertices: &Buffer,
        instances: &Buffer,
        vertex_count: u32,
    ) {
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("shared-overlay"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &targets.overlay_msaa,
                depth_slice: None,
                resolve_target: Some(&targets.overlay),
                ops: Operations {
                    load: LoadOp::Clear(Color::TRANSPARENT),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: &targets.depth,
                depth_ops: None,
                stencil_ops: None,
            }),
            ..Default::default()
        });
        pass.set_pipeline(&self.geometry);
        pass.set_bind_group(0, camera, &[]);
        pass.set_vertex_buffer(0, vertices.slice(..));
        pass.set_vertex_buffer(1, instances.slice(..));
        pass.draw(0..vertex_count, 0..1);
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
        if let (Some(rows), Some(columns)) = (&self.rows, &self.columns) {
            Self::screen(
                encoder,
                rows,
                &targets.overlay_glow_rows,
                LoadOp::Clear(Color::TRANSPARENT),
            );
            Self::screen(
                encoder,
                columns,
                &targets.overlay_glow,
                LoadOp::Clear(Color::TRANSPARENT),
            );
        }
        Self::screen(encoder, &self.composite, output, LoadOp::Load);
    }
}
