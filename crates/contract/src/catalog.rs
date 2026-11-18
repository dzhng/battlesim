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
//! deep-merge, lists of objects with ids (mounts) merge by id, a unit's
//! variant component written as another variant replaces its parent's, a
//! unit's `parts` gather along the chain, any other value is replaced
//! ([`merge_entry`]); then a type's `parts` apply in order, merged the same
//! way. Merging is idempotent, so a resolved catalog serialises to a
//! document that resolves to itself. Then every entry is checked, and a
//! failure names it ([`CatalogError`]).
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::observation::SoundCategory;
use crate::scenario::{Blocks, CoverTier, WeightClass};
use crate::weapons::MountDefinition;

pub mod inheritance;
pub use inheritance::{inherit, merge, merge_entry};

mod mobility;
use mobility::air_problem;
pub use mobility::{Arrival, FlightModel, Mobility, MAX_SPEED_KMH};
mod sensors;
pub use sensors::Sensors;
mod charges;
pub use charges::{ActiveProtection, FlareCapability, SmokeCapability};
mod capabilities;
pub use capabilities::{
    AfterburnerCapability, AirCapability, Capabilities, Carriage, Deploy, RadarReceiver,
    RefundWeights, Supply,
};

mod checks;
mod hull;
use checks::{check, check_card_labels, check_prop, check_roster, check_soldier, mount_labels};
pub use hull::Hull;
mod props;
pub use props::{
    BuildingRemains, Destroyed, HpScale, PropAppearance, PropAppearanceStatus, PropBody,
    PropCatalog, PropKind, PropPlacement, PropType,
};

/// A unit type's place in the catalog: its rank among the ids in sorted order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TypeIndex(pub u16);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Faction {
    Us,
    Europe,
    Eastern,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Rec,
    Inf,
    Veh,
    Sup,
    Hel,
    Air,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RosterMembership {
    pub factions: Vec<Faction>,
    pub family_name: String,
    pub category: Category,
    pub variant: String,
}

/// Unimplemented equipment remains card data, outside physical type admission.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlannedCapability {
    pub reason: String,
    pub profile: String,
    pub weapons: Vec<PlannedWeapon>,
}

/// A planned unit's weapon as its card shows it; it has no row to fire.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlannedWeapon {
    /// A concise label (`crate::labels`).
    pub name: String,
    /// Its generated icon, `assets/icons/weapons/<icon>.svg`.
    pub icon: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlannedUnit {
    pub name: String,
    pub description: String,
    pub faction: String,
    pub family: String,
    pub roles: Vec<String>,
    pub cost: u32,
    pub roster: RosterMembership,
    pub planned: PlannedCapability,
}

/// A picker projection; availability is decided by this catalog alone.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct UnitCard<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub description: &'a str,
    pub family: &'a str,
    pub cost: u32,
    pub roster: &'a RosterMembership,
    pub disabled_reason: Option<&'a str>,
    /// A disabled card's planned weapons, as its card shows them; empty for
    /// a unit type, whose weapons are its mounts.
    pub planned_weapons: &'a [PlannedWeapon],
}

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roster: Option<RosterMembership>,
    /// Tags from the role registry: what scripts and the AI select by.
    pub roles: Vec<String>,
    /// Purchase price and automatic target-value priority share one owner.
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
    /// Intrinsic visual concealment, multiplied by ground and movement.
    pub concealment: f64,
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

/// The height band a unit occupies, which decides what can engage it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AltitudeLayer {
    Ground,
    /// Below every building's obstacle height plus clearance.
    LowAir,
    /// The one layer that bends physics: its heights are lowered from a
    /// real jet's for play, and nothing in it sees the ground.
    HighAir,
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
    planned: BTreeMap<String, PlannedUnit>,
}

