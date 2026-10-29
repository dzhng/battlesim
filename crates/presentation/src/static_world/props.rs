//! The map's props and what a side knows of them, as appearances: which
//! installed scenery appearance draws each prop the simulation places, fitted
//! to its box, and the walk that chooses what a view draws of them, as the
//! static world's batches.
//!
//! The simulation's box is the authority (sight, rounds and movement meet
//! it); an appearance is authored to a declared box (its footprint) and
//! fitted to each placed box:
//! - a prop kind is drawn by the appearances of the scenery kind its catalog
//!   `appearance` binds (the world layout's `propAppearance`), never by a
//!   list here;
//! - a kind's appearances are those of no region and those of the map's own
//!   region, never another region's; where the map's region has looks of its
//!   own for the kind, only those;
//! - an appearance is chosen among them by the footprint nearest the box;
//!   a wreck is the one exception: it is its own unit's wreck, whatever box
//!   it lies on, so you can tell which unit died by looking;
//! - it is scaled per axis from its footprint to the box, except a module (a
//!   wall, a fence, a sandbag line) that is repeated along the box's long
//!   side instead of stretched;
//! - where the appearance has paints (a car's), each body wears one, chosen
//!   by the authored prop it is, so a shoved car keeps its colour;
//! - a kind no appearance is fitted to (a body the simulation places before
//!   its model is made) is the stand-in kit's unit box stretched to its box
//!   and tinted by its kind, where the stand-in is installed: a body the
//!   simulation holds is never invisible. A forest's trees and a building's
//!   parts are drawn by the scenery and the buildings, never here.
//!
//! What a side draws is what it knows: the map's props, less those a known
//! prop replaces, plus every known prop not destroyed (less a building's
//! part, which its building's state draws, and a wreck the cook-off draws
//! moving). A replacement is atomic: the list that drops a wall carries its
//! rubble, and a fall the side has not seen leaves the wall standing. A
//! body it watches being shoved is drawn where the frame blends it to, ahead
//! of its knowledge, which re-learns a shoved body only in steps; once the
//! shove ends it holds there until that knowledge moves.
//!
//! Each drawn model takes the units' tier rule (`view::model_detail`) by its
//! appearance's size grown by its largest scale, and is listed by look (its
//! appearance, state, and whether movers stand on it) and tier.
mod admission;
mod bodies;
pub use bodies::{map_props, KnownBody, MapProp};

use crate::records::{CAST_PASS, VIEW_PASS};
use crate::seed::Key;
use crate::shadow::{CasterVolumes, CASCADES};
use crate::static_world::batches::{List, Population};
use crate::static_world::index::BoundsIndex;
use crate::static_world::walk::Record;
use crate::view::{caster_tier, model_detail, model_fog, View, CULLED, SHADOW_MARGIN_M, TIERS};
use crate::world::{Bounds, Corners, Detail};
use serde::Deserialize;
use sim::publication::reader::PropDisplayPose;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::f64::consts::FRAC_PI_2;

/// What draws a prop's row (`Appearance.draws`): a model fogged face by
/// face, or one movers stand on, which takes the ground paint.
pub const DRAWS_PROP: u32 = 7;
pub const DRAWS_PROP_GROUND: u32 = 8;

/// The stand-in kit, and its one module: a box a metre a side, its base
/// centred on the origin (`scene-assets` `standInKit.ts`).
pub const STAND_IN_KIT: &str = "city_kit_prototype";
pub const STAND_IN_MODULE: &str = "unit_box";
/// The state a prop appearance draws.
const STATE: &str = "default";
/// The scenery kind that draws a unit's wreck: each unit its own.
const WRECK: &str = "wreck";
/// Square chunks the map's props are bucketed in, metres.
const CHUNK_M: f64 = 64.0;
/// The authored sun can stretch a small caster's ground shadow beyond its
/// own silhouette. Keep twice the projected diameter before omitting it.
const SHADOW_FOOTPRINT_MARGIN: f64 = 2.0;

/// A prop kind's binding in the world layout (`propAppearance`).
#[derive(Clone, Debug, Deserialize)]
struct Binding {
    drawn_by: String,
    stand_in_tint: [f64; 3],
    #[serde(default)]
    modular: bool,
    #[serde(default)]
    map_only: bool,
}

