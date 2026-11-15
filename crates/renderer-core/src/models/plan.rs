//! Canonical CPU row inputs shared by preflight, geometry and card admission.
use super::card_store::CardKey;
use super::store::GeometryKey;
use super::*;
use contract::identity::ContentId;
use std::collections::BTreeMap;

pub(super) struct RowPlan {
    pub geometry: GeometryKey,
    pub palette: Option<Vec<Joint>>,
    pub card: Option<CardKey>,
}

pub(super) fn rows(
    appearances: &[presentation::records::Appearance],
    strings: &str,
    bundles: &BTreeMap<String, (Arc<Bundle>, ContentId)>,
    textures: &BTreeMap<String, Arc<crate::bundle::Texture>>,
    rest_palette: impl Fn(u32) -> Option<Vec<Joint>>,
    track_side: impl Fn(u32, usize) -> Option<presentation::world::TrackSide>,
) -> Result<Vec<Option<RowPlan>>, String> {
    let texture_shapes = [true, false].map(|srgb| {
        let edge = textures
            .values()
            .filter(|t| t.srgb == srgb)
            .map(|t| t.size)
            .max()
            .unwrap_or(1);
        [edge, edge, edge.ilog2() + 1]
    });
    let text = |at: u32, len: u32| &strings[at as usize..(at + len) as usize];
    appearances
        .iter()
        .enumerate()
        .map(|(row, a)| {
            if a.draws != 0 && a.draws != DRAWS_PROP && a.draws != DRAWS_PROP_GROUND {
                return Ok(None);
            }
            let name = text(a.name, a.name_len);
            let state = text(a.state, a.state_len);
            let (bundle, content) = &bundles[name];
            let palette = match bundle.shape {
                Shape::Skinned(_) | Shape::Articulated(_) => rest_palette(row as u32),
                _ => None,
            };
            let mut geometry = GeometryKey {
                content: content.clone(),
                state: String::new(),
                tracks: Vec::new(),
            };
            match &bundle.shape {
                Shape::Skinned(_) => {
                    palette
                        .as_ref()
                        .ok_or_else(|| format!("{name}: no rest pose for row {row}"))?;
                }
                Shape::Articulated(nodes) => {
                    geometry.tracks = (0..nodes.len())
                        .map(|node| match track_side(row as u32, node) {
                            Some(presentation::world::TrackSide::Left) => 1,
                            Some(presentation::world::TrackSide::Right) => 2,
                            None => 0,
                        })
                        .collect();
                }
                Shape::Static(states) => {
                    // A piece the bundle lacks (a wreck with no turret to
                    // throw) is a row nothing draws.
                    let found = states
                        .iter()
                        .find(|s| s.name == state)
                        .or_else(|| state.is_empty().then(|| states.first()).flatten());
                    let Some(state) = found else {
                        return Ok(None);
                    };
                    geometry.state = state.name.clone();
                }
            }
            let card = match &bundle.shape {
                Shape::Skinned(tiers) if !tiers.is_empty() => {
                    let baked_palette = palette.as_ref().expect("skinned palette");
                    Some(CardKey::new(
                        &geometry,
                        baked_palette,
                        &crate::impostor::Spec::runtime(),
                        texture_shapes,
                    ))
                }
                _ => None,
            };
            Ok(Some(RowPlan {
                geometry,
                palette,
                card,
            }))
        })
        .collect()
}