impl Catalog {
    pub fn card(&self, id: &str) -> Option<UnitCard<'_>> {
        if let Some((id, unit)) = self.planned.get_key_value(id) {
            return Some(UnitCard {
                id,
                name: &unit.name,
                description: &unit.description,
                family: &unit.family,
                cost: unit.cost,
                roster: &unit.roster,
                disabled_reason: Some(&unit.planned.reason),
                planned_weapons: &unit.planned.weapons,
            });
        }
        let index = self.index(id)?;
        let unit = self.get(index);
        Some(UnitCard {
            id: self.id(index),
            name: &unit.name,
            description: &unit.description,
            family: &unit.family,
            cost: unit.cost,
            roster: unit.roster.as_ref()?,
            disabled_reason: None,
            planned_weapons: &[],
        })
    }

    pub fn cards(&self) -> impl Iterator<Item = UnitCard<'_>> {
        self.ids
            .iter()
            .chain(self.planned.keys())
            .filter_map(|id| self.card(id))
    }

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
    /// that exist among `weapons` (the rules' rows, resolved). An unpowered
    /// row (a bomb) is carried only by a winged aircraft: its pass is the
    /// one flight that plans a release.
    pub fn check_weapons(&self, weapons: &crate::weapons::WeaponRules) -> Result<(), CatalogError> {
        let invalid = |mounts: &[MountDefinition], flight: Option<FlightModel>| {
            mounts.iter().find_map(|m| {
                if let Some(w) = m.weapons.iter().find(|w| !weapons.contains_key(*w)) {
                    return Some(format!(
                        "mount {:?} names weapon row {w:?}, which does not exist",
                        m.id
                    ));
                }
                let bomb = m.weapons.iter().find(|w| {
                    weapons[*w].ballistics.delivery == crate::ballistics::Delivery::Unpowered
                })?;
                (flight != Some(FlightModel::FixedWing)).then(|| {
                    format!(
                        "mount {:?} carries unpowered row {bomb:?}, which only a winged aircraft's pass can release",
                        m.id
                    )
                })
            })
        };
        for (id, s) in &self.soldiers {
            if let Some(error) = invalid(&s.mounts, None) {
                let (section, id) = ("soldiers", id.clone());
                return Err(CatalogError::Invalid { section, id, error });
            }
        }
        for (id, t) in self.ids.iter().zip(&self.types) {
            if let Some(error) = invalid(&t.mounts, t.mobility.flight()) {
                return Err(CatalogError::Rule {
                    id: id.clone(),
                    error,
                });
            }
        }
        Ok(())
    }

    /// A self-contained view for readers outside the simulation: every
    /// type with its id, carried mounts and vehicle class, in index order, beside the
    /// roles, parts and soldier kinds; and the resolved `documents` a
    /// scenario's rules carry as their catalog.
    pub fn view(&self) -> Value {
        let units: Vec<Value> = self
            .indices()
            .map(|t| {
                let mut v = serde_json::to_value(self.get(t)).expect("types serialize");
                v["id"] = Value::from(self.id(t));
                v["mounts"] = serde_json::to_value(self.mounts(t)).expect("mounts serialize");
                v["vehicle_class"] = Value::from(self.get(t).vehicle_class());
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
            "cards": self.cards().collect::<Vec<_>>(),
            "props": props,
        })
    }
}

mod unit_type;
pub use unit_type::Landing;

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

/// The sections a catalog document may hold.
pub const SECTIONS: [&str; 5] = ["roles", "parts", "soldiers", "units", "props"];

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
        check_card_labels("soldiers", id, &mount_labels(&s.mounts), &[])?;
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
    let mut planned = BTreeMap::new();
    let mut physical = BTreeMap::new();
    for (id, unit) in units {
        if unit.get("planned").is_some() {
            planned.insert(id, unit);
        } else {
            physical.insert(id, unit);
        }
    }
    let planned: BTreeMap<String, PlannedUnit> = parse("units", planned)?;
    let types: BTreeMap<String, UnitType> = parse("units", physical)?;
    for (id, unit) in &planned {
        check_roster(id, &unit.roster)?;
        let weapons = unit.planned.weapons.iter().enumerate();
        let mut labels = vec![
            (String::new(), unit.name.as_str()),
            ("roster.family_name".into(), &unit.roster.family_name),
        ];
        labels.extend(
            weapons
                .clone()
                .map(|(i, w)| (format!("planned.weapons[{i}]"), w.name.as_str())),
        );
        let icons: Vec<_> = weapons
            .map(|(i, w)| (format!("planned.weapons[{i}]"), w.icon.as_str()))
            .collect();
        check_card_labels("units", id, &labels, &icons)?;
        if unit.planned.reason.trim().is_empty()
            || unit.planned.profile.trim().is_empty()
            || unit.cost == 0
        {
            return Err(CatalogError::Invalid {
                section: "units",
                id: id.clone(),
                error: "planned unit needs a reason, profile and positive cost".into(),
            });
        }
    }
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
        if let Some(roster) = &t.roster {
            check_roster(id, roster)?;
        }
        let mut labels = vec![(String::new(), t.name.as_str())];
        labels.extend(
            t.roster
                .iter()
                .map(|r| ("roster.family_name".into(), r.family_name.as_str())),
        );
        labels.extend(mount_labels(&t.mounts));
        let protection = t.capabilities.active_protection.as_ref();
        let field = || "capabilities.active_protection".to_string();
        labels.extend(protection.map(|p| (field(), p.name.as_str())));
        let icons: Vec<_> = protection
            .map(|p| (field(), p.icon.as_str()))
            .into_iter()
            .collect();
        check_card_labels("units", id, &labels, &icons)?;
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
        planned,
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
            match out.iter_mut().find(|c| c.def.squad && c.def.id == m.id) {
                Some(c) if c.def == *m => c.carriers.push(k),
                Some(_) => {
                    return Err(CatalogError::Rule {
                        id: id.to_string(),
                        error: format!(
                            "slot {k} ({slot}) carries a different mount with id {:?}",
                            m.id
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
        let mut units: BTreeMap<&str, Value> = self
            .ids
            .iter()
            .map(String::as_str)
            .zip(&self.types)
            .map(|(id, unit)| (id, serde_json::to_value(unit).expect("unit serialization")))
            .collect();
        for (id, unit) in &self.planned {
            units.insert(
                id,
                serde_json::to_value(unit).expect("planned unit serialization"),
            );
        }
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
