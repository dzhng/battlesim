//! Immutable placed-building facts and each state's live physical membership.
use super::PropId;
use contract::map::BuildingDefinition;
use std::collections::BTreeMap;

#[derive(Clone, Copy)]
struct Member {
    building: PropId,
    structure: PropId,
}
#[derive(Clone)]
struct Building {
    definition: BuildingDefinition,
    area_m2: f64,
    owner: Option<PropId>,
    parts: Vec<PropId>,
    history: Vec<PropId>,
}
#[derive(Clone)]
pub(super) struct Buildings {
    facts: BTreeMap<PropId, Building>,
    members: BTreeMap<PropId, Member>,
    physical_digest: Option<u64>,
}
impl Buildings {
    pub fn new(definitions: &[BuildingDefinition]) -> Self {
        let mut facts = BTreeMap::new();
        let mut members = BTreeMap::new();
        for definition in definitions {
            let mut parts: Vec<_> = definition.parts.iter().map(|p| p.prop).collect();
            parts.sort_unstable();
            for &part in &parts {
                members.insert(
                    part,
                    Member {
                        building: definition.owner,
                        structure: definition.owner,
                    },
                );
            }
            facts.insert(
                definition.owner,
                Building {
                    definition: definition.clone(),
                    area_m2: footprint_area(&definition.geometry),
                    owner: Some(definition.owner),
                    history: parts.clone(),
                    parts,
                },
            );
        }
        // Hash immutable physical facts once.
        let mut physical = crate::digest::Digest::default();
        for building in facts.values() {
            let def = &building.definition;
            let bytes = serde_json::to_vec(&def.geometry).unwrap();
            physical
                .u64(def.owner as u64)
                .u64(bytes.len() as u64)
                .bytes(&bytes);
        }
        Self {
            physical_digest: (!facts.is_empty()).then(|| physical.finish()),
            facts,
            members,
        }
    }
    pub fn digest(&self, d: &mut crate::digest::Digest) {
        if let Some(physical) = self.physical_digest {
            d.bytes(b"building physical facts").u64(physical);
        }
        for (&id, member) in &self.members {
            if id != member.structure {
                d.bytes(b"building member")
                    .u64(id as u64)
                    .u64(member.building as u64)
                    .u64(member.structure as u64);
            }
        }
    }
    pub fn definitions(&self) -> impl Iterator<Item = &BuildingDefinition> {
        self.facts.values().map(|b| &b.definition)
    }
    pub fn identity(&self, part: PropId) -> Option<PropId> {
        self.members.get(&part).map(|p| p.building)
    }
    pub fn owner(&self, part: PropId) -> PropId {
        self.members.get(&part).map_or(part, |p| p.structure)
    }
    pub fn definition(&self, part: PropId) -> Option<&BuildingDefinition> {
        Some(&self.facts.get(&self.identity(part)?)?.definition)
    }
    pub fn footprint_area(&self, part: PropId) -> Option<f64> {
        Some(self.facts.get(&self.identity(part)?)?.area_m2)
    }
    pub fn parts(&self, part: PropId) -> Option<Vec<PropId>> {
        let building = self.facts.get(&self.identity(part)?)?;
        let owner = self.owner(part);
        Some(if building.owner == Some(owner) {
            building.parts.clone()
        } else {
            self.members
                .iter()
                .filter(|(_, member)| member.structure == owner)
                .map(|(&id, _)| id)
                .collect()
        })
    }
    pub fn current_parts(&self, identity: PropId) -> &[PropId] {
        self.facts
            .get(&identity)
            .map_or(&[], |b| b.parts.as_slice())
    }
    pub fn historical_parts(&self, identity: PropId) -> &[PropId] {
        self.facts
            .get(&identity)
            .map_or(&[], |b| b.history.as_slice())
    }
    /// New bodies share a fresh integrity owner, while facts and the source part
    /// they replace retain their original identity through arbitrary chains.
    pub fn replace(&mut self, owner: PropId, replacements: &[(PropId, PropId)]) {
        let Some(identity) = self.identity(owner) else {
            return;
        };
        let mut parts: Vec<_> = replacements.iter().map(|&(_, new)| new).collect();
        parts.sort_unstable();
        let active = parts.first().copied();
        for &(_, new) in replacements {
            self.members.insert(
                new,
                Member {
                    building: identity,
                    structure: active.unwrap(),
                },
            );
        }
        let building = self.facts.get_mut(&identity).unwrap();
        building.history.extend(&parts);
        building.parts = parts;
        building.owner = active;
    }
}

