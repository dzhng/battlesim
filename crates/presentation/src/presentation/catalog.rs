//! Catalog admission stages geometry facts while the active observer keeps running.
mod static_art;
pub(super) use static_art::scenery_from_art;
use static_art::StaticDraft;

use super::lab::{self, Asset, ModelInput, Pose};
use super::*;
use crate::static_world::ground::Ground;
use crate::world::Detail;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU32, Ordering};

static NEXT_REVISION: AtomicU32 = AtomicU32::new(1);
pub(super) fn next_revision() -> u32 {
    NEXT_REVISION.fetch_add(1, Ordering::Relaxed)
}
/// The world's records, read once per load.
#[derive(Clone, Default)]
pub struct World {
    pub appearances: Vec<Row>,
    pub strings: String,
    pub(super) rows: HashMap<(String, String, u32, u32, bool), u32>,
}

impl World {
    pub(super) fn row(
        &mut self,
        name: &str,
        state: &str,
        lies: u32,
        skin: u32,
        placed: bool,
    ) -> u32 {
        self.drawn_row(name, state, lies, skin, placed, 0)
    }

    pub(super) fn drawn_row(
        &mut self,
        name: &str,
        state: &str,
        lies: u32,
        skin: u32,
        placed: bool,
        draws: u32,
    ) -> u32 {
        let key = (name.to_owned(), state.to_owned(), lies, draws, placed);
        if let Some(&row) = self.rows.get(&key) {
            return row;
        }
        let mut text = |s: &str| {
            let at = self.strings.len() as u32;
            self.strings.push_str(s);
            (at, s.len() as u32)
        };
        let (name_at, name_len) = text(name);
        let (state_at, state_len) = text(state);
        let row = self.appearances.len() as u32;
        self.appearances.push(Row {
            name: name_at,
            name_len,
            state: state_at,
            state_len,
            lies,
            skin,
            placed: u32::from(placed),
            draws,
        });
        self.rows.insert(key, row);
        row
    }
}

/// Authored placement metadata lives in the asset catalog, outside mesh BGAT.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Placement {
    pub footprint: Option<Vec3>,
    pub scenery: Option<String>,
    pub family: Option<String>,
    pub paints: Option<Vec<Vec3>>,
    pub wreck: Option<String>,
}

/// Owned prospective catalog. Its rows are the exact rows moved into the live
/// presentation at commit; no frame, observation or observer is duplicated.
pub struct CatalogDraft {
    revision: u32,
    world: World,
    assets: BTreeMap<String, Asset>,
    detail: Detail,
    frontier: (usize, usize),
    battle: Option<BattleCatalog>,
    statics: StaticDraft,
}

struct BattleCatalog {
    units: Arc<UnitsSection>,
    tables: Arc<ClipTables>,
    inverse_binds: HashMap<String, Vec<Mat4>>,
}

/// A synchronous reader shared by native admission and staged replacement.
#[derive(Clone, Copy)]
pub struct CatalogView<'a> {
    presentation: &'a Presentation,
    draft: Option<&'a CatalogDraft>,
}

impl CatalogDraft {
    pub fn needs_scenery(&self) -> bool {
        self.statics.needs_scenery()
    }

    /// Finest scenery geometry comes from the same shared bundle decoder the
    /// GPU admission reads. The resulting placement facts remain prospective.
    pub fn stage_scenery(
        &mut self,
        presentation: &Presentation,
        finest: &[&[f32]],
    ) -> Result<(), String> {
        if !self.is_current(presentation) {
            return Err("appearance catalog changed during admission".into());
        }
        self.statics.stage_scenery(presentation, finest)
    }

    pub fn is_current(&self, presentation: &Presentation) -> bool {
        self.revision == presentation.catalog_revision
    }

    pub fn view<'a>(&'a self, presentation: &'a Presentation) -> Result<CatalogView<'a>, String> {
        if !self.is_current(presentation) {
            return Err("appearance catalog changed during admission".into());
        }
        Ok(CatalogView {
            presentation,
            draft: Some(self),
        })
    }
}

