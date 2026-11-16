//! What a battle's units and effects draw from, resolved from the
//! repository's own files: the battle's rules and unit
//! catalog, and the runtime catalog's bundles and skeleton clips. It is
//! `load_world`'s `units` and `effects` and each skinned body's clip table,
//! for a battle fielding a given set of unit types, as a page running those
//! units installs them.
//!
//! A caller reads the files; this decides. [`Fielding::contents`] names the
//! runtime contents a battle reads, and [`Fielding::resolve`] takes their
//! bytes: the native client reads them from the runtime directory (feature
//! `files`), the page fetches them as it fetches any bundle.
//!
//! The records are built in the shape `load_world` reads (`world.rs`) and
//! checked by it, so the one reader is also the one schema.
use crate::appearance::{articulated, skinned};
use crate::assets::{Container, BUNDLE};
use crate::settings::PresentationSettings;
use crate::units::drawn::ClipTables;
use crate::world::{EffectsSection, UnitsSection};
use contract::catalog::Mobility;
use serde_json::{json, Map, Value};
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// A wreck's pieces a cook-off throws (`WRECK_PIECES`) and its debris state.
const WRECK_HULL: &str = "hull";
const WRECK_TURRET: &str = "turret";
const WRECK_DEBRIS: &str = "debris";

/// A battle's units and effects, and the clip tables its bodies sample.
pub struct Fielded {
    pub units: UnitsSection,
    pub effects: EffectsSection,
    pub tables: ClipTables,
}

/// [`Fielded`]'s sections as `load_world` reads them, before its reader
/// checks them.
pub struct Sections {
    pub units: Value,
    pub effects: Value,
    pub tables: ClipTables,
}

impl Sections {
    /// The sections as `load_world` checks them.
    pub fn typed(self) -> Result<Fielded, String> {
        Ok(Fielded {
            units: serde_json::from_value(self.units).map_err(|e| format!("units: {e}"))?,
            effects: serde_json::from_value(self.effects).map_err(|e| format!("effects: {e}"))?,
            tables: self.tables,
        })
    }
}

/// A runtime content's bytes, inflated, by its hash. File readers can own
/// these; a boundary can lend its input without another complete catalog copy.
pub type Read<'a> = dyn Fn(&str) -> Result<Cow<'a, [u8]>, String> + 'a;

/// Facts derived from runtime bundles by a reusable fielding owner.
#[derive(Default)]
pub struct Facts {
    appearances: HashMap<(String, Option<String>), AppearanceFacts>,
    wrecks: HashMap<String, WreckFacts>,
}

pub(crate) struct BundleFacts<'a> {
    pub facts: &'a mut Facts,
    pub bytes: &'a HashMap<String, Cow<'a, [u8]>>,
}

#[derive(Clone)]
struct AppearanceFacts {
    record: Value,
    table: Option<Vec<f32>>,
}

#[derive(Clone)]
struct WreckFacts {
    bounds: Option<[Value; 2]>,
    lies: Option<[f64; 3]>,
    underside: f64,
    debris_top: Option<f64>,
}

/// A battle fielding `kinds` of the resolved unit catalog `catalog`
/// (`Catalog::view`) under `rules` (how long a soldier holds his firing
/// pose, how a dead hull stops), each side fighting for the faction
/// `factions` names (blue, red; none: one look each), with visual settings
/// (`fixtures/presentation.json`) and art from `runtime`. Weapon inheritance
/// comes from the battle's `rules.weapons` rows.
pub struct Fielding<'a> {
    pub presentation: &'a Value,
    pub rules: &'a Value,
    pub catalog: &'a Value,
    pub runtime: &'a Value,
    pub kinds: &'a [String],
    pub factions: Option<[&'a str; 2]>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Content {
    Appearance {
        name: String,
        bundle: String,
        skeleton: Option<String>,
    },
    Wreck {
        name: String,
        bundle: String,
    },
}
impl Content {
    pub fn hashes(&self) -> Vec<String> {
        match self {
            Self::Appearance {
                bundle, skeleton, ..
            } => std::iter::once(bundle.clone())
                .chain(skeleton.clone())
                .collect(),
            Self::Wreck { bundle, .. } => vec![bundle.clone()],
        }
    }
}

