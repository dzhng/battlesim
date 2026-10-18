//! Portable, self-contained scenario downloads.
//!
//! The bundle is a strict JSON envelope around the already-resolved
//! `ScenarioDefinition`. It is deliberately the only file format at the
//! download/upload boundary: map geometry and rules remain owned by their
//! existing contract types, while this module owns package identity, player
//! admission and embedded user bytes.
use crate::identity::{bytes_hash, is_sha256, json_hash};
use crate::ids::Side;
use crate::scenario::ScenarioDefinition;
use crate::script::CompiledScript;
use serde::{Deserialize, Serialize};

pub const FORMAT_VERSION: &str = "scenario-bundle-2";
pub const MAX_BUNDLE_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_ASSET_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_SCRIPT_SOURCE_BYTES: usize = 256 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BundleMode {
    Sandbox,
    Economy,
}

pub use crate::players::ControllerKind;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BundleMapIdentity {
    pub map_hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_catalog_hash: Option<String>,
}

pub use crate::scenario::AuthoredScript;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VictoryDefinition {
    pub source: String,
}

/// An embedded user file or a reference to immutable built-in content.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BundleAsset {
    pub id: String,
    pub media_type: String,
    pub sha256: String,
    pub provenance: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bytes_base64: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub builtin: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioBundle {
    pub format_version: String,
    pub runtime_build: String,
    pub map: BundleMapIdentity,
    pub rules_hash: String,
    pub scenario: ScenarioDefinition,
    pub mode: BundleMode,
    pub victory: VictoryDefinition,
    #[serde(default)]
    pub authored_scripts: Vec<AuthoredScript>,
    #[serde(default)]
    pub assets: Vec<BundleAsset>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BundleDiagnostic {
    pub code: &'static str,
    pub location: String,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BundleIdentity {
    pub sha256: String,
    pub map_hash: String,
    pub rules_hash: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BundleError {
    Invalid(Vec<BundleDiagnostic>),
}

impl std::fmt::Display for BundleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(errors) => f.write_str(
                &errors
                    .iter()
                    .map(|e| e.message.as_str())
                    .collect::<Vec<_>>()
                    .join("; "),
            ),
        }
    }
}

impl std::error::Error for BundleError {}

impl ScenarioBundle {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, BundleError> {
        self.validate()?;
        serde_json::to_vec(self).map_err(|error| {
            BundleError::Invalid(vec![diagnostic("serialization", "$", error.to_string())])
        })
    }

    pub fn identity(&self) -> Result<BundleIdentity, BundleError> {
        let bytes = self.canonical_bytes()?;
        Ok(BundleIdentity {
            sha256: bytes_hash(&bytes),
            map_hash: self.map.map_hash.clone(),
            rules_hash: self.rules_hash.clone(),
        })
    }

    /// Canonical JSON for the resolved scenario consumed by the normal battle
    /// preparation path. The bundle envelope remains the admission boundary;
    /// callers never reconstruct map or rules fields themselves.
    pub fn scenario_json(&self) -> Result<String, BundleError> {
        self.validate()?;
        let mut scenario = self.scenario.clone();
        scenario.authored_scripts = self.authored_scripts.clone();
        scenario.victory = Some(self.victory.source.clone());
        serde_json::to_string(&scenario).map_err(|error| {
            BundleError::Invalid(vec![diagnostic(
                "serialization",
                "$.scenario",
                error.to_string(),
            )])
        })
    }

    pub fn to_json(&self) -> Result<String, BundleError> {
        let bytes = self.canonical_bytes()?;
        String::from_utf8(bytes).map_err(|error| {
            BundleError::Invalid(vec![diagnostic("serialization", "$", error.to_string())])
        })
    }

