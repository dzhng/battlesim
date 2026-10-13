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
//! rubble, and a fall the side has not seen leaves the wall standing.
//!
//! Each drawn model takes the units' tier rule (`view::model_detail`) by its
//! appearance's size grown by its largest scale, and is listed by look (its
//! appearance, state, and whether movers stand on it) and tier.
use crate::seed::Key;
use crate::static_world::batches::{List, Population, VIEW_PASS};
use crate::static_world::fallen::Known;
use crate::static_world::walk::{Record, Shadow};
use crate::view::{model_detail, View, CULLED, SHADOW_MARGIN_M};
use crate::world::{Bounds, Corners, Detail};
use serde::Deserialize;
use sim::publication::reader::KnownProp;
use sim::world::export::PROP_STRIDE;
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
const TIERS: usize = 4;

/// A prop kind's binding in the world layout (`propAppearance`).
#[derive(Clone, Debug, Deserialize)]
struct Binding {
    drawn_by: String,
    #[serde(default)]
    modular: bool,
    #[serde(default)]
    map_only: bool,
}

/// The world layout (`sim::world::export::layout_json`) as the props read
/// it: the prop kinds, what draws each, the kinds that stop a mover class,
/// and each unit type's hull model.
#[derive(Deserialize)]
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
/// state's bounds when it is installed with its module, the stand-ins' tints by kind (sRGB,
/// with a `default`; `presentation.stand_ins`), the map's regional family,
/// and the models' tier thresholds (`presentation.models`).
#[derive(Clone, Debug, Deserialize)]
pub struct PropInput {
    pub appearances: Vec<Art>,
    #[serde(default)]
    pub wrecks: BTreeMap<String, String>,
    #[serde(default)]
    pub stand_in: Option<Corners>,
    #[serde(default)]
    pub stand_in_tints: Option<BTreeMap<String, [f64; 3]>>,
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
    layout: PropLayout,
    candidates: Vec<Candidate>,
    by_kind: HashMap<String, Vec<usize>>,
    wrecks: HashMap<String, usize>,
    /// The kinds whose body stops no mover class: surfaces movers stand on.
    walked_on: HashSet<String>,
    stand_in: Option<(Bounds, HashMap<String, [f32; 3]>)>,
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
    /// The fit of `input`'s art under `layout`; a stand-in tint must be an
    /// sRGB colour, and the tints must give a `default`.
    pub fn new(layout: PropLayout, input: PropInput) -> Result<Self, String> {
        if let Some(tints) = &input.stand_in_tints {
            if !tints.contains_key("default") {
                return Err("presentation.stand_ins.tints needs a default".into());
            }
            if let Some((kind, _)) = tints
                .iter()
                .find(|(_, c)| c.iter().any(|v| !(0.0..=1.0).contains(v)))
            {
                return Err(format!(
                    "presentation.stand_ins.tints: {kind} must be [r, g, b] in [0, 1]"
                ));
            }
        }
        let PropInput {
            appearances,
            wrecks: unit_wrecks,
            stand_in,
            stand_in_tints,
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
        let stand_in = stand_in.zip(stand_in_tints).map(|(corners, tints)| {
            let tints = tints.into_iter().map(|(k, c)| (k, linear(c))).collect();
            (corners.bounds(), tints)
        });
        let mut art = PropArt {
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
            let misfit: f64 = (0..3).map(|a| (b.half[a] / f[a]).ln().abs()).sum();
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
            if let (true, Some((size, tints))) = (self.stands_in(&b.kind), &self.stand_in) {
                let [hx, hy, hz] = b.half;
                let tint = tints
                    .get(&b.kind)
                    .or_else(|| tints.get("default"))
                    .copied()
                    .unwrap_or([1.0; 3]);
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

/// A prop authored in the static map: its id and box.
#[derive(Clone, Debug)]
pub struct MapProp {
    pub id: f64,
    pub body: PropBox,
}

/// The map's props, from the world's exported records (`prop_record`).
pub fn map_props(records: &[f32], layout: &PropLayout) -> Vec<MapProp> {
    records
        .as_chunks::<PROP_STRIDE>()
        .0
        .iter()
        .map(|p| {
            let f = |i: usize| f64::from(p[i]);
            let wreck = p[10];
            MapProp {
                id: f(0) + f(1) * 65536.0,
                body: PropBox {
                    kind: layout.prop_kinds[p[2] as usize].clone(),
                    center: [f(3), f(4)],
                    yaw: f(5),
                    half: [f(6), f(7), f(8)],
                    base_z: f(9),
                    wreck_of: (wreck >= 0.0).then(|| layout.unit_kinds[wreck as usize].clone()),
                },
            }
        })
        .collect()
}

/// A prop a side knows: the map prop whose place it took (none for a prop
/// that is no map prop's, a wreck), whether it was destroyed with nothing
/// left, its id (the cook-off names a wreck by it), and its box where it
/// stands (none: a lab's stand-in that names only what the buildings read).
#[derive(Clone, Debug, PartialEq)]
pub struct KnownBody {
    pub authored: Option<f64>,
    pub id: Option<f64>,
    pub kind: String,
    pub destroyed: bool,
    pub body: Option<PropBox>,
}

impl KnownBody {
    /// The publication's known prop.
    pub fn published(k: &KnownProp) -> Self {
        KnownBody {
            authored: k.authored_prop,
            id: k.id,
            kind: k.kind.to_string(),
            destroyed: k.destroyed,
            body: Some(PropBox {
                kind: k.kind.to_string(),
                center: k.center,
                yaw: k.yaw,
                half: k.half,
                base_z: k.base_z,
                wreck_of: k.wreck_of.as_deref().map(str::to_owned),
            }),
        }
    }

    /// A lab's stand-in for one.
    pub fn lab(k: &Known) -> Self {
        KnownBody {
            authored: k.authored_prop,
            id: k.id,
            kind: k.kind.clone(),
            destroyed: k.destroyed,
            body: k.center.map(|center| PropBox {
                kind: k.kind.clone(),
                center,
                yaw: k.yaw,
                half: k.half,
                base_z: k.base_z,
                wreck_of: k.wreck_of.clone(),
            }),
        }
    }
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
    /// Per map model, whether a known prop replaced its prop.
    hidden: Vec<bool>,
    /// What the side knows, as given, and the known wrecks drawn moving.
    known: Vec<KnownBody>,
    moving: Vec<f64>,
    known_models: Vec<Model>,
    known_bodies: usize,
    /// The rows the looks draw, set when the world loads.
    pub rows: Vec<u32>,
    lists: Vec<Vec<Record>>,
    prints: Vec<u64>,
}

impl PropPopulation {
    /// The map's `props` drawn from `art`.
    pub fn new(art: PropArt, props: Vec<MapProp>) -> Self {
        let mut looks = Looks::default();
        art.looks(
            &props.iter().map(|p| p.body.clone()).collect::<Vec<_>>(),
            &mut looks,
        );
        let mut map = Vec::new();
        let mut bodies = Vec::new();
        for p in props {
            if art.is_tree(&p.body.kind) {
                continue;
            }
            let first = map.len();
            art.fit(&p.body, Some(p.id), &mut looks, &mut map);
            bodies.push((p, first..map.len()));
        }
        let by_id = bodies
            .iter()
            .enumerate()
            .map(|(i, (p, _))| (p.id as u64, i))
            .collect();
        let mut cells: BTreeMap<(i64, i64), Chunk> = BTreeMap::new();
        for (i, m) in map.iter().enumerate() {
            let [x, y] = [m.record[0], m.record[1]].map(f64::from);
            let cell = ((y / CHUNK_M).floor() as i64, (x / CHUNK_M).floor() as i64);
            let r = m.radius + SHADOW_MARGIN_M;
            let z = f64::from(m.record[2]) + m.size / 2.0;
            let b = [x - r, y - r, z - r, x + r, y + r, z + r];
            let chunk = cells.entry(cell).or_insert(Chunk {
                bounds: b,
                models: Vec::new(),
            });
            for k in 0..3 {
                chunk.bounds[k] = chunk.bounds[k].min(b[k]);
                chunk.bounds[k + 3] = chunk.bounds[k + 3].max(b[k + 3]);
            }
            chunk.models.push(i);
        }
        let lists = vec![Vec::new(); looks.list.len() * TIERS];
        PropPopulation {
            hidden: vec![false; map.len()],
            prints: vec![0; lists.len()],
            lists,
            chunks: cells.into_values().collect(),
            map,
            by_id,
            bodies,
            looks,
            art,
            known: Vec::new(),
            moving: Vec::new(),
            known_models: Vec::new(),
            known_bodies: 0,
            rows: Vec::new(),
        }
    }

    /// What the side knows from now on; true when that changed.
    pub fn set_known(&mut self, known: Vec<KnownBody>) -> bool {
        if known == self.known {
            return false;
        }
        self.known = known;
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
        self.looks = looks;
    }

    /// Models placed: the map's not hidden, then the known's.
    fn placed(&self) -> impl Iterator<Item = &Model> {
        let map = self.map.iter().zip(&self.hidden).filter(|(_, &h)| !h);
        map.map(|(m, _)| m).chain(&self.known_models)
    }

    /// The bodies drawn (the map's not replaced, then the known standing),
    /// those a building draws, those no model draws, every model placed,
    /// and the models drawing what the side knows.
    pub fn stats(&self) -> [usize; 5] {
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
        ]
    }

    /// Every model placed, as a probe reads it: its look and its record.
    pub fn models(&self) -> impl Iterator<Item = (&Look, &Record)> {
        self.placed().map(|m| (&self.looks.list[m.look], &m.record))
    }
}

impl Population for PropPopulation {
    fn select(&mut self, view: &View, _shadow: Option<&Shadow>) -> bool {
        let mut lists = vec![Vec::new(); self.lists.len()];
        let this = &*self;
        let mut take = |m: &Model| {
            let at = [m.record[0], m.record[1], m.record[2]].map(f64::from);
            let tier = model_detail(&this.art.detail, view, at, m.size, m.radius, false, None);
            if tier != CULLED {
                lists[m.look * TIERS + tier as usize].push(m.record);
            }
        };
        for chunk in this.chunks.iter().filter(|c| view.sees_box(&c.bounds)) {
            for &i in chunk.models.iter().filter(|&&i| !this.hidden[i]) {
                take(&this.map[i]);
            }
        }
        for m in &this.known_models {
            take(m);
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
        for (k, list) in self.lists.iter().enumerate() {
            visit(List {
                row: self.rows[k / TIERS],
                tier: (k % TIERS) as u32,
                pass: VIEW_PASS,
                records: list,
                print: self.prints[k],
            });
        }
    }
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
mod tests {
    use super::*;
    use crate::view::Camera;

    /// Walls (modular), cars (painted), wrecks, a lamp with no art, a
    /// forest's trunk and a building's part; a car and a wall stop movers, a
    /// lamp and a deck don't.
    fn layout() -> PropLayout {
        PropLayout::parse(
            &serde_json::json!({
                "propKinds": ["wall", "car", "heavy_wreck", "lamp", "trunk", "part", "deck"],
                "propAppearance": {
                    "wall": { "drawn_by": "wall", "modular": true },
                    "car": { "drawn_by": "car" },
                    "heavy_wreck": { "drawn_by": "wreck" },
                    "lamp": { "drawn_by": "lamp" },
                    "trunk": { "drawn_by": "forest" },
                    "part": { "drawn_by": "building", "map_only": true },
                    "deck": { "drawn_by": "deck", "map_only": true },
                },
                "blockingPropKinds": { "wheeled": ["wall", "car", "heavy_wreck", "part"] },
                "unitKinds": ["tank", "jeep"],
                "unitAppearance": { "tank": "tank_model" },
            })
            .to_string(),
        )
        .unwrap()
    }

    fn art(name: &str, scenery: &str, family: Option<&str>, footprint: [f64; 3]) -> Art {
        Art {
            name: name.into(),
            scenery: Some(scenery.into()),
            family: family.map(Into::into),
            footprint,
            paints: None,
            // Drawn 2 m long, reaching 1 m from its foot.
            bounds: Corners {
                min: [-1.0, 0.0, 0.0],
                max: [1.0, 0.0, 0.0],
            },
        }
    }

    fn input() -> PropInput {
        let mut car = art("car_a", "car", None, [2.0, 1.0, 0.8]);
        car.paints = Some(vec![[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]);
        PropInput {
            appearances: vec![
                art("wall_long", "wall", None, [2.0, 0.2, 1.0]),
                art("wall_short", "wall", None, [0.5, 0.2, 1.0]),
                car,
                art("car_b", "car", None, [4.0, 2.0, 1.5]),
                art("car_paris", "car", Some("paris"), [2.0, 1.0, 0.8]),
                art("car_china", "car", Some("china"), [2.0, 1.0, 0.8]),
                art("tank_wreck", "wreck", None, [3.5, 1.8, 1.2]),
                art("deck_a", "deck", None, [5.0, 2.0, 0.5]),
            ],
            wrecks: [("tank_model".to_owned(), "tank_wreck".to_owned())].into(),
            stand_in: Some(Corners {
                min: [-0.5, -0.5, 0.0],
                max: [0.5, 0.5, 1.0],
            }),
            stand_in_tints: Some(
                [
                    ("default".to_owned(), [0.5, 0.5, 0.5]),
                    ("lamp".to_owned(), [1.0, 1.0, 0.0]),
                ]
                .into(),
            ),
            family: None,
            detail: Detail {
                lod_px: [200.0, 80.0, 30.0],
                impostor_px: 10.0,
            },
            dynamic: true,
        }
    }

    fn body(kind: &str, half: [f64; 3]) -> PropBox {
        PropBox {
            kind: kind.into(),
            center: [10.0, 20.0],
            yaw: 0.3,
            half,
            base_z: 1.5,
            wreck_of: None,
        }
    }

    /// The models `b` is drawn as, by look.
    fn drawn(art: &PropArt, b: &PropBox, authored: Option<f64>) -> Vec<(Look, Record)> {
        let (mut looks, mut out) = (Looks::default(), Vec::new());
        art.fit(b, authored, &mut looks, &mut out);
        out.iter()
            .map(|m| (looks.list[m.look].clone(), m.record))
            .collect()
    }

    fn names(models: &[(Look, Record)]) -> Vec<&str> {
        models.iter().map(|(l, _)| l.0.as_str()).collect()
    }

    #[test]
    fn a_box_takes_the_footprint_nearest_it_scaled_per_axis_onto_it() {
        let art = PropArt::new(layout(), input()).unwrap();
        let big = drawn(&art, &body("car", [4.2, 2.1, 1.4]), Some(1.0));
        assert_eq!(names(&big), ["car_b"]);
        let r = big[0].1;
        assert_eq!(&r[..4], &[10.0, 20.0, 1.5, 0.3]);
        assert_eq!(&r[12..15], &[1.05, 1.05, (1.4f64 / 1.5) as f32]);
        // Unpainted art wears no tint; a mesh draws no card.
        assert_eq!(&r[8..12], &[1.0, 1.0, 1.0, -1.0]);
        assert_eq!(
            names(&drawn(&art, &body("car", [2.0, 1.0, 0.8]), None)),
            ["car_a"]
        );
    }

    #[test]
    fn a_maps_region_draws_its_own_looks_and_never_anothers() {
        let paris = PropArt::new(
            layout(),
            PropInput {
                family: Some("paris".into()),
                ..input()
            },
        )
        .unwrap();
        assert_eq!(
            names(&drawn(&paris, &body("car", [4.0, 2.0, 1.5]), None)),
            ["car_paris"]
        );
        let none = PropArt::new(layout(), input()).unwrap();
        let mut looks = Looks::default();
        none.looks(&[], &mut looks);
        assert!(looks
            .list
            .iter()
            .all(|(n, ..)| n != "car_paris" && n != "car_china"));
    }

    #[test]
    fn a_wreck_is_its_own_units_whatever_box_it_lies_on() {
        let art = PropArt::new(layout(), input()).unwrap();
        let mut wreck = body("heavy_wreck", [9.0, 9.0, 9.0]);
        wreck.wreck_of = Some("tank".into());
        assert_eq!(names(&drawn(&art, &wreck, None)), ["tank_wreck"]);
        // A unit type with no wreck of its own is the stand-in box.
        wreck.wreck_of = Some("jeep".into());
        assert_eq!(names(&drawn(&art, &wreck, None)), [STAND_IN_KIT]);
    }

    #[test]
    fn a_module_repeats_along_the_boxs_long_side_unstretched() {
        let art = PropArt::new(layout(), input()).unwrap();
        // 10 m long along its x: the nearest whole number of 2 m modules (2.5
        // rounds up), each fitted to a third of it.
        let wall = drawn(&art, &body("wall", [5.0, 0.2, 1.0]), None);
        assert_eq!(names(&wall), ["wall_long"; 3]);
        // 4.5 m along its y: five short modules turned to it, end to end.
        let wall = drawn(&art, &body("wall", [0.2, 2.25, 1.0]), None);
        assert_eq!(names(&wall), ["wall_short"; 5]);
        let yaw = 0.3 + FRAC_PI_2;
        for (k, (_, r)) in wall.iter().enumerate() {
            let along = -2.25 + 0.9 * (k as f64 + 0.5);
            assert_eq!(r[0], (10.0 + along * libm::cos(yaw)) as f32);
            assert_eq!(r[3], yaw as f32);
            assert_eq!(&r[12..15], &[0.9, 1.0, 1.0]);
        }
    }

    #[test]
    fn a_car_keeps_its_paint_wherever_it_is_shoved() {
        let art = PropArt::new(layout(), input()).unwrap();
        let car = body("car", [2.0, 1.0, 0.8]);
        let mut shoved = car.clone();
        shoved.center = [40.0, -3.0];
        let tint = |b: &PropBox, a| drawn(&art, b, a)[0].1[8..11].to_vec();
        assert_eq!(tint(&car, Some(17.0)), tint(&shoved, Some(17.0)));
        let paints: HashSet<_> = (0..40)
            .map(|id| {
                tint(&car, Some(id as f64))
                    .iter()
                    .map(|c| c.to_bits())
                    .collect::<Vec<_>>()
            })
            .collect();
        assert_eq!(paints.len(), 3, "every paint is worn");
    }

    #[test]
    fn a_kind_without_art_is_the_stand_in_box_and_a_tree_or_part_never_is() {
        let art = PropArt::new(layout(), input()).unwrap();
        let lamp = drawn(&art, &body("lamp", [0.2, 0.3, 3.0]), None);
        assert_eq!(
            lamp[0].0,
            (STAND_IN_KIT.into(), STAND_IN_MODULE.into(), DRAWS_PROP)
        );
        assert_eq!(&lamp[0].1[12..15], &[0.4, 0.6, 6.0]);
        assert_eq!(&lamp[0].1[8..11], &linear([1.0, 1.0, 0.0]));
        assert!(drawn(&art, &body("trunk", [0.3, 0.3, 8.0]), None).is_empty());
        assert!(drawn(&art, &body("part", [5.0, 5.0, 5.0]), None).is_empty());
        let bare = PropArt::new(
            layout(),
            PropInput {
                stand_in: None,
                ..input()
            },
        )
        .unwrap();
        assert!(drawn(&bare, &body("lamp", [0.2, 0.3, 3.0]), None).is_empty());
    }

    #[test]
    fn a_surface_movers_stand_on_takes_the_ground_paint() {
        let art = PropArt::new(layout(), input()).unwrap();
        let deck = drawn(&art, &body("deck", [5.0, 2.0, 0.5]), None);
        assert_eq!(deck[0].0 .2, DRAWS_PROP_GROUND);
        assert_eq!(
            drawn(&art, &body("car", [2.0, 1.0, 0.8]), None)[0].0 .2,
            DRAWS_PROP
        );
    }

    /// The map: a car, a wall, a deck and a forest trunk.
    fn town() -> PropPopulation {
        let layout = layout();
        let record = |id: f32, kind: f32, x: f32, half: [f32; 3]| {
            [
                id, 0.0, kind, x, 0.0, 0.0, half[0], half[1], half[2], 0.0, -1.0,
            ]
        };
        let records = [
            record(1.0, 1.0, 0.0, [2.0, 1.0, 0.8]),
            record(2.0, 0.0, 20.0, [2.0, 0.2, 1.0]),
            record(3.0, 6.0, 40.0, [5.0, 2.0, 0.5]),
            record(4.0, 4.0, 60.0, [0.3, 0.3, 8.0]),
        ]
        .concat();
        let props = map_props(&records, &layout);
        let mut p = PropPopulation::new(PropArt::new(layout, input()).unwrap(), props);
        p.rows = (0..p.looks.list.len() as u32).collect();
        p
    }

    fn appearances(p: &PropPopulation) -> Vec<&str> {
        let mut names: Vec<_> = p.models().map(|(l, _)| l.0.as_str()).collect();
        names.sort();
        names
    }

    fn known(authored: Option<f64>, kind: &str, x: f64, destroyed: bool) -> KnownBody {
        KnownBody {
            authored,
            id: Some(100.0 + x),
            kind: kind.into(),
            destroyed,
            body: Some(PropBox {
                center: [x, 0.0],
                ..body(kind, [2.0, 1.0, 0.8])
            }),
        }
    }

    #[test]
    fn a_side_draws_the_map_less_what_it_knows_replaced_plus_what_it_knows() {
        let mut p = town();
        assert_eq!(appearances(&p), ["car_a", "deck_a", "wall_long"]);
        // The car shoved, and the wall seen destroyed with nothing left.
        assert!(p.set_known(vec![
            known(Some(1.0), "car", 5.0, false),
            known(Some(2.0), "wall", 20.0, true),
        ]));
        assert_eq!(appearances(&p), ["car_a", "deck_a"]);
        let car = p.models().find(|(l, _)| l.0 == "car_a").unwrap().1;
        assert_eq!(car[0], 5.0);
        assert_eq!(p.stats(), [2, 0, 0, 2, 1]);
        // Knowing it again changes nothing; forgetting stands the map again.
        assert!(!p.set_known(p.known.clone()));
        assert!(p.set_known(vec![]));
        assert_eq!(appearances(&p).len(), 3);
    }

    #[test]
    fn a_renderer_installs_the_maps_looks_and_every_body_a_battle_can_leave() {
        let art = PropArt::new(layout(), input()).unwrap();
        let installs = |props: &[PropBox]| {
            let mut looks = Looks::default();
            art.looks(props, &mut looks);
            let mut names: Vec<_> = looks.list.into_iter().map(|l| l.0).collect();
            names.sort();
            names.dedup();
            names
        };
        // A deck is only ever a map's: installed for a map that has one.
        let none = installs(&[]);
        assert_eq!(
            none,
            [
                "car_a",
                "car_b",
                STAND_IN_KIT,
                "tank_wreck",
                "wall_long",
                "wall_short"
            ]
        );
        assert!(installs(&[body("deck", [5.0, 2.0, 0.5])]).contains(&"deck_a".to_owned()));
    }

    #[test]
    fn a_second_replacement_never_stands_the_map_prop_again() {
        let mut p = town();
        // The wall fell to rubble, and that rubble was shoved: both name the
        // wall as the prop they came from.
        p.set_known(vec![
            known(Some(2.0), "wall", 20.0, true),
            known(Some(2.0), "car", 30.0, false),
        ]);
        assert_eq!(appearances(&p), ["car_a", "car_a", "deck_a"]);
    }

    #[test]
    fn a_wreck_drawn_moving_is_left_out_until_it_lies_still() {
        let mut p = town();
        let mut wreck = known(None, "heavy_wreck", 80.0, false);
        wreck.body.as_mut().unwrap().wreck_of = Some("tank".into());
        p.set_known(vec![wreck]);
        assert!(appearances(&p).contains(&"tank_wreck"));
        assert!(p.set_moving(vec![180.0]));
        assert!(!appearances(&p).contains(&"tank_wreck"));
        assert!(p.set_moving(vec![]));
        assert!(appearances(&p).contains(&"tank_wreck"));
    }

    #[test]
    fn a_view_draws_what_it_sees_by_its_projected_height() {
        let mut p = town();
        let camera = |eye: [f64; 3], target: [f64; 3]| {
            View::new(&Camera {
                eye,
                target,
                up: [0.0, 0.0, 1.0],
                fov_y: 0.8,
                near: 1.0,
                size: [1280, 720],
            })
        };
        let drawn = |p: &PropPopulation| {
            let mut out = Vec::new();
            p.each_list(&mut |l| {
                if !l.records.is_empty() {
                    out.push((
                        p.looks.list[l.row as usize].0.clone(),
                        l.tier,
                        l.records.len(),
                    ));
                }
            });
            out
        };
        // Close on the car, looking along +x: it is near, the far deck small.
        assert!(p.select(&camera([-8.0, 0.0, 2.0], [0.0, 0.0, 0.0]), None));
        let near = drawn(&p);
        assert!(near.contains(&("car_a".into(), 0, 1)), "{near:?}");
        assert!(
            near.iter().any(|(n, t, _)| n == "deck_a" && *t > 0),
            "{near:?}"
        );
        // Looking away, nothing is drawn; the same view again changes nothing.
        assert!(p.select(&camera([-8.0, 0.0, 2.0], [-20.0, 0.0, 0.0]), None));
        assert!(drawn(&p).is_empty());
        assert!(!p.select(&camera([-8.0, 0.0, 2.0], [-20.0, 0.0, 0.0]), None));
    }
}