impl Fielding<'_> {
    /// Absorb one authored content into the battle's retained facts.
    pub fn absorb(
        &self,
        facts: &mut Facts,
        content: &Content,
        read: &Read<'_>,
    ) -> Result<(), String> {
        let catalog = Catalog::new(self.catalog)?;
        let art = Art::new(self.runtime, &catalog, self.kinds, self.factions)?;
        let bytes = content
            .hashes()
            .into_iter()
            .map(|hash| {
                let bytes = read(&hash)?;
                Ok((hash, bytes))
            })
            .collect::<Result<HashMap<_, _>, String>>()?;
        match content {
            Content::Appearance { name, .. } => {
                appearance_facts(facts, &art, &bytes, name)?;
            }
            Content::Wreck { name, .. } => {
                effects::wreck_facts(facts, &art, &bytes, name)?;
            }
        }
        Ok(())
    }

    /// Missing fact contents, with the source hashes each derivation needs.
    pub fn pending(&self, facts: &Facts) -> Result<Vec<Content>, String> {
        let catalog = Catalog::new(self.catalog)?;
        let art = Art::new(self.runtime, &catalog, self.kinds, self.factions)?;
        let (_, drawn) = unit_types(&catalog, &art, self.kinds, self.factions)?;
        let mut pending = BTreeMap::new();
        for name in drawn {
            let (bundle, skeleton) = art.key(&name)?;
            if !facts
                .appearances
                .contains_key(&(bundle.clone(), skeleton.clone()))
            {
                pending
                    .entry((bundle.clone(), skeleton.clone()))
                    .or_insert(Content::Appearance {
                        name,
                        bundle,
                        skeleton,
                    });
            }
        }
        for name in hull_wrecks(&catalog, &art).values() {
            let bundle = art.bundle(name)?.to_owned();
            if !facts.wrecks.contains_key(&bundle) {
                pending.insert(
                    (bundle.clone(), None),
                    Content::Wreck {
                        name: name.clone(),
                        bundle,
                    },
                );
            }
        }
        Ok(pending.into_values().collect())
    }

    /// The runtime contents [`Fielding::resolve`] reads, by hash, sorted:
    /// each drawn appearance's bundle, each fielded hull's wreck and every
    /// skeleton's clips.
    pub fn contents(&self, facts: &Facts) -> Result<Vec<String>, String> {
        Ok(self
            .pending(facts)?
            .iter()
            .flat_map(Content::hashes)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect())
    }

    /// The battle's sections, each content [`Fielding::contents`] names
    /// read by `read`.
    pub fn resolve(&self, facts: &mut Facts, read: &Read<'_>) -> Result<Sections, String> {
        let presentation = PresentationSettings::from_value(self.presentation)?;
        let catalog = Catalog::new(self.catalog)?;
        let looks = crate::looks::WeaponLooks::new(
            presentation.weapon_looks.clone(),
            &self.rules["weapons"],
        )?;
        looks.check_mounts(self.catalog)?;
        effects::check_smoke_screen_looks(&presentation, &catalog, self.kinds)?;
        let bytes: HashMap<String, Cow<'_, [u8]>> = self
            .contents(facts)?
            .into_iter()
            .map(|hash| {
                let value = read(&hash)?;
                Ok((hash, value))
            })
            .collect::<Result<_, String>>()?;
        let art = Art::new(self.runtime, &catalog, self.kinds, self.factions)?;
        let mut bundle_facts = BundleFacts {
            facts,
            bytes: &bytes,
        };
        let (mut units, tables) = units(
            &presentation,
            self.rules,
            &catalog,
            &art,
            &mut bundle_facts,
            self.kinds,
            self.factions,
        )?;
        for unit in units["types"].as_object_mut().unwrap().values_mut() {
            for mount in unit["mounts"].as_array_mut().unwrap() {
                mount["weapons"] = Value::Array(
                    mount["weapons"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|id| {
                            let weapon = id.as_str().unwrap();
                            json!({"kind": weapon, "kick": looks.get(weapon).kick.expect("fielded firing kick")})
                        })
                        .collect(),
                );
            }
        }
        let effects = effects::section(
            &presentation,
            &looks,
            self.rules,
            &catalog,
            &art,
            &mut bundle_facts,
            self.kinds,
        )?;
        Ok(Sections {
            units,
            effects,
            tables,
        })
    }
}

#[cfg(all(feature = "files", not(target_arch = "wasm32")))]
pub use files::{encounter_kinds, fielded};

mod effects;
#[cfg(all(feature = "files", not(target_arch = "wasm32")))]
mod files {
    use super::{Facts, Fielded, Fielding};
    use crate::assets::Runtime;
    use serde_json::Value;
    use std::collections::BTreeSet;
    use std::path::Path;

