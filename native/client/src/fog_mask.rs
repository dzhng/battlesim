//! The fog mask passes (`frame/fogMaskPass.ts`): how unseen looks on screen.
//! The world pass resolves its fog mask (unseen, seen and ground coverage
//! per pixel) beside the lit world; then, in screen space, the distances to
//! each side of the edge along rows and down columns (only when the look
//! draws an edge), the fogged world composed from the lit one into post's
//! input, and after post the rim along the edge on the seen side. The look
//! is the presentation crate's `FogLook` record, uploaded as it is.
use presentation::records::FogLook;
use std::rc::Rc;
use wgpu::*;

use crate::frame::{Inputs, Pass, Targets};
use crate::pipelines::Pipelines;

/// The distances (when the look has an edge) and the fogged world.
pub struct FogMask {
    resources: Rc<renderer_core::fog_mask::FogMaskResources>,
}

/// The rim over post's output, when the look draws one.
pub struct FogRim {
    resources: Rc<renderer_core::fog_mask::FogMaskResources>,
}

/// The fog mask passes for `look` over `targets`' world.
pub fn fog_mask(
    device: &Device,
    pipelines: &Pipelines,
    targets: &Targets,
    look: &FogLook,
) -> (FogMask, FogRim) {
    let resources = renderer_core::fog_mask::fog_mask(
        device,
        pipelines,
        targets,
        bytemuck::bytes_of(look),
        look.has_edge(),
        look.has_rim(),
    );
    let resources = Rc::new(resources);
    (
        FogMask {
            resources: resources.clone(),
        },
        FogRim { resources },
    )
}

impl Pass for FogMask {
    fn name(&self) -> &'static str {
        "fog-mask"
    }

    /// After the world and its effects: the distances, then the fogged world
    /// into `hdr`, post's input.
    fn encode(&self, inputs: &Inputs, encoder: &mut CommandEncoder) {
        self.resources.encode(encoder, inputs.targets);
    }
}

impl Pass for FogRim {
    fn name(&self) -> &'static str {
        "fog-rim"
    }

    /// After post: the rim over the display target.
    fn encode(&self, inputs: &Inputs, encoder: &mut CommandEncoder) {
        self.resources.encode_rim(encoder, inputs.targets);
    }
}