/// The world layout (`sim::world::export::layout_json`) as the props read
/// it: the prop kinds, what draws each, the kinds that stop a mover class,
/// and each unit type's hull model.
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PropLayout {
    prop_kinds: Vec<String>,
    prop_appearance: HashMap<String, Binding>,
    blocking_prop_kinds: HashMap<String, Vec<String>>,
    unit_kinds: Vec<String>,
    unit_appearance: HashMap<String, String>,
}

impl PropLayout {
    pub fn parse(layout: &str) -> Result<Self, String> {
        serde_json::from_str(layout).map_err(|e| format!("layout: {e}"))
    }
}

/// An installed static appearance a prop may be drawn as: its scenery kind,
/// regional family, the box it is authored to (half extents), its paints
/// (sRGB) and its first state's bounds, which say how big it is drawn.
#[derive(Clone, Debug, Deserialize)]
pub struct Art {
    pub name: String,
    #[serde(default)]
    pub scenery: Option<String>,
    #[serde(default)]
    pub family: Option<String>,
    pub footprint: [f64; 3],
    #[serde(default)]
    pub paints: Option<Vec<[f64; 3]>>,
    pub bounds: Corners,
}

/// What the props are drawn from: the installed appearances a prop may be
/// drawn as, each vehicle appearance's own wreck, the stand-in kit's first
/// state's bounds when it is installed with its module, the map's regional family,
/// and the models' tier thresholds (`presentation.models`).
#[derive(Clone, Debug, Deserialize)]
pub struct PropInput {
    pub appearances: Vec<Art>,
    #[serde(default)]
    pub wrecks: BTreeMap<String, String>,
    #[serde(default)]
    pub stand_in: Option<Corners>,
    #[serde(default)]
    pub family: Option<String>,
    pub detail: Detail,
    #[serde(default = "default_dynamic")]
    pub dynamic: bool,
}

fn default_dynamic() -> bool {
    true
}

/// A tint as the GPU reads it, linear.
fn linear(c: [f64; 3]) -> [f32; 3] {
    c.map(|c| super::srgb_to_linear(c) as f32)
}

/// An appearance a kind chooses among.
struct Candidate {
    name: String,
    footprint: [f64; 3],
    /// Linear tints its paintable surfaces take.
    paints: Option<Vec<[f32; 3]>>,
    size: Bounds,
}

/// The fit: each bound kind's candidates in choosing order, each unit
/// type's wreck, and the stand-in.
pub struct PropArt {
    source: PropInput,
    layout: PropLayout,
    candidates: Vec<Candidate>,
    by_kind: HashMap<String, Vec<usize>>,
    wrecks: HashMap<String, usize>,
    /// The kinds whose body stops no mover class: surfaces movers stand on.
    walked_on: HashSet<String>,
    stand_in: Option<Bounds>,
    detail: Detail,
    dynamic: bool,
}

/// A prop's box: centred at `center` on ground level `base_z`, turned by
/// `yaw`, and, for a wreck, its unit type.
#[derive(Clone, Debug, PartialEq)]
pub struct PropBox {
    pub kind: String,
    pub center: [f64; 2],
    pub yaw: f64,
    pub half: [f64; 3],
    pub base_z: f64,
    pub wreck_of: Option<String>,
}

/// Where a shoved body stands at the drawn moment.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Shoved {
    center: [f64; 2],
    yaw: f64,
    base_z: f64,
}

impl Shoved {
    /// `b` moved here.
    fn on(&self, b: &PropBox) -> PropBox {
        PropBox {
            center: self.center,
            yaw: self.yaw,
            base_z: self.base_z,
            ..b.clone()
        }
    }
}

/// What a model is drawn as: an appearance, its state, and `Appearance.draws`.
pub type Look = (String, String, u32);

/// One model a prop draws: its look (an index into the looks), its record,
/// and the height and reach its tier and culling are chosen by.
#[derive(Clone, Debug, PartialEq)]
pub struct Model {
    pub look: usize,
    pub record: Record,
    size: f64,
    radius: f64,
}

