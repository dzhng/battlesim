//! One admission boundary for acquired physical maps; no IO or world construction.
use crate::generation::GenerationRequest;
use crate::identity::GenerationIdentity;
use crate::map::{BuildingDefinition, MapDefinition, SavedBuilding, SavedMap};
use crate::templates::TemplateGeometryCatalog;
use serde::{de::Error, Deserialize, Deserializer, Serialize};

/// Directory identity, independent of route names and physical content identity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct MapId(String);

impl MapId {
    pub fn new(value: &str) -> Result<Self, &'static str> {
        let alphanumeric = |b: u8| b.is_ascii_lowercase() || b.is_ascii_digit();
        if !value.as_bytes().first().copied().is_some_and(alphanumeric)
            || !value
                .bytes()
                .all(|b| alphanumeric(b) || b == b'-' || b == b'_')
        {
            return Err("catalogue id must be a lowercase ASCII directory name, starting with a letter or digit");
        }
        Ok(Self(value.into()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for MapId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        Self::new(&value).map_err(D::Error::custom)
    }
}

/// What a saved map is for, as its `meta.json` `category` says. Players
/// never fight on a saved map (their battles are generated), so a saved map
/// is a test's or the menu's own; any other value is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MapCategory {
    /// Ground a test, a lab or a benchmark runs on.
    Test,
    /// A battlefield the main menu's backdrop films.
    Menu,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MapSource {
    /// A saved map of the catalogue, `fixtures/maps/<id>/`.
    Catalogue { id: MapId },
    /// A map the generator makes from `request`. It needs no catalogue
    /// folder: the request, with the build that reads it, is the map.
    Generated { request: GenerationRequest },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MapIdentity {
    Authored {
        map_hash: String,
        template_catalog_hash: Option<String>,
    },
    Generated {
        generation: GenerationIdentity,
    },
}

impl MapIdentity {
    pub fn map_hash(&self) -> &str {
        match self {
            Self::Authored { map_hash, .. } => map_hash,
            Self::Generated { generation } => &generation.map_hash,
        }
    }

    pub fn template_catalog_hash(&self) -> Option<&str> {
        match self {
            Self::Authored {
                template_catalog_hash,
                ..
            } => template_catalog_hash.as_deref(),
            Self::Generated { generation } => Some(&generation.template_catalog_hash),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SourceReceipt {
    Repository {
        path: String,
        revision: String,
        sha256: String,
    },
    Supplied {
        label: String,
        sha256: String,
    },
}

/// The physical catalogue a map's buildings are materialized from: a library
/// and a selection of its templates, never a copied template file.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogueSelection {
    /// The library's file name, beside the catalogue's `maps/` folder
    /// (`fixtures/<library>`). It is only where an adapter finds the library:
    /// what admits the library is the catalogue hash the map names.
    pub library: String,
    /// The templates of the library that are the map's catalogue; `None` for
    /// the whole library.
    pub template_ids: Option<Vec<String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapSources {
    pub identity: MapIdentity,
    pub catalogue: CatalogueSelection,
    pub inputs: Vec<SourceReceipt>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapAdmission {
    pub max_authored_parts: u32,
    pub max_bay_positions: u64,
}

impl MapAdmission {
    /// What the saved catalogue's adapters admit (`fixtures/maps/<id>/`):
    /// any map within the released resource envelope. Editable generation
    /// allowances must fit this envelope, and lowering them never lowers
    /// admission of a released saved map. A larger saved map is refused.
    pub const CATALOGUE: Self = Self {
        max_authored_parts: 250_000,
        max_bay_positions: 600_000,
    };
}

#[derive(Debug, Serialize)]
pub struct ResolvedMap {
    pub definition: MapDefinition,
    pub identity: MapIdentity,
}

/// What a resolution answers across a JSON boundary: the resolved map, or the
/// refusal with its code and location.
#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ResolveOutcome {
    Ok { result: Box<ResolvedMap> },
    Error { error: ResolveError },
}

impl From<Result<ResolvedMap, ResolveError>> for ResolveOutcome {
    fn from(result: Result<ResolvedMap, ResolveError>) -> Self {
        match result {
            Ok(result) => Self::Ok {
                result: Box::new(result),
            },
            Err(error) => Self::Error { error },
        }
    }
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolveCode {
    /// An adapter was asked for an address that is not a catalogue id.
    InvalidId,
    /// An adapter found no document where the id says it is.
    MissingDocument,
    /// A saved encounter is not an encounter definition.
    InvalidEncounter,
    /// A saved map's listing (`meta.json`) does not say what it is for.
    InvalidListing,
    InvalidMap,
    InvalidSources,
    IdentityMismatch,
    InvalidCatalogue,
    TemplateMismatch,
    CatalogueMismatch,
    ComplexityLimit,
    InvalidAuthoredIds,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct ResolveError {
    pub code: ResolveCode,
    pub location: String,
    pub message: String,
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.location, self.message)
    }
}
impl std::error::Error for ResolveError {}

impl MapSources {
    /// `SOURCES.json`, parsed and checked on its own. An adapter reads it
    /// first, to learn which library to fetch (`catalogue.library`).
    pub fn from_json(sources_json: &str) -> Result<Self, ResolveError> {
        let sources: MapSources =
            serde_json::from_str(sources_json).map_err(|e: serde_json::Error| ResolveError {
                code: ResolveCode::InvalidSources,
                location: "SOURCES.json".into(),
                message: e.to_string(),
            })?;
        let library = &sources.catalogue.library;
        if library
            .strip_suffix(".json")
            .is_none_or(|stem| MapId::new(stem).is_err())
        {
            return Err(ResolveError {
                code: ResolveCode::InvalidSources,
                location: "SOURCES.json.catalogue.library".into(),
                message: "the physical library is one lowercase ASCII file name ending in .json, never a path".into(),
            });
        }
        if let MapIdentity::Generated { generation } = &sources.identity {
            generation.validate().map_err(|error| ResolveError {
                code: ResolveCode::InvalidSources,
                location: format!("SOURCES.json.identity.generation.{}", error.field),
                message: error.message.into(),
            })?;
        }
        if sources.inputs.is_empty() {
            return Err(ResolveError {
                code: ResolveCode::InvalidSources,
                location: "SOURCES.json.inputs".into(),
                message: "map provenance requires input receipts".into(),
            });
        }
        for (i, input) in sources.inputs.iter().enumerate() {
            let sha256 = match input {
                SourceReceipt::Repository {
                    path,
                    revision,
                    sha256,
                } => {
                    let relative = !path.contains(['\\', ':'])
                        && !path.chars().any(char::is_control)
                        && path
                            .split('/')
                            .all(|p| !p.is_empty() && p != "." && p != "..");
                    if !relative {
                        return Err(receipt_error(
                            i,
                            "path",
                            "input path must be normalized and repository-relative",
                        ));
                    }
                    if revision.trim().is_empty() {
                        return Err(receipt_error(
                            i,
                            "revision",
                            "input revision must be nonempty",
                        ));
                    }
                    sha256
                }
                SourceReceipt::Supplied { label, sha256 } => {
                    if label.trim().is_empty() {
                        return Err(receipt_error(
                            i,
                            "label",
                            "supplied input label must be nonempty",
                        ));
                    }
                    sha256
                }
            };
            if !crate::identity::is_sha256(sha256) {
                return Err(receipt_error(
                    i,
                    "sha256",
                    "input content hash must be canonical SHA256",
                ));
            }
        }
        Ok(sources)
    }
}

/// Admit a saved map: `map.json` (a [`SavedMap`]), the `SOURCES.json` that
/// pins it, and the physical library those sources name. Each building is
/// materialized here from its template at its frame, so the definition every
/// consumer receives holds no geometry a template did not make. The map's
/// content hash is the hash of that resolved definition.
pub fn resolve(
    map_json: &str,
    sources_json: &str,
    library_json: &str,
    admission: MapAdmission,
) -> Result<ResolvedMap, ResolveError> {
    let saved: SavedMap =
        serde_json::from_str(map_json).map_err(|e: serde_json::Error| ResolveError {
            code: ResolveCode::InvalidMap,
            location: "map.json".into(),
            message: e.to_string(),
        })?;
    let (mut definition, buildings) = saved.with_buildings(Vec::new());
    if let Some(error) = crate::map::validate_header(
        definition.size,
        definition.fog_cell_m,
        definition.height_grid_m,
        definition.slope_cutoff_deg,
    )
    .first()
    {
        return Err(ResolveError {
            code: ResolveCode::InvalidMap,
            location: format!("map.json.{}", error.field),
            message: error.message.into(),
        });
    }
    crate::river::validate(&definition).map_err(|message| ResolveError {
        code: ResolveCode::InvalidMap,
        location: "map.json".into(),
        message,
    })?;
    let sources = MapSources::from_json(sources_json)?;
    let catalogue = physical_library(library_json).map_err(|message| ResolveError {
        code: ResolveCode::InvalidCatalogue,
        location: "physical catalogue".into(),
        message,
    })?;
    let catalogue = match &sources.catalogue.template_ids {
        None => catalogue,
        Some(ids) => {
            let templates = ids
                .iter()
                .map(|id| {
                    catalogue
                        .templates()
                        .iter()
                        .find(|t| &t.id == id)
                        .cloned()
                        .ok_or_else(|| ResolveError {
                            code: ResolveCode::InvalidCatalogue,
                            location: "SOURCES.json.catalogue.template_ids".into(),
                            message: format!(
                                "selected template {id:?} is absent from the physical library"
                            ),
                        })
                })
                .collect::<Result<Vec<_>, _>>()?;
            TemplateGeometryCatalog::new(templates).map_err(|message| ResolveError {
                code: ResolveCode::InvalidCatalogue,
                location: "SOURCES.json.catalogue.template_ids".into(),
                message,
            })?
        }
    };
    if definition.template_catalog_hash.as_deref() != sources.identity.template_catalog_hash() {
        return Err(ResolveError {
            code: ResolveCode::IdentityMismatch,
            location: "SOURCES.json.identity.template_catalog_hash".into(),
            message: "source identity and physical map name different catalogues".into(),
        });
    }
    if definition
        .template_catalog_hash
        .as_deref()
        .is_some_and(|hash| hash != catalogue.hash())
        || (!buildings.is_empty() && definition.template_catalog_hash.is_none())
    {
        return Err(ResolveError {
            code: ResolveCode::CatalogueMismatch,
            location: "map.json.template_catalog_hash".into(),
            message: "physical map does not name the selected catalogue's content hash".into(),
        });
    }
    // Both allowances are counted from the templates, before any building is
    // materialized.
    let mut templates = Vec::with_capacity(buildings.len());
    let mut parts = definition.props.len() as u64;
    let mut bays = 0u64;
    for (i, building) in buildings.iter().enumerate() {
        let template = catalogue
            .templates()
            .iter()
            .find(|t| t.id == building.template_id)
            .ok_or_else(|| ResolveError {
                code: ResolveCode::TemplateMismatch,
                location: format!("map.json.buildings[{i}].template_id"),
                message: format!(
                    "building {i} (owner {}) names template {:?}, which the map's physical catalogue does not hold",
                    building.owner, building.template_id
                ),
            })?;
        parts += building.parts.len().max(template.parts.len()) as u64;
        if parts > u64::from(admission.max_authored_parts) {
            return Err(ResolveError {
                code: ResolveCode::ComplexityLimit,
                location: "map.json.authored_parts".into(),
                message: "authored physical parts exceed acquisition admission".into(),
            });
        }
        let count = template
            .bay_position_count()
            .map_err(|message| ResolveError {
                code: ResolveCode::TemplateMismatch,
                location: format!("map.json.buildings[{i}].template_id"),
                message,
            })?;
        bays = bays
            .checked_add(count as u64)
            .filter(|n| *n <= admission.max_bay_positions)
            .ok_or_else(|| ResolveError {
                code: ResolveCode::ComplexityLimit,
                location: "map.json.bay_positions".into(),
                message: "cumulative bay positions exceed acquisition admission".into(),
            })?;
        templates.push(template);
    }
    if parts > u64::from(admission.max_authored_parts) {
        return Err(ResolveError {
            code: ResolveCode::ComplexityLimit,
            location: "map.json.authored_parts".into(),
            message: "authored physical parts exceed acquisition admission".into(),
        });
    }
    definition.buildings.reserve_exact(buildings.len());
    for (i, (building, template)) in buildings.into_iter().zip(templates).enumerate() {
        let SavedBuilding {
            owner,
            kind,
            frame,
            parts,
            ..
        } = building;
        definition.buildings.push(
            BuildingDefinition::materialize(template, frame, kind, owner, parts).map_err(
                |message| ResolveError {
                    code: ResolveCode::TemplateMismatch,
                    location: format!("map.json.buildings[{i}].frame"),
                    message,
                },
            )?,
        );
    }
    definition
        .authored_props()
        .map_err(|message| ResolveError {
            code: ResolveCode::InvalidAuthoredIds,
            location: "map.json.authored_parts".into(),
            message,
        })?;
    let map_hash = crate::identity::json_hash(&definition).map_err(|e| ResolveError {
        code: ResolveCode::InvalidMap,
        location: "map.json".into(),
        message: e.to_string(),
    })?;
    if map_hash != sources.identity.map_hash() {
        return Err(ResolveError {
            code: ResolveCode::IdentityMismatch,
            location: "SOURCES.json.identity.map_hash".into(),
            message: format!(
                "saved content hash does not match the resolved physical map, whose content hash is {map_hash}"
            ),
        });
    }
    Ok(ResolvedMap {
        definition,
        identity: sources.identity,
    })
}

/// A physical library as its file holds it: a canonical catalogue (`{ hash,
/// templates }`, the hash checked against the geometry) or the list of
/// descriptors the map generator reads.
fn physical_library(library_json: &str) -> Result<TemplateGeometryCatalog, String> {
    if library_json.trim_start().starts_with('[') {
        TemplateGeometryCatalog::new(serde_json::from_str(library_json).map_err(|e| e.to_string())?)
    } else {
        TemplateGeometryCatalog::from_json(library_json)
    }
}

fn receipt_error(index: usize, field: &str, message: &str) -> ResolveError {
    ResolveError {
        code: ResolveCode::InvalidSources,
        location: format!("SOURCES.json.inputs[{index}].{field}"),
        message: message.into(),
    }
}
