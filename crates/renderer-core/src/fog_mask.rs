//! Shared screen-space fog-mask resources and pass order.
//!
//! Fog sight and material classification remain shell-specific. This owner
//! starts at the common presentation boundary: the resolved look uniform,
//! distance targets, fogged-world composite, and optional post rim. Native and
//! browser shells therefore share target bindings and pass ordering without
//! sharing TypeGPU or presentation types.

use wgpu::*;

use crate::pipelines::Pipelines;
use crate::Targets;

const VIEW_BYTES: usize = 16;

struct Screen {
    pipeline: RenderPipeline,
    group: BindGroup,
}

pub struct FogMaskResources {
    /// The presentation `FogLook` uniform consumed by every screen-space fog
    /// bind group. The shell updates this buffer at the presentation boundary;
    /// keeping it here makes fog look data a shared renderer resource instead
    /// of a shell-only staging record.
    look: Buffer,
    distances: Option<[Screen; 2]>,
    compose: Screen,
    rim: Option<Screen>,
}

fn screen(
    device: &Device,
    pipelines: &Pipelines,
    name: &str,
    layout: &str,
    resources: &[BindingResource<'_>],
) -> Screen {
    Screen {
        pipeline: pipelines.render(name).clone(),
        group: crate::bind(device, pipelines.layout(layout), name, resources),
    }
}

/// Build the common fog mask resources. The shell resolves whether the look
/// needs edge distances/rim because that policy belongs to its presentation
/// record type; the core only receives the two booleans.
pub fn fog_mask(
    device: &Device,
    pipelines: &Pipelines,
    targets: &Targets,
    look_bytes: &[u8],
    has_edge: bool,
    has_rim: bool,
) -> FogMaskResources {
    let look = crate::buffer(
        device,
        "fog-look",
        BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        look_bytes,
    );
    let view = crate::buffer(
        device,
        "fog-mask-view",
        BufferUsages::UNIFORM,
        &[0; VIEW_BYTES],
    );
    let tex = BindingResource::TextureView;
    let distances = has_edge.then(|| {
        [
            screen(
                device,
                pipelines,
                "fogMaskRows",
                "rowsLayout",
                &[look.as_entire_binding(), tex(&targets.fog_mask)],
            ),
            screen(
                device,
                pipelines,
                "fogMaskColumns",
                "columnsLayout",
                &[
                    look.as_entire_binding(),
                    tex(&targets.fog_mask),
                    tex(&targets.fog_distance_rows),
                ],
            ),
        ]
    });
    let compose_resources = [
        look.as_entire_binding(),
        view.as_entire_binding(),
        tex(&targets.lit),
        tex(&targets.fog_mask),
        tex(&targets.fog_distance),
    ];
    let compose = screen(
        device,
        pipelines,
        "fogMaskCompose",
        "composeLayout",
        &compose_resources,
    );
    let rim = has_rim.then(|| {
        screen(
            device,
            pipelines,
            "fogMaskRim",
            "composeLayout",
            &compose_resources,
        )
    });
    FogMaskResources {
        look,
        distances,
        compose,
        rim,
    }
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

impl FogMaskResources {
    /// Replace the resolved presentation fog look consumed by the next pass.
    /// The caller owns the presentation record shape; this owner only requires
    /// the bytes to fit the buffer allocated when the target set was built.
    pub fn write_look(&self, queue: &Queue, bytes: &[u8]) -> Result<(), String> {
        if bytes.len() > self.look.size() as usize {
            return Err(format!(
                "fog look record is {} bytes, buffer is {} bytes",
                bytes.len(),
                self.look.size()
            ));
        }
        if !bytes.is_empty() {
            queue.write_buffer(&self.look, 0, bytes);
        }
        Ok(())
    }

    /// Encode distance rows/columns and compose the lit world into the post
    /// input, in the same order as the browser fog mask owner.
    pub fn encode(&self, encoder: &mut CommandEncoder, targets: &Targets) {
        for (screen, target) in self.distances.as_ref().into_iter().flat_map(|screens| {
            screens
                .iter()
                .zip([&targets.fog_distance_rows, &targets.fog_distance])
        }) {
            draw(encoder, screen, target, None, LoadOp::Clear(Color::WHITE));
        }
        draw(
            encoder,
            &self.compose,
            &targets.hdr_msaa,
            Some(&targets.hdr),
            LoadOp::Clear(Color::TRANSPARENT),
        );
    }

    /// Encode the optional seen-side rim over the post output.
    pub fn encode_rim(&self, encoder: &mut CommandEncoder, targets: &Targets) {
        if let Some(rim) = &self.rim {
            draw(encoder, rim, &targets.display, None, LoadOp::Load);
        }
    }
}
