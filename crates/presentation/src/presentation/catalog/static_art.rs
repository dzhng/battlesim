//! Replaceable placement facts, distinct from the knowledge observing them.
use super::*;
use crate::static_world::ground::kinds;
use crate::static_world::props::{Art, STAND_IN_KIT, STAND_IN_MODULE};

pub(super) struct StaticDraft {
    pub(super) buildings: Option<BuildingPopulation>,
    pub(super) props: Option<PropPopulation>,
    pub(super) scenery: Option<SceneryWalk>,
    /// The scenery's rows in the draft's world, once `scenery_required`.
    scenery_rows: Vec<Vec<u32>>,
    scenery_required: bool,
}

/// Each building module's row: its kit, and the module as the state.
pub(in crate::presentation) fn building_rows(world: &mut World, buildings: &Buildings) -> Vec<u32> {
    (0..buildings.modules.len())
        .map(|m| {
            let (kit, module) = buildings.module(m);
            world.row(kit, module, NONE, 0, true)
        })
        .collect()
}

/// Each scenery kind's row per way it draws. Scenery draws its appearances'
/// summer state (winter will be the next biome's).
fn scenery_rows(world: &mut World, kinds: &[String]) -> Vec<Vec<u32>> {
    DRAWS
        .iter()
        .map(|&d| {
            kinds
                .iter()
                .map(|name| world.drawn_row(name, "summer", NONE, 0, true, d as u32))
                .collect()
        })
        .collect()
}

impl StaticDraft {
    pub(super) fn stage(
        p: &Presentation,
        assets: &Facts,
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
                let rows = building_rows(world, &buildings);
                Ok::<_, String>(BuildingPopulation {
                    walk: active.walk.replace_buildings(buildings),
                    rows,
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
        let scenery_rows = p
            .scenery
            .as_ref()
            .map_or_else(Vec::new, |walk| scenery_rows(world, &walk.scenery.kinds));
        Ok(Self {
            buildings,
            props,
            scenery: None,
            scenery_rows,
            scenery_required: p.scenery.is_some(),
        })
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
        next.rows = self.scenery_rows.clone();
        self.scenery = Some(next);
        Ok(())
    }

    /// Validation and current knowledge transfer precede every active-owner move.
    pub(super) fn adopt_knowledge(&mut self, p: &Presentation) -> Result<(), String> {
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
        if let Some(next) = self.scenery {
            p.scenery = Some(next);
        } else if let Some(active) = &mut p.scenery {
            active.rows = self.scenery_rows;
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
    assets: &Facts,
    placements: &BTreeMap<String, Placement>,
) -> Result<(), String> {
    let mut appearances: BTreeMap<String, Art> = std::mem::take(&mut input.appearances)
        .into_iter()
        .map(|a| (a.name.clone(), a))
        .collect();
    for (name, asset) in assets.iter() {
        match placements.get(name) {
            Some(placement) => {
                appearances.remove(name);
                if let Some(footprint) = placement.footprint {
                    let (bounds, meshes) = asset.first_state().ok_or_else(|| {
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
                            meshes,
                        },
                    );
                }
            }
            None => {
                if let Some(existing) = appearances.get_mut(name) {
                    (existing.bounds, existing.meshes) = asset.first_state().ok_or_else(|| {
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

impl Presentation {
    /// True once the ground's scenery has placement facts, including empty maps.
    pub fn scenery_is_placed(&self) -> bool {
        self.scenery.is_some()
    }

    /// Places the ground's scenery, given each of its kinds' finest mesh
    /// (`Ground::scenery_kinds` order) in the renderer's world vertex
    /// (`ground::mesh::VERTEX_FLOATS`, 1 in the last float for foliage):
    /// their sizes, a tree's trunk and crown, and its stump (`kinds`) are
    /// read off it. Rows are admitted into the current canonical world.
    pub fn place_scenery(&mut self, finest: &[&[f32]]) -> Result<(), String> {
        let ground = self
            .ground
            .as_ref()
            .ok_or("scenery stands on a ground: set_ground first")?;
        let walk = catalog::scenery_from_art(ground, finest, self.scenery_quality)?;
        self.catalog_state.renew();
        self.clear_lab_rows();
        self.scenery = Some(walk);
        self.register_scenery_rows();
        self.register_lab_rows();
        self.felled_from = None;
        self.cleared_from = (0, 0);
        self.statics.invalidate();
        Ok(())
    }

    pub(in crate::presentation) fn register_scenery_rows(&mut self) {
        if let Some(walk) = &mut self.scenery {
            walk.rows = scenery_rows(&mut self.world, &walk.scenery.kinds);
        }
    }
}
