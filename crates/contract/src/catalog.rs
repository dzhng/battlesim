//! The catalog: every unit type and every prop type as data, addressed by
//! string id in JSON and by a dense index at runtime ([`TypeIndex`],
//! [`PropKind`]). Nothing lists the types in code; behaviour comes from a
//! type's components (a unit's body, mobility, sensors, mounts and
//! capabilities; a prop's body row), never from its id.
//!
//! Authored, the catalog is a list of documents (the files under
//! `fixtures/units/` and `fixtures/props/`), each holding any of five
//! sections: `roles` (the role tags scripts and the AI select by), `parts`
//! (reusable upgrades), `soldiers` (soldier kinds a squad's slots name),
//! `units` (the unit types) and `props` (the prop types). An entry of
//! `parts`, `soldiers`, `units` or `props` may `extends` another of its
//! section and give only what differs, and an `abstract` one exists only to
//! be extended. [`resolve`] flattens everything once, at load: objects
//! deep-merge, lists of named objects (mounts) merge by name, a unit's
//! variant component written as another variant replaces its parent's, a
//! unit's `parts` gather along the chain, any other value is replaced
//! ([`merge_entry`]); then a type's `parts` apply in order, merged the same
//! way. Merging is idempotent, so a resolved catalog serialises to a
//! document that resolves to itself. Then every entry is checked, and a
//! broken one fails naming it ([`CatalogError`]).
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::observation::SoundCategory;
use crate::scenario::{Armor, Blocks, CoverTier, PushClass, SightShape, WeightClass};
use crate::weapons::MountDefinition;

/// A unit type's place in the catalog: its rank among the ids in sorted order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TypeIndex(pub u16);

/// One unit type, fully specified.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitType {
    /// What the player reads: the unit card, callouts and selection.
    pub name: String,
    pub description: String,
    pub faction: String,
    /// UI grouping and balance (a file under the faction).
    pub family: String,
    /// Tags from the role registry: what scripts and the AI select by.
    pub roles: Vec<String>,
    /// Target priority and, later, army points.
    pub cost: u32,
    pub body: Body,
    pub mobility: Mobility,
    pub sensors: Sensors,
    /// A hull's mounts, in order. A squad's come from its soldiers.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mounts: Vec<MountDefinition>,
    #[serde(default)]
    pub capabilities: Capabilities,
    pub sound: Sound,
    /// A hull's model (an asset catalog appearance). A squad draws its
    /// soldiers' own appearance sets.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub appearance: Option<String>,
    /// Upgrade parts already applied, kept so the model can be checked for
    /// each part's hardware.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parts: Vec<String>,
}

/// What a unit is made of: soldiers, or one hull.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Body {
    /// Soldier kinds, one per slot; slot `k` is the squad's `k`th soldier.
    Squad {
        slots: Vec<String>,
    },
    Hull(Hull),
}

/// A vehicle's body (Q3, Q14, Q19): a box like any prop, whose weight class
/// is also its cover tier's source (Q24), live or wrecked.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hull {
    /// Half length (along heading), half width, half height.
    pub half_extents_m: [f64; 3],
    /// Sight height above the hull's base.
    pub eye_m: f64,
    pub hp: f64,
    pub armor: Armor,
    pub weight_class: WeightClass,
    pub push_class: PushClass,
    /// The prop type it leaves when destroyed (a `props` id).
    pub wreck: String,
}

/// No mover's top speed may exceed this (light wheeled vehicles do 110).
pub const MAX_SPEED_KMH: f64 = 130.0;

/// How a unit moves (Q29, Q30). Open to new variants (rotor, air). Every
/// mover has its own two top speeds, in km/h: `offroad_kmh` on open ground
/// and `road_kmh` on a full road. A surface kind's `speed_factor` scales the
/// road speed, never below the off-road one (`Rules::surfaces`).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Mobility {
    Foot {
        offroad_kmh: f64,
        road_kmh: f64,
    },
    /// Tracks pivot on the spot.
    Tracked {
        offroad_kmh: f64,
        road_kmh: f64,
        turn_deg_s: f64,
        /// Reverse speed as a fraction of forward.
        reverse_fraction: f64,
    },
    /// Wheels hold a minimum turning radius and never pivot.
    Wheeled {
        offroad_kmh: f64,
        road_kmh: f64,
        turn_deg_s: f64,
        turning_radius_m: f64,
        reverse_fraction: f64,
    },
}

impl Mobility {
    /// `(offroad_kmh, road_kmh)`.
    pub fn speeds_kmh(&self) -> (f64, f64) {
        match *self {
            Mobility::Foot {
                offroad_kmh,
                road_kmh,
            }
            | Mobility::Tracked {
                offroad_kmh,
                road_kmh,
                ..
            }
            | Mobility::Wheeled {
                offroad_kmh,
                road_kmh,
                ..
            } => (offroad_kmh, road_kmh),
        }
    }

    /// `(off-road, road)` top speeds in metres per second.
    pub fn speeds_mps(&self) -> (f64, f64) {
        let (offroad, road) = self.speeds_kmh();
        (offroad / 3.6, road / 3.6)
    }
}

/// A unit's one sight.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sensors {
    /// Ground range in the open, before shape.
    pub ground_m: f64,
    /// Reach by direction as multipliers of `ground_m`; 1/1/1 sees evenly.
    pub sight_shape: SightShape,
    /// The turret mount the optics turn with; absent, they look along the hull.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on: Option<String>,
}

/// Optional abilities, present only on the types that have them.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capabilities {
    /// Sets up in place before it can serve (L01), and packs up to move.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deploy: Option<Deploy>,
    /// Serves nearby units from a finite stock (L05); needs `deploy`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supply: Option<Supply>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Deploy {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pack_seconds: Option<f64>,
    /// Full setup duration. Packing uses this unless `pack_seconds` is authored.
    pub seconds: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Supply {
    /// Its stock at the start, unless the scenario says otherwise.
    pub stock: u32,
}

