//! Replaceable placement facts, distinct from the knowledge observing them.
use super::*;
use crate::static_world::ground::kinds;
use crate::static_world::props::{Art, STAND_IN_KIT, STAND_IN_MODULE};

pub(super) struct StaticDraft {
    pub(super) buildings: Option<BuildingPopulation>,
    pub(super) props: Option<PropPopulation>,
    pub(super) scenery: Option<SceneryWalk>,
    scenery_required: bool,
}

impl StaticDraft {
    pub(super) fn stage(
        p: &Presentation,
        assets: &BTreeMap<String, Asset>,
        placements: &BTreeMap<String, Placement>,
        world: &mut World,
    ) -> Result<Self, String> {
        let buildings = p
            .buildings
            .as_ref()
            .map(|active| {
                let buildings = active.walk.buildings.replace_bounds(|name, state| {
                    let Some(asset) = assets.get(name) else {
                        return Ok(None);
                    };
                    let b = asset.state_corners(state).ok_or_else(|| {
                        format!("{name}: replacement lacks required building module {state}")
                    })?;
                    Ok(Some(
                        [b.min[0], b.min[1], b.min[2], b.max[0], b.max[1], b.max[2]]
                            .map(|v| v as f32),
                    ))
                })?;
                Ok::<_, String>(BuildingPopulation {
                    walk: active.walk.replace_buildings(buildings),
                    rows: active.rows.clone(),
                    changes: 0,
                })
            })
            .transpose()?;
        let mut props = p
            .props
            .as_ref()
            .map(|active| active.replace_art(|input| patch_props(input, assets, placements)))
            .transpose()?;
        if let Some(props) = &mut props {
            props.rows = props
                .looks
                .list
                .iter()
                .map(|(name, state, draws)| world.drawn_row(name, state, NONE, 0, true, *draws))
                .collect();
        }
        Ok(Self {
            buildings,
            props,
            scenery: None,
            scenery_required: p.scenery.is_some(),
        })
    }
    pub(super) fn needs_scenery(&self) -> bool {
        self.scenery_required
    }

    pub(super) fn stage_scenery(
        &mut self,
        p: &Presentation,
        finest: &[&[f32]],
    ) -> Result<(), String> {
        if !self.scenery_required {
            return Ok(());
        }
        let ground = p.ground().ok_or("scenery requires ground")?;
        let mut next = scenery_from_art(ground, finest, p.scenery_quality)?;
        next.rows = p
            .scenery
            .as_ref()
            .expect("scenery admission source")
            .rows
            .clone();
        self.scenery = Some(next);
        Ok(())
    }

    /// Validation and current knowledge transfer precede every active-owner move.
    pub(super) fn adopt_knowledge(&mut self, p: &Presentation) -> Result<(), String> {
        if self.scenery_required && self.scenery.is_none() {
            return Err("prepare scenery placement facts before catalog commit".into());
        }
        if let Some(buildings) = &mut self.buildings {
            if let Some(&b) = buildings.walk.set_fallen(&p.fallen).first() {
                return Err(format!(
                    "replacement lacks known state of building {}",
                    buildings.walk.buildings.template(b)
                ));
            }
            buildings.walk.set_lod_scale(p.detail_scale[1]);
        }
        if let (Some(next), Some(active)) = (&mut self.props, &p.props) {
            next.adopt_knowledge(active);
        }
        if let (Some(next), Some(active)) = (&mut self.scenery, &p.scenery) {
            next.adopt_view_state(active, p.ground().ok_or("scenery requires ground")?);
        }
        Ok(())
    }
    pub(super) fn commit(self, p: &mut Presentation) {
        p.buildings = self.buildings;
        p.props = self.props;
        if self.scenery_required {
            p.scenery = self.scenery;
        }
    }
    pub(super) fn stump_mesh(&self, row: u32) -> Option<&[f32]> {
        let walk = self.scenery.as_ref()?;
        let stumps = walk.rows.get(Draws::Stump as usize - 1)?;
        let kind = stumps.iter().position(|&r| r == row)?;
        walk.stump_meshes.get(kind)?.as_deref()
    }
}

fn patch_props(
    input: &mut PropInput,
    assets: &BTreeMap<String, Asset>,
    placements: &BTreeMap<String, Placement>,
) -> Result<(), String> {
    let mut appearances: BTreeMap<String, Art> = std::mem::take(&mut input.appearances)
        .into_iter()
        .map(|a| (a.name.clone(), a))
        .collect();
    for (name, asset) in assets {
        match placements.get(name) {
            Some(placement) => {
                appearances.remove(name);
                if let Some(footprint) = placement.footprint {
                    let bounds = asset.first_corners().ok_or_else(|| {
                        format!("{name}: prop placement requires static state bounds")
                    })?;
                    appearances.insert(
                        name.clone(),
                        Art {
                            name: name.clone(),
                            scenery: placement.scenery.clone(),
                            family: placement.family.clone(),
                            footprint,
                            paints: placement.paints.clone(),
                            bounds,
                        },
                    );
                }
            }
            None => {
                if let Some(existing) = appearances.get_mut(name) {
                    existing.bounds = asset.first_corners().ok_or_else(|| {
                        format!("{name}: prop replacement requires static state bounds")
                    })?;
                }
            }
        }
    }
    for (name, placement) in placements {
        match &placement.wreck {
            Some(wreck) => {
                input.wrecks.insert(name.clone(), wreck.clone());
            }
            None => {
                input.wrecks.remove(name);
            }
        }
    }
    input.appearances = appearances.into_values().collect();
    if let Some(asset) = assets.get(STAND_IN_KIT) {
        input.stand_in = asset.state_corners(STAND_IN_MODULE);
    }
    Ok(())
}

pub(in crate::presentation) fn scenery_from_art(
    ground: &Ground,
    finest: &[&[f32]],
    quality: SceneryQuality,
) -> Result<SceneryWalk, String> {
    let names = ground.scenery_kinds();
    if finest.len() != names.len() {
        return Err(format!(
            "scenery: {} kinds' meshes for {} kinds",
            finest.len(),
            names.len()
        ));
    }
    let trees = &ground.biome.trees;
    let kinds = names
        .iter()
        .zip(finest)
        .map(|(name, tier)| {
            let tree = trees.species.iter().any(|s| &s.appearance == name);
            kinds::kind(tier, tree.then_some(&trees.felled)).map_err(|e| format!("{name}: {e}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let sizes: Vec<_> = kinds.iter().map(|k| k.size).collect();
    let trunks = kinds.iter().map(|k| k.trunk).collect();
    let mut walk = SceneryWalk::new(ground.scenery(&sizes), sizes, trunks);
    walk.stump_meshes = kinds.into_iter().map(|k| k.stump).collect();
    walk.set_quality(quality);
    Ok(walk)
}
