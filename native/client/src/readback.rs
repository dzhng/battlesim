//! Desktop blocking display readback; GPU frame ownership stays in renderer-core.
use wgpu::*;

/// The display target's pixels after `encoder` runs, as RGBA rows.
pub fn read_display(
    device: &Device,
    queue: &Queue,
    targets: &renderer_core::Targets,
    mut encoder: CommandEncoder,
) -> Vec<u8> {
    let [w, h] = targets.size;
    let row = (w * 4).next_multiple_of(COPY_BYTES_PER_ROW_ALIGNMENT);
    let buffer = device.create_buffer(&BufferDescriptor {
        label: Some("plate"),
        size: u64::from(row * h),
        usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        targets.display_texture.as_image_copy(),
        TexelCopyBufferInfo {
            buffer: &buffer,
            layout: TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: None,
            },
        },
        targets.display_texture.size(),
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
