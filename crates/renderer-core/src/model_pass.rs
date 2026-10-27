//! Shared authored-model prepass and lit draw encoding.

use wgpu::*;

use crate::pipelines::Pipelines;
use crate::Targets;

/// Encode the model depth prepass over the supplied packed vertex stream.
#[allow(clippy::too_many_arguments)]
pub fn encode_prepass(
    encoder: &mut CommandEncoder,
    targets: &Targets,
    pipelines: &Pipelines,
    camera: &BindGroup,
    materials: &BindGroup,
    vertices: &Buffer,
    instances: &Buffer,
    vertex_count: u32,
    instance_count: u32,
    runs: Option<&[(u32, u32, u32, u32)]>,
) {
    let pipeline = pipelines.bound("modelPrepass");
    let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
        label: Some("shared-production-model-prepass"),
        color_attachments: &[],
        depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
            view: &targets.depth,
            depth_ops: Some(Operations {
                load: LoadOp::Load,
                store: StoreOp::Store,
            }),
            stencil_ops: None,
        }),
        ..Default::default()
    });
    pipeline.set(
        &mut pass,
        &[("typegpuCameraLayout", camera), ("modelLayout", materials)],
    );
    pass.set_vertex_buffer(0, vertices.slice(..));
    pass.set_vertex_buffer(1, instances.slice(..));
    draw_runs(&mut pass, vertex_count, instance_count, runs);
}

/// Encode the lit model pass over the shared world and fog attachments.
#[allow(clippy::too_many_arguments)]
pub fn encode_opaque(
    encoder: &mut CommandEncoder,
    targets: &Targets,
    pipelines: &Pipelines,
    camera: &BindGroup,
    materials: &BindGroup,
    fog: &BindGroup,
    environment: &BindGroup,
    vertices: &Buffer,
    instances: &Buffer,
    vertex_count: u32,
    instance_count: u32,
    runs: Option<&[(u32, u32, u32, u32)]>,
) {
    let pipeline = pipelines.bound("modelOpaque");
    let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
        label: Some("shared-model-opaque"),
        color_attachments: &[
            Some(RenderPassColorAttachment {
                view: &targets.hdr_msaa,
                depth_slice: None,
                resolve_target: Some(&targets.lit),
                ops: Operations {
                    load: LoadOp::Load,
                    store: StoreOp::Store,
                },
            }),
            Some(RenderPassColorAttachment {
                view: &targets.fog_mask_msaa,
                depth_slice: None,
                resolve_target: Some(&targets.fog_mask),
                ops: Operations {
                    load: LoadOp::Load,
                    store: StoreOp::Store,
                },
            }),
        ],
        depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
            view: &targets.depth,
            depth_ops: Some(Operations {
                load: LoadOp::Load,
                store: StoreOp::Store,
            }),
            stencil_ops: None,
        }),
        ..Default::default()
    });
    pipeline.set(
        &mut pass,
        &[
            ("typegpuCameraLayout", camera),
            ("modelLayout", materials),
            ("fogLayout", fog),
            ("environment", environment),
        ],
    );
    pass.set_vertex_buffer(0, vertices.slice(..));
    pass.set_vertex_buffer(1, instances.slice(..));
    draw_runs(&mut pass, vertex_count, instance_count, runs);
}

fn draw_runs<'a>(
    pass: &mut RenderPass<'a>,
    vertex_count: u32,
    instance_count: u32,
    runs: Option<&[(u32, u32, u32, u32)]>,
) {
    if let Some(runs) = runs {
        for &(first, count, vertex_first, vertex_count) in runs {
            pass.draw(
                vertex_first..vertex_first.saturating_add(vertex_count),
                first..first.saturating_add(count),
            );
        }
    } else {
        pass.draw(0..vertex_count, 0..instance_count);
    }
}
