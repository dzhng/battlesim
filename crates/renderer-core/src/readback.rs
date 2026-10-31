//! Explicit, bounded GPU snapshots. Encoding queues copies before mapping can
//! await, so a later submitted frame cannot replace the requested contents.
use std::future::poll_fn;
use std::ops::Range;
use std::sync::{Arc, Mutex};
use std::task::{Poll, Waker};
use wgpu::*;

pub struct Readback {
    buffer: Buffer,
    bytes: u64,
}

#[derive(Default)]
struct Completion {
    result: Option<Result<(), BufferAsyncError>>,
    waker: Option<Waker>,
}

impl Readback {
    fn allocate(device: &Device, bytes: u64) -> Result<Self, String> {
        if bytes == 0 || bytes > device.limits().max_buffer_size {
            return Err(format!(
                "readback size {bytes} exceeds available buffer bounds"
            ));
        }
        Ok(Self {
            buffer: device.create_buffer(&BufferDescriptor {
                label: Some("explicit-readback"),
                size: bytes,
                usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
                mapped_at_creation: false,
            }),
            bytes,
        })
    }

    pub fn buffer(
        device: &Device,
        encoder: &mut CommandEncoder,
        source: &Buffer,
        range: Range<u64>,
    ) -> Result<Self, String> {
        let bytes = range
            .end
            .checked_sub(range.start)
            .ok_or("readback buffer range is reversed")?;
        if range.end > source.size()
            || !range.start.is_multiple_of(COPY_BUFFER_ALIGNMENT)
            || !bytes.is_multiple_of(COPY_BUFFER_ALIGNMENT)
            || !source.usage().contains(BufferUsages::COPY_SRC)
        {
            return Err("readback requires an aligned range inside a COPY_SRC buffer".into());
        }
        let read = Self::allocate(device, bytes)?;
        encoder.copy_buffer_to_buffer(source, range.start, &read.buffer, 0, bytes);
        Ok(read)
    }

    pub async fn read(self, device: &Device) -> Result<Vec<u8>, String> {
        let completion = Arc::new(Mutex::new(Completion::default()));
        let mapped = completion.clone();
        self.buffer.map_async(MapMode::Read, .., move |result| {
            let wake = {
                let mut mapped = mapped.lock().expect("readback completion");
                mapped.result = Some(result);
                mapped.waker.take()
            };
            if let Some(wake) = wake {
                wake.wake();
            }
        });
        // Browser WebGPU completes from its event loop. Native callers explicitly
        // requested a snapshot, so polling here may wait for that GPU submission.
        #[cfg(not(target_arch = "wasm32"))]
        if let Err(error) = device.poll(PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(30)),
        }) {
            self.buffer.unmap();
            return Err(error.to_string());
        }
        #[cfg(target_arch = "wasm32")]
        let _ = device;
        poll_fn(|cx| {
            let mut completion = completion.lock().expect("readback completion");
            match completion.result.take() {
                Some(result) => Poll::Ready(result),
                None => {
                    completion.waker = Some(cx.waker().clone());
                    Poll::Pending
                }
            }
        })
        .await
        .map_err(|e| e.to_string())?;
        let view = self
            .buffer
            .get_mapped_range(..)
            .map_err(|e| e.to_string())?;
        let bytes = view[..self.bytes as usize].to_vec();
        drop(view);
        self.buffer.unmap();
        Ok(bytes)
    }
}

/// Raw stored RGBA bytes, top row first, independent of row-copy alignment or
/// RGBA/BGRA storage. The caller supplies an actual copyable display texture.
pub struct RgbaReadback {
    readback: Readback,
    width: u32,
    height: u32,
    padded_row: u32,
    bgra: bool,
}