/// What a unit sounds like to the enemy, and how far it carries (hearing).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sound {
    pub profile: SoundCategory,
    pub loudness_m: f64,
}

/// A soldier kind: what one squad slot is.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoldierKind {
    pub name: String,
    pub description: String,
    pub hp: f64,
    /// The appearances a soldier of this kind may wear, one picked per soldier.
    pub appearance: Vec<String>,
    /// The weapons he carries. A `special` one passes to the next living
    /// soldier when he falls; any other is lost with him.
    pub mounts: Vec<MountDefinition>,
}

/// A role tag from the registry.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Role {
    pub name: String,
    pub description: String,
    /// The NATO-style symbol's modifiers, drawn in order inside its frame.
    pub symbol: Vec<String>,
}

/// A reusable upgrade: overrides and additions merged into a type after
/// inheritance. A part that sets a capability the simulation doesn't build
/// is refused, because the type then fails to parse.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Part {
    pub name: String,
    pub description: String,
    /// Model node names (a trailing `*` matches any suffix) a type listing
    /// this part must draw: the part's hardware.
    #[serde(default)]
    pub nodes: Vec<String>,
    /// Merged into the type, like an `extends` child.
    pub patch: Value,
}

/// A prop type's place in the catalog: its rank among the prop ids.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PropKind(pub u16);

/// One prop type (Q19, Q28): what a body of it blocks, stops, hides and
/// weighs, what it becomes when destroyed, and what draws it. The rules read
/// its body's columns, never its id; a new obstacle is a new entry.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PropType {
    pub body: PropBody,
    /// What a destroyed body becomes; with `body.hp`, and only with it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub destroyed: Option<Destroyed>,
    pub appearance: PropAppearance,
}

/// A prop type's body row. Each column is independent and has its own
/// readers: `blocks` navigation and collision; `stops_rounds` flight;
/// `occludes` the fog sweep, sensing and the renderer's sight-light
/// occluders; `weight_class` pushing; `cover_tier` cover. A row that blocks
/// nobody, stops no rounds and gives no cover but occludes for a
/// `lifetime_s` is a smoke screen.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PropBody {
    pub blocks: Blocks,
    pub stops_rounds: bool,
    /// Hides what lies behind it from sight (Q25: only big static bodies).
    pub occludes: bool,
    pub weight_class: WeightClass,
    /// The cover it gives infantry (Q4, Q24): from behind a body that blocks
    /// infantry, or from inside a ground body (rubble) that does not.
    #[serde(default)]
    pub cover_tier: Option<CoverTier>,
    /// A transient body: it goes this long after it appears.
    #[serde(default)]
    pub lifetime_s: Option<f64>,
    /// How much of a fog cell one such body conceals, in [0, 1) (Q21): a
    /// cell's foliage is `1 − Π(1 − conceals)` over the concealing bodies
    /// whose canopy covers it.
    #[serde(default)]
    pub conceals: f64,
    /// Integrity (Q17, 34c): the structural damage it takes to destroy one
    /// such body; with `building_floor_bands` this is the coefficient per
    /// square metre and floor band. None: ordinary fire never destroys it.
    #[serde(default)]
    pub hp: Option<f64>,
    #[serde(default, skip_serializing_if = "HpScale::is_fixed")]
    pub hp_scale: HpScale,
    /// The share of a direct round's structural damage it takes (blast is
    /// not scaled); 1 when absent.
    #[serde(default = "one")]
    pub armor: f64,
    /// It falls rather than slides (Q16): a vehicle that can shove it knocks
    /// it down, and a vehicle that can knock it down clears a lane of the
    /// forest ground it stood on. A tree.
    #[serde(default)]
    pub topples: bool,
    /// A squad can garrison it: a fighting position its soldiers hold from
    /// inside (Q22, the named garrison mechanic).
    #[serde(default)]
    pub garrison: bool,
}

/// A fixed integrity value, or a coefficient per square metre and usable floor band.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HpScale {
    #[default]
    Fixed,
    BuildingFloorBands,
}
impl HpScale {
    fn is_fixed(&self) -> bool {
        *self == Self::Fixed
    }
}

fn one() -> f64 {
    1.0
}

/// A destroyed body's state (Q17): gone, gone and its ground cleared (a
/// tree: open ground, like a lane a tank knocks through), or another prop
/// type on the same plan at `height_m` (a building's ruin, sandbags'
/// rubble, a lighter wreck).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Destroyed {
    Removed,
    Cleared,
    Into {
        prop: String,
        height_m: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        building: Option<BuildingRemains>,
    },
}

/// A building's data-driven collapse height and tall terminal shell.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildingRemains {
    pub height_fraction: f64,
    pub max_height_m: f64,
    pub collapse_max_floors: usize,
    pub gutted_prop: String,
}

/// What draws a prop type (presentation reads it; the rules never do).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PropAppearance {
    /// Physical catalog entry whose fitted art is still awaiting acceptance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<PropAppearanceStatus>,
    /// The asset catalog's appearances that draw it: the scenery kind (or
    /// `building`) whose appearances are fitted to its box, `forest` for the
    /// trees a forest draws itself.
    pub drawn_by: String,
    /// Drawn by repeating one module along the box's long side (a wall, a
    /// fence), not stretched.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub modular: bool,
    /// Only a map places one: a battle never leaves or drops one, so only the
    /// appearances a map uses are loaded.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub map_only: bool,
    /// Remains drawn as the body they stand in place of, in this state of its
    /// appearance (a building's ruin).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remains_state: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PropAppearanceStatus {
    SystemsOnly,
}

