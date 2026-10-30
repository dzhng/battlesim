//! Ground marks rasterized against world-only depth, then sampled by painted
//! world materials. The exported shaders own pull, pulse and color-range rules.
use bytemuck::{Pod, Zeroable};
use serde::Deserialize;
use wgpu::*;

use crate::overlay::MeshDraw;
use crate::pipelines::{Bound, Pipelines};
use crate::Targets;

/// Exact `GroundPaintStyle` shader uniform; configuration names remain those
/// of the authored paint style, and alignment words are never app inputs.
#[repr(C)]
#[derive(Clone, Copy, Debug, Deserialize, Pod, Zeroable)]
#[serde(deny_unknown_fields)]
pub struct Paint {
    pub albedo: f32,
    pub emissive: f32,
    pub fog_keep: f32,
    pub grass_glow: f32,
    pub grass_falloff_m: f32,
    pub saturation: f32,
    #[serde(skip)]
    pub pads: [f32; 2],
}

impl Paint {
    pub fn validate(&self) -> Result<(), String> {
        if !(self.albedo > 0.0 && self.albedo <= 1.0)
            || !(self.emissive >= 0.0 && self.emissive <= 8.0)
            || !(self.fog_keep >= 0.0 && self.fog_keep <= 1.0)
            || !(self.grass_glow >= 0.0 && self.grass_glow <= 8.0)
            || !(self.grass_falloff_m > 0.0 && self.grass_falloff_m <= 4.0)
            || !(self.saturation >= 0.0 && self.saturation <= 3.0)
        {
            return Err("ground paint: albedo in (0, 1], emissive in [0, 8], fog_keep in [0, 1], grass_glow in [0, 8], grass_falloff_m in (0, 4], saturation in [0, 3]".into());
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Default)]
pub struct PaintGeometry<'a> {
    pub still: Option<MeshDraw<'a>>,
    pub marching: Option<MeshDraw<'a>>,
    pub pointer: Option<MeshDraw<'a>>,
}

pub struct PaintResources {
    still: Bound,
    marching: Bound,
}

impl PaintResources {
    pub fn new(pipelines: &Pipelines) -> Self {
        Self {
            still: pipelines.bound("paintStill"),
            marching: pipelines.bound("paintMarching"),
        }
    }

    /// Always clear, including hidden/empty frames. The scratch multisampled
    /// overlay is discarded: x-ray clears it before display overlays load it.
    pub fn encode(
        &self,
        encoder: &mut CommandEncoder,
        targets: &Targets,
        camera: &BindGroup,
        geometry: PaintGeometry<'_>,
        shown: bool,
    ) {
        // Metal can skip resolves on untouched tiles; clear the stored paint
        // independently so removing a mark cannot retain the preceding frame.
        encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("ground-paint-clear"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &targets.paint,
                depth_slice: None,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::TRANSPARENT),
                    store: StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("ground-paint"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &targets.overlay_msaa,
                depth_slice: None,
                resolve_target: Some(&targets.paint),
                ops: Operations {
                    load: LoadOp::Clear(Color::TRANSPARENT),
                    store: StoreOp::Discard,
                },
            })],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: &targets.depth,
                depth_ops: None,
                stencil_ops: None,
            }),
            ..Default::default()
        });
        if shown {
            for (pipeline, mesh) in [
                (&self.still, geometry.still),
                (&self.marching, geometry.marching),
                (&self.still, geometry.pointer),
            ] {
                if let Some(mesh) = mesh {
                    pipeline.set(&mut pass, &[("cameraLayout", camera)]);
                    mesh.draw(&mut pass);
                }
            }
        }
    }
}