    /// Admit a builder draft by deriving the two content identities owned by
    /// this contract, then applying the same strict topology and asset checks
    /// as an uploaded file. Runtime uploads must use `from_json`, which keeps
    /// the durable bytes immutable.
    pub fn from_authoring_json(json: &str) -> Result<Self, BundleError> {
        if json.len() > MAX_BUNDLE_BYTES {
            return Err(BundleError::Invalid(vec![diagnostic(
                "size_limit",
                "$",
                "bundle exceeds the 32 MiB limit",
            )]));
        }
        let mut draft: serde_json::Value = serde_json::from_str(json).map_err(|error| {
            BundleError::Invalid(vec![diagnostic("malformed", "$", error.to_string())])
        })?;
        normalize_authoring_units(&mut draft)?;
        let mut bundle: Self = serde_json::from_value(draft).map_err(|error| {
            BundleError::Invalid(vec![diagnostic("malformed", "$", error.to_string())])
        })?;
        bundle.map.map_hash = json_hash(&bundle.scenario.map).map_err(|error| {
            BundleError::Invalid(vec![diagnostic("identity", "$.map", error.to_string())])
        })?;
        bundle.map.template_catalog_hash = bundle.scenario.map.template_catalog_hash.clone();
        bundle.rules_hash = json_hash(&bundle.scenario.rules).map_err(|error| {
            BundleError::Invalid(vec![diagnostic(
                "identity",
                "$.rules_hash",
                error.to_string(),
            )])
        })?;
        bundle.validate()?;
        Ok(bundle)
    }

    pub fn from_json(json: &str) -> Result<Self, BundleError> {
        if json.len() > MAX_BUNDLE_BYTES {
            return Err(BundleError::Invalid(vec![diagnostic(
                "size_limit",
                "$",
                "bundle exceeds the 32 MiB limit",
            )]));
        }
        let bundle: Self = serde_json::from_str(json).map_err(|error| {
            BundleError::Invalid(vec![diagnostic("malformed", "$", error.to_string())])
        })?;
        bundle.validate()?;
        let canonical = serde_json::to_vec(&bundle).expect("validated bundle serializes");
        if canonical != json.as_bytes() {
            return Err(BundleError::Invalid(vec![diagnostic(
                "non_canonical",
                "$",
                "bundle bytes are not canonical JSON",
            )]));
        }
        Ok(bundle)
    }

    /// Resolve a requested player controller for one play session. At most one
    /// human id is selected; an unselected human-capable player is passive AI.
    pub fn effective_controllers(
        &self,
        active_human: Option<&str>,
    ) -> Result<Vec<(String, ControllerKind)>, BundleError> {
        self.validate()?;
        if let Some(id) = active_human {
            if !self
                .scenario
                .players
                .iter()
                .any(|player| player.id == id && player.human_capable)
            {
                return Err(BundleError::Invalid(vec![diagnostic(
                    "controller_topology",
                    "$.scenario.players",
                    "active human must name a human-capable player",
                )]));
            }
        }
        Ok(self
            .scenario
            .players
            .iter()
            .map(|player| {
                let controller = match player.controller {
                    ControllerKind::ActiveAi => ControllerKind::ActiveAi,
                    ControllerKind::PassiveAi => ControllerKind::PassiveAi,
                    ControllerKind::Human => {
                        if active_human == Some(player.id.as_str()) {
                            ControllerKind::Human
                        } else {
                            ControllerKind::PassiveAi
                        }
                    }
                };
                (player.id.clone(), controller)
            })
            .collect())
    }