impl<'a> CatalogView<'a> {
    pub fn world(&self) -> &'a World {
        self.draft.map_or(&self.presentation.world, |d| &d.world)
    }
    pub fn ground(&self) -> Option<&'a Ground> {
        self.presentation.ground()
    }
    pub fn rest_palette(&self, row: u32) -> Option<Vec<Joint>> {
        let world = self.world();
        let r = world.appearances.get(row as usize)?;
        let name = &world.strings[r.name as usize..(r.name + r.name_len) as usize];
        if let Some(asset) = self.draft.and_then(|d| d.assets.get(name)) {
            return asset_rest_palette(asset, r.lies);
        }
        self.presentation.rest_palette(self.active_row(row)?)
    }
    pub fn stump_mesh(&self, row: u32) -> Option<&'a [f32]> {
        if let Some(draft) = self.draft.filter(|d| d.statics.needs_scenery()) {
            return draft.statics.stump_mesh(row);
        }
        self.presentation.stump_mesh(self.active_row(row)?)
    }
    fn active_row(&self, row: u32) -> Option<u32> {
        let w = self.world();
        let r = w.appearances.get(row as usize)?;
        let name = &w.strings[r.name as usize..(r.name + r.name_len) as usize];
        let state = &w.strings[r.state as usize..(r.state + r.state_len) as usize];
        self.presentation
            .world
            .rows
            .get(&(name.into(), state.into(), r.lies, r.draws, r.placed != 0))
            .copied()
    }
}

pub(super) fn asset_rest_palette(asset: &Asset, lies: u32) -> Option<Vec<Joint>> {
    if lies == NONE {
        return asset.palette(&Pose::Rest).map(|p| p.0);
    }
    let Asset::Animated { appearance, .. } = asset else {
        return None;
    };
    let Appearance::Skinned { corpse_poses, .. } = appearance.as_ref() else {
        return None;
    };
    let pose = corpse_poses.get(lies as usize)?;
    asset
        .palette(&Pose::Clip {
            clip: pose.clip.clone(),
            phase: pose.phase,
        })
        .map(|p| p.0)
}

impl Presentation {
    /// Canonical battle/static rows precede the replaceable tooling suffix.
    pub fn canonical_appearance_rows(&self) -> usize {
        self.lab
            .frontier
            .map_or(self.world.appearances.len(), |(rows, _)| rows)
    }