impl PropArt {
    /// The fit of `input`'s art under `layout`, whose prop bindings own colour.
    pub fn new(layout: PropLayout, input: PropInput) -> Result<Self, String> {
        for (kind, binding) in &layout.prop_appearance {
            if binding
                .stand_in_tint
                .iter()
                .any(|v| !(0.0..=1.0).contains(v))
            {
                return Err(format!(
                    "prop {kind}: appearance.stand_in_tint must be [r, g, b] in [0, 1]"
                ));
            }
        }
        let source = input.clone();
        let PropInput {
            appearances,
            wrecks: unit_wrecks,
            stand_in,
            family,
            detail,
            dynamic,
        } = input;
        let blocking: HashSet<&String> = layout.blocking_prop_kinds.values().flatten().collect();
        let walked_on = layout
            .prop_appearance
            .keys()
            .filter(|k| !blocking.contains(k))
            .cloned()
            .collect();
        let stand_in = stand_in.map(|corners| corners.bounds());
        let mut art = PropArt {
            source,
            candidates: Vec::new(),
            by_kind: HashMap::new(),
            wrecks: HashMap::new(),
            walked_on,
            stand_in,
            detail,
            dynamic,
            layout,
        };
        let index: HashMap<&str, usize> = appearances
            .iter()
            .enumerate()
            .map(|(i, a)| (a.name.as_str(), i))
            .collect();
        for a in &appearances {
            art.candidates.push(Candidate {
                name: a.name.clone(),
                footprint: a.footprint,
                paints: a
                    .paints
                    .as_ref()
                    .map(|p| p.iter().map(|&c| linear(c)).collect()),
                size: a.bounds.bounds(),
            });
        }
        for (unit, model) in &art.layout.unit_appearance {
            if let Some(&c) = unit_wrecks.get(model).and_then(|w| index.get(w.as_str())) {
                art.wrecks.insert(unit.clone(), c);
            }
        }
        let mut own: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, a) in appearances.iter().enumerate() {
            let Some(scenery) = a.scenery.as_deref().filter(|&s| s != WRECK) else {
                continue;
            };
            if a.family.is_some() && a.family != family {
                continue;
            }
            let lists = if a.family.is_none() {
                &mut art.by_kind
            } else {
                &mut own
            };
            for (kind, b) in &art.layout.prop_appearance {
                if b.drawn_by == scenery {
                    lists.entry(kind.clone()).or_default().push(i);
                }
            }
        }
        art.by_kind.extend(own);
        for list in art.by_kind.values_mut() {
            list.sort_by(|&a, &b| art.candidates[a].name.cmp(&art.candidates[b].name));
        }
        Ok(art)
    }

    fn binding(&self, kind: &str) -> Option<&Binding> {
        self.layout.prop_appearance.get(kind)
    }

    /// The appearance drawing `b`: a wreck its unit's own, anything else the
    /// one whose footprint is nearest the box (least total log scale).
    fn choose(&self, b: &PropBox) -> Option<usize> {
        if self.binding(&b.kind).is_some_and(|x| x.drawn_by == WRECK) {
            return b
                .wreck_of
                .as_ref()
                .and_then(|u| self.wrecks.get(u))
                .copied();
        }
        let mut best = None;
        let mut least = f64::INFINITY;
        for &c in self.by_kind.get(&b.kind).into_iter().flatten() {
            let f = self.candidates[c].footprint;
            let misfit: f64 = (0..3).map(|a| libm::log(b.half[a] / f[a]).abs()).sum();
            if misfit < least {
                (least, best) = (misfit, Some(c));
            }
        }
        best
    }

    /// Whether `kind` is drawn as the stand-in box: it is bound, no
    /// appearance is fitted to it, and neither a forest nor a building draws it.
    fn stands_in(&self, kind: &str) -> bool {
        self.stand_in.is_some()
            && self
                .binding(kind)
                .is_some_and(|b| b.drawn_by != "forest" && b.drawn_by != "building")
            && !self.by_kind.contains_key(kind)
    }

    fn draws(&self, kind: &str) -> u32 {
        if self.walked_on.contains(kind) {
            DRAWS_PROP_GROUND
        } else {
            DRAWS_PROP
        }
    }

    /// Whether `kind` is a forest's tree: the scenery draws it.
    fn is_tree(&self, kind: &str) -> bool {
        self.binding(kind).is_some_and(|b| b.drawn_by == "forest")
    }

    /// Every look drawing the map's `props` could need: the one each takes,
    /// and every look of a kind not `map_only` (a battle can leave or place
    /// those anywhere: rubble, a dropped crate, a sandbag line), every
    /// unit's own wreck and the stand-in a kind without art takes.
    pub fn looks(&self, props: &[PropBox], looks: &mut Looks) {
        let mut models = Vec::new();
        for p in props.iter().filter(|p| !self.is_tree(&p.kind)) {
            self.fit(p, None, looks, &mut models);
        }
        if !self.dynamic {
            return;
        }
        let mut kinds: Vec<&String> = self.layout.prop_appearance.keys().collect();
        kinds.sort();
        for kind in kinds {
            let b = &self.layout.prop_appearance[kind];
            if b.map_only {
                continue;
            }
            let draws = self.draws(kind);
            for &c in self.by_kind.get(kind).into_iter().flatten() {
                looks.intern(&self.candidates[c].name, STATE, draws);
            }
            if b.drawn_by == WRECK {
                let mut units: Vec<_> = self.wrecks.iter().collect();
                units.sort();
                for (_, &c) in units {
                    looks.intern(&self.candidates[c].name, STATE, draws);
                }
            }
            if self.stands_in(kind) {
                looks.intern(STAND_IN_KIT, STAND_IN_MODULE, DRAWS_PROP);
            }
        }
    }

    /// The models drawing `b` (the authored prop `authored` where it is one),
    /// appended to `out`: several for a repeated module, the stand-in for a
    /// kind with no art, none for a kind neither draws.
    pub fn fit(&self, b: &PropBox, authored: Option<f64>, looks: &mut Looks, out: &mut Vec<Model>) {
        let Some(c) = self.choose(b) else {
            if let (true, Some(size)) = (self.stands_in(&b.kind), &self.stand_in) {
                let [hx, hy, hz] = b.half;
                let tint = linear(self.layout.prop_appearance[&b.kind].stand_in_tint);
                let look = looks.intern(STAND_IN_KIT, STAND_IN_MODULE, DRAWS_PROP);
                let scale = [2.0 * hx, 2.0 * hy, 2.0 * hz];
                out.push(placed(look, b, 0.0, scale, 0.0, tint, *size));
            }
            return;
        };
        let cand = &self.candidates[c];
        let [fx, fy, fz] = cand.footprint;
        let [hx, hy, hz] = b.half;
        let look = looks.intern(&cand.name, STATE, self.draws(&b.kind));
        let size = cand.size;
        if !self.binding(&b.kind).is_some_and(|x| x.modular) {
            let tint = cand
                .paints
                .as_ref()
                .map_or([1.0; 3], |p| paint_of(p, b, authored));
            out.push(placed(
                look,
                b,
                0.0,
                [hx / fx, hy / fy, hz / fz],
                0.0,
                tint,
                size,
            ));
            return;
        }
        // A module runs along its own long axis; turn it to the box's long side.
        let turn = if (hx >= hy) == (fx >= fy) {
            0.0
        } else {
            FRAC_PI_2
        };
        let (long, across) = if turn == 0.0 { (hx, hy) } else { (hy, hx) };
        let count = (long / fx).round().max(1.0);
        let step = 2.0 * long / count;
        let scale = [long / (count * fx), across / fy, hz / fz];
        for k in 0..count as usize {
            let along = -long + step * (k as f64 + 0.5);
            out.push(placed(look, b, along, scale, turn, [1.0; 3], size));
        }
    }
}