/// The physical owner that will place a prop and its later destruction states.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PropPlacement {
    Ordinary,
    Aggregate,
}

/// The prop types, in id order: [`PropKind`] is a rank.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PropCatalog {
    ids: Vec<String>,
    types: Vec<PropType>,
}

impl PropCatalog {
    /// Validate every state against its placement owner. Catalog resolution
    /// already proves the chain acyclic and each gutted alternative terminal.
    pub fn check_placement(&self, kind: PropKind, placement: PropPlacement) -> Result<(), String> {
        let check = |kind| {
            let body = self.get(kind).body;
            match placement {
                PropPlacement::Ordinary
                    if body.garrison || body.hp_scale == HpScale::BuildingFloorBands =>
                {
                    Err(format!("prop {:?} requires a placed aggregate for garrison or building-scaled integrity", self.id(kind)))
                }
                PropPlacement::Aggregate
                    if body.weight_class != crate::scenario::WeightClass::Immovable =>
                {
                    Err(format!("aggregate state {:?} requires an immovable body until composite motion exists", self.id(kind)))
                }
                _ => Ok(()),
            }
        };
        let mut kind = kind;
        loop {
            check(kind)?;
            let Some(Destroyed::Into { prop, building, .. }) = &self.get(kind).destroyed else {
                return Ok(());
            };
            if let Some(policy) = building {
                check(self.kind(&policy.gutted_prop))?;
            }
            kind = self.kind(prop);
        }
    }

    /// Every prop type's index, in id order.
    pub fn kinds(&self) -> impl Iterator<Item = PropKind> {
        (0..self.types.len() as u16).map(PropKind)
    }

    pub fn index(&self, id: &str) -> Option<PropKind> {
        self.ids
            .binary_search_by(|k| k.as_str().cmp(id))
            .ok()
            .map(|i| PropKind(i as u16))
    }

    /// The prop type named `id`, which a caller holds from this catalog.
    pub fn kind(&self, id: &str) -> PropKind {
        self.index(id)
            .unwrap_or_else(|| panic!("no prop type {id:?}"))
    }

    pub fn id(&self, k: PropKind) -> &str {
        &self.ids[k.0 as usize]
    }

    /// The ids in index order: the publications' prop type table.
    pub fn ids(&self) -> &[String] {
        &self.ids
    }

    pub fn get(&self, k: PropKind) -> &PropType {
        &self.types[k.0 as usize]
    }

    /// The prop type named `id`, which a caller holds from this catalog.
    pub fn by_id(&self, id: &str) -> &PropType {
        self.get(self.kind(id))
    }
}

/// A mount as a unit carries it: its row, and for a squad the slots whose
/// soldiers carry it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CarriedMount {
    #[serde(flatten)]
    pub def: MountDefinition,
    /// Slot indices; empty on a hull.
    pub carriers: Vec<usize>,
}

/// The resolved catalog. Types are held in id order: [`TypeIndex`] is a rank.
#[derive(Clone, Debug, PartialEq)]
pub struct Catalog {
    roles: BTreeMap<String, Role>,
    parts: BTreeMap<String, Part>,
    soldiers: BTreeMap<String, SoldierKind>,
    ids: Vec<String>,
    types: Vec<UnitType>,
    mounts: Vec<Vec<CarriedMount>>,
    props: PropCatalog,
}

impl Catalog {
    /// Every type's index, in id order.
    pub fn indices(&self) -> impl Iterator<Item = TypeIndex> {
        (0..self.types.len() as u16).map(TypeIndex)
    }

    pub fn index(&self, id: &str) -> Option<TypeIndex> {
        self.ids
            .binary_search_by(|k| k.as_str().cmp(id))
            .ok()
            .map(|i| TypeIndex(i as u16))
    }

    pub fn id(&self, t: TypeIndex) -> &str {
        &self.ids[t.0 as usize]
    }

    /// The ids in index order: the publication's type table.
    pub fn ids(&self) -> &[String] {
        &self.ids
    }

    pub fn get(&self, t: TypeIndex) -> &UnitType {
        &self.types[t.0 as usize]
    }

    /// The type named `id`; a caller holding an id from the same catalog.
    pub fn by_id(&self, id: &str) -> &UnitType {
        let t = self
            .index(id)
            .unwrap_or_else(|| panic!("no unit type {id}"));
        self.get(t)
    }

    /// The mounts a unit of this type carries, in order: a hull's own, or
    /// its soldiers' in slot order, with only squad rifles grouped.
    pub fn mounts(&self, t: TypeIndex) -> &[CarriedMount] {
        &self.mounts[t.0 as usize]
    }

    pub fn soldier(&self, id: &str) -> &SoldierKind {
        &self.soldiers[id]
    }

    pub fn soldiers(&self) -> &BTreeMap<String, SoldierKind> {
        &self.soldiers
    }

    /// The prop types.
    pub fn props(&self) -> &PropCatalog {
        &self.props
    }

    /// Every mount, a type's own or a soldier kind's, names weapon rows
    /// that exist among `weapons` (the rules' rows, resolved).
    pub fn check_weapons(&self, weapons: &crate::weapons::WeaponRules) -> Result<(), CatalogError> {
        let missing = |mounts: &[MountDefinition]| {
            mounts.iter().find_map(|m| {
                let w = m.weapons.iter().find(|w| !weapons.contains_key(*w))?;
                Some(format!(
                    "mount {:?} names weapon row {w:?}, which does not exist",
                    m.name
                ))
            })
        };
        for (id, s) in &self.soldiers {
            if let Some(error) = missing(&s.mounts) {
                let (section, id) = ("soldiers", id.clone());
                return Err(CatalogError::Invalid { section, id, error });
            }
        }
        for (id, t) in self.ids.iter().zip(&self.types) {
            if let Some(error) = missing(&t.mounts) {
                return Err(CatalogError::Rule {
                    id: id.clone(),
                    error,
                });
            }
        }
        Ok(())
    }

