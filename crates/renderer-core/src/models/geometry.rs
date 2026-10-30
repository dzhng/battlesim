//! Authored geometry packed and posed in the exported model vertex ABI.
use crate::bundle::Mesh;
use presentation::records::Joint;

pub const VERTEX_BYTES: usize = 48;

/// A mesh's vertices in the shared 48-byte shader layout: `rigid` binds
/// every vertex wholly to one palette joint, else the mesh's own skin;
/// `track` is 1 or 2 for a track's links (their scroll), else 0.
pub fn pack_vertices(mesh: &Mesh, material_base: usize, rigid: Option<u8>, track: u16) -> Vec<u8> {
    let n = mesh.vertices();
    let mut material = vec![0u16; n];
    for d in &mesh.draws {
        for &i in &mesh.indices[d.first..d.first + d.count] {
            material[i as usize] = (material_base + d.material) as u16;
        }
    }
    let mut out = vec![0u8; n * VERTEX_BYTES];
    for (v, o) in out.as_chunks_mut::<VERTEX_BYTES>().0.iter_mut().enumerate() {
        for c in 0..3 {
            o[c * 4..c * 4 + 4].copy_from_slice(&mesh.positions[v * 3 + c].to_le_bytes());
        }
        for c in 0..4 {
            o[12 + c * 2..14 + c * 2].copy_from_slice(&mesh.normals[v * 4 + c].to_le_bytes());
        }
        o[20..24].copy_from_slice(&mesh.uvs[v * 2].to_le_bytes());
        o[24..28].copy_from_slice(&mesh.uvs[v * 2 + 1].to_le_bytes());
        o[28..32].copy_from_slice(&mesh.colors[v * 4..v * 4 + 4]);
        let weights = match (rigid, &mesh.joints, &mesh.weights) {
            (None, Some(joints), Some(weights)) => {
                o[32..36].copy_from_slice(&joints[v * 4..v * 4 + 4]);
                // Narrowed to unorm8, still summing to exactly one.
                let w: [u16; 4] = std::array::from_fn(|c| weights[v * 4 + c]);
                let mut narrow = w.map(|x| f64::round(f64::from(x) / 65535.0 * 255.0) as i32);
                let heaviest = (0..4).fold(0, |h, c| if w[c] > w[h] { c } else { h });
                narrow[heaviest] += 255 - narrow.iter().sum::<i32>();
                narrow.map(|x| x as u8)
            }
            _ => {
                o[32..36].fill(rigid.unwrap_or(0));
                [255, 0, 0, 0]
            }
        };
        o[36..40].copy_from_slice(&weights);
        if let Some(t) = &mesh.tangents {
            for c in 0..4 {
                o[40 + c] = f64::round(f64::from(t[v * 4 + c]) / 258.0) as i8 as u8;
            }
        }
        o[44..46].copy_from_slice(&material[v].to_le_bytes());
        o[46..48].copy_from_slice(&track.to_le_bytes());
    }
    out
}

/// `mesh` skinned once by `palette` (bind to model) into a rigid mesh
/// with positions, normals and tangents posed and the skin dropped.
pub fn posed_mesh(mesh: &Mesh, palette: &[Joint]) -> Mesh {
    let mut posed = mesh.clone();
    posed.joints = None;
    posed.weights = None;
    let (Some(joints), Some(weights)) = (&mesh.joints, &mesh.weights) else {
        return posed;
    };
    let moved = |j: &Joint, p: [f32; 3], w: f32| -> [f32; 3] {
        let row = |r: &[f32; 4]| r[0] * p[0] + r[1] * p[1] + r[2] * p[2] + r[3] * w;
        [row(&j.x), row(&j.y), row(&j.z)]
    };
    let unit = |s: &[i16]| [s[0], s[1], s[2]].map(|c| f32::from(c) / 32767.0);
    let n = mesh.vertices();
    posed.normals.fill(0);
    if let Some(t) = &mut posed.tangents {
        t.fill(0);
    }
    for v in 0..n {
        let p = [0, 1, 2].map(|c| mesh.positions[v * 3 + c]);
        let normal = unit(&mesh.normals[v * 4..]);
        let tangent = mesh.tangents.as_ref().map(|t| unit(&t[v * 4..]));
        let (mut sp, mut sn, mut st) = ([0.0f32; 3], [0.0f32; 3], [0.0f32; 3]);
        for k in 0..4 {
            let w = f32::from(weights[v * 4 + k]) / 65535.0;
            if w == 0.0 {
                continue;
            }
            let j = &palette[joints[v * 4 + k] as usize];
            let add = |sum: &mut [f32; 3], a: [f32; 3]| (0..3).for_each(|c| sum[c] += a[c] * w);
            add(&mut sp, moved(j, p, 1.0));
            add(&mut sn, moved(j, normal, 0.0));
            if let Some(t) = tangent {
                add(&mut st, moved(j, t, 0.0));
            }
        }
        posed.positions[v * 3..v * 3 + 3].copy_from_slice(&sp);
        let len = |a: [f32; 3]| (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
        let snorm = |a: [f32; 3], l: f32| a.map(|c| f64::round(f64::from(c / l * 32767.0)) as i16);
        let ln = len(sn);
        if ln > 0.0 {
            posed.normals[v * 4..v * 4 + 3].copy_from_slice(&snorm(sn, ln));
        }
        if let (Some(out), Some(src)) = (&mut posed.tangents, &mesh.tangents) {
            let lt = len(st);
            if lt > 1e-8 {
                out[v * 4..v * 4 + 3].copy_from_slice(&snorm(st, lt));
                out[v * 4 + 3] = src[v * 4 + 3];
            }
        }
    }
    posed
}
