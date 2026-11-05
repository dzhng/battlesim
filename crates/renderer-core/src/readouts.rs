//! Unit info cards (readouts) drawn as laid-out screen-space primitives: a
//! shell hands each frame's rectangles, sprites and strokes, in display
//! pixels, and the pass draws them into its own premultiplied sRGB layer,
//! then lays that layer over the finished look through the overlay's
//! composite with no halo: the display, or as an HLG signal the HDR look's
//! target, exactly as the overlay itself. The cards never touch the overlay
//! target, so the overlay halo never brightens them.
//!
//! Primitives:
//! - a rectangle: a fill and a border, or a Gaussian-blurred fill (a card's
//!   backing scrim);
//! - a sprite: a premultiplied atlas rectangle (a glyph, its shadow, an
//!   icon), at an opacity;
//! - a stroke: a segment or a circle of a width, dashed or whole, over at
//!   most two blurred shadows (a leader and the ring it starts from).
//!
//! Colours are straight-alpha sRGB, as the browser computes them; the shader
//! premultiplies them.

use crate::pipelines::Pipelines;
use crate::records::OverlayGlow;
use crate::OVERLAY_FORMAT;
use wgpu::*;

/// One instance: a quad and what it draws, nine `vec4f`s (the
/// `readoutQuads` pipeline's instance layout).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ReadoutQuad {
    /// The quad in normalised device coordinates: x0, y0, x1, y1.
    pub ndc: [f32; 4],
    /// The same quad in display pixels, top-left origin.
    pub px: [f32; 4],
    /// What it draws, in display pixels: a rectangle (x0, y0, x1, y1), a
    /// segment (ax, ay, bx, by) or a circle (cx, cy, r, 0).
    pub shape: [f32; 4],
    /// A sprite's atlas rectangle, normalised: u0, v0, u1, v1.
    pub uv: [f32; 4],
    /// A rectangle's fill, a stroke's ink; a sprite's alpha is its opacity.
    pub ink: [f32; 4],
    /// A rectangle's border, a stroke's first shadow.
    pub a: [f32; 4],
    /// A stroke's second shadow.
    pub b: [f32; 4],
    /// Kind (0 rectangle, 1 sprite, 2 segment, 3 circle); a border or stroke
    /// width; a blur or first shadow's sigma; the second shadow's sigma.
    pub params: [f32; 4],
    /// A dash's on and off lengths and the phase it starts at; on 0 is whole.
    pub dash: [f32; 4],
}

const RECT: f32 = 0.0;
const SPRITE: f32 = 1.0;
const SEGMENT: f32 = 2.0;
const CIRCLE: f32 = 3.0;

/// A stroke's shadow: its colour (straight alpha) and blur sigma in pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Shadow {
    pub color: [f32; 4],
    pub sigma_px: f32,
}

/// How a stroke is drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Line {
    pub width_px: f32,
    pub ink: [f32; 4],
    /// On and off lengths, or none for a whole line.
    pub dash: Option<[f32; 2]>,
    /// Up to two shadows, the first painted over the second.
    pub shadows: [Option<Shadow>; 2],
}

/// One frame's readout primitives, in paint order, for a display of `size`
/// pixels and an atlas of `atlas` pixels.
pub struct ReadoutDraw {
    size: [f32; 2],
    atlas: [f32; 2],
    quads: Vec<ReadoutQuad>,
}

impl ReadoutDraw {
    pub fn new(size: [u32; 2], atlas: [u32; 2]) -> Self {
        Self {
            size: size.map(|v| v.max(1) as f32),
            atlas: atlas.map(|v| v.max(1) as f32),
            quads: Vec::new(),
        }
    }

    pub fn quads(&self) -> &[ReadoutQuad] {
        &self.quads
    }

    fn push(&mut self, px: [f32; 4], mut quad: ReadoutQuad) {
        let [w, h] = self.size;
        quad.px = px;
        quad.ndc = [
            px[0] / w * 2.0 - 1.0,
            1.0 - px[1] / h * 2.0,
            px[2] / w * 2.0 - 1.0,
            1.0 - px[3] / h * 2.0,
        ];
        self.quads.push(quad);
    }