    /// A self-contained view for readers outside the simulation: every
    /// type with its id and carried mounts, in index order, beside the
    /// roles, parts and soldier kinds; and the resolved `documents` a
    /// scenario's rules carry as their catalog.
    pub fn view(&self) -> Value {
        let units: Vec<Value> = self
            .indices()
            .map(|t| {
                let mut v = serde_json::to_value(self.get(t)).expect("types serialize");
                v["id"] = Value::from(self.id(t));
                v["mounts"] = serde_json::to_value(self.mounts(t)).expect("mounts serialize");
                v
            })
            .collect();
        let props: BTreeMap<&str, &PropType> = self
            .props
            .ids
            .iter()
            .map(String::as_str)
            .zip(&self.props.types)
            .collect();
        serde_json::json!({
            "documents": self,
            "roles": self.roles,
            "parts": self.parts,
            "soldiers": self.soldiers,
            "units": units,
            "props": props,
        })
    }
}

impl UnitType {
    /// A vehicle: it has a hull.
    pub fn hull(&self) -> Option<&Hull> {
        match &self.body {
            Body::Hull(h) => Some(h),
            Body::Squad { .. } => None,
        }
    }

    /// Infantry: it has soldier slots.
    pub fn slots(&self) -> Option<&[String]> {
        match &self.body {
            Body::Squad { slots } => Some(slots),
            Body::Hull(_) => None,
        }
    }

    /// Soldiers at full strength (0 for a hull).
    pub fn squad_size(&self) -> usize {
        self.slots().map_or(0, <[String]>::len)
    }

    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }
}

/// Why a catalog failed to load: every error names the entry at fault.
#[derive(Clone, Debug, PartialEq)]
pub enum CatalogError {
    UnknownSection(String),
    NotAnObject(String),
    Duplicate {
        section: &'static str,
        id: String,
    },
    UnknownParent {
        section: &'static str,
        id: String,
        parent: String,
    },
    Cycle {
        section: &'static str,
        chain: Vec<String>,
    },
    /// A leaf missing a field, carrying one it shouldn't, or out of shape.
    Invalid {
        section: &'static str,
        id: String,
        error: String,
    },
    UnknownRole {
        id: String,
        role: String,
    },
    UnknownSoldier {
        id: String,
        soldier: String,
    },
    UnknownPart {
        id: String,
        part: String,
    },
    /// A wrong component combination or a bad cross-reference.
    Rule {
        id: String,
        error: String,
    },
    /// A prop type's body row, destroyed state or wreck reference is wrong.
    Prop {
        id: String,
        error: String,
    },
    /// One document writes a key twice inside one object: a JSON reader
    /// would keep only the last, silently dropping an entry.
    DuplicateKey {
        key: String,
        line: usize,
        column: usize,
    },
    /// A document's text is not JSON.
    Syntax(String),
}

impl std::fmt::Display for CatalogError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use CatalogError::*;
        match self {
            UnknownSection(s) => write!(f, "unknown catalog section {s:?}"),
            NotAnObject(what) => write!(f, "{what} must be a JSON object"),
            Duplicate { section, id } => write!(f, "{section}.{id} is defined twice"),
            UnknownParent {
                section,
                id,
                parent,
            } => {
                write!(f, "{section}.{id} extends {parent:?}, which does not exist")
            }
            Cycle { section, chain } => {
                write!(f, "{section}: extends loops: {}", chain.join(" -> "))
            }
            Invalid { section, id, error } => write!(f, "{section}.{id}: {error}"),
            UnknownRole { id, role } => write!(f, "units.{id}: unknown role {role:?}"),
            UnknownSoldier { id, soldier } => {
                write!(f, "units.{id}: unknown soldier kind {soldier:?}")
            }
            UnknownPart { id, part } => write!(f, "units.{id}: unknown part {part:?}"),
            Rule { id, error } => write!(f, "units.{id}: {error}"),
            Prop { id, error } => write!(f, "props.{id}: {error}"),
            DuplicateKey { key, line, column } => write!(
                f,
                "key {key:?} is written twice in one object (line {line}, column {column})"
            ),
            Syntax(e) => write!(f, "a catalog document is not JSON: {e}"),
        }
    }
}

/// Parse one catalog document's text (or a list of them), refusing any key
/// written twice inside one object.
pub fn parse_document(text: &str) -> Result<Value, CatalogError> {
    let repeated = std::cell::RefCell::new(None);
    let mut de = serde_json::Deserializer::from_str(text);
    let parsed = serde::de::DeserializeSeed::deserialize(Unique(&repeated), &mut de)
        .and_then(|v| de.end().map(|_| v));
    parsed.map_err(|e| match repeated.take() {
        Some(key) => CatalogError::DuplicateKey {
            key,
            line: e.line(),
            column: e.column(),
        },
        None => CatalogError::Syntax(e.to_string()),
    })
}

/// A JSON value whose objects never repeat a key; the first repeated key
/// is kept for the error.
struct Unique<'a>(&'a std::cell::RefCell<Option<String>>);

impl<'de> serde::de::DeserializeSeed<'de> for Unique<'_> {
    type Value = Value;
    fn deserialize<D: serde::Deserializer<'de>>(self, d: D) -> Result<Value, D::Error> {
        d.deserialize_any(self)
    }
}

