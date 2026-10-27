//! Production vertex packing used by the browser parity checkpoint.

pub fn fixture_vertices() -> (Vec<u8>, u32) {
    let positions = [
        [-1.0f32, -1.0, 0.0],
        [1.0, -1.0, 0.0],
        [1.0, 1.0, 0.0],
        [-1.0, -1.0, 0.0],
        [1.0, 1.0, 0.0],
        [-1.0, 1.0, 0.0],
    ];
    let colours = [
        [31u8, 107, 184, 255],
        [31, 107, 184, 255],
        [209, 122, 31, 255],
        [31, 107, 184, 255],
        [209, 122, 31, 255],
        [31, 107, 184, 255],
    ];
    pack::<6>(positions.into_iter().zip(colours), 6)
}

pub fn mesh_vertices(
    positions: &[f32],
    colours: &[u8],
    indices: &[u32],
) -> Result<(Vec<u8>, u32), String> {
    if !positions.len().is_multiple_of(3) || colours.len() != positions.len() / 3 * 4 {
        return Err("production model attributes have different lengths".into());
    }
    let mut bytes = Vec::with_capacity(indices.len() * 48);
    for &index in indices {
        let index = index as usize;
        if index >= positions.len() / 3 {
            return Err("production model index is outside the vertex array".into());
        }
        bytes.extend_from_slice(bytemuck::cast_slice(&positions[index * 3..index * 3 + 3]));
        bytes.extend_from_slice(&[0; 8]);
        bytes.extend_from_slice(&[0; 8]);
        bytes.extend_from_slice(&colours[index * 4..index * 4 + 4]);
        bytes.extend_from_slice(&[0; 4]);
        bytes.extend_from_slice(&[255, 0, 0, 0]);
        bytes.extend_from_slice(&[0, 0, 127, 127]);
        bytes.extend_from_slice(&[0; 4]);
    }
    Ok((bytes, indices.len() as u32))
}

#[allow(clippy::too_many_arguments)]
pub fn encode_prepass(
    encoder: &mut wgpu::CommandEncoder,
    targets: &renderer_core::Targets,
    pipelines: &renderer_core::pipelines::Pipelines,
    camera: &wgpu::BindGroup,
    materials: &wgpu::BindGroup,
    vertices: &wgpu::Buffer,
    instances: &wgpu::Buffer,
    vertex_count: u32,
) {
    let pipeline = pipelines.bound("modelPrepass");
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("shared-production-model-prepass"),
        color_attachments: &[],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: &targets.depth,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        }),
        ..Default::default()
    });
    pipeline.set(
        &mut pass,
        &[("typegpuCameraLayout", camera), ("modelLayout", materials)],
    );
    pass.set_vertex_buffer(0, vertices.slice(..));
    pass.set_vertex_buffer(1, instances.slice(..));
    pass.draw(0..vertex_count, 0..1);
}

fn pack<const N: usize>(
    vertices: impl Iterator<Item = ([f32; 3], [u8; 4])>,
    count: u32,
) -> (Vec<u8>, u32) {
    let mut bytes = Vec::with_capacity(N * 48);
    for (position, colour) in vertices {
        bytes.extend_from_slice(bytemuck::cast_slice(&position));
        bytes.extend_from_slice(&[0; 8]);
        bytes.extend_from_slice(&[0; 8]);
        bytes.extend_from_slice(&colour);
        bytes.extend_from_slice(&[0; 4]);
        bytes.extend_from_slice(&[255, 0, 0, 0]);
        bytes.extend_from_slice(&[0, 0, 127, 127]);
        bytes.extend_from_slice(&[0; 4]);
    }
    (bytes, count)
}