impl RgbaReadback {
    pub fn encode(
        device: &Device,
        encoder: &mut CommandEncoder,
        source: &Texture,
    ) -> Result<Self, String> {
        let bgra = match source.format() {
            TextureFormat::Rgba8Unorm | TextureFormat::Rgba8UnormSrgb => false,
            TextureFormat::Bgra8Unorm | TextureFormat::Bgra8UnormSrgb => true,
            _ => return Err("RGBA readback requires an RGBA8 or BGRA8 texture".into()),
        };
        if source.dimension() != TextureDimension::D2
            || source.depth_or_array_layers() != 1
            || source.sample_count() != 1
            || !source.usage().contains(TextureUsages::COPY_SRC)
        {
            return Err(
                "RGBA readback requires a single-layer, single-sample COPY_SRC texture".into(),
            );
        }
        let width = source.width();
        let height = source.height();
        let padded_row = width
            .checked_mul(4)
            .ok_or("RGBA row overflows")?
            .checked_next_multiple_of(COPY_BYTES_PER_ROW_ALIGNMENT)
            .ok_or("RGBA row alignment overflows")?;
        let readback = Readback::allocate(device, u64::from(padded_row) * u64::from(height))?;
        encoder.copy_texture_to_buffer(
            source.as_image_copy(),
            TexelCopyBufferInfo {
                buffer: &readback.buffer,
                layout: TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_row),
                    rows_per_image: Some(height),
                },
            },
            source.size(),
        );
        Ok(Self {
            readback,
            width,
            height,
            padded_row,
            bgra,
        })
    }

    pub async fn read(self, device: &Device) -> Result<Vec<u8>, String> {
        let bytes = self.readback.read(device).await?;
        let row = self.width as usize * 4;
        let mut rgba = Vec::with_capacity(row * self.height as usize);
        for line in bytes.chunks_exact(self.padded_row as usize) {
            rgba.extend_from_slice(&line[..row]);
        }
        if self.bgra {
            for pixel in rgba.as_chunks_mut::<4>().0 {
                pixel.swap(0, 2);
            }
        }
        Ok(rgba)
    }
}

/// Raw stored RGBA16F half-float bytes, top row first.
/// The caller supplies a COPY_SRC Rgba16Float texture.
pub struct Rgba16fReadback {
    readback: Readback,
    width: u32,
    height: u32,
    padded_row: u32,
}

impl Rgba16fReadback {
    pub fn encode(
        device: &Device,
        encoder: &mut CommandEncoder,
        source: &Texture,
    ) -> Result<Self, String> {
        if source.format() != wgpu::TextureFormat::Rgba16Float {
            return Err("Rgba16fReadback requires an Rgba16Float texture".into());
        }
        if source.dimension() != TextureDimension::D2
            || source.depth_or_array_layers() != 1
            || source.sample_count() != 1
            || !source.usage().contains(TextureUsages::COPY_SRC)
        {
            return Err(
                "Rgba16fReadback requires a single-layer, single-sample COPY_SRC texture".into(),
            );
        }
        let width = source.width();
        let height = source.height();
        let padded_row = width
            .checked_mul(8)
            .ok_or("Rgba16f row overflows")?
            .checked_next_multiple_of(COPY_BYTES_PER_ROW_ALIGNMENT)
            .ok_or("Rgba16f row alignment overflows")?;
        let readback = Readback::allocate(device, u64::from(padded_row) * u64::from(height))?;
        encoder.copy_texture_to_buffer(
            source.as_image_copy(),
            TexelCopyBufferInfo {
                buffer: &readback.buffer,
                layout: TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_row),
                    rows_per_image: Some(height),
                },
            },
            source.size(),
        );
        Ok(Self {
            readback,
            width,
            height,
            padded_row,
        })
    }

    /// Read back the texture data as half-float values (8 bytes per pixel: R16G16B16A16).
    pub async fn read(self, device: &Device) -> Result<Vec<u8>, String> {
        let bytes = self.readback.read(device).await?;
        let row = self.width as usize * 8;
        let mut rgba16f = Vec::with_capacity(row * self.height as usize);
        for line in bytes.chunks_exact(self.padded_row as usize) {
            rgba16f.extend_from_slice(&line[..row]);
        }
        Ok(rgba16f)
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }
}