/// The paint `b` wears of `paints`: drawn from the authored prop it is,
/// else from where it stands, so the same car is the same colour in every
/// battle.
fn paint_of(paints: &[[f32; 3]], b: &PropBox, authored: Option<f64>) -> [f32; 3] {
    let key = match authored {
        Some(id) => Key::new().num(id),
        None => Key::new()
            .num((b.center[0] * 10.0).round())
            .num((b.center[1] * 10.0).round()),
    };
    let sample = key.unit();
    paints[(sample * paints.len() as f64).floor() as usize]
}

/// A model on `b`, shifted `along` its local +x after turning by `turn`.
fn placed(
    look: usize,
    b: &PropBox,
    along: f64,
    scale: [f64; 3],
    turn: f64,
    tint: [f32; 3],
    size: Bounds,
) -> Model {
    let yaw = b.yaw + turn;
    let at = [
        b.center[0] + along * libm::cos(yaw),
        b.center[1] + along * libm::sin(yaw),
        b.base_z,
    ];
    let grow = scale[0].max(scale[1]).max(scale[2]);
    let mut record = [0.0f32; 16];
    record[..4].copy_from_slice(&[at[0] as f32, at[1] as f32, at[2] as f32, yaw as f32]);
    record[8..11].copy_from_slice(&tint);
    record[11] = -1.0;
    record[12..15].copy_from_slice(&scale.map(|s| s as f32));
    Model {
        look,
        record,
        size: size.size * grow,
        radius: size.radius * grow,
    }
}

