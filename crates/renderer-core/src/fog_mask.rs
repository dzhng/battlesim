//! Canonical live fog-look uniform, screen bindings and pass admission.
//! The same FogLook record controls edge distances and the display rim on
//! every target, including transitions while a renderer remains alive.
use presentation::records::FogLook;
use wgpu::*;

use crate::pipelines::Pipelines;
use crate::Targets;

const VIEW_BYTES: usize = 16;

/// The shared compose shader's views over actual resolved mask samples.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum FogMaskView {
    #[default]
    None = 0,
    Fog = 1,
    Ground = 2,
    Classes = 3,
}

struct Screen {
    pipeline: RenderPipeline,
    group: BindGroup,
}

pub struct FogMaskResources {
    /// The presentation `FogLook` uniform consumed by every screen-space fog
    /// bind group. The runtime updates it from canonical presentation.
    look: Buffer,
    record: FogLook,
    view: Buffer,
    mask_view: FogMaskView,
    distances: [Screen; 2],
    compose: Screen,
    rim: Screen,
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

/// Build screen bindings once; the current canonical look decides which draw.
pub fn fog_mask(
    device: &Device,
    queue: &Queue,
    pipelines: &Pipelines,
    targets: &Targets,
    record: &FogLook,
) -> FogMaskResources {
    let look = crate::buffer(
        device,
        queue,
        "fog-look",
        BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        bytemuck::bytes_of(record),
    );
    let view = crate::buffer(
        device,
        queue,
        "fog-mask-view",
        BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        &[0; VIEW_BYTES],
    );
    let tex = BindingResource::TextureView;
    let distances = [
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
    ];
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
    let rim = screen(
        device,
        pipelines,
        "fogMaskRim",
        "composeLayout",
        &compose_resources,
    );
    FogMaskResources {
        look,
        record: *record,
        view,
        mask_view: FogMaskView::None,
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
    pub fn set_mask_view(&mut self, queue: &Queue, view: FogMaskView) {
        if self.mask_view == view {
            return;
        }
        self.mask_view = view;
        queue.write_buffer(&self.view, 0, bytemuck::cast_slice(&[view as u32, 0, 0, 0]));
    }

    pub fn mask_view(&self) -> FogMaskView {
        self.mask_view
    }

    /// Update the uniform and draw decisions from the same canonical record.
    pub fn set_look(&mut self, queue: &Queue, look: &FogLook) {
        self.record = *look;
        queue.write_buffer(&self.look, 0, bytemuck::bytes_of(look));
    }

    pub fn look(&self) -> FogLook {
        self.record
    }

    /// Encode the enabled distances, then compose the lit world into post input.
    pub fn encode(&self, encoder: &mut CommandEncoder, targets: &Targets) {
        if self.record.has_edge() && self.mask_view == FogMaskView::None {
            for (screen, target) in self
                .distances
                .iter()
                .zip([&targets.fog_distance_rows, &targets.fog_distance])
            {
                draw(encoder, screen, target, None, LoadOp::Clear(Color::WHITE));
            }
        }
        draw(
            encoder,
            &self.compose,
            &targets.hdr_msaa,
            Some(&targets.hdr),
            LoadOp::Clear(Color::TRANSPARENT),
        );
    }

    /// Encode the optional seen-side rim over the completed display target.
    pub fn encode_rim(&self, encoder: &mut CommandEncoder, targets: &Targets) {
        if self.record.has_rim() && self.mask_view == FogMaskView::None {
            draw(encoder, &self.rim, &targets.display, None, LoadOp::Load);
        }
    }
}

impl FogMaskResources {
    pub fn record_memory(&self, memory: &mut crate::memory::ResourceMemory) {
        memory.buffer(&self.look);
        memory.buffer(&self.view);
    }
}