    /// A rectangle: `fill` inside a `border_px` border, or with `blur_px`
    /// above zero the fill blurred by that sigma (the border ignored).
    pub fn rect(
        &mut self,
        rect: [f32; 4],
        fill: [f32; 4],
        border: [f32; 4],
        border_px: f32,
        blur_px: f32,
    ) {
        if fill[3] <= 0.0 && (border[3] <= 0.0 || border_px <= 0.0) {
            return;
        }
        let reach = 3.0 * blur_px.max(0.0) + 1.0;
        self.push(
            [
                rect[0] - reach,
                rect[1] - reach,
                rect[2] + reach,
                rect[3] + reach,
            ],
            ReadoutQuad {
                shape: rect,
                ink: fill,
                a: border,
                params: [RECT, border_px.max(0.0), blur_px.max(0.0), 0.0],
                ..Default::default()
            },
        );
    }

    /// Atlas pixels `from` (x0, y0, x1, y1), premultiplied, drawn over
    /// display pixels `to` at `opacity`.
    pub fn sprite(&mut self, from: [f32; 4], to: [f32; 4], opacity: f32) {
        if opacity <= 0.0 {
            return;
        }
        let [aw, ah] = self.atlas;
        self.push(
            to,
            ReadoutQuad {
                uv: [from[0] / aw, from[1] / ah, from[2] / aw, from[3] / ah],
                ink: [1.0, 1.0, 1.0, opacity],
                params: [SPRITE, 0.0, 0.0, 0.0],
                ..Default::default()
            },
        );
    }

    fn stroke(&mut self, kind: f32, bounds: [f32; 4], shape: [f32; 4], line: &Line, phase: f32) {
        let [first, second] = line.shadows.map(|s| s.unwrap_or_default());
        let reach = line.width_px / 2.0 + 3.0 * first.sigma_px.max(second.sigma_px) + 1.0;
        let [on, off] = line.dash.unwrap_or([0.0, 0.0]);
        self.push(
            [
                bounds[0] - reach,
                bounds[1] - reach,
                bounds[2] + reach,
                bounds[3] + reach,
            ],
            ReadoutQuad {
                shape,
                ink: line.ink,
                a: first.color,
                b: second.color,
                params: [kind, line.width_px, first.sigma_px, second.sigma_px],
                dash: [on, off, phase, 0.0],
                ..Default::default()
            },
        );
    }

    /// A polyline through `points`, its dash running on from one segment to
    /// the next.
    pub fn polyline(&mut self, points: &[[f32; 2]], line: &Line) {
        let mut phase = 0.0;
        for pair in points.windows(2) {
            let ([ax, ay], [bx, by]) = (pair[0], pair[1]);
            let bounds = [ax.min(bx), ay.min(by), ax.max(bx), ay.max(by)];
            self.stroke(SEGMENT, bounds, [ax, ay, bx, by], line, phase);
            phase += libm::hypotf(bx - ax, by - ay);
        }
    }

    /// A circle of radius `r` round `centre`.
    pub fn circle(&mut self, centre: [f32; 2], r: f32, line: &Line) {
        let [cx, cy] = centre;
        let bounds = [cx - r, cy - r, cx + r, cy + r];
        self.stroke(CIRCLE, bounds, [cx, cy, r, 0.0], line, 0.0);
    }
}

/// The readout pass: its layer, the atlas its sprites come from, the quads
/// of the frame last prepared, and the composite over the look.
pub struct ReadoutResources {
    quads: RenderPipeline,
    atlas_group: BindGroup,
    instances: Buffer,
    capacity: u64,
    count: u32,
    size: [u32; 2],
    atlas_size: [u32; 2],
    layer: TextureView,
    composite: RenderPipeline,
    composite_hdr: RenderPipeline,
    source: BindGroup,
    /// Kept for its lifetime: the composite's halo-less glow.
    _no_glow: Buffer,
}

const QUAD_BYTES: u64 = std::mem::size_of::<ReadoutQuad>() as u64;

