//! The fog mask passes (`frame/fogMaskPass.ts`): how unseen looks on screen.
//! The world pass resolves its fog mask (unseen, seen and ground coverage
//! per pixel) beside the lit world; then, in screen space, the distances to
//! each side of the edge along rows and down columns (only when the look
//! draws an edge), the fogged world composed from the lit one into post's
//! input, and after post the rim along the edge on the seen side. The look
//! is the presentation crate's `FogLook` record, uploaded as it is.
use presentation::records::FogLook;
use wgpu::*;

use crate::frame::{Inputs, Pass, Targets};
use crate::gpu::{bind, buffer};
use crate::pipelines::Pipelines;

/// `FogMaskView`: what compose draws (0 the fogged world; the lab's mask
/// views are not a native view).
const VIEW_BYTES: usize = 16;

struct Screen {
    pipeline: RenderPipeline,
    group: BindGroup,
}

/// The distances (when the look has an edge) and the fogged world.
pub struct FogMask {
    distances: Option<[(Screen, TextureView); 2]>,
    compose: Screen,
}

/// The rim over post's output, when the look draws one.
pub struct FogRim {
    rim: Option<Screen>,
}

/// The fog mask passes for `look` over `targets`' world.
pub fn fog_mask(
    device: &Device,
    pipelines: &Pipelines,
    targets: &Targets,
    look: &FogLook,
) -> (FogMask, FogRim) {
    let uniform = |label, bytes: &[u8]| buffer(device, label, BufferUsages::UNIFORM, bytes);
    let look_buffer = uniform("fog-look", bytemuck::bytes_of(look));
    let view = uniform("fog-mask-view", &[0; VIEW_BYTES]);
    let tex = BindingResource::TextureView;
    let screen = |name: &str, layout: &str, resources: &[BindingResource]| Screen {
        pipeline: pipelines.render(name).clone(),
        group: bind(device, pipelines.layout(layout), name, resources),
    };
    let distances = look.has_edge().then(|| {
        [
            (
                screen(
                    "fogMaskRows",
                    "rowsLayout",
                    &[look_buffer.as_entire_binding(), tex(&targets.fog_mask)],
                ),
                targets.fog_distance_rows.clone(),
            ),
            (
                screen(
                    "fogMaskColumns",
                    "columnsLayout",
                    &[
                        look_buffer.as_entire_binding(),
                        tex(&targets.fog_mask),
                        tex(&targets.fog_distance_rows),
                    ],
                ),
                targets.fog_distance.clone(),
            ),
        ]
    });
    let composed = [
        look_buffer.as_entire_binding(),
        view.as_entire_binding(),
        tex(&targets.lit),
        tex(&targets.fog_mask),
        tex(&targets.fog_distance),
    ];
    let compose = screen("fogMaskCompose", "composeLayout", &composed);
    let rim = look
        .has_rim()
        .then(|| screen("fogMaskRim", "composeLayout", &composed));
    (FogMask { distances, compose }, FogRim { rim })
}

fn draw(
    encoder: &mut CommandEncoder,
    screen: &Screen,
    target: &TextureView,
    resolve: Option<&TextureView>,
    load: LoadOp<Color>,
) {
    let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
        label: Some("fog-mask"),
        color_attachments: &[Some(RenderPassColorAttachment {
            view: target,
            depth_slice: None,
            resolve_target: resolve,
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

impl Pass for FogMask {
    fn name(&self) -> &'static str {
        "fog-mask"
    }

    /// After the world and its effects: the distances, then the fogged world
    /// into `hdr`, post's input.
    fn encode(&self, inputs: &Inputs, encoder: &mut CommandEncoder) {
        let t = inputs.targets;
        for (screen, target) in self.distances.iter().flatten() {
            draw(encoder, screen, target, None, LoadOp::Clear(Color::WHITE));
        }
        let clear = LoadOp::Clear(Color::TRANSPARENT);
        draw(encoder, &self.compose, &t.hdr_msaa, Some(&t.hdr), clear);
    }
}

impl Pass for FogRim {
    fn name(&self) -> &'static str {
        "fog-rim"
    }

    /// After post: the rim over the display target.
    fn encode(&self, inputs: &Inputs, encoder: &mut CommandEncoder) {
        if let Some(rim) = &self.rim {
            draw(encoder, rim, &inputs.targets.display, None, LoadOp::Load);
        }
    }
}
