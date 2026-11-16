//! A routing row uses the same geometry/rest owner during warming and admission.
use super::admission::articulated_meshes;
use super::plan::RowPlan;
use super::*;
use crate::runtime::AdmissionWork;

pub(super) struct RowInput {
    pub appearance: presentation::records::Appearance,
    pub name: String,
    pub bundle: Arc<Bundle>,
    pub plan: RowPlan,
}

impl RowInput {
    pub fn admit(
        &self,
        device: &Device,
        queue: &Queue,
        store: &SharedArtStore,
        work: &mut crate::runtime::AdmissionWork,
    ) -> Result<Row, String> {
        let a = &self.appearance;
        let name = self.name.as_str();
        let bundle = self.bundle.as_ref();
        let content = &self.plan.geometry.content;
        let plan = &self.plan;
        let prop = a.draws == DRAWS_PROP || a.draws == DRAWS_PROP_GROUND;
        let base = store
            .lock()
            .expect("art store lock")
            .slots
            .base(content)
            .expect("reserved content");
        let label = name;
        let single = |mesh: &Mesh, rigid, work: &mut AdmissionWork| -> Result<Drawable, String> {
            Ok(drawable(
                device,
                queue,
                label,
                bundle,
                &[(mesh, pack_vertices(mesh, base, rigid, 0)?)],
                work,
            ))
        };
        let rest = || {
            plan.palette
                .clone()
                .ok_or(format!("{name}: no rest pose for row"))
        };
        let row = match &bundle.shape {
            Shape::Skinned(tiers) => {
                let palette = rest()?;
                let geometry = ArtStore::geometry(store, plan.geometry.clone(), work, |work| {
                    let tiers = tiers
                        .iter()
                        .map(|m| single(m, None, work))
                        .collect::<Result<_, _>>()?;
                    Ok(ModelGeometry {
                        tiers,
                        rests: Default::default(),
                    })
                })?;
                let rest = tiers.first().map(|mesh| {
                    geometry.rest_pose(&palette, work, || {
                        RestPose::new(&posed_mesh(mesh, &palette), palette.clone())
                    })
                });
                Row {
                    rest,
                    kind: if a.lies == NONE {
                        Kind::Soldier
                    } else {
                        Kind::Corpse
                    },
                    building: a.draws == 0 && a.placed != 0,
                    prop,
                    card: None,
                    card_key: plan.card.clone(),
                    geometry,
                }
            }
            Shape::Articulated(nodes) => {
                let count = nodes.first().map_or(0, |n| n.tiers.len());
                let palette = plan.palette.clone();
                let geometry = ArtStore::geometry(store, plan.geometry.clone(), work, |work| {
                    let tiers = (0..count)
                        .map(|t| {
                            let meshes = articulated_meshes(nodes, t, base, |node| {
                                match plan.geometry.tracks[node] {
                                    1 => Some(presentation::world::TrackSide::Left),
                                    2 => Some(presentation::world::TrackSide::Right),
                                    _ => None,
                                }
                            })?;
                            Ok(drawable(device, queue, label, bundle, &meshes, work))
                        })
                        .collect::<Result<_, String>>()?;
                    Ok(ModelGeometry {
                        tiers,
                        rests: Default::default(),
                    })
                })?;
                let rest = palette.as_ref().map(|palette| {
                    geometry.rest_pose(palette, work, || {
                        RestPose::articulated(nodes, palette.clone())
                    })
                });
                Row {
                    rest,
                    kind: Kind::Vehicle,
                    building: a.draws == 0 && a.placed != 0,
                    prop,
                    geometry,
                    card: None,
                    card_key: plan.card.clone(),
                }
            }
            Shape::Static(states) => {
                let s = states
                    .iter()
                    .find(|s| s.name == plan.geometry.state)
                    .expect("planned authored state");
                let geometry = ArtStore::geometry(store, plan.geometry.clone(), work, |work| {
                    let tiers = s
                        .tiers
                        .iter()
                        .map(|m| single(m, Some(0), work))
                        .collect::<Result<_, _>>()?;
                    Ok(ModelGeometry {
                        tiers,
                        rests: Default::default(),
                    })
                })?;
                let palette = RestPose::identity();
                let rest = s.tiers.first().map(|mesh| {
                    geometry.rest_pose(&palette, work, || RestPose::new(mesh, palette.clone()))
                });
                Row {
                    rest,
                    kind: if prop { Kind::Prop } else { Kind::Piece },
                    building: a.draws == 0 && a.placed != 0,
                    prop,
                    geometry,
                    card: None,
                    card_key: plan.card.clone(),
                }
            }
        };
        Ok(row)
    }
}
