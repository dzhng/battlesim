//! Catalog admission stages geometry facts while the active observer keeps running.
mod facts;
pub(in crate::presentation) use facts::Facts;
mod static_art;
use static_art::StaticDraft;
pub(super) use static_art::{building_rows, scenery_from_art};

use super::lab::{self, Asset, ModelInput, Pose};
use super::*;
use crate::static_world::ground::Ground;
use crate::world::Detail;
use contract::identity::ContentId;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

static NEXT_REVISION: AtomicU32 = AtomicU32::new(1);
pub(super) fn next_revision() -> u32 {
    NEXT_REVISION.fetch_add(1, Ordering::Relaxed)
}

/// What a catalog draft is checked against: a revision that every change to
/// what rows resolve to renews, and the loaded battle fielded again (with a
/// kind it newly fields), held until an admission carries its rows and art.
pub(super) struct CatalogState {
    revision: u32,
    ground_revision: u32,
    staged: Option<StagedBattle>,
}

struct StagedBattle {
    grown: GrownBattle,
    tables: ClipTables,
}

impl CatalogState {
    pub(super) fn new() -> Self {
        Self {
            revision: next_revision(),
            ground_revision: 0,
            staged: None,
        }
    }

    /// Drafts staged before a change to what rows resolve to are stale.
    pub(super) fn renew(&mut self) {
        self.revision = next_revision();
    }

    pub(super) fn renew_ground(&mut self) {
        self.renew();
        self.ground_revision = self.ground_revision.wrapping_add(1);
    }

    pub(super) fn ground_revision(&self) -> u32 {
        self.ground_revision
    }
}
/// The world's records, read once per load.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct World {
    pub appearances: Vec<Row>,
    pub strings: String,
    pub(super) rows: HashMap<(String, String, u32, u32, bool), u32>,
}

impl World {
    /// The catalog name of an appearance row in this world's UTF-8 arena.
    pub fn name(&self, row: &Row) -> &str {
        &self.strings[row.name as usize..(row.name + row.name_len) as usize]
    }

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
    assets: Facts,
    detail: Detail,
    frontier: (usize, usize),
    battle: Option<BattleCatalog>,
    statics: StaticDraft,
}

struct BattleCatalog {
    units: Arc<UnitsSection>,
    tables: Arc<ClipTables>,
    inverse_binds: HashMap<String, Vec<Mat4>>,
    /// The staged battle this draft draws, when it carries one in.
    grown: Option<GrownBattle>,
}

#[derive(Clone)]
struct GrownBattle {
    generation: u32,
    input: BattleInput,
    effects: EffectPresentation,
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
        self.revision == presentation.catalog_state.revision
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

enum CatalogAppearance<'a> {
    Asset(&'a Asset),
    Battle(&'a Appearance),
}

impl<'a> CatalogView<'a> {
    /// Resolve by row name: prospective draft assets, then the loaded battle,
    /// then installed lab assets. Names may overlap: a draft intentionally
    /// replaces active art. Track-side lookup must use that same prospective
    /// rig as mesh admission, whose node indices belong to the replacement.
    fn appearance(&self, row: u32) -> Option<CatalogAppearance<'a>> {
        let world = self.world();
        let name = world.name(world.appearances.get(row as usize)?);
        if let Some(asset) = self.draft.and_then(|d| d.assets.get(name)) {
            return Some(CatalogAppearance::Asset(asset));
        }
        if let Some(appearance) = self
            .battle_units()
            .and_then(|units| units.appearances.get(name))
        {
            return Some(CatalogAppearance::Battle(appearance));
        }
        self.presentation
            .lab
            .assets
            .get(name)
            .map(CatalogAppearance::Asset)
    }

    /// The battle's catalog as this view resolves it: the draft's, else the
    /// loaded one.
    fn battle_units(&self) -> Option<&'a UnitsSection> {
        match self.draft.and_then(|d| d.battle.as_ref()) {
            Some(battle) => Some(&battle.units),
            None => self.presentation.loaded.as_ref().map(|l| &*l.units),
        }
    }