/// Integrate the union of the part footprints. Corners and edge crossings
/// split the plane into slabs whose covered vertical length is linear.
fn footprint_area(geometry: &contract::templates::MaterializedBuilding) -> f64 {
    use crate::math::v2;
    let boxes: Vec<_> = geometry
        .parts
        .iter()
        .map(|part| {
            let [hx, hy, _] = part.half_extents;
            [v2(-hx, -hy), v2(hx, -hy), v2(hx, hy), v2(-hx, hy)]
                .map(|p| v2(part.center[0], part.center[1]) + p.rotated(part.yaw))
        })
        .collect();
    let edges: Vec<_> = boxes
        .iter()
        .flat_map(|b| (0..4).map(move |i| (b[i], b[(i + 1) % 4])))
        .collect();
    let mut cuts: Vec<_> = boxes.iter().flat_map(|b| b.iter().map(|p| p.x)).collect();
    for (i, &(a, b)) in edges.iter().enumerate() {
        for &(c, d) in &edges[i + 1..] {
            let ab = b - a;
            let cd = d - c;
            let determinant = ab.cross(cd);
            if determinant == 0.0 {
                continue;
            }
            let t = (c - a).cross(cd) / determinant;
            let u = (c - a).cross(ab) / determinant;
            if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
                cuts.push(a.x + t * ab.x);
            }
        }
    }
    cuts.sort_by(f64::total_cmp);
    cuts.dedup();
    cuts.windows(2)
        .map(|slab| {
            let x = (slab[0] + slab[1]) / 2.0;
            let mut spans: Vec<_> = boxes
                .iter()
                .filter_map(|b| {
                    let mut ys: Vec<_> = (0..4)
                        .filter_map(|i| {
                            let (a, b) = (b[i], b[(i + 1) % 4]);
                            (x > a.x.min(b.x) && x < a.x.max(b.x))
                                .then(|| a.y + (b.y - a.y) * (x - a.x) / (b.x - a.x))
                        })
                        .collect();
                    ys.sort_by(f64::total_cmp);
                    (ys.len() >= 2).then(|| (ys[0], *ys.last().unwrap()))
                })
                .collect();
            spans.sort_by(|a, b| a.0.total_cmp(&b.0));
            let mut covered = 0.0;
            let mut end = f64::NEG_INFINITY;
            for (a, b) in spans {
                covered += (b - a.max(end)).max(0.0);
                end = end.max(b);
            }
            (slab[1] - slab[0]) * covered
        })
        .sum()
}

#[cfg(test)]
mod tests {
    #[test]
    fn overlapping_rotated_footprints_count_once() {
        use contract::templates::{MaterializedBuilding, MaterializedPart, PlacementFrame};
        let geometry = MaterializedBuilding {
            template_id: "overlap".into(),
            frame: PlacementFrame {
                translation: [400.0, 300.0, 0.0],
                yaw: 0.0,
            },
            parts: (0..2)
                .map(|i| MaterializedPart {
                    id: i.to_string(),
                    center: [400.0, 300.0],
                    yaw: i as f64 * std::f64::consts::FRAC_PI_4,
                    half_extents: [2.0, 2.0, 4.0],
                    base_z: 0.0,
                })
                .collect(),
            height_m: 8.0,
            floor_z: vec![0.0],
            entrances: None,
            edges: vec![],
        };
        // Square plus four triangles outside it: overlap contributes no extra bulk.
        let area = 64.0 - 32.0 * 2.0_f64.sqrt();
        assert!((super::footprint_area(&geometry) - area).abs() < 1e-9);
    }
}