    pub fn validate(&self) -> Result<(), BundleError> {
        let mut errors = Vec::new();
        if self.format_version != FORMAT_VERSION {
            errors.push(diagnostic(
                "unsupported_version",
                "$.format_version",
                format!("expected {FORMAT_VERSION}"),
            ));
        }
        if self.runtime_build.trim().is_empty() {
            errors.push(diagnostic(
                "invalid_identity",
                "$.runtime_build",
                "runtime build is empty",
            ));
        }
        let map_hash = json_hash(&self.scenario.map).expect("map serializes");
        if self.map.map_hash != map_hash {
            errors.push(diagnostic(
                "identity_mismatch",
                "$.map.map_hash",
                "map hash does not match resolved map bytes",
            ));
        }
        if self.map.template_catalog_hash != self.scenario.map.template_catalog_hash {
            errors.push(diagnostic(
                "identity_mismatch",
                "$.map.template_catalog_hash",
                "template catalogue identity does not match the resolved map",
            ));
        }
        for error in crate::maps::admit_world(&self.scenario.map, &self.scenario.rules) {
            errors.push(diagnostic(
                "invalid_map",
                format!("$.scenario.{}", error.location),
                error.message,
            ));
        }
        for (location, message) in self.scenario.unknown_catalog_references() {
            errors.push(diagnostic(
                "unknown_catalog_reference",
                format!("$.scenario.{location}"),
                message,
            ));
        }
        let rules_hash = json_hash(&self.scenario.rules).expect("rules serialize");
        if self.rules_hash != rules_hash {
            errors.push(diagnostic(
                "identity_mismatch",
                "$.rules_hash",
                "rules hash does not match resolved rules",
            ));
        }
        if !self.scenario.authored_scripts.is_empty() {
            errors.push(diagnostic(
                "duplicate_script_owner",
                "$.scenario.authored_scripts",
                "authored scripts belong in the bundle envelope",
            ));
        }
        if self.scenario.victory.is_some() {
            errors.push(diagnostic(
                "duplicate_victory_owner",
                "$.scenario.victory",
                "victory belongs in the bundle envelope",
            ));
        }
        if let Some(setup) = &self.scenario.skirmish {
            if let Err(error) = crate::players::validate_players(&self.scenario.players) {
                errors.push(diagnostic("invalid_players", "$.scenario.players", error));
            }
            if let Err(error) = crate::players::validate_players_per_side(
                &self.scenario.players,
                setup.rules.max_players_per_side as usize,
            ) {
                errors.push(diagnostic("invalid_players", "$.scenario.players", error));
            }
            if let Err(error) =
                crate::players::validate_entry_counts(&self.scenario.players, &setup.sites.entries)
            {
                errors.push(diagnostic(
                    "invalid_entries",
                    "$.scenario.skirmish.sites.entries",
                    error,
                ));
            }
        } else {
            if self.scenario.players.is_empty() {
                errors.push(diagnostic(
                    "invalid_players",
                    "$.scenario.players",
                    "a scenario bundle must name its players",
                ));
            }
            let mut ids = std::collections::BTreeSet::new();
            for player in &self.scenario.players {
                if player.id.trim().is_empty() || !ids.insert(player.id.as_str()) {
                    errors.push(diagnostic(
                        "invalid_players",
                        "$.scenario.players",
                        "player ids must be non-empty and unique",
                    ));
                }
                if player.controller == ControllerKind::Human && !player.human_capable {
                    errors.push(diagnostic(
                        "invalid_players",
                        "$.scenario.players",
                        "human player must be human_capable",
                    ));
                }
                match (player.side, player.faction) {
                    (Side::Neutral, Some(_)) => errors.push(diagnostic(
                        "invalid_players",
                        "$.scenario.players",
                        "neutral players cannot carry a faction",
                    )),
                    (Side::Blue | Side::Red, None) => errors.push(diagnostic(
                        "invalid_players",
                        "$.scenario.players",
                        "combatant players must carry a faction",
                    )),
                    _ => {}
                }
                if player.side == Side::Neutral && player.human_capable {
                    errors.push(diagnostic(
                        "invalid_players",
                        "$.scenario.players",
                        "neutral players cannot be human-capable",
                    ));
                }
            }
        }
        validate_sorted_unique(
            &self
                .authored_scripts
                .iter()
                .map(|s| s.id.as_str())
                .collect::<Vec<_>>(),
            "$.authored_scripts",
            &mut errors,
        );
        validate_sorted_unique(
            &self
                .assets
                .iter()
                .map(|a| a.id.as_str())
                .collect::<Vec<_>>(),
            "$.assets",
            &mut errors,
        );
        for (i, unit) in self.scenario.units.iter().enumerate() {
            if let Some(owner) = &unit.owner {
                match self.scenario.players.iter().find(|p| p.id == *owner) {
                    None => errors.push(diagnostic(
                        "invalid_unit_owner",
                        format!("$.scenario.units[{i}].owner"),
                        format!("unknown player {owner}"),
                    )),
                    Some(player) if player.side != unit.side => errors.push(diagnostic(
                        "side_mismatch",
                        format!("$.scenario.units[{i}].owner"),
                        "unit owner must share its side",
                    )),
                    Some(_) => {}
                }
            } else {
                errors.push(diagnostic(
                    "unowned_unit",
                    format!("$.scenario.units[{i}].owner"),
                    "every scenario unit must name a player",
                ));
            }
        }
        if self.mode == BundleMode::Economy
            && self.scenario.units.iter().any(|u| u.side == Side::Neutral)
        {
            errors.push(diagnostic(
                "unsupported_topology",
                "$.mode",
                "economy mode does not support neutral units",
            ));
        }
        if self.mode == BundleMode::Economy
            && (!self.scenario.units.is_empty()
                || !self.scenario.events.is_empty()
                || !self.scenario.scripts.is_empty()
                || !self.authored_scripts.is_empty()
                || self.scenario.opponent.is_some()
                || self.scenario.encounter.is_some())
        {
            errors.push(diagnostic(
                "invalid_economy_setup",
                "$.scenario",
                "economy bundles must leave authored forces and scripted encounter state empty",
            ));
        }
        if matches!(self.mode, BundleMode::Economy) != self.scenario.skirmish.is_some() {
            errors.push(diagnostic(
                "mode_mismatch",
                "$.mode",
                "economy bundles must carry binary skirmish setup and sandbox bundles must not",
            ));
        }
        for (i, script) in self.authored_scripts.iter().enumerate() {
            if script.id.trim().is_empty() {
                errors.push(diagnostic(
                    "invalid_script",
                    format!("$.authored_scripts[{i}].id"),
                    "script id is empty",
                ));
            }
            if script.source.len() > MAX_SCRIPT_SOURCE_BYTES {
                errors.push(diagnostic(
                    "size_limit",
                    format!("$.authored_scripts[{i}].source"),
                    "script source exceeds the 256 KiB limit",
                ));
            } else if let Err(error) = CompiledScript::compile(&script.source) {
                for problem in error.diagnostics {
                    errors.push(diagnostic(
                        problem.code,
                        format!(
                            "$.authored_scripts[{i}].source:{}:{}",
                            problem.span.line, problem.span.column
                        ),
                        problem.message,
                    ));
                }
            }
            if script.side == Side::Neutral {
                errors.push(diagnostic(
                    "invalid_script",
                    format!("$.authored_scripts[{i}].side"),
                    "neutral scripts may observe but cannot author a controller",
                ));
            }
            if let Some(player) = &script.player {
                match self
                    .scenario
                    .players
                    .iter()
                    .find(|candidate| candidate.id == *player)
                {
                    None => errors.push(diagnostic(
                        "invalid_script_player",
                        format!("$.authored_scripts[{i}].player"),
                        format!("script names unknown player {player}"),
                    )),
                    Some(candidate) if candidate.side != script.side => errors.push(diagnostic(
                        "script_side_mismatch",
                        format!("$.authored_scripts[{i}].side"),
                        "script side must match its player",
                    )),
                    Some(_) => {}
                }
            }
        }
        if self.victory.source.len() > MAX_SCRIPT_SOURCE_BYTES {
            errors.push(diagnostic(
                "size_limit",
                "$.victory.source",
                "victory source exceeds the 256 KiB limit",
            ));
        } else if let Err(error) = CompiledScript::compile(&self.victory.source) {
            for problem in error.diagnostics {
                errors.push(diagnostic(
                    problem.code,
                    format!(
                        "$.victory.source:{}:{}",
                        problem.span.line, problem.span.column
                    ),
                    problem.message,
                ));
            }
        }
        for (i, asset) in self.assets.iter().enumerate() {
            if asset.id.trim().is_empty()
                || asset.media_type.trim().is_empty()
                || asset.provenance.trim().is_empty()
            {
                errors.push(diagnostic(
                    "invalid_asset",
                    format!("$.assets[{i}]"),
                    "asset id, media type, and provenance are required",
                ));
            }
            if !is_sha256(&asset.sha256) {
                errors.push(diagnostic(
                    "invalid_hash",
                    format!("$.assets[{i}].sha256"),
                    "asset hash must be lowercase SHA-256",
                ));
            }
            match (&asset.bytes_base64, &asset.builtin) {
                (Some(encoded), None) => match base64_decode(encoded) {
                    Ok(bytes) if bytes.len() <= MAX_ASSET_BYTES => {
                        if bytes_hash(&bytes) != asset.sha256 {
                            errors.push(diagnostic(
                                "asset_hash_mismatch",
                                format!("$.assets[{i}].bytes_base64"),
                                "embedded bytes do not match sha256",
                            ));
                        }
                    }
                    Ok(_) => errors.push(diagnostic(
                        "size_limit",
                        format!("$.assets[{i}].bytes_base64"),
                        "asset exceeds the 8 MiB limit",
                    )),
                    Err(message) => errors.push(diagnostic(
                        "invalid_encoding",
                        format!("$.assets[{i}].bytes_base64"),
                        message,
                    )),
                },
                (None, Some(name)) if is_sha256(name) => {}
                (None, Some(_)) => errors.push(diagnostic(
                    "invalid_hash",
                    format!("$.assets[{i}].builtin"),
                    "built-in asset references must be lowercase SHA-256 content hashes",
                )),
                _ => errors.push(diagnostic(
                    "invalid_asset",
                    format!("$.assets[{i}]"),
                    "asset must contain exactly one of bytes_base64 or builtin",
                )),
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(BundleError::Invalid(errors))
        }
    }
}

fn diagnostic(
    code: &'static str,
    location: impl Into<String>,
    message: impl Into<String>,
) -> BundleDiagnostic {
    BundleDiagnostic {
        code,
        location: location.into(),
        message: message.into(),
    }
}

fn validate_sorted_unique(values: &[&str], location: &str, errors: &mut Vec<BundleDiagnostic>) {
    for pair in values.windows(2) {
        if pair[0] >= pair[1] {
            errors.push(diagnostic(
                "non_canonical_order",
                location,
                "ids must be strictly sorted and unique",
            ));
            break;
        }
    }
}

/// The tactical editor keeps a stable integer id beside each unit so it can
/// select and assign units while the draft changes. `UnitSetup` deliberately
/// does not carry that editor concern, so authoring strips the field before
/// deserializing the strict portable scenario. When present, ids must already
/// be the vector indexes; otherwise player ownership would silently point at a
/// different unit after the field is removed.
fn normalize_authoring_units(value: &mut serde_json::Value) -> Result<(), BundleError> {
    let Some(units) = value
        .get_mut("scenario")
        .and_then(|scenario| scenario.get_mut("units"))
        .and_then(serde_json::Value::as_array_mut)
    else {
        return Ok(());
    };
    for (index, unit) in units.iter_mut().enumerate() {
        let Some(object) = unit.as_object_mut() else {
            continue;
        };
        let Some(id) = object.remove("id") else {
            continue;
        };
        if id.as_u64() != Some(index as u64) {
            return Err(BundleError::Invalid(vec![diagnostic(
                "non_canonical_order",
                format!("$.scenario.units[{index}].id"),
                "editor unit ids must be contiguous vector indexes",
            )]));
        }
    }
    Ok(())
}

fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    let mut chunk = [0u8; 4];
    let mut count = 0;
    for byte in input.bytes() {
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => 64,
            _ => return Err("asset bytes are not valid base64".into()),
        };
        chunk[count] = value;
        count += 1;
        if count == 4 {
            if chunk[0] == 64 || chunk[1] == 64 || (chunk[2] == 64 && chunk[3] != 64) {
                return Err("asset bytes have invalid base64 padding".into());
            }
            out.push((chunk[0] << 2) | (chunk[1] >> 4));
            if chunk[2] != 64 {
                out.push((chunk[1] << 4) | (chunk[2] >> 2));
            }
            if chunk[3] != 64 {
                out.push((chunk[2] << 6) | chunk[3]);
            }
            count = 0;
        }
    }
    if count != 0 {
        return Err("asset bytes have incomplete base64 padding".into());
    }
    Ok(out)
}

/// Small dependency-free encoder used by native probes and browser codecs.
pub fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let a = chunk[0];
        let b = *chunk.get(1).unwrap_or(&0);
        let c = *chunk.get(2).unwrap_or(&0);
        out.push(TABLE[(a >> 2) as usize] as char);
        out.push(TABLE[((a << 4 | b >> 4) & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            TABLE[((b << 2 | c >> 6) & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[(c & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}