impl ReadoutResources {
    /// A pass for a display of `size` pixels whose sprites come from `atlas`,
    /// `atlas_size` premultiplied RGBA8 pixels, rows top first.
    pub fn new(
        device: &Device,
        queue: &Queue,
        pipelines: &Pipelines,
        size: [u32; 2],
        atlas_size: [u32; 2],
        atlas: &[u8],
    ) -> Result<Self, String> {
        let [aw, ah] = atlas_size.map(|v| v.max(1));
        if atlas.len() != (aw * ah * 4) as usize && !(atlas.is_empty() && atlas_size == [0, 0]) {
            return Err(format!(
                "a {aw}×{ah} readout atlas needs {} bytes, not {}",
                aw * ah * 4,
                atlas.len()
            ));
        }
        let texture = |label, width, height, usage| {
            device.create_texture(&TextureDescriptor {
                label: Some(label),
                size: Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: OVERLAY_FORMAT,
                usage,
                view_formats: &[],
            })
        };
        let atlas_texture = texture(
            "readout-atlas",
            aw,
            ah,
            TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        );
        let blank = [0u8; 4];
        queue.write_texture(
            atlas_texture.as_image_copy(),
            if atlas.is_empty() { &blank } else { atlas },
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(aw * 4),
                rows_per_image: Some(ah),
            },
            Extent3d {
                width: aw,
                height: ah,
                depth_or_array_layers: 1,
            },
        );
        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("readout-atlas"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..Default::default()
        });
        let atlas_view = atlas_texture.create_view(&Default::default());
        let atlas_group = crate::bind(
            device,
            pipelines.layout("sampled"),
            "readout-atlas",
            &[
                BindingResource::TextureView(&atlas_view),
                BindingResource::Sampler(&sampler),
            ],
        );
        // Exactly the target it is laid over, pixel for pixel: the runtime's
        // targets are already at their floor (`render_size`), and flooring
        // again here would stretch the frame's quads over a taller layer.
        let size = size.map(|v| v.max(1));
        let layer = texture(
            "readout-layer",
            size[0],
            size[1],
            TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
        )
        .create_view(&Default::default());
        // The overlay's composite, without its halo: the layer stands in for
        // the halo it never reads.
        let no_glow = crate::buffer(
            device,
            queue,
            "readout-no-glow",
            BufferUsages::UNIFORM,
            bytemuck::bytes_of(&OverlayGlow {
                radius_px: 1.0,
                ..Default::default()
            }),
        );
        let source = crate::bind(
            device,
            pipelines.layout("overlaySource"),
            "readout-composite",
            &[
                no_glow.as_entire_binding(),
                BindingResource::TextureView(&layer),
                BindingResource::TextureView(&layer),
            ],
        );
        Ok(Self {
            quads: pipelines.render("readoutQuads").clone(),
            atlas_group,
            instances: Self::instance_buffer(device, 64),
            capacity: 64,
            count: 0,
            size,
            atlas_size: [aw, ah],
            layer,
            composite: pipelines.render("overlayComposite").clone(),
            composite_hdr: pipelines.render("overlayCompositeHdr").clone(),
            source,
            _no_glow: no_glow,
        })
    }

    fn instance_buffer(device: &Device, quads: u64) -> Buffer {
        device.create_buffer(&BufferDescriptor {
            label: Some("readout-quads"),
            size: quads * QUAD_BYTES,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    /// The display size the layer was made for.
    pub fn size(&self) -> [u32; 2] {
        self.size
    }

    /// The atlas's size, for a `ReadoutDraw` of this pass.
    pub fn atlas_size(&self) -> [u32; 2] {
        self.atlas_size
    }

    /// Uploads the frame's primitives; the next encode draws them.
    pub fn prepare(&mut self, device: &Device, queue: &Queue, draw: &ReadoutDraw) {
        debug_assert_eq!(
            draw.size,
            self.size.map(|v| v as f32),
            "a readout draw laid out for another display size"
        );
        let quads = draw.quads();
        let needed = quads.len() as u64;
        if needed > self.capacity {
            self.capacity = needed.next_power_of_two();
            self.instances = Self::instance_buffer(device, self.capacity);
        }
        if !quads.is_empty() {
            queue.write_buffer(&self.instances, 0, bytemuck::cast_slice(quads));
        }
        self.count = quads.len() as u32;
    }

    /// Draws the prepared primitives into the layer and lays it over
    /// `display`, or with `hdr_output` over that HLG target instead.
    pub fn encode(
        &self,
        encoder: &mut CommandEncoder,
        display: &TextureView,
        hdr_output: Option<&TextureView>,
    ) {
        if self.count == 0 {
            return;
        }
        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("readout-layer"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &self.layer,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::TRANSPARENT),
                        store: StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&self.quads);
            pass.set_bind_group(0, &self.atlas_group, &[]);
            pass.set_vertex_buffer(
                0,
                self.instances.slice(..u64::from(self.count) * QUAD_BYTES),
            );
            pass.draw(0..6, 0..self.count);
        }
        let (pipeline, output) = match hdr_output {
            Some(hdr) => (&self.composite_hdr, hdr),
            None => (&self.composite, display),
        };
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("readout-composite"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: output,
                depth_slice: None,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Load,
                    store: StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &self.source, &[]);
        pass.draw(0..3, 0..1);
    }
}