impl<'de> serde::de::Visitor<'de> for Unique<'_> {
    type Value = Value;
    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("a JSON value")
    }
    fn visit_bool<E>(self, v: bool) -> Result<Value, E> {
        Ok(Value::Bool(v))
    }
    fn visit_i64<E>(self, v: i64) -> Result<Value, E> {
        Ok(Value::from(v))
    }
    fn visit_u64<E>(self, v: u64) -> Result<Value, E> {
        Ok(Value::from(v))
    }
    fn visit_f64<E>(self, v: f64) -> Result<Value, E> {
        Ok(Value::from(v))
    }
    fn visit_str<E>(self, v: &str) -> Result<Value, E> {
        Ok(Value::from(v))
    }
    fn visit_string<E>(self, v: String) -> Result<Value, E> {
        Ok(Value::String(v))
    }
    fn visit_unit<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<Value, A::Error> {
        let mut out = Vec::new();
        while let Some(v) = seq.next_element_seed(Unique(self.0))? {
            out.push(v);
        }
        Ok(Value::Array(out))
    }
    fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
        let mut out = Map::new();
        while let Some(key) = map.next_key::<String>()? {
            if out.contains_key(&key) {
                let e = serde::de::Error::custom(format!("key {key:?} is written twice"));
                *self.0.borrow_mut() = Some(key);
                return Err(e);
            }
            let v = map.next_value_seed(Unique(self.0))?;
            out.insert(key, v);
        }
        Ok(Value::Object(out))
    }
}

impl std::error::Error for CatalogError {}

const SECTIONS: [&str; 5] = ["roles", "parts", "soldiers", "units", "props"];

/// Resolve authored catalog documents into flat records.
pub fn resolve(documents: &[Value]) -> Result<Catalog, CatalogError> {
    let mut sections: BTreeMap<&'static str, Map<String, Value>> = BTreeMap::new();
    for doc in documents {
        let doc = doc
            .as_object()
            .ok_or_else(|| CatalogError::NotAnObject("a catalog document".into()))?;
        for (key, entries) in doc {
            let section = *SECTIONS
                .iter()
                .find(|s| **s == key)
                .ok_or_else(|| CatalogError::UnknownSection(key.clone()))?;
            let entries = entries
                .as_object()
                .ok_or_else(|| CatalogError::NotAnObject(format!("section {key}")))?;
            let into = sections.entry(section).or_default();
            for (id, entry) in entries {
                if into.insert(id.clone(), entry.clone()).is_some() {
                    return Err(CatalogError::Duplicate {
                        section,
                        id: id.clone(),
                    });
                }
            }
        }
    }
    let mut section = |name| sections.remove(name).unwrap_or_default();
    let roles: BTreeMap<String, Role> = parse("roles", section("roles").into_iter().collect())?;
    let parts: BTreeMap<String, Part> = parse("parts", inherit("parts", &section("parts"))?)?;
    let soldiers: BTreeMap<String, SoldierKind> =
        parse("soldiers", inherit("soldiers", &section("soldiers"))?)?;
    for (id, s) in &soldiers {
        check_soldier(id, s)?;
    }
    let mut units = inherit("units", &section("units"))?;
    for (id, unit) in &mut units {
        let listed: Vec<String> = match unit.get("parts") {
            Some(p) => serde_json::from_value(p.clone()).map_err(|e| CatalogError::Invalid {
                section: "units",
                id: id.clone(),
                error: format!("parts: {e}"),
            })?,
            None => Vec::new(),
        };
        for part in listed {
            let p = parts.get(&part).ok_or_else(|| CatalogError::UnknownPart {
                id: id.clone(),
                part: part.clone(),
            })?;
            merge_entry("units", unit, &p.patch);
        }
    }
    let types: BTreeMap<String, UnitType> = parse("units", units)?;
    let (ids, types): (Vec<String>, Vec<UnitType>) = types.into_iter().unzip();
    let props: BTreeMap<String, PropType> = parse("props", inherit("props", &section("props"))?)?;
    let (prop_ids, prop_types): (Vec<String>, Vec<PropType>) = props.into_iter().unzip();
    let props = PropCatalog {
        ids: prop_ids,
        types: prop_types,
    };
    for k in props.kinds() {
        check_prop(&props, k)?;
    }
    let mut mounts = Vec::with_capacity(types.len());
    for (id, t) in ids.iter().zip(&types) {
        check(id, t, &roles, &soldiers, &props)?;
        mounts.push(carried(id, t, &soldiers)?);
    }
    Ok(Catalog {
        roles,
        parts,
        soldiers,
        ids,
        types,
        mounts,
        props,
    })
}

/// Parse each flat entry of a section into its record.
fn parse<T: serde::de::DeserializeOwned>(
    section: &'static str,
    entries: BTreeMap<String, Value>,
) -> Result<BTreeMap<String, T>, CatalogError> {
    entries
        .into_iter()
        .map(|(id, v)| {
            let record = serde_json::from_value(v).map_err(|e| CatalogError::Invalid {
                section,
                id: id.clone(),
                error: e.to_string(),
            })?;
            Ok((id, record))
        })
        .collect()
}

