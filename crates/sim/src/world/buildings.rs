//! Immutable placed-building facts and each state's live physical membership.
use super::PropId;
use contract::map::BuildingDefinition;
use std::collections::BTreeMap;

#[derive(Clone, Copy)]
struct Member {
    building: PropId,
    structure: PropId,
}
struct Building {
    definition: BuildingDefinition,
    owner: Option<PropId>,
    parts: Vec<PropId>,
    history: Vec<PropId>,
}
pub(super) struct Buildings {
    facts: BTreeMap<PropId, Building>,
    members: BTreeMap<PropId, Member>,
    physical_digest: Option<u64>,
}
impl Buildings {
    pub fn new(definitions: &[BuildingDefinition], types: &contract::catalog::PropCatalog) -> Self {
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
                    owner: Some(definition.owner),
                    history: parts.clone(),
                    parts,
                },
            );
        }
        // Original box faces are implicit in their collider geometry. Only
        // additional physical facts add digest state; hash immutable facts once.
        let mut physical = crate::digest::Digest::default();
        let mut added = false;
        for building in facts.values() {
            let geometry = &building.definition.geometry;
            let implicit = types.by_id(&building.definition.kind).body.garrison
                && geometry.parts.len() == 1
                && geometry.floor_z.is_none()
                && geometry.entrances.is_none()
                && geometry.edges.len() == 4
                && geometry.parts[0].center
                    == [geometry.frame.translation[0], geometry.frame.translation[1]]
                && geometry.parts[0].base_z == geometry.frame.translation[2]
                && geometry.parts.iter().all(|p| {
                    p.yaw == geometry.frame.yaw
                        && contract::templates::Facade::ALL.into_iter().all(|facade| {
                            let half = facade.axes(p.half_extents).3;
                            geometry
                                .edges
                                .iter()
                                .filter(|e| e.part == p.id && e.facade == facade)
                                .count()
                                == 1
                                && geometry.edges.iter().any(|e| {
                                    e.part == p.id
                                        && e.facade == facade
                                        && e.exposed
                                        && e.bays.is_none()
                                        && e.span_m == [-half, half]
                                })
                        })
                });
            if !implicit {
                added = true;
                let def = &building.definition;
                let bytes = serde_json::to_vec(geometry).unwrap();
                physical
                    .u64(def.owner as u64)
                    .u64(bytes.len() as u64)
                    .bytes(&bytes);
            }
        }
        Self {
            facts,
            members,
            physical_digest: added.then(|| physical.finish()),
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
    pub fn states(&self) -> impl Iterator<Item = (PropId, &[PropId], &[PropId])> {
        self.facts
            .iter()
            .map(|(&id, b)| (id, b.history.as_slice(), b.parts.as_slice()))
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
