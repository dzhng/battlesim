//! High air over the finished world and page overlays. Spent world colour,
//! fog mask and depth targets serve again; only the transparent output is new.
use wgpu::*;

use crate::environment::Environment;
use crate::fog::{Fog, Layer};
use crate::models::{Models, Surface};
use crate::pipelines::{Bound, Pipelines};
use crate::post::PostResources;
use crate::{Targets, DISPLAY_FORMAT};

pub struct HighAir {
    texture: Texture,
    output: TextureView,
    solid: Bound,
    cutout: Bound,
    surfaces: [(Bound, Surface); 3],
    glass: Bound,
    cards: Bound,
}

impl HighAir {
    pub fn new(device: &Device, pipelines: &Pipelines, targets: &Targets) -> Self {
        let texture = device.create_texture(&TextureDescriptor {
            label: Some("high-air-output"),
            size: Extent3d {
                width: targets.size[0],
                height: targets.size[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: DISPLAY_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT
                | TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        Self {
            output: texture.create_view(&Default::default()),
            texture,
            solid: pipelines.bound("modelPrepass"),
            cutout: pipelines.bound("modelPrepassCutout"),
            surfaces: [
                (pipelines.bound("modelOpaque"), Surface::Opaque),
                (pipelines.bound("modelCutout"), Surface::Cutout),
                (pipelines.bound("modelRooms"), Surface::Room),
            ],
            glass: pipelines.bound("modelBlended"),
            cards: pipelines.bound("modelCards"),
        }
    }

    pub fn output(&self) -> &TextureView {
        &self.output
    }

    pub fn texture(&self) -> &Texture {
        &self.texture
    }

    /// Debug views keep high air in the inspected world and clear this plane.
    pub fn clear(&self, encoder: &mut CommandEncoder) {
        encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("high-air-clear"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &self.output,
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

    /// Encode after world post and overlay composition have consumed their
    /// targets. This depth starts empty: world geometry never hides high air.
    pub fn encode(
        &self,
        encoder: &mut CommandEncoder,
        targets: &Targets,
        environment: &Environment,
        models: &Models,
        fog: &Fog,
        post: &PostResources,
    ) {
        if !models.has_high_meshes() {
            self.clear(encoder);
            return;
        }
        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("high-air-depth"),
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &targets.depth,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(0.0),
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            let camera = ("cameraLayout", &environment.camera);
            for (pipeline, surface) in [
                (&self.solid, Surface::Solid),
                (&self.cutout, Surface::Cutout),
            ] {
                pipeline.set(&mut pass, &[camera, models.group()]);
                models.draw_high_air(&mut pass, surface);
            }
        }
        {
            let attachment = |view, resolve_target| {
                Some(RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target,
                    ops: Operations {
                        load: LoadOp::Clear(Color::TRANSPARENT),
                        store: StoreOp::Discard,
                    },
                })
            };
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("high-air"),
                color_attachments: &[
                    attachment(&targets.hdr_msaa, Some(&targets.hdr)),
                    attachment(&targets.fog_mask_msaa, None),
                ],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &targets.depth,
                    depth_ops: Some(Operations {
                        load: LoadOp::Load,
                        store: StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            for layer in [
                Layer::Units,
                Layer::Faces,
                Layer::Ground,
                Layer::PaintedFaces,
            ] {
                let mut groups = environment.groups().to_vec();
                groups.extend([models.group(), fog.group(layer)]);
                for (pipeline, surface) in &self.surfaces {
                    pipeline.set(&mut pass, &groups);
                    models.draw_high_air_fogged(&mut pass, *surface, layer);
                }
                if let Some(cards) = models.card_group() {
                    let mut groups = environment.groups().to_vec();
                    groups.extend([fog.group(layer), cards]);
                    self.cards.set(&mut pass, &groups);
                    models.draw_high_air_cards(&mut pass, layer);
                }
            }
            let mut groups = environment.groups().to_vec();
            groups.push(models.group());
            self.glass.set(&mut pass, &groups);
            models.draw_high_air(&mut pass, Surface::Blended);
        }
        post.encode_high_plane(encoder, &self.output);
    }

    pub fn record_memory(&self, memory: &mut crate::memory::ResourceMemory) {
        memory.texture(&self.texture);
    }
}