    fn read_json(path: &Path) -> Result<Value, String> {
        let text = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        serde_json::from_slice(&text).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// The unit types encounter `encounter` of saved map `map` fields, sorted
    /// and distinct.
    pub fn encounter_kinds(root: &Path, map: &str, encounter: &str) -> Result<Vec<String>, String> {
        let path = root
            .join("fixtures/maps")
            .join(map)
            .join("encounters")
            .join(format!("{encounter}.json"));
        let units = read_json(&path)?["units"].clone();
        let kinds: BTreeSet<String> = units
            .as_array()
            .ok_or(format!("{}: no units", path.display()))?
            .iter()
            .map(|u| u["kind"].as_str().map(str::to_owned))
            .collect::<Option<_>>()
            .ok_or(format!("{}: a unit has no kind", path.display()))?;
        Ok(kinds.into_iter().collect())
    }

    /// What a battle fielding `kinds` draws from: its supplied rules and
    /// resolved catalog, with art read from the runtime directory.
    pub fn fielded(
        root: &Path,
        rules: &contract::scenario::Rules,
        kinds: &[String],
    ) -> Result<Fielded, String> {
        let catalog = rules.catalog.view();
        let rules = serde_json::to_value(rules).map_err(|e| e.to_string())?;
        let runtime = Runtime::open(&root.join("assets/runtime"))?;
        let presentation = read_json(&root.join("fixtures/presentation.json"))?;
        let mut facts = Facts::default();
        Fielding {
            presentation: &presentation,
            rules: &rules,
            catalog: &catalog,
            runtime: &runtime.catalog,
            kinds,
            factions: None,
        }
        .resolve(&mut facts, &|hash| {
            runtime.read(hash, "bundle.bin").map(Into::into)
        })?
        .typed()
    }
}

/// The building parts' prop kinds a gutted building's become
/// (`destroyed.into.building.gutted_prop`), and those a collapse leaves
/// (`destroyed.into.prop`), sorted.
pub fn building_remains(view: &Value) -> (BTreeSet<String>, BTreeSet<String>) {
    let (mut remains, mut shells) = (BTreeSet::new(), BTreeSet::new());
    for t in view["props"]
        .as_object()
        .into_iter()
        .flat_map(|p| p.values())
    {
        let into = &t["destroyed"]["into"];
        if into["building"].is_object() {
            remains.extend(into["prop"].as_str().map(str::to_owned));
            shells.extend(into["building"]["gutted_prop"].as_str().map(str::to_owned));
        }
    }
    (remains, shells)
}

/// The resolved unit catalog (`Catalog::view`), as presentation reads it.
struct Catalog<'a> {
    view: &'a Value,
    types: HashMap<&'a str, &'a Value>,
}

impl<'a> Catalog<'a> {
    fn new(view: &'a Value) -> Result<Self, String> {
        let types = view["units"]
            .as_array()
            .ok_or("catalog: no units")?
            .iter()
            .filter_map(|t| Some((t["id"].as_str()?, t)))
            .collect();
        Ok(Self { view, types })
    }

    fn ids(&self) -> impl Iterator<Item = &'a str> + '_ {
        self.view["units"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|t| t["id"].as_str())
    }

    fn get(&self, kind: &str) -> Result<&'a Value, String> {
        self.types
            .get(kind)
            .copied()
            .ok_or(format!("no unit type {kind}"))
    }

    fn hull(&self, kind: &str) -> Result<Option<&'a Value>, String> {
        Ok(self.get(kind)?["body"].get("hull"))
    }

    fn slots(&self, kind: &str) -> Result<Vec<&'a str>, String> {
        Ok(self.get(kind)?["body"]["squad"]["slots"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect())
    }

    fn soldier(&self, kind: &str) -> Result<&'a Value, String> {
        let s = &self.view["soldiers"][kind];
        s.is_object()
            .then_some(s)
            .ok_or(format!("no soldier kind {kind}"))
    }

    /// Every appearance `kinds` wear: each hull's model, each slot's soldier
    /// set and every operator's kit (`appearancesFor`).
    fn worn(&self, kinds: &[String]) -> Result<BTreeSet<String>, String> {
        let mut worn = BTreeSet::new();
        let add_mounts = |mounts: &Value, worn: &mut BTreeSet<String>| {
            for m in mounts.as_array().into_iter().flatten() {
                let kit = &m["operator_appearance"];
                worn.extend(names(&kit["active"]).chain(names(&kit["carried"])));
            }
        };
        let mut soldiers = BTreeSet::new();
        for kind in kinds {
            let t = self.get(kind)?;
            worn.extend(t["appearance"].as_str().map(str::to_owned));
            add_mounts(&t["mounts"], &mut worn);
            soldiers.extend(self.slots(kind)?);
        }
        for s in soldiers {
            let row = self.soldier(s)?;
            worn.extend(names(&row["appearance"]));
            add_mounts(&row["mounts"], &mut worn);
        }
        Ok(worn)
    }
}

