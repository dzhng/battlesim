//! One admission boundary for acquired physical maps; no IO or world construction.
use crate::identity::GenerationIdentity;
use crate::map::MapDefinition;
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

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MapSource {
    Catalogue { id: MapId },
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

/// Selection from the shared physical library, never a copied template file.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogueSelection {
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

#[derive(Debug, Serialize)]
pub struct ResolvedMap {
    pub definition: MapDefinition,
    pub identity: MapIdentity,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolveCode {
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

pub fn resolve(
    map_json: &str,
    sources_json: &str,
    library_json: &str,
    admission: MapAdmission,
) -> Result<ResolvedMap, ResolveError> {
    let definition: MapDefinition =
        serde_json::from_str(map_json).map_err(|e: serde_json::Error| ResolveError {
            code: ResolveCode::InvalidMap,
            location: "map.json".into(),
            message: e.to_string(),
        })?;
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
    let sources: MapSources =
        serde_json::from_str(sources_json).map_err(|e: serde_json::Error| ResolveError {
            code: ResolveCode::InvalidSources,
            location: "SOURCES.json".into(),
            message: e.to_string(),
        })?;
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
    let parts = definition
        .buildings
        .iter()
        .try_fold(definition.props.len(), |n, b| {
            n.checked_add(b.parts.len().max(b.geometry.parts.len()))
        });
    if parts.is_none_or(|count| count as u64 > u64::from(admission.max_authored_parts)) {
        return Err(ResolveError {
            code: ResolveCode::ComplexityLimit,
            location: "map.json.authored_parts".into(),
            message: "authored physical parts exceed acquisition admission".into(),
        });
    }
    let map_hash = crate::identity::json_hash(&definition).map_err(|e| ResolveError {
        code: ResolveCode::InvalidMap,
        location: "map.json".into(),
        message: e.to_string(),
    })?;
    if map_hash != sources.identity.map_hash() {
        return Err(ResolveError {
            code: ResolveCode::IdentityMismatch,
            location: "SOURCES.json.identity.map_hash".into(),
            message: "saved content hash does not match the physical map".into(),
        });
    }
    let catalogue =
        TemplateGeometryCatalog::from_json(library_json).map_err(|message| ResolveError {
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
        || (!definition.buildings.is_empty() && definition.template_catalog_hash.is_none())
    {
        return Err(ResolveError {
            code: ResolveCode::CatalogueMismatch,
            location: "map.json.template_catalog_hash".into(),
            message: "physical map does not name the selected catalogue's content hash".into(),
        });
    }
    let mut bays = 0u64;
    for (i, building) in definition.buildings.iter().enumerate() {
        let template = physical_template(&catalogue, building, i)?;
        let count = template
            .bay_position_count()
            .map_err(|message| ResolveError {
                code: ResolveCode::TemplateMismatch,
                location: format!("map.json.buildings[{i}].geometry"),
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
    }
    definition
        .authored_props()
        .map_err(|message| ResolveError {
            code: ResolveCode::InvalidAuthoredIds,
            location: "map.json.authored_parts".into(),
            message,
        })?;
    for (i, building) in definition.buildings.iter().enumerate() {
        let location = format!("map.json.buildings[{i}].geometry");
        let template = physical_template(&catalogue, building, i)?;
        let expected = template
            .materialize(building.geometry.frame)
            .map_err(|message| ResolveError {
                code: ResolveCode::TemplateMismatch,
                location: location.clone(),
                message,
            })?;
        if expected != building.geometry
            || template.category != building.category
            || template.regional_family != building.regional_family
        {
            return Err(ResolveError {
                code: ResolveCode::TemplateMismatch,
                location,
                message: "saved physical facts disagree with their materialized template".into(),
            });
        }
    }
    Ok(ResolvedMap {
        definition,
        identity: sources.identity,
    })
}

fn physical_template<'a>(
    catalogue: &'a TemplateGeometryCatalog,
    building: &crate::map::BuildingDefinition,
    index: usize,
) -> Result<&'a crate::templates::BuildingTemplateDescriptor, ResolveError> {
    catalogue
        .templates()
        .iter()
        .find(|t| t.id == building.geometry.template_id)
        .ok_or_else(|| ResolveError {
            code: ResolveCode::TemplateMismatch,
            location: format!("map.json.buildings[{index}].geometry"),
            message: "building template is absent from its physical catalogue".into(),
        })
}

fn receipt_error(index: usize, field: &str, message: &str) -> ResolveError {
    ResolveError {
        code: ResolveCode::InvalidSources,
        location: format!("SOURCES.json.inputs[{index}].{field}"),
        message: message.into(),
    }
}