    pub fn world(&self) -> &'a World {
        self.draft.map_or(&self.presentation.world, |d| &d.world)
    }
    pub fn ground(&self) -> Option<&'a Ground> {
        self.presentation.ground()
    }
    pub fn rest_palette(&self, row: u32) -> Option<Vec<Joint>> {
        match self.appearance(row)? {
            CatalogAppearance::Asset(asset) => {
                asset_rest_palette(asset, self.world().appearances[row as usize].lies)
            }
            CatalogAppearance::Battle(_) => match self.draft.and_then(|d| d.battle.as_ref()) {
                Some(b) => {
                    let r = &self.world().appearances[row as usize];
                    let name = self.world().name(r);
                    posing::rest_palette(&b.units, &b.tables, &b.inverse_binds, name, r.lies)
                }
                None => self.presentation.rest_palette(self.active_row(row)?),
            },
        }
    }
    /// A node's canonical running-gear side, including prospective catalogs.
    pub fn track_side(&self, row: u32, node: usize) -> Option<crate::world::TrackSide> {
        let appearance = match self.appearance(row)? {
            CatalogAppearance::Asset(Asset::Animated { appearance, .. }) => appearance.as_ref(),
            CatalogAppearance::Asset(Asset::Static(_)) => return None,
            CatalogAppearance::Battle(appearance) => appearance,
        };
        let Appearance::Articulated { rig, .. } = appearance else {
            return None;
        };
        rig.track_side(node)
    }
    pub fn stump_mesh(&self, row: u32) -> Option<&'a [f32]> {
        if let Some(draft) = self.draft.filter(|draft| draft.statics.scenery.is_some()) {
            return draft.statics.stump_mesh(row);
        }
        self.presentation.stump_mesh(self.active_row(row)?)
    }
    fn active_row(&self, row: u32) -> Option<u32> {
        let w = self.world();
        let r = w.appearances.get(row as usize)?;
        let name = w.name(r);
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
    /// A content identity is held for delta staging only when its geometry
    /// facts survived the last catalog commit.
    pub fn retains_catalog_facts(&self, content: &ContentId) -> bool {
        self.lab.assets.contains_content(content)
    }

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
        wanted: BTreeMap<String, ContentId>,
        facts: BTreeMap<ContentId, Asset>,
        detail: Detail,
        placements: BTreeMap<String, Placement>,
    ) -> Result<CatalogDraft, String> {
        let assets = self.lab.assets.stage(wanted, facts)?;
        let battle = self.stage_battle_catalog(&assets)?;
        let (rows, _) = self
            .lab
            .frontier
            .unwrap_or((self.world.appearances.len(), self.world.strings.len()));
        let mut world = World::default();
        for r in self.world.appearances.iter().take(rows) {
            let text = |at: u32, len: u32| &self.world.strings[at as usize..(at + len) as usize];
            let name = self.world.name(r);
            // Dynamic battle rows are rebuilt from the new facts, so retired
            // corpse poses cannot leak through. That renumbers the rows after
            // them: static populations resolve theirs again by appearance.
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
                posing::register_appearance(&mut world, name, appearance);
            }
            if let Some(name) = self.loaded.as_ref().and_then(|l| l.input.stand_in.as_ref()) {
                posing::register_stand_in(&mut world, &name.appearance);
            }
            if let Some(grown) = &battle.grown {
                posing::register_wreck_rows(&mut world, &grown.input.effects);
            }
        }
        let statics = StaticDraft::stage(self, &assets, &placements, &mut world)?;
        let frontier = (world.appearances.len(), world.strings.len());
        lab::register_rows(&mut world, &assets);
        Ok(CatalogDraft {
            revision: self.catalog_state.revision,
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
            let units = (*battle.units).clone();
            match battle.grown {
                Some(grown) => {
                    loaded.input = BattleInput {
                        units,
                        ..grown.input
                    };
                    loaded.effects.replace_presentation(grown.effects);
                    let staged = &mut self.catalog_state.staged;
                    if staged
                        .as_ref()
                        .is_some_and(|s| s.grown.generation == grown.generation)
                    {
                        *staged = None;
                    }
                }
                None => loaded.input.units = units,
            }
            loaded.units = battle.units;
            loaded.tables = battle.tables;
            loaded.inverse_binds = battle.inverse_binds;
        }
        self.catalog_state.renew();
        self.statics.invalidate();
        Ok(true)
    }

    /// Stage the loaded battle fielded again (with a kind it newly fields)
    /// for the next catalog admission, which draws it without loading the
    /// world anew: what the side saw, the effects in flight and every pose
    /// carry on. False when `input` is another battle (another side, sun or
    /// body), or drops a clip a live pose plays: that world loads anew.
    pub fn stage_battle(&mut self, input: WorldInput, tables: ClipTables) -> Result<bool, String> {
        let (Some(loaded), Some(battle)) = (&self.loaded, input.battle) else {
            return Ok(false);
        };
        let effects = EffectPresentation::from_value(&battle.effects.presentation)?;
        let (a, b) = (&loaded.input, &battle);
        let same = a.side == b.side
            && a.tick_hz == b.tick_hz
            && [a.soldier.radius_m, a.soldier.height_m] == [b.soldier.radius_m, b.soldier.height_m]
            && [a.xray.own, a.xray.selected, a.xray.ally]
                == [b.xray.own, b.xray.selected, b.xray.ally]
            && a.blocked == b.blocked
            && a.stand_in == b.stand_in
            && loaded.shadow_fall == shadow_fall(input.sun_azimuth, input.sun_elevation)
            && loaded.effects.presentation().capacity == effects.capacity;
        // A live pose the new catalog cannot draw needs the world loaded anew.
        if !same || loaded.driver.validate_catalog(&battle.units).is_err() {
            return Ok(false);
        }
        self.catalog_state.staged = Some(StagedBattle {
            grown: GrownBattle {
                generation: next_revision(),
                input: battle,
                effects,
            },
            tables,
        });
        Ok(true)
    }

    /// What a staged battle draws, by appearance: its units' and its wrecks'
    /// looks, whose art the admission that carries it in must bring.
    pub fn staged_appearances(&self) -> Vec<String> {
        let Some(staged) = &self.catalog_state.staged else {
            return Vec::new();
        };
        let input = &staged.grown.input;
        (input.units.appearances.keys())
            .chain(
                input
                    .effects
                    .wreck_looks
                    .values()
                    .map(|look| &look.appearance),
            )
            .cloned()
            .collect()
    }

    fn stage_battle_catalog(&self, assets: &Facts) -> Result<Option<BattleCatalog>, String> {
        let Some(loaded) = &self.loaded else {
            return Ok(None);
        };
        let staged = self.catalog_state.staged.as_ref();
        let (units, tables) = match staged {
            Some(s) => (&s.grown.input.units, &s.tables),
            None => (&*loaded.units, &*loaded.tables),
        };
        if staged.is_none()
            && !assets
                .keys()
                .any(|name| units.appearances.contains_key(name))
        {
            return Ok(None);
        }
        let mut units = units.clone();
        let mut tables = tables.clone();
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
            grown: staged.map(|s| s.grown.clone()),
        }))
    }
}