fn names(list: &Value) -> impl Iterator<Item = String> + '_ {
    list.as_array()
        .into_iter()
        .flatten()
        .filter_map(|n| n.as_str().map(str::to_owned))
}

/// Whether catalog type `t` flies (`airborne`).
pub(crate) fn airborne(t: &Value) -> bool {
    t["mobility"].get("air").is_some()
}

/// `t`'s mobility, as the catalog parses it.
fn mobility(t: &Value) -> Result<Mobility, String> {
    serde_json::from_value::<Mobility>(t["mobility"].clone())
        .map_err(|e| format!("a type's mobility: {e}"))
}

/// A mover's top speed, km/h: its road speed, or an aircraft's cruise.
fn top_speed_kmh(t: &Value) -> f64 {
    let m = &t["mobility"];
    let (way, field) = match () {
        _ if m.get("air").is_some() => ("air", "cruise_kmh"),
        _ if m.get("tracked").is_some() => ("tracked", "road_kmh"),
        _ if m.get("wheeled").is_some() => ("wheeled", "road_kmh"),
        _ => ("foot", "road_kmh"),
    };
    m[way][field].as_f64().unwrap_or(0.0)
}

/// The installed art: the runtime catalog's entries for what the battle
/// wears, each worn vehicle's wreck and each side's faction look, and how
/// their contents are read.
struct Art<'a> {
    runtime: &'a Value,
    installed: BTreeSet<String>,
}

impl<'a> Art<'a> {
    fn new(
        runtime: &'a Value,
        catalog: &Catalog,
        kinds: &[String],
        factions: Option<[&str; 2]>,
    ) -> Result<Self, String> {
        let entries = &runtime["appearances"];
        let worn = catalog.worn(kinds)?;
        let mut installed = worn.clone();
        for name in &worn {
            let e = &entries[name];
            installed.extend(e["wreck"].as_str().map(str::to_owned));
            for f in factions.into_iter().flatten() {
                installed.extend(e["factions"][f].as_str().map(str::to_owned));
            }
        }
        installed.retain(|n| entries[n].is_object());
        Ok(Self { runtime, installed })
    }

    fn has(&self, name: &str) -> bool {
        self.installed.contains(name)
    }

    fn entry(&self, name: &str) -> &Value {
        &self.runtime["appearances"][name]
    }

    /// The content appearance `name`'s bundle is.
    fn bundle(&self, name: &str) -> Result<&str, String> {
        self.entry(name)["bundle"]
            .as_str()
            .ok_or(format!("appearance {name}: no bundle"))
    }

    fn key(&self, name: &str) -> Result<(String, Option<String>), String> {
        let entry = self.entry(name);
        let bundle = self.bundle(name)?.to_owned();
        let skeleton = entry["skeleton"]
            .as_str()
            .map(|id| {
                self.runtime["skeletons"][id]
                    .as_str()
                    .ok_or(format!("skeleton {id} is not installed"))
                    .map(str::to_owned)
            })
            .transpose()?;
        Ok((bundle, skeleton))
    }
}

fn appearance_facts(
    facts: &mut Facts,
    art: &Art,
    bytes: &HashMap<String, Cow<'_, [u8]>>,
    name: &str,
) -> Result<AppearanceFacts, String> {
    let key = art.key(name)?;
    if let Some(facts) = facts.appearances.get(&key) {
        return Ok(facts.clone());
    }
    let bundle_bytes = bytes
        .get(&key.0)
        .ok_or(format!("content {} was not read", key.0))?;
    let what = format!("bundle {name}");
    let bundle = Container::<Value>::parse(bundle_bytes, BUNDLE, &what)?;
    let fail = |e: String| format!("appearance {name}: {e}");
    let (record, table) = match bundle.content["kind"].as_str() {
        Some("skinned") => {
            let skeleton_hash = key.1.as_ref().ok_or("no skeleton")?;
            let skeleton_bytes = bytes
                .get(skeleton_hash)
                .ok_or(format!("content {skeleton_hash} was not read"))?;
            let skeleton = Container::<Value>::parse(skeleton_bytes, BUNDLE, skeleton_hash)?;
            skinned(&bundle, &skeleton).map_err(fail)?
        }
        Some("articulated") => (articulated(&bundle).map_err(fail)?, Vec::new()),
        other => return Err(fail(format!("draws a unit but is an {other:?} bundle"))),
    };
    let derived = AppearanceFacts {
        record,
        table: (!table.is_empty()).then_some(table),
    };
    facts.appearances.insert(key, derived.clone());
    Ok(derived)
}