/// The looks props draw, interned: each becomes a row of the world.
#[derive(Default)]
pub struct Looks {
    pub list: Vec<Look>,
    index: HashMap<Look, usize>,
}

impl Looks {
    fn intern(&mut self, name: &str, state: &str, draws: u32) -> usize {
        let key = (name.to_owned(), state.to_owned(), draws);
        if let Some(&i) = self.index.get(&key) {
            return i;
        }
        self.list.push(key.clone());
        self.index.insert(key, self.list.len() - 1);
        self.list.len() - 1
    }
}

// Enclose the padded visibility sphere, including the camera's edge margin.
fn model_bounds(m: &Model) -> [f64; 6] {
    let [x, y, z] = [m.record[0], m.record[1], m.record[2]].map(f64::from);
    let z = z + m.size / 2.0;
    let r = m.radius + SHADOW_MARGIN_M;
    [x - r, y - r, z - r, x + r, y + r, z + r]
}

/// Map props bucketed by ground: each chunk's box holds its models' spheres
/// as the tier rule tests them, so a chunk out of view holds none in it.
struct Chunk {
    bounds: [f64; 6],
    models: Vec<usize>,
}

/// The side's props: the map's fitted once, what it knows fitted as it
/// learns, and what the last view drew of them by look and tier.
pub struct PropPopulation {
    art: PropArt,
    pub looks: Looks,
    /// Every map prop drawn as a body (not a tree), and its models' range in `map`.
    bodies: Vec<(MapProp, std::ops::Range<usize>)>,
    by_id: HashMap<u64, usize>,
    map: Vec<Model>,
    chunks: Vec<Chunk>,
    chunk_index: BoundsIndex,
    known_index: BoundsIndex,
    candidates: Vec<usize>,
    known_candidates: Vec<usize>,
    visited: usize,
    /// Per map model, whether a known prop replaced its prop.
    hidden: Vec<bool>,
    /// What the side knows, as given, and the known wrecks drawn moving.
    known: Vec<KnownBody>,
    moving: Vec<f64>,
    /// Where the side watches its known bodies shoved, by id: drawn there
    /// ahead of its knowledge, which re-learns a shoved body only in steps.
    shoved: BTreeMap<u64, Shoved>,
    known_models: Vec<Model>,
    known_bodies: usize,
    /// The rows the looks draw, set when the world loads.
    pub rows: Vec<u32>,
    lists: Vec<Vec<Record>>,
    prints: Vec<u64>,
}

impl PropPopulation {
    /// What the side knows from now on; true when that changed.
    pub fn set_known(&mut self, known: Vec<KnownBody>) -> bool {
        if known == self.known {
            return false;
        }
        // A body no longer shoved holds its pose until the side's knowledge
        // of it moves.
        let body = |known: &[KnownBody], id: u64| {
            known
                .iter()
                .find(|k| k.id.is_some_and(|k| k as u64 == id))
                .and_then(|k| k.body.clone())
        };
        let old = &self.known;
        self.shoved
            .retain(|&id, _| body(old, id).is_some_and(|b| Some(b) == body(&known, id)));
        self.known = known;
        self.refit();
        true
    }

    /// The known bodies the side watches shoved this frame, blended between
    /// ticks; true when what is drawn changed.
    pub fn set_shoved(&mut self, shoved: &[PropDisplayPose]) -> bool {
        let mut next = self.shoved.clone();
        for s in shoved {
            next.insert(
                s.prop_id as u64,
                Shoved {
                    center: s.center,
                    yaw: s.yaw,
                    base_z: s.base_z,
                },
            );
        }
        if next == self.shoved {
            return false;
        }
        self.shoved = next;
        self.refit();
        true
    }

