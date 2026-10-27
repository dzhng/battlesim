//! Shared authored-model vertex ABI.
//!
//! Bundle readers stay in the platform shells; this module owns the one
//! 48-byte stream that the exported model pipelines consume.

/// Source attributes for one indexed authored mesh. `material_ids` is already
/// expanded per index because the packed stream is non-indexed after material
/// routing.
pub struct VertexInput<'a> {
    pub positions: &'a [f32],
    pub normals: &'a [i16],
    pub uvs: &'a [f32],
    pub colors: &'a [u8],
    pub joints: Option<&'a [u8]>,
    pub weights: Option<&'a [u16]>,
    pub tangents: Option<&'a [i16]>,
    pub indices: &'a [u32],
    pub material_ids: &'a [u16],
}

/// Pack the exported model vertex layout: position, snorm16 normal, UV,
/// colour, joints, normalized weights, snorm8 tangent and material/track ids.
pub fn pack(input: VertexInput<'_>) -> Result<(Vec<u8>, u32), String> {
    let vertex_count = input.positions.len() / 3;
    if input.material_ids.len() != input.indices.len()
        || !input.positions.len().is_multiple_of(3)
        || input.normals.len() != vertex_count * 4
        || input.uvs.len() != vertex_count * 2
        || input.colors.len() != vertex_count * 4
        || input
            .joints
            .is_some_and(|joints| joints.len() != vertex_count * 4)
        || input
            .weights
            .is_some_and(|weights| weights.len() != vertex_count * 4)
        || input
            .tangents
            .is_some_and(|tangents| tangents.len() != vertex_count * 4)
    {
        return Err("authored model attributes have different lengths".into());
    }
    let mut bytes = Vec::with_capacity(input.indices.len() * 48);
    for (draw_index, &raw_index) in input.indices.iter().enumerate() {
        let index = raw_index as usize;
        if index >= vertex_count {
            return Err("authored model index is outside the vertex array".into());
        }
        bytes.extend_from_slice(bytemuck::cast_slice(
            &input.positions[index * 3..index * 3 + 3],
        ));
        bytes.extend_from_slice(bytemuck::cast_slice(
            &input.normals[index * 4..index * 4 + 4],
        ));
        bytes.extend_from_slice(bytemuck::cast_slice(&input.uvs[index * 2..index * 2 + 2]));
        bytes.extend_from_slice(&input.colors[index * 4..index * 4 + 4]);
        if let Some(joints) = input.joints {
            bytes.extend_from_slice(&joints[index * 4..index * 4 + 4]);
        } else {
            bytes.extend_from_slice(&[0, 0, 0, 0]);
        }
        let mut weights = [0u8; 4];
        if let Some(source) = input.weights {
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
        if let Some(tangents) = input.tangents {
            bytes.extend(
                tangents[index * 4..index * 4 + 4]
                    .iter()
                    .map(|&component| (i32::from(component) / 258).clamp(-128, 127) as u8),
            );
        } else {
            bytes.extend_from_slice(&[0, 0, 0, 0]);
        }
        bytes.extend_from_slice(bytemuck::cast_slice(&[
            input.material_ids[draw_index],
            0u16,
        ]));
    }
    Ok((bytes, input.indices.len() as u32))
}

#[cfg(test)]
mod tests {
    use super::{pack, VertexInput};

    #[test]
    fn preserves_the_exported_vertex_abi() {
        let (bytes, count) = pack(VertexInput {
            positions: &[1.0, 2.0, 3.0],
            normals: &[1, 2, 3, 4],
            uvs: &[0.25, 0.75],
            colors: &[9, 8, 7, 6],
            joints: Some(&[0, 1, 2, 3]),
            weights: Some(&[65_535, 0, 0, 0]),
            tangents: Some(&[258, -258, 0, 32767]),
            indices: &[0],
            material_ids: &[7],
        })
        .unwrap();
        assert_eq!(count, 1);
        assert_eq!(bytes.len(), 48);
        assert_eq!(&bytes[28..32], &[9, 8, 7, 6]);
        assert_eq!(&bytes[32..36], &[0, 1, 2, 3]);
        assert_eq!(&bytes[36..40], &[255, 0, 0, 0]);
        assert_eq!(&bytes[40..44], &[1, 255, 0, 127]);
        assert_eq!(&bytes[44..48], &[7, 0, 0, 0]);
    }
}
