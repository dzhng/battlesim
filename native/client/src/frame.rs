//! The frame: one ordered list of passes, encoded in order each drawn frame
//! (Bevy 0.19+'s fixed pass order without a render graph; `battleFrame.ts`
//! is the browser's). A pass reads the frame's `View` and the sized targets
//! and encodes its work; the list is the whole frame.
use presentation::records::View;
use wgpu::*;

/// The lit world before post.
pub const HDR_FORMAT: TextureFormat = TextureFormat::Rgba16Float;
/// What post writes: the browser's canvas format.
pub const DISPLAY_FORMAT: TextureFormat = TextureFormat::Bgra8Unorm;
/// 4× is the count WebGPU guarantees for every renderable format.
pub const MSAA: u32 = 4;

/// The frame's size-dependent targets.
pub struct Targets {
    pub size: [u32; 2],
    /// The world, multisampled, resolved into `hdr`.
    pub hdr_msaa: TextureView,
    /// Post's input.
    pub hdr: TextureView,
    /// Post's output: presented, and read back for a plate.
    pub display: TextureView,
    pub display_texture: Texture,
}

impl Targets {
    pub fn new(device: &Device, size: [u32; 2]) -> Self {
        let texture = |label, format, samples, usage| {
            device.create_texture(&TextureDescriptor {
                label: Some(label),
                size: Extent3d {
                    width: size[0],
                    height: size[1],
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: samples,
                dimension: TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            })
        };
        let view = |t: Texture| t.create_view(&Default::default());
        let render = TextureUsages::RENDER_ATTACHMENT;
        let display_texture = texture(
            "display",
            DISPLAY_FORMAT,
            1,
            render | TextureUsages::COPY_SRC,
        );
        Self {
            size,
            hdr_msaa: view(texture("hdr-msaa", HDR_FORMAT, MSAA, render)),
            hdr: view(texture(
                "hdr",
                HDR_FORMAT,
                1,
                render | TextureUsages::TEXTURE_BINDING,
            )),
            display: display_texture.create_view(&Default::default()),
            display_texture,
        }
    }
}

pub trait Pass {
    fn name(&self) -> &'static str;
    fn encode(&self, queue: &Queue, view: &View, targets: &Targets, encoder: &mut CommandEncoder);
}

/// The passes in the order they draw.
pub struct Passes {
    pub targets: Targets,
    pub passes: Vec<Box<dyn Pass>>,
}

impl Passes {
    /// Encodes every pass for `view` into the display target.
    pub fn encode(&self, device: &Device, queue: &Queue, view: &View) -> CommandEncoder {
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("battle-frame"),
        });
        for pass in &self.passes {
            encoder.push_debug_group(pass.name());
            pass.encode(queue, view, &self.targets, &mut encoder);
            encoder.pop_debug_group();
        }
        encoder
    }

    /// The display target's pixels after `encoder` runs, as RGBA rows.
    pub fn read_display(
        &self,
        device: &Device,
        queue: &Queue,
        mut encoder: CommandEncoder,
    ) -> Vec<u8> {
        let [w, h] = self.targets.size;
        let row = (w * 4).next_multiple_of(COPY_BYTES_PER_ROW_ALIGNMENT);
        let buffer = device.create_buffer(&BufferDescriptor {
            label: Some("plate"),
            size: u64::from(row * h),
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            self.targets.display_texture.as_image_copy(),
            TexelCopyBufferInfo {
                buffer: &buffer,
                layout: TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: None,
                },
            },
            self.targets.display_texture.size(),
        );
        queue.submit([encoder.finish()]);
        buffer
            .slice(..)
            .map_async(MapMode::Read, |r| r.expect("plate readback"));
        device
            .poll(PollType::wait_indefinitely())
            .expect("plate readback");
        let mapped = buffer.slice(..).get_mapped_range().expect("mapped");
        let mut rgba = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            let start = (y * row) as usize;
            for bgra in mapped[start..start + (w * 4) as usize].as_chunks::<4>().0 {
                rgba.extend_from_slice(&[bgra[2], bgra[1], bgra[0], 255]);
            }
        }
        rgba
    }
}