/// Running-gear gauge belongs to every ground appearance, including faction looks.
fn track_gauge(art: &Art, name: &str) -> Result<f64, String> {
    art.entry(name)["track_gauge"]
        .as_f64()
        .filter(|v| v.is_finite() && *v > 0.0)
        .ok_or(format!(
            "appearance {name}: track_gauge must be finite and positive"
        ))
}

/// Every catalog type's facts and the appearance sets fielded types draw from
/// per side, and every appearance those sets name.
fn unit_types(
    catalog: &Catalog,
    art: &Art,
    kinds: &[String],
    factions: Option<[&str; 2]>,
) -> Result<(Map<String, Value>, BTreeSet<String>), String> {
    let look = |side: usize, list: &mut dyn Iterator<Item = String>| -> Vec<String> {
        list.filter(|n| art.has(n))
            .map(|n| {
                let faction = factions.and_then(|f| art.entry(&n)["factions"][f[side]].as_str());
                match faction {
                    Some(look) if art.has(look) => look.to_owned(),
                    _ => n,
                }
            })
            .collect()
    };
    let mut types = Map::new();
    let mut drawn = BTreeSet::new();
    let fielded: BTreeSet<&str> = kinds.iter().map(String::as_str).collect();
    for kind in catalog.ids().collect::<BTreeSet<_>>() {
        let t = catalog.get(kind)?;
        let has_art = fielded.contains(kind);
        let hull = catalog.hull(kind)?;
        let ground = matches!(
            mobility(t)?,
            Mobility::Tracked { .. } | Mobility::Wheeled { .. }
        );
        // Its vehicle class, as the catalog derives it from its physics.
        let class = t["vehicle_class"].as_str().map(str::to_owned);
        let mounts: &[Value] = t["mounts"].as_array().map_or(&[], Vec::as_slice);
        let installed = t["appearance"].as_str().filter(|a| has_art && art.has(a));
        let rigs = installed.map(|a| &art.entry(a)["mounts"]);
        let mut draws = Map::new();
        for (side, name) in ["blue", "red"].into_iter().enumerate() {
            let hull_look = match (has_art, hull) {
                (false, _) => vec![],
                (true, Some(_)) => look(
                    side,
                    &mut names(&json!([t["appearance"].as_str().unwrap_or("")])),
                ),
                (true, None) => vec![],
            };
            let half_track = if has_art && ground {
                match hull_look.first() {
                    Some(name) => {
                        hull.unwrap()["half_extents_m"][1].as_f64().unwrap()
                            * track_gauge(art, name)?
                    }
                    None => 0.0,
                }
            } else {
                0.0
            };
            let slots = if has_art {
                catalog
                    .slots(kind)?
                    .iter()
                    .map(|s| Ok(look(side, &mut names(&catalog.soldier(s)?["appearance"]))))
                    .collect::<Result<Vec<_>, String>>()?
            } else {
                Vec::new()
            };
            let operated: Vec<Value> = if has_art {
                mounts
                    .iter()
                    .map(|m| {
                        let kit = &m["operator_appearance"];
                        if kit.is_object() {
                            json!({
                                "active": look(side, &mut names(&kit["active"])),
                                "carried": look(side, &mut names(&kit["carried"])),
                            })
                        } else {
                            Value::Null
                        }
                    })
                    .collect()
            } else {
                vec![Value::Null; mounts.len()]
            };
            drawn.extend(hull_look.iter().cloned());
            drawn.extend(slots.iter().flatten().cloned());
            for o in &operated {
                drawn.extend(names(&o["active"]).chain(names(&o["carried"])));
            }
            draws.insert(
                name.into(),
                json!({ "hull": hull_look, "halfTrack": half_track, "slots": slots, "operated": operated }),
            );
        }
        let roles: Vec<&str> = mounts
            .iter()
            .map(|m| {
                if hull.is_none() {
                    return "hand";
                }
                let id = m["id"].as_str().unwrap_or("");
                match rigs.and_then(|r| r[id].as_str()).filter(|r| !r.is_empty()) {
                    Some(rig) => rig,
                    None if !m["turret"].as_bool().unwrap_or(false) && m["on"].is_null() => "hull",
                    None => "hand",
                }
            })
            .collect();
        let mount_rows: Vec<Value> = mounts
            .iter()
            .map(|m| {
                let on = mounts
                    .iter()
                    .position(|r| !m["on"].is_null() && r["id"] == m["on"]);
                let pose = &m["operator_appearance"]["active_pose"];
                json!({
                    "weapons": m["weapons"],
                    "on": on,
                    "activePose": if pose.is_object() { pose.clone() } else { Value::Null },
                })
            })
            .collect();
        types.insert(
            kind.to_owned(),
            json!({
                "hull": hull.is_some(),
                "vehicleClass": class,
                "airborne": airborne(t),
                "layer": mobility(t)?.layer(),
                "flight": mobility(t)?.flight(),
                "turnRadS": match mobility(t)? {
                    Mobility::Air { turn_deg_s, .. } => Some(turn_deg_s.to_radians()),
                    _ => None,
                },
                "cruiseMps": match mobility(t)? {
                    Mobility::Air { cruise_kmh, .. } => Some(cruise_kmh / 3.6),
                    _ => None,
                },
                "roles": roles,
                "mounts": mount_rows,
                "draws": draws,
            }),
        );
    }
    Ok((types, drawn))
}