/// Flatten `extends` chains; abstract entries are dropped.
pub fn inherit(
    section: &'static str,
    entries: &Map<String, Value>,
) -> Result<BTreeMap<String, Value>, CatalogError> {
    fn flat(
        section: &'static str,
        id: &str,
        entries: &Map<String, Value>,
        done: &mut BTreeMap<String, Value>,
        chain: &mut Vec<String>,
    ) -> Result<Value, CatalogError> {
        if let Some(v) = done.get(id) {
            return Ok(v.clone());
        }
        if chain.iter().any(|c| c == id) {
            chain.push(id.to_string());
            return Err(CatalogError::Cycle {
                section,
                chain: chain.clone(),
            });
        }
        let mut own = entries[id].clone();
        let own_map = own
            .as_object_mut()
            .ok_or_else(|| CatalogError::NotAnObject(format!("{section}.{id}")))?;
        let parent = own_map.remove("extends");
        let resolved = match parent {
            None => own,
            Some(Value::String(parent)) => {
                if !entries.contains_key(&parent) {
                    return Err(CatalogError::UnknownParent {
                        section,
                        id: id.to_string(),
                        parent,
                    });
                }
                chain.push(id.to_string());
                let mut base = flat(section, &parent, entries, done, chain)?;
                chain.pop();
                base.as_object_mut()
                    .expect("resolved entries are objects")
                    .remove("abstract");
                merge_entry(section, &mut base, &own);
                base
            }
            Some(other) => {
                return Err(CatalogError::Invalid {
                    section,
                    id: id.to_string(),
                    error: format!("extends must be an id, not {other}"),
                })
            }
        };
        done.insert(id.to_string(), resolved.clone());
        Ok(resolved)
    }
    let mut done = BTreeMap::new();
    for id in entries.keys() {
        flat(section, id, entries, &mut done, &mut Vec::new())?;
    }
    Ok(done
        .into_iter()
        .filter(|(_, v)| v.get("abstract") != Some(&Value::Bool(true)))
        .map(|(id, mut v)| {
            v.as_object_mut()
                .expect("resolved entries are objects")
                .remove("abstract");
            (id, v)
        })
        .collect())
}

/// A unit type's components written as a one-key variant object
/// (`{ "hull": … }`, `{ "tracked": … }`).
const VARIANTS: [&str; 2] = ["body", "mobility"];

/// Merge an `extends` child (or a part's patch) into an entry of `section`,
/// as [`merge`] does, with two rules for unit types: a component the child
/// writes as another variant (`mobility: { wheeled }` over `tracked`)
/// replaces the parent's rather than merging both, and `parts` gather along
/// the chain, the parent's first. Idempotent.
pub fn merge_entry(section: &str, base: &mut Value, over: &Value) {
    let parts = match (section, base.as_object_mut(), over.as_object()) {
        ("units", Some(b), Some(o)) => {
            for key in VARIANTS {
                let swapped = match (b.get(key).and_then(variant), o.get(key).and_then(variant)) {
                    (Some(was), Some(now)) => was != now,
                    _ => false,
                };
                if swapped {
                    b.remove(key);
                }
            }
            let listed = |v: Option<&Value>| -> Vec<Value> {
                v.and_then(Value::as_array).cloned().unwrap_or_default()
            };
            let mut parts = listed(b.get("parts"));
            for p in listed(o.get("parts")) {
                if !parts.contains(&p) {
                    parts.push(p);
                }
            }
            Some(parts).filter(|p| !p.is_empty())
        }
        _ => None,
    };
    merge(base, over);
    if let Some(parts) = parts {
        base["parts"] = Value::Array(parts);
    }
}

/// The variant a one-key object names.
fn variant(v: &Value) -> Option<&str> {
    let o = v.as_object()?;
    (o.len() == 1).then(|| o.keys().next().map(String::as_str))?
}

/// Merge `over` into `base`: objects key by key, lists of named objects by
/// name (a new name is appended), anything else replaced. Idempotent.
pub fn merge(base: &mut Value, over: &Value) {
    match (base, over) {
        (Value::Object(b), Value::Object(o)) => {
            for (k, v) in o {
                match b.get_mut(k) {
                    Some(slot) => merge(slot, v),
                    None => {
                        b.insert(k.clone(), v.clone());
                    }
                }
            }
        }
        (Value::Array(b), Value::Array(o)) if named(b) && named(o) => {
            for item in o {
                let name = &item["name"];
                match b.iter_mut().find(|x| &x["name"] == name) {
                    Some(slot) => merge(slot, item),
                    None => b.push(item.clone()),
                }
            }
        }
        (b, o) => *b = o.clone(),
    }
}

/// A non-empty list whose every item is an object with a string `name`.
fn named(list: &[Value]) -> bool {
    !list.is_empty()
        && list
            .iter()
            .all(|x| x.get("name").is_some_and(Value::is_string))
}