    /// Forget every shove: another side's stream, or the same one again,
    /// starts from its own knowledge. True when that changed what is drawn.
    pub fn forget_shoved(&mut self) -> bool {
        if self.shoved.is_empty() {
            return false;
        }
        self.shoved.clear();
        self.refit();
        true
    }

    /// The known props whose wreck is drawn moving (a cook-off): left out
    /// until it lies still. True when that changed.
    pub fn set_moving(&mut self, moving: Vec<f64>) -> bool {
        if moving == self.moving {
            return false;
        }
        self.moving = moving;
        self.refit();
        true
    }

    /// The map's models the known replace hidden, the known that stand fitted.
    fn refit(&mut self) {
        self.hidden.fill(false);
        self.known_models.clear();
        self.known_bodies = 0;
        let mut looks = std::mem::take(&mut self.looks);
        for k in &self.known {
            if k.id.is_some_and(|id| self.moving.contains(&id)) {
                continue;
            }
            let replaced = k.authored.and_then(|a| self.by_id.get(&(a as u64)));
            if let Some(&b) = replaced {
                self.hidden[self.bodies[b].1.clone()].fill(true);
            }
            if let (false, Some(body)) = (k.destroyed, &k.body) {
                let shoved = k.id.and_then(|id| self.shoved.get(&(id as u64)));
                let shoved = shoved.map(|s| s.on(body));
                let body = shoved.as_ref().unwrap_or(body);
                let before = looks.list.len();
                self.art
                    .fit(body, k.authored, &mut looks, &mut self.known_models);
                self.known_bodies += 1;
                // A look no row was made for is not drawn.
                if looks.list.len() > before {
                    self.known_models.retain(|m| m.look < before);
                    looks.list.truncate(before);
                    looks.index.retain(|_, &mut i| i < before);
                }
            }
        }
        self.known_index = BoundsIndex::new(self.known_models.iter().map(model_bounds));
        self.looks = looks;
    }

    /// Models placed: the map's not hidden, then the known's.
    fn placed(&self) -> impl Iterator<Item = &Model> {
        let map = self.map.iter().zip(&self.hidden).filter(|(_, &h)| !h);
        map.map(|(m, _)| m).chain(&self.known_models)
    }

    /// The bodies drawn (the map's not replaced, then the known standing),
    /// those a building draws, those no model draws, every model placed,
    /// the models drawing what the side knows, and bounds visited by the last query.
    pub fn stats(&self) -> [usize; 6] {
        let replaced: HashSet<u64> = self
            .known
            .iter()
            .filter(|k| !k.id.is_some_and(|id| self.moving.contains(&id)))
            .filter_map(|k| k.authored.map(|a| a as u64))
            .collect();
        let standing = self
            .bodies
            .iter()
            .filter(|(p, _)| !replaced.contains(&(p.id as u64)));
        let (mut bodies, mut parts, mut unmodelled) = (0, 0, 0);
        for (p, models) in standing {
            bodies += 1;
            if self
                .art
                .binding(&p.body.kind)
                .is_some_and(|b| b.drawn_by == "building")
            {
                parts += 1;
            } else if models.is_empty() {
                unmodelled += 1;
            }
        }
        [
            bodies + self.known_bodies,
            parts,
            unmodelled,
            self.placed().count(),
            self.known_models.len(),
            self.visited,
        ]
    }

    /// Every model placed, as a probe reads it: its look and its record.
    pub fn models(&self) -> impl Iterator<Item = (&Look, &Record)> {
        self.placed().map(|m| (&self.looks.list[m.look], &m.record))
    }
}