/// The `units` section and its clip tables: each fielded type's facts and
/// the appearance sets it draws from per side, and every one of those
/// appearances' joints and clips, or rig.
fn units(
    presentation: &PresentationSettings,
    rules: &Value,
    catalog: &Catalog,
    art: &Art,
    bundle_facts: &mut BundleFacts,
    kinds: &[String],
    factions: Option<[&str; 2]>,
) -> Result<(Value, ClipTables), String> {
    let (types, drawn) = unit_types(catalog, art, kinds, factions)?;
    let mut body_half = Map::new();
    for kind in catalog.ids() {
        let half = catalog.hull(kind)?.map(|h| {
            json!([
                h["half_extents_m"][0],
                h["half_extents_m"][1],
                h["half_extents_m"][2]
            ])
        });
        body_half.insert(kind.to_owned(), half.unwrap_or(Value::Null));
    }
    let mut appearances = Map::new();
    let mut tables = ClipTables::new();
    let mut table_of: Vec<(Vec<f32>, String)> = Vec::new();
    for name in &drawn {
        let appearance = appearance_facts(bundle_facts.facts, art, bundle_facts.bytes, name)?;
        let mut record = appearance.record;
        if let Some(table) = appearance.table {
            let table_name = match table_of.iter().find(|(t, _)| *t == table) {
                Some((_, n)) => n.clone(),
                None => {
                    let n = format!("clips-{}", table_of.len());
                    tables.insert(n.clone(), table.clone());
                    table_of.push((table, n.clone()));
                    n
                }
            };
            record["table"] = json!(table_name);
        }
        appearances.insert(name.clone(), record);
    }
    let sides = &art.runtime["sides"];
    let units = json!({
        "pose": presentation.pose,
        "transit": presentation.transit,
        "leanHold": number(rules, "cover", "lean_hold_s")?,
        "detail": presentation.models,
        "tints": { "blue": sides["blue"], "red": sides["red"] },
        "bodyHalf": body_half,
        "types": types,
        "appearances": appearances,
    });
    Ok((units, tables))
}

/// Each hulled unit type's wreck, where its installed model (its
/// `appearance`, as `layout.unitAppearance` names it) has an installed one,
/// by type.
fn hull_wrecks(catalog: &Catalog, art: &Art) -> BTreeMap<String, String> {
    catalog
        .ids()
        .filter_map(|unit| {
            let t = catalog.types.get(unit)?;
            t["body"].get("hull")?;
            let model = t["appearance"].as_str().filter(|m| art.has(m))?;
            let wreck = art.entry(model)["wreck"].as_str().filter(|w| art.has(w))?;
            Some((unit.to_owned(), wreck.to_owned()))
        })
        .collect()
}

/// The number at `section`.`key` of `rules` (`section` dotted).
fn number(rules: &Value, section: &str, key: &str) -> Result<f64, String> {
    section.split('.').fold(rules, |v, k| &v[k])[key]
        .as_f64()
        .ok_or(format!("rules: no {section}.{key}"))
}