/// The component rules one type must keep: known roles and soldier kinds,
/// mounts where its body carries them, optics on a turret, supply deployed.
fn check(
    id: &str,
    t: &UnitType,
    roles: &BTreeMap<String, Role>,
    soldiers: &BTreeMap<String, SoldierKind>,
    props: &PropCatalog,
) -> Result<(), CatalogError> {
    let rule = |error: &str| {
        Err(CatalogError::Rule {
            id: id.to_string(),
            error: error.to_string(),
        })
    };
    if t.roles.is_empty() {
        return rule("a type needs at least one role");
    }
    if let Some(role) = t.roles.iter().find(|r| !roles.contains_key(*r)) {
        return Err(CatalogError::UnknownRole {
            id: id.to_string(),
            role: role.clone(),
        });
    }
    match &t.body {
        Body::Squad { slots } => {
            if slots.is_empty() {
                return rule("a squad needs at least one slot");
            }
            if let Some(s) = slots.iter().find(|s| !soldiers.contains_key(*s)) {
                return Err(CatalogError::UnknownSoldier {
                    id: id.to_string(),
                    soldier: s.clone(),
                });
            }
            if !t.mounts.is_empty() {
                return rule("a squad's mounts come from its soldiers");
            }
            if t.appearance.is_some() {
                return rule("a squad draws its soldiers' appearances, not its own");
            }
            if !matches!(t.mobility, Mobility::Foot { .. }) {
                return rule("a squad moves on foot");
            }
        }
        Body::Hull(h) => {
            if matches!(t.mobility, Mobility::Foot { .. }) {
                return rule("a hull does not move on foot");
            }
            let Some(wreck) = props.index(&h.wreck) else {
                return rule(&format!("its wreck {:?} is not a prop type", h.wreck));
            };
            if let Err(error) = props.check_placement(wreck, PropPlacement::Ordinary) {
                return rule(&format!("its wreck {:?}: {error}", h.wreck));
            }
            if t.appearance.is_none() {
                return rule("a hull needs an appearance");
            }
        }
    }
    if let Some(on) = &t.sensors.on {
        if !t.mounts.iter().any(|m| &m.name == on && m.turret) {
            return rule(&format!(
                "sensors.on names {on:?}, which is not a turret mount"
            ));
        }
    }
    if t.capabilities.supply.is_some() && t.capabilities.deploy.is_none() {
        return rule("supply serves only when deployed: it needs deploy");
    }
    // A hull's mounts: each on the hull or an earlier turret, firing from
    // its own muzzle, and neither a squad's weapon nor a soldier's.
    for (i, m) in t.mounts.iter().enumerate() {
        if let Some(on) = &m.on {
            if !t.mounts[..i].iter().any(|c| &c.name == on && c.turret) {
                return rule(&format!(
                    "mount {:?} is on {on:?}, which is not an earlier turret mount",
                    m.name
                ));
            }
        }
        if m.muzzle_m.is_none() {
            return rule(&format!("mount {:?} on a hull needs muzzle_m", m.name));
        }
        if m.squad || m.special {
            return rule(&format!(
                "mount {:?} on a hull is neither squad nor special",
                m.name
            ));
        }
    }
    if let Body::Hull(h) = &t.body {
        // A wreck keeps its live vehicle's cover (Q24).
        let row = props.by_id(&h.wreck).body.cover_tier;
        let want = h.weight_class.cover_tier();
        if row != want {
            let name = |t: Option<CoverTier>| t.map_or("no".into(), |t| lower(&t));
            return rule(&format!(
                "its wreck {:?} gives {} cover, not its {} weight's {} (Q24)",
                h.wreck,
                name(row),
                lower(&h.weight_class),
                name(want)
            ));
        }
        if !(h.half_extents_m.iter().all(|&e| positive(e)) && positive(h.eye_m) && positive(h.hp)) {
            return rule("a hull needs finite positive extents, eye height and hp");
        }
        let armor = h.armor;
        if ![armor.front, armor.side, armor.rear, armor.roof]
            .into_iter()
            .all(at_least_zero)
        {
            return rule("body.hull.armor needs finite nonnegative protection on every face");
        }
        let chance = armor.ricochet;
        if ![chance.front, chance.side, chance.rear, chance.roof]
            .into_iter()
            .all(|p| p.is_finite() && (0.0..=1.0).contains(&p))
        {
            return rule("body.hull.armor.ricochet needs probabilities in [0, 1] on every face");
        }
    }
    ranges(t).map_or(Ok(()), rule)
}

/// A type's numbers in the ranges the simulation divides and eases by.
fn ranges(t: &UnitType) -> Option<&'static str> {
    let s = t.sensors.sight_shape;
    let (offroad_kmh, road_kmh) = t.mobility.speeds_kmh();
    if !(offroad_kmh > 0.0 && road_kmh >= offroad_kmh && road_kmh <= MAX_SPEED_KMH) {
        return Some("mobility: speeds must satisfy 0 < offroad_kmh <= road_kmh <= 130");
    }
    let bad = match t.mobility {
        Mobility::Foot { .. } => None,
        Mobility::Tracked {
            turn_deg_s,
            reverse_fraction,
            ..
        }
        | Mobility::Wheeled {
            turn_deg_s,
            reverse_fraction,
            ..
        } => {
            if !positive(turn_deg_s) {
                Some("mobility: turn_deg_s must be positive")
            } else if !(reverse_fraction > 0.0 && reverse_fraction <= 1.0) {
                Some("mobility.reverse_fraction must lie in (0, 1]")
            } else {
                None
            }
        }
    };
    bad.or_else(|| match t.mobility {
        Mobility::Wheeled {
            turning_radius_m, ..
        } if !positive(turning_radius_m) => {
            Some("a wheeled vehicle needs a positive turning_radius_m")
        }
        _ => None,
    })
    .or_else(|| {
        (!at_least_zero(t.sound.loudness_m)).then_some("sound.loudness_m must not be negative")
    })
    .or_else(|| (!positive(t.sensors.ground_m)).then_some("sensors.ground_m must be positive"))
    .or_else(|| {
        (!(positive(s.rear) && s.rear <= s.side && s.side <= s.front && s.front.is_finite()))
            .then_some("sensors.sight_shape must have 0 < rear <= side <= front")
    })
    .or_else(|| {
        t.capabilities
            .deploy
            .is_some_and(|d| {
                !at_least_zero(d.seconds) || d.pack_seconds.is_some_and(|s| !at_least_zero(s))
            })
            .then_some("capabilities.deploy durations must not be negative")
    })
}

/// Finite and positive.
fn positive(x: f64) -> bool {
    x.is_finite() && x > 0.0
}

/// Finite and zero or more.
fn at_least_zero(x: f64) -> bool {
    x.is_finite() && x >= 0.0
}

/// A serde enum's name as the documents write it.
fn lower(v: &impl std::fmt::Debug) -> String {
    format!("{v:?}").to_lowercase()
}

