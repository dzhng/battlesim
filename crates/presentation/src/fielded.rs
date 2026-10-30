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

/// A runtime content's bytes, inflated, by its hash.
pub type Read<'a> = dyn Fn(&str) -> Result<Vec<u8>, String> + 'a;

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

impl Fielding<'_> {
    /// The runtime contents [`Fielding::resolve`] reads, by hash, sorted:
    /// each drawn appearance's bundle, each fielded hull's wreck and every
    /// skeleton's clips.
    pub fn contents(&self) -> Result<Vec<String>, String> {
        let catalog = Catalog::new(self.catalog)?;
        let art = Art::new(self.runtime, &catalog, self.kinds, self.factions)?;
        let (_, drawn) = unit_types(&catalog, &art, self.kinds, self.factions)?;
        let mut contents = BTreeSet::new();
        for name in drawn.iter().chain(hull_wrecks(&catalog, &art).values()) {
            contents.insert(art.bundle(name)?.to_owned());
        }
        for hash in self.runtime["skeletons"].as_object().into_iter().flatten() {
            contents.extend(hash.1.as_str().map(str::to_owned));
        }
        Ok(contents.into_iter().collect())
    }

    /// The battle's sections, each content [`Fielding::contents`] names
    /// read by `read`.
    pub fn resolve(&self, read: &Read<'_>) -> Result<Sections, String> {
        let presentation = PresentationSettings::from_value(self.presentation)?;
        let catalog = Catalog::new(self.catalog)?;
        let looks = crate::looks::WeaponLooks::new(
            presentation.weapon_looks.clone(),
            &self.rules["weapons"],
        )?;
        for row in self.catalog["units"]
            .as_array()
            .into_iter()
            .flatten()
            .chain(
                self.catalog["soldiers"]
                    .as_object()
                    .into_iter()
                    .flat_map(|s| s.values()),
            )
        {
            for mount in row["mounts"].as_array().into_iter().flatten() {
                for weapon in mount["weapons"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                {
                    looks.get(weapon).check_firing(weapon)?;
                }
            }
        }
        let art = Art {
            read,
            ..Art::new(self.runtime, &catalog, self.kinds, self.factions)?
        };
        let (mut units, tables) = units(
            &presentation,
            self.rules,
            &catalog,
            &art,
            self.kinds,
            self.factions,
        )?;
        for unit in units["types"].as_object_mut().unwrap().values_mut() {
            for mount in unit["mounts"].as_array_mut().unwrap() {
                let weapon = mount["weapon"].as_str().ok_or("mount without a weapon")?;
                mount["kick"] =
                    serde_json::to_value(looks.get(weapon).kick.expect("fielded firing kick"))
                        .map_err(|e| e.to_string())?;
            }
        }
        let effects = effects::section(
            &presentation,
            &looks,
            self.rules,
            &catalog,
            &art,
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
    use super::{Fielded, Fielding};
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
        Fielding {
            presentation: &presentation,
            rules: &rules,
            catalog: &catalog,
            runtime: &runtime.catalog,
            kinds,
            factions: None,
        }
        .resolve(&|hash| runtime.read(hash, "bundle.bin"))?
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

/// Whether `t` flies (`airborne`).
fn airborne(t: &Value) -> bool {
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
    read: &'a Read<'a>,
    installed: BTreeSet<String>,
}

/// A reader of no content: art whose bytes are not asked for.
fn unread(hash: &str) -> Result<Vec<u8>, String> {
    Err(format!("content {hash} was not read"))
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
        Ok(Self {
            runtime,
            read: &unread,
            installed,
        })
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

    /// Content `hash`, inflated.
    fn bytes(&self, hash: &str) -> Result<Vec<u8>, String> {
        (self.read)(hash)
    }
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

/// Each fielded type's facts and the appearance sets it draws from per
/// side, and every appearance those sets name.
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
    for kind in kinds.iter().collect::<BTreeSet<_>>() {
        let t = catalog.get(kind)?;
        let hull = catalog.hull(kind)?;
        let ground = matches!(
            mobility(t)?,
            Mobility::Tracked { .. } | Mobility::Wheeled { .. }
        );
        // Its vehicle class, as the catalog derives it from its physics.
        let class = t["vehicle_class"].as_str().map(str::to_owned);
        let mounts: &[Value] = t["mounts"].as_array().map_or(&[], Vec::as_slice);
        let installed = t["appearance"].as_str().filter(|a| art.has(a));
        let rigs = installed.map(|a| &art.entry(a)["mounts"]);
        let mut draws = Map::new();
        for (side, name) in ["blue", "red"].into_iter().enumerate() {
            let hull_look = match hull {
                Some(_) => look(
                    side,
                    &mut names(&json!([t["appearance"].as_str().unwrap_or("")])),
                ),
                None => vec![],
            };
            let half_track = if ground {
                let name = hull_look
                    .first()
                    .ok_or(format!("{kind}: no hull appearance"))?;
                hull.unwrap()["half_extents_m"][1].as_f64().unwrap() * track_gauge(art, name)?
            } else {
                0.0
            };
            let slots = catalog
                .slots(kind)?
                .iter()
                .map(|s| Ok(look(side, &mut names(&catalog.soldier(s)?["appearance"]))))
                .collect::<Result<Vec<_>, String>>()?;
            let operated: Vec<Value> = mounts
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
                .collect();
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
                    "weapon": m["weapons"][0],
                    "on": on,
                    "activePose": if pose.is_object() { pose.clone() } else { Value::Null },
                })
            })
            .collect();
        types.insert(
            kind.clone(),
            json!({
                "hull": hull.is_some(),
                "vehicleClass": class,
                "airborne": airborne(t),
                "layer": mobility(t)?.layer(),
                "flight": mobility(t)?.flight(),
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
    kinds: &[String],
    factions: Option<[&str; 2]>,
) -> Result<(Value, ClipTables), String> {
    let (types, drawn) = unit_types(catalog, art, kinds, factions)?;
    let mut appearances = Map::new();
    let mut tables = ClipTables::new();
    let mut table_of: Vec<(Vec<f32>, String)> = Vec::new();
    for name in &drawn {
        let bytes = art.bytes(art.bundle(name)?)?;
        let what = format!("bundle {name}");
        let bundle = Container::<Value>::parse(&bytes, BUNDLE, &what)?;
        let fail = |e: String| format!("appearance {name}: {e}");
        let record = match bundle.content["kind"].as_str() {
            Some("skinned") => {
                let skeleton_id = bundle.content["skeleton"].as_str().ok_or("no skeleton")?;
                let hash = art.runtime["skeletons"][skeleton_id]
                    .as_str()
                    .ok_or(format!("skeleton {skeleton_id} is not installed"))?;
                let skeleton_bytes = art.bytes(hash)?;
                let skeleton = Container::<Value>::parse(&skeleton_bytes, BUNDLE, skeleton_id)?;
                let (mut record, table) = skinned(&bundle, &skeleton).map_err(fail)?;
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
                record
            }
            Some("articulated") => articulated(&bundle).map_err(fail)?,
            other => return Err(fail(format!("draws a unit but is a {other:?} bundle"))),
        };
        appearances.insert(name.clone(), record);
    }
    let sides = &art.runtime["sides"];
    let units = json!({
        "pose": presentation.pose,
        "transit": presentation.transit,
        "leanHold": number(rules, "cover", "lean_hold_s")?,
        "detail": presentation.models,
        "tints": { "blue": sides["blue"], "red": sides["red"] },
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
