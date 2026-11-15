//! Immutable authored rest palettes and bounds retained with shared geometry.
use super::*;

pub(super) struct RestPose {
    pub(super) bounds: [[f64; 3]; 2],
    pub(super) palette: Vec<Joint>,
}
impl RestPose {
    pub fn identity() -> Vec<Joint> {
        vec![Joint {
            x: [1., 0., 0., 0.],
            y: [0., 1., 0., 0.],
            z: [0., 0., 1., 0.],
        }]
    }
    pub fn articulated(nodes: &[crate::bundle::Node], palette: Vec<Joint>) -> Self {
        let mut positions = Vec::new();
        for (index, node) in nodes.iter().enumerate() {
            if let Some(mesh) = node.tiers.first() {
                let joint = &palette[index];
                for point in mesh.positions.as_chunks::<3>().0 {
                    for row in [joint.x, joint.y, joint.z] {
                        positions.push(
                            row[0] * point[0] + row[1] * point[1] + row[2] * point[2] + row[3],
                        );
                    }
                }
            }
        }
        Self::new(
            &Mesh {
                positions,
                ..Default::default()
            },
            palette,
        )
    }

    pub fn new(mesh: &Mesh, palette: Vec<Joint>) -> Self {
        let mut bounds = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
        for point in mesh.positions.as_chunks::<3>().0.iter() {
            for c in 0..3 {
                bounds[0][c] = bounds[0][c].min(f64::from(point[c]));
                bounds[1][c] = bounds[1][c].max(f64::from(point[c]));
            }
        }
        Self { bounds, palette }
    }
}

impl ModelGeometry {
    /// Rest facts follow the packed geometry owner, with exact palette bytes
    /// distinguishing far poses of the same unposed living mesh. Weak entries
    /// retain neither retired poses nor geometry after their final row drops.
    pub(super) fn rest_pose(
        &self,
        palette: &[Joint],
        work: &mut crate::runtime::AdmissionWork,
        derive: impl FnOnce() -> RestPose,
    ) -> Arc<RestPose> {
        let mut rests = self.rests.lock().expect("geometry rest poses");
        rests.retain(|_, rest| rest.strong_count() > 0);
        let key = bytemuck::cast_slice(palette).to_vec();
        if let Some(rest) = rests.get(&key).and_then(std::sync::Weak::upgrade) {
            return rest;
        }
        let rest = Arc::new(derive());
        work.rest_poses_derived += 1;
        rests.insert(key, Arc::downgrade(&rest));
        rest
    }
}
