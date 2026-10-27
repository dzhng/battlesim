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
    materials: &[u16],
) -> Result<(Vec<u8>, u32), String> {
    if !positions.len().is_multiple_of(3) || colours.len() != positions.len() / 3 * 4 {
        return Err("production model attributes have different lengths".into());
    }
    let mut bytes = Vec::with_capacity(indices.len() * 48);
    for (draw_index, &index) in indices.iter().enumerate() {
        let index = index as usize;
        if index >= positions.len() / 3 {
            return Err("production model index is outside the vertex array".into());
        }
        bytes.extend_from_slice(bytemuck::cast_slice(&positions[index * 3..index * 3 + 3]));
        // snorm16x4 normal: +Z, with a zero w component.
        bytes.extend_from_slice(&[0, 0, 0, 0, 255, 127, 0, 0]);
        bytes.extend_from_slice(&[0, 0, 0, 0, 255, 127, 0, 0]);
        bytes.extend_from_slice(&colours[index * 4..index * 4 + 4]);
        // Rigid diagnostic skinning: joint zero with full weight. The
        // exported model shader indexes the palette directly, so a sentinel
        // joint or zero weights would erase the mesh before lighting.
        bytes.extend_from_slice(&[0, 0, 0, 0]);
        bytes.extend_from_slice(&[255, 0, 0, 0]);
        bytes.extend_from_slice(&[0, 0, 127, 127]);
        let material = materials.get(draw_index).copied().unwrap_or(0);
        bytes.extend_from_slice(bytemuck::cast_slice(&[material, 0u16]));
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
    instance_count: u32,
    runs: Option<&[(u32, u32, u32, u32)]>,
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
    draw_runs(&mut pass, vertex_count, instance_count, runs);
}

/// Lit-model pass using the exported material/environment/fog contract.
#[allow(clippy::too_many_arguments)]
pub fn encode_opaque(
    encoder: &mut wgpu::CommandEncoder,
    targets: &renderer_core::Targets,
    pipelines: &renderer_core::pipelines::Pipelines,
    camera: &wgpu::BindGroup,
    materials: &wgpu::BindGroup,
    fog: &wgpu::BindGroup,
    environment: &wgpu::BindGroup,
    vertices: &wgpu::Buffer,
    instances: &wgpu::Buffer,
    vertex_count: u32,
    instance_count: u32,
    runs: Option<&[(u32, u32, u32, u32)]>,
) {
    let pipeline = pipelines.bound("modelOpaque");
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("shared-model-opaque"),
        color_attachments: &[
            Some(wgpu::RenderPassColorAttachment {
                view: &targets.hdr_msaa,
                depth_slice: None,
                resolve_target: Some(&targets.lit),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            }),
            Some(wgpu::RenderPassColorAttachment {
                view: &targets.fog_mask_msaa,
                depth_slice: None,
                resolve_target: Some(&targets.fog_mask),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            }),
        ],
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
        &[
            ("typegpuCameraLayout", camera),
            ("modelLayout", materials),
            ("fogLayout", fog),
            ("environment", environment),
        ],
    );
    pass.set_vertex_buffer(0, vertices.slice(..));
    pass.set_vertex_buffer(1, instances.slice(..));
    draw_runs(&mut pass, vertex_count, instance_count, runs);
}

fn draw_runs<'a>(
    pass: &mut wgpu::RenderPass<'a>,
    vertex_count: u32,
    instance_count: u32,
    runs: Option<&[(u32, u32, u32, u32)]>,
) {
    if let Some(runs) = runs {
        for &(first, count, vertex_first, vertex_count) in runs {
            pass.draw(
                vertex_first..vertex_first.saturating_add(vertex_count),
                first..first.saturating_add(count),
            );
        }
    } else {
        pass.draw(0..vertex_count, 0..instance_count);
    }
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