    pub fn catalog(&self) -> CatalogView<'_> {
        CatalogView {
            presentation: self,
            draft: None,
        }
    }

    /// Parse source assets before this call. Staging changes no active rows,
    /// poses, model requests or knowledge, and holds no observer borrow.
    pub fn stage_catalog(
        &self,
        assets: BTreeMap<String, Asset>,
        detail: Detail,
        placements: BTreeMap<String, Placement>,
    ) -> Result<CatalogDraft, String> {
        let battle = self.stage_battle_catalog(&assets)?;
        let (rows, _) = self
            .lab
            .frontier
            .unwrap_or((self.world.appearances.len(), self.world.strings.len()));
        let mut world = World::default();
        for r in self.world.appearances.iter().take(rows) {
            let text = |at: u32, len: u32| &self.world.strings[at as usize..(at + len) as usize];
            let name = text(r.name, r.name_len);
            // Static rows retain their indices. Dynamic battle rows are rebuilt
            // from the new facts, so retired corpse poses cannot leak through.
            if r.placed == 0
                && battle
                    .as_ref()
                    .is_some_and(|b| b.units.appearances.contains_key(name))
            {
                continue;
            }
            world.drawn_row(
                name,
                text(r.state, r.state_len),
                r.lies,
                r.skin,
                r.placed != 0,
                r.draws,
            );
        }
        if let Some(battle) = &battle {
            for (name, appearance) in &battle.units.appearances {
                match appearance {
                    Appearance::Skinned {
                        joints,
                        corpse_poses,
                        ..
                    } => {
                        world.row(name, "", NONE, joints.len() as u32, false);
                        for lies in 0..corpse_poses.len() {
                            world.row(name, "", lies as u32, 0, false);
                        }
                    }
                    Appearance::Articulated { nodes, .. } => {
                        world.row(name, "", NONE, nodes.len() as u32, false);
                    }
                }
            }
        }
        let statics = StaticDraft::stage(self, &assets, &placements, &mut world)?;
        let frontier = (world.appearances.len(), world.strings.len());
        lab::register_rows(&mut world, &assets);
        Ok(CatalogDraft {
            revision: self.catalog_revision,
            world,
            assets,
            detail,
            frontier,
            battle,
            statics,
        })
    }

    /// Returns false for stale work. All validation precedes the infallible
    /// moves; the renderer commits its matching GPU owners in the same call.
    pub fn commit_catalog(
        &mut self,
        mut draft: CatalogDraft,
        models: Vec<ModelInput>,
    ) -> Result<bool, String> {
        if !draft.is_current(self) {
            return Ok(false);
        }
        lab::validate_models(&models)?;
        if let (Some(loaded), Some(battle)) = (&self.loaded, &draft.battle) {
            loaded.driver.validate_catalog(&battle.units)?;
        }
        draft.statics.adopt_knowledge(self)?;
        draft.statics.commit(self);
        self.world = draft.world;
        self.lab.assets = draft.assets;
        self.lab.detail = Some(draft.detail);
        self.lab.frontier = Some(draft.frontier);
        self.lab.models = models;
        if let (Some(loaded), Some(battle)) = (&mut self.loaded, draft.battle) {
            loaded
                .drawn
                .replace_catalog(battle.units.clone(), battle.tables.clone());
            loaded.input.units = (*battle.units).clone();
            loaded.units = battle.units;
            loaded.tables = battle.tables;
            loaded.inverse_binds = battle.inverse_binds;
        }
        self.catalog_revision = next_revision();
        self.statics.invalidate();
        Ok(true)
    }

    fn stage_battle_catalog(
        &self,
        assets: &BTreeMap<String, Asset>,
    ) -> Result<Option<BattleCatalog>, String> {
        let Some(loaded) = &self.loaded else {
            return Ok(None);
        };
        if !assets
            .keys()
            .any(|name| loaded.units.appearances.contains_key(name))
        {
            return Ok(None);
        }
        let mut units = (*loaded.units).clone();
        let mut tables = (*loaded.tables).clone();
        for (name, old) in &mut units.appearances {
            let Some(asset) = assets.get(name) else {
                continue;
            };
            let Asset::Animated { appearance, table } = asset else {
                return Err(format!("{name}: battle appearance must remain animated"));
            };
            if std::mem::discriminant(old) != std::mem::discriminant(appearance.as_ref()) {
                return Err(format!("{name}: battle appearance changed model kind"));
            }
            *old = *appearance.clone();
            if let Appearance::Skinned { table: key, .. } = old {
                *key = format!("catalog:{name}");
                tables.insert(key.clone(), table.clone());
            }
        }
        loaded.driver.validate_catalog(&units)?;
        let mut inverse_binds = HashMap::new();
        for (name, a) in &units.appearances {
            if let Appearance::Skinned {
                joints,
                inverse_binds: given,
                ..
            } = a
            {
                inverse_binds.insert(name.clone(), skin::inverse_binds(joints, given));
            }
        }
        tables.retain(|key, _| {
            units
                .appearances
                .values()
                .any(|a| matches!(a,Appearance::Skinned {table,..} if table==key))
        });
        Ok(Some(BattleCatalog {
            units: Arc::new(units),
            tables: Arc::new(tables),
            inverse_binds,
        }))
    }
}
