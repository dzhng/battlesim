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
    renderer_core::model_vertex::pack(renderer_core::model_vertex::VertexInput {
        positions: &mesh.positions,
        normals: &mesh.normals,
        uvs: &mesh.uvs,
        colors: &mesh.colors,
        joints: mesh.joints.as_deref(),
        weights: mesh.weights.as_deref(),
        tangents: mesh.tangents.as_deref(),
        indices: &mesh.indices,
        material_ids: materials,
    })
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
