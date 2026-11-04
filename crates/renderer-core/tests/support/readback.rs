//! Typed buffer readback after the test's encoded GPU work completes.
use wgpu::*;

pub fn pixels<T: bytemuck::Pod>(
    device: &Device,
    queue: &Queue,
    encoder: CommandEncoder,
    buffer: &Buffer,
) -> Vec<T> {
    queue.submit([encoder.finish()]);
    buffer.map_async(MapMode::Read, .., |r| r.unwrap());
    device.poll(PollType::wait_indefinitely()).unwrap();
    let mapped = buffer.get_mapped_range(..).unwrap();
    let pixels = bytemuck::cast_slice(&mapped).to_vec();
    drop(mapped);
    buffer.unmap();
    pixels
}
