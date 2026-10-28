//! Shared world-pass attachment setup.

use wgpu::*;

use crate::Targets;

use crate::fog::Layer;

#[derive(Clone, Copy, Debug)]
pub enum Surface {
    Opaque,
    Cutout,
    Room,
    Blended,
}

/// Resource-specific drawing for the production world pass. The core owns
/// the pass boundaries and order; a shell only adapts its ground, scenery,
/// models, fog groups and sky resources.
pub trait WorldInputs {
    fn draw_sky(&self, pass: &mut RenderPass<'_>);
    fn draw_ground(&self, pass: &mut RenderPass<'_>, layer: Layer);
    fn draw_skirt(&self, pass: &mut RenderPass<'_>, layer: Layer);
    fn bind_models(&self, pass: &mut RenderPass<'_>, layer: Layer, surface: Surface);
    fn draw_models(&self, pass: &mut RenderPass<'_>, layer: Layer, surface: Surface);
    fn draw_scenery(&self, pass: &mut RenderPass<'_>, layer: Layer);
    fn draw_grass(&self, pass: &mut RenderPass<'_>, layer: Layer);
    fn draw_backdrop(&self, pass: &mut RenderPass<'_>, layer: Layer);
    fn draw_water(&self, pass: &mut RenderPass<'_>, layer: Layer);
    fn bind_glass(&self, pass: &mut RenderPass<'_>);
    fn draw_glass(&self, pass: &mut RenderPass<'_>);
}

/// Begin the lit world pass with the shared HDR, fog-mask and reverse-Z
/// attachments. Resource-specific draw calls stay in the platform adapter.
pub fn begin_world_pass<'a>(
    encoder: &'a mut CommandEncoder,
    targets: &'a Targets,
    label: &'static str,
    first: bool,
    kept: bool,
) -> RenderPass<'a> {
    let store = if kept {
        StoreOp::Store
    } else {
        StoreOp::Discard
    };
    encoder.begin_render_pass(&RenderPassDescriptor {
        label: Some(label),
        color_attachments: &[
            Some(RenderPassColorAttachment {
                view: &targets.hdr_msaa,
                depth_slice: None,
                resolve_target: Some(&targets.lit),
                ops: Operations {
                    load: LoadOp::Load,
                    store,
                },
            }),
            Some(RenderPassColorAttachment {
                view: &targets.fog_mask_msaa,
                depth_slice: None,
                resolve_target: Some(&targets.fog_mask),
                ops: Operations {
                    load: if first {
                        LoadOp::Clear(Color::TRANSPARENT)
                    } else {
                        LoadOp::Load
                    },
                    store,
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
    })
}

/// Encode the sky attachment. This is separate from `encode_world` so a
/// shell with regional terrain uploads can submit those regions between the
/// sky and the final world pass without duplicating the production ordering.
pub fn encode_sky<I: WorldInputs>(inputs: &I, targets: &Targets, encoder: &mut CommandEncoder) {
    let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
        label: Some("sky"),
        color_attachments: &[Some(RenderPassColorAttachment {
            view: &targets.hdr_msaa,
            depth_slice: None,
            resolve_target: Some(&targets.lit),
            ops: Operations {
                load: LoadOp::Clear(Color::BLACK),
                store: StoreOp::Store,
            },
        })],
        ..Default::default()
    });
    inputs.draw_sky(&mut pass);
}

/// Encode the production lit-world sequence after the sky. `draw_ground`
/// allows scar-region shells to render terrain separately while sharing all
/// remaining world/material ordering.
pub fn encode_world<I: WorldInputs>(
    inputs: &I,
    targets: &Targets,
    encoder: &mut CommandEncoder,
    first: bool,
    kept: bool,
    draw_ground: bool,
) {
    let mut pass = begin_world_pass(encoder, targets, "world", first, kept);
    if draw_ground {
        inputs.draw_ground(&mut pass, Layer::PaintedGround);
    }
    inputs.draw_skirt(&mut pass, Layer::Faces);
    for layer in [
        Layer::Units,
        Layer::Faces,
        Layer::Ground,
        Layer::PaintedFaces,
    ] {
        for surface in [Surface::Opaque, Surface::Cutout, Surface::Room] {
            inputs.bind_models(&mut pass, layer, surface);
            inputs.draw_models(&mut pass, layer, surface);
        }
    }
    inputs.draw_scenery(&mut pass, Layer::Faces);
    inputs.draw_grass(&mut pass, Layer::PaintedGround);
    inputs.draw_backdrop(&mut pass, Layer::PaintedGround);
    inputs.draw_water(&mut pass, Layer::PaintedFaces);
    inputs.bind_glass(&mut pass);
    inputs.draw_glass(&mut pass);
}