/// The rules one soldier kind must keep: living hp, and hand weapons only,
/// never both a squad's and a special one.
fn check_soldier(id: &str, s: &SoldierKind) -> Result<(), CatalogError> {
    let invalid = |error: String| {
        Err(CatalogError::Invalid {
            section: "soldiers",
            id: id.to_string(),
            error,
        })
    };
    if !positive(s.hp) {
        return invalid("hp must be positive".into());
    }
    for m in &s.mounts {
        if m.squad && m.special {
            return invalid(format!(
                "mount {:?} is a squad weapon or a special one, not both",
                m.name
            ));
        }
        if m.turret || m.on.is_some() || m.pivot_m != [0.0; 3] || m.muzzle_m.is_some() {
            return invalid(format!(
                "mount {:?} is a hand weapon: no turret, on, pivot_m or muzzle_m",
                m.name
            ));
        }
    }
    Ok(())
}

/// The rules one prop type must keep: a transient body lives a positive
/// time; `hp` and `destroyed` go together, `hp` positive and `armor` in
/// [0, 1]; only a toppling body's destroyed state is cleared ground; and a
/// destroyed state ends: each `into` names a prop type, at a positive
/// height, never back up its chain.
fn check_prop(props: &PropCatalog, k: PropKind) -> Result<(), CatalogError> {
    let id = props.id(k);
    let t = props.get(k);
    let rule = |error: &str| {
        Err(CatalogError::Prop {
            id: id.to_string(),
            error: error.to_string(),
        })
    };
    let b = &t.body;
    if b.lifetime_s.is_some_and(|s| s <= 0.0) {
        return rule("lifetime_s must be positive");
    }
    if b.hp.is_some() != t.destroyed.is_some() {
        return rule("hp and destroyed go together");
    }
    if b.hp_scale == HpScale::BuildingFloorBands && b.hp.is_none() {
        return rule("building_floor_bands integrity requires hp");
    }
    if b.hp.is_some_and(|h| !h.is_finite() || h <= 0.0) || !(0.0..=1.0).contains(&b.armor) {
        return rule("hp must be positive and armor within [0, 1]");
    }
    if t.destroyed == Some(Destroyed::Cleared) && !b.topples {
        return rule("only a toppling body's destroyed state is cleared ground");
    }
    let mut state_type = t;
    let mut next = state_type.destroyed.as_ref();
    for _ in 0..=props.types.len() {
        let Some(Destroyed::Into {
            prop,
            height_m,
            building,
        }) = next
        else {
            return Ok(());
        };
        if !height_m.is_finite() || *height_m <= 0.0 {
            return rule("destroyed.into.height_m must be positive");
        }
        if let Some(building) = building {
            if !building.height_fraction.is_finite()
                || building.height_fraction <= 0.0
                || !building.max_height_m.is_finite()
                || building.max_height_m < *height_m
                || building.collapse_max_floors == 0
            {
                return rule("building remains require positive height fraction, bounded height, and floor threshold");
            }
            let Some(gutted) = props.index(&building.gutted_prop) else {
                return rule("building gutted_prop must name a prop type");
            };
            let shell = props.get(gutted);
            if shell.body.hp.is_some() || shell.body.garrison || shell.destroyed.is_some() {
                return rule("a gutted shell must be terminal and ungarrisonable");
            }
            if shell.body.blocks != state_type.body.blocks
                || shell.body.occludes != state_type.body.occludes
                || shell.body.stops_rounds != state_type.body.stops_rounds
                || shell.body.cover_tier != Some(CoverTier::Medium)
            {
                return rule("a gutted shell must keep blocking, sight and round stopping, with medium cover");
            }
        }
        let Some(into) = props.index(prop) else {
            return rule(&format!(
                "destroyed into {prop:?}, which is not a prop type"
            ));
        };
        state_type = props.get(into);
        next = state_type.destroyed.as_ref();
    }
    rule("its destroyed states loop")
}

/// The mounts a type carries: a hull's own rows, or each soldier kind's in
/// slot order. Only squad weapons share a row; other guns remain physical
/// instances, even when their authored names match.
fn carried(
    id: &str,
    t: &UnitType,
    soldiers: &BTreeMap<String, SoldierKind>,
) -> Result<Vec<CarriedMount>, CatalogError> {
    let Some(slots) = t.slots() else {
        return Ok(t
            .mounts
            .iter()
            .map(|m| CarriedMount {
                def: m.clone(),
                carriers: Vec::new(),
            })
            .collect());
    };
    let mut out: Vec<CarriedMount> = Vec::new();
    for (k, slot) in slots.iter().enumerate() {
        for m in &soldiers[slot].mounts {
            if !m.squad {
                out.push(CarriedMount {
                    def: m.clone(),
                    carriers: vec![k],
                });
                continue;
            }
            match out.iter_mut().find(|c| c.def.squad && c.def.name == m.name) {
                Some(c) if c.def == *m => c.carriers.push(k),
                Some(_) => {
                    return Err(CatalogError::Rule {
                        id: id.to_string(),
                        error: format!(
                            "slot {k} ({slot}) carries a different mount named {:?}",
                            m.name
                        ),
                    })
                }
                None => out.push(CarriedMount {
                    def: m.clone(),
                    carriers: vec![k],
                }),
            }
        }
    }
    Ok(out)
}

/// A catalog travels as documents: resolved, it is one document that
/// resolves to itself.
impl Serialize for Catalog {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let units: BTreeMap<&str, &UnitType> = self
            .ids
            .iter()
            .map(String::as_str)
            .zip(&self.types)
            .collect();
        let props: BTreeMap<&str, &PropType> = self
            .props
            .ids
            .iter()
            .map(String::as_str)
            .zip(&self.props.types)
            .collect();
        let doc = serde_json::json!({
            "roles": self.roles,
            "parts": self.parts,
            "soldiers": self.soldiers,
            "units": units,
            "props": props,
        });
        [doc].serialize(s)
    }
}

impl<'de> Deserialize<'de> for Catalog {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let documents = Vec::<Value>::deserialize(d)?;
        resolve(&documents).map_err(serde::de::Error::custom)
    }
}