impl Population for PropPopulation {
    fn select(&mut self, view: &View, shadow: Option<&CasterVolumes>) -> bool {
        self.candidates.clear();
        self.known_candidates.clear();
        let intersects = |b: &[f64; 6]| view.sees_box(b) || shadow.is_some_and(|s| s.intersects(b));
        self.visited = self
            .chunk_index
            .query(intersects, |i| self.candidates.push(i))
            + self
                .known_index
                .query(intersects, |i| self.known_candidates.push(i));
        // Spatial traversal order must not alter instance order or list prints.
        self.candidates.sort_unstable();
        self.known_candidates.sort_unstable();
        let mut lists = vec![Vec::new(); self.lists.len()];
        let this = &*self;
        let n = self.looks.list.len() * TIERS;
        let mut take = |m: &Model| {
            let at = [m.record[0], m.record[1], m.record[2]].map(f64::from);
            let tier = model_detail(&this.art.detail, view, at, m.size, m.radius, false, None);
            if tier != CULLED {
                lists[m.look * TIERS + tier as usize].push(m.record);
            }
            // Shadow membership is independent of camera visibility and detail.
            let [x, y, z] = at;
            let center_z = z + m.size / 2.0;
            let r = m.radius;
            let bounds = [x - r, y - r, center_z - r, x + r, y + r, center_z + r];
            // Offscreen models remain eligible: their shadow may still land in
            // the visible ground. Only a visible model with an imperceptible
            // conservative projected footprint may skip its caster pass.
            let visible = view.sees_sphere([x, y, center_z], r);
            let tiny_visible = visible
                && this.art.detail.shadow_caster_px > 0.0
                && projected_footprint_px(view, at, r) < this.art.detail.shadow_caster_px;
            let mask = shadow.map_or(0, |s| if tiny_visible { 0 } else { s.mask(&bounds) });
            if mask != 0 {
                let cast = if tier == CULLED {
                    TIERS - 1
                } else {
                    caster_tier(tier as usize)
                };
                for cascade in 0..CASCADES {
                    if mask & (1 << cascade) != 0 {
                        lists[(1 + cascade) * n + m.look * TIERS + cast].push(m.record);
                    }
                }
            }
        };
        for &c in &this.candidates {
            let chunk = &this.chunks[c];
            for &i in chunk.models.iter().filter(|&&i| !this.hidden[i]) {
                take(&this.map[i]);
            }
        }
        for &i in &this.known_candidates {
            take(&this.known_models[i]);
        }
        let mut changed = false;
        for (k, list) in lists.into_iter().enumerate() {
            if list != self.lists[k] {
                self.lists[k] = list;
                self.prints[k] += 1;
                changed = true;
            }
        }
        changed
    }

    fn each_list(&self, visit: &mut dyn FnMut(List)) {
        let n = self.looks.list.len() * TIERS;
        for (index, list) in self.lists.iter().enumerate() {
            let k = index % n;
            let tier = k % TIERS;
            let pass = if index < n { VIEW_PASS } else { CAST_PASS };
            visit(List {
                row: self.rows[k / TIERS],
                tier: tier as u32,
                pass,
                caster: List::own_caster(pass, tier as u32),
                cascade_mask: if index < n { 0 } else { 1 << (index / n - 1) },
                fog: model_fog(
                    false,
                    true,
                    self.looks.list[k / TIERS].2 == DRAWS_PROP_GROUND,
                ),
                records: list,
                print: self.prints[index],
            });
        }
    }
}

/// Conservative screen footprint of a model's horizontal bound. This is used
/// only for visible models; offscreen casters are retained for long shadows.
fn projected_footprint_px(view: &View, at: [f64; 3], radius: f64) -> f64 {
    // Camera-forward depth, rather than Euclidean eye distance, stays
    // conservative for a visible model near a frustum edge. Subtracting the
    // sphere radius uses the nearest point of the bound.
    let depth = (-(view.view[2] as f64 * at[0]
        + view.view[6] as f64 * at[1]
        + view.view[10] as f64 * at[2]
        + view.view[14] as f64)
        - radius)
        .max(1e-3);
    SHADOW_FOOTPRINT_MARGIN * 2.0 * radius * view.pixels_per_metre / depth
}

/// The appearances drawing the map's props (`records` under `layout`) from
/// `input`'s art could need (`PropArt::looks`), by name, sorted: what a
/// renderer installs for them.
pub fn appearances(records: &[f32], layout: &str, input: PropInput) -> Result<Vec<String>, String> {
    let layout = PropLayout::parse(layout)?;
    let props: Vec<PropBox> = map_props(records, &layout)
        .into_iter()
        .map(|p| p.body)
        .collect();
    let mut looks = Looks::default();
    PropArt::new(layout, input)?.looks(&props, &mut looks);
    let mut names: Vec<String> = looks.list.into_iter().map(|(name, ..)| name).collect();
    names.sort();
    names.dedup();
    Ok(names)
}

#[cfg(test)]
mod tests;
