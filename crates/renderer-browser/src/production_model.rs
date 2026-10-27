//! Production vertex packing used by the browser parity checkpoint.

use native_client::bundle::Mesh;

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

pub fn mesh_vertices(mesh: &Mesh, materials: &[u16]) -> Result<(Vec<u8>, u32), String> {
    let vertex_count = mesh.positions.len() / 3;
    if materials.len() != mesh.indices.len()
        || !mesh.positions.len().is_multiple_of(3)
        || mesh.normals.len() != vertex_count * 4
        || mesh.uvs.len() != vertex_count * 2
        || mesh.colors.len() != vertex_count * 4
        || mesh
            .joints
            .as_ref()
            .is_some_and(|joints| joints.len() != vertex_count * 4)
        || mesh
            .weights
            .as_ref()
            .is_some_and(|weights| weights.len() != vertex_count * 4)
        || mesh
            .tangents
            .as_ref()
            .is_some_and(|tangents| tangents.len() != vertex_count * 4)
    {
        return Err("production model attributes have different lengths".into());
    }
    let mut bytes = Vec::with_capacity(mesh.indices.len() * 48);
    for (draw_index, &index) in mesh.indices.iter().enumerate() {
        let index = index as usize;
        if index >= vertex_count {
            return Err("production model index is outside the vertex array".into());
        }
        bytes.extend_from_slice(bytemuck::cast_slice(
            &mesh.positions[index * 3..index * 3 + 3],
        ));
        bytes.extend_from_slice(bytemuck::cast_slice(
            &mesh.normals[index * 4..index * 4 + 4],
        ));
        bytes.extend_from_slice(bytemuck::cast_slice(&mesh.uvs[index * 2..index * 2 + 2]));
        bytes.extend_from_slice(&mesh.colors[index * 4..index * 4 + 4]);
        if let Some(joints) = &mesh.joints {
            bytes.extend_from_slice(&joints[index * 4..index * 4 + 4]);
        } else {
            bytes.extend_from_slice(&[0, 0, 0, 0]);
        }
        let mut weights = [0u8; 4];
        if let Some(source) = &mesh.weights {
            let source = &source[index * 4..index * 4 + 4];
            let mut heaviest = 0;
            for (at, &weight) in source.iter().enumerate() {
                weights[at] = ((u32::from(weight) * 255 + 32_767) / 65_535) as u8;
                if weight > source[heaviest] {
                    heaviest = at;
                }
            }
            let correction = 255 - weights.iter().map(|&weight| i16::from(weight)).sum::<i16>();
            if correction >= 0 {
                weights[heaviest] = weights[heaviest].saturating_add(correction as u8);
            } else {
                weights[heaviest] = weights[heaviest].saturating_sub((-correction) as u8);
            }
        } else {
            weights[0] = 255;
        }
        bytes.extend_from_slice(&weights);
        if let Some(tangents) = &mesh.tangents {
            bytes.extend(
                tangents[index * 4..index * 4 + 4]
                    .iter()
                    .map(|&component| (i32::from(component) / 258).clamp(-128, 127) as u8),
            );
        } else {
            bytes.extend_from_slice(&[0, 0, 0, 0]);
        }
        bytes.extend_from_slice(bytemuck::cast_slice(&[materials[draw_index], 0u16]));
    }
    Ok((bytes, mesh.indices.len() as u32))
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

#[cfg(test)]
mod tests {
    use super::mesh_vertices;
    use native_client::bundle::Mesh;

    #[test]
    fn authored_mesh_preserves_the_exported_vertex_abi() {
        let mesh = Mesh {
            positions: vec![1.0, 2.0, 3.0],
            normals: vec![1, 2, 3, 4],
            tangents: Some(vec![258, -258, 0, 32767]),
            uvs: vec![0.25, 0.75],
            colors: vec![9, 8, 7, 6],
            joints: Some(vec![0, 1, 2, 3]),
            weights: Some(vec![65_535, 0, 0, 0]),
            indices: vec![0],
            draws: Vec::new(),
        };
        let (bytes, count) = mesh_vertices(&mesh, &[7]).unwrap();
        assert_eq!(count, 1);
        assert_eq!(bytes.len(), 48);
        assert_eq!(&bytes[28..32], &[9, 8, 7, 6]);
        assert_eq!(&bytes[32..36], &[0, 1, 2, 3]);
        assert_eq!(&bytes[36..40], &[255, 0, 0, 0]);
        assert_eq!(&bytes[40..44], &[1, 255, 0, 127]);
        assert_eq!(&bytes[44..48], bytemuck::cast_slice(&[7u16, 0u16]));
    }
}
