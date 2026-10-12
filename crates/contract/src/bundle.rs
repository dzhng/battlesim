//! Portable, self-contained scenario downloads.
//!
//! The bundle is a strict JSON envelope around the already-resolved
//! `ScenarioDefinition`. It is deliberately the only file format at the
//! download/upload boundary: map geometry and rules remain owned by their
//! existing contract types, while this module owns package identity, player
//! admission and embedded user bytes.
use crate::catalog::Faction;
use crate::identity::{bytes_hash, is_sha256, json_hash};
use crate::ids::{Side, UnitId};
use crate::scenario::ScenarioDefinition;
use serde::{Deserialize, Serialize};

pub const FORMAT_VERSION: &str = "scenario-bundle-1";
pub const MAX_BUNDLE_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_ASSET_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_SCRIPT_SOURCE_BYTES: usize = 256 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BundleMode {
    Sandbox,
    Economy,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControllerKind {
    Human,
    ActiveAi,
    PassiveAi,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerSlot {
    pub id: String,
    pub side: Side,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub faction: Option<Faction>,
    pub controller: ControllerKind,
    #[serde(default)]
    pub units: Vec<UnitId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BundleMapIdentity {
    pub map_hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_catalog_hash: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoredScript {
    pub id: String,
    pub source: String,
}

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
    pub slots: Vec<PlayerSlot>,
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

    pub fn to_json(&self) -> Result<String, BundleError> {
        let bytes = self.canonical_bytes()?;
        String::from_utf8(bytes).map_err(|error| {
            BundleError::Invalid(vec![diagnostic("serialization", "$", error.to_string())])
        })
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
        let rules_hash = json_hash(&self.scenario.rules).expect("rules serialize");
        if self.rules_hash != rules_hash {
            errors.push(diagnostic(
                "identity_mismatch",
                "$.rules_hash",
                "rules hash does not match resolved rules",
            ));
        }
        validate_sorted_unique(
            &self.slots.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(),
            "$.slots",
            &mut errors,
        );
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
        let mut humans = 0;
        let mut assigned = std::collections::BTreeSet::new();
        for (i, slot) in self.slots.iter().enumerate() {
            if slot.id.trim().is_empty() {
                errors.push(diagnostic(
                    "invalid_slot",
                    format!("$.slots[{i}].id"),
                    "slot id is empty",
                ));
            }
            if slot.controller == ControllerKind::Human {
                humans += 1;
            }
            if slot.side == Side::Neutral && slot.controller != ControllerKind::PassiveAi {
                errors.push(diagnostic(
                    "invalid_controller",
                    format!("$.slots[{i}].controller"),
                    "neutral slots must use passive_ai",
                ));
            }
            for unit in &slot.units {
                if unit.0 as usize >= self.scenario.units.len() {
                    errors.push(diagnostic(
                        "invalid_unit_owner",
                        format!("$.slots[{i}].units"),
                        format!("unit {} is outside scenario.units", unit.0),
                    ));
                } else if !assigned.insert(*unit) {
                    errors.push(diagnostic(
                        "duplicate_unit_owner",
                        format!("$.slots[{i}].units"),
                        format!("unit {} is assigned more than once", unit.0),
                    ));
                } else if self.scenario.units[unit.0 as usize].side != slot.side {
                    errors.push(diagnostic(
                        "side_mismatch",
                        format!("$.slots[{i}].units"),
                        format!("unit {} belongs to a different side", unit.0),
                    ));
                }
            }
            if slot.units.windows(2).any(|pair| pair[0] >= pair[1]) {
                errors.push(diagnostic(
                    "non_canonical_order",
                    format!("$.slots[{i}].units"),
                    "unit ids must be strictly sorted and unique",
                ));
            }
        }
        if humans > 1 {
            errors.push(diagnostic(
                "controller_topology",
                "$.slots",
                "at most one human slot is allowed",
            ));
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
        if matches!(self.mode, BundleMode::Economy) != self.scenario.skirmish.is_some() {
            errors.push(diagnostic(
                "mode_mismatch",
                "$.mode",
                "economy bundles must carry binary skirmish setup and sandbox bundles must not",
            ));
        }
        for (i, _unit) in self.scenario.units.iter().enumerate() {
            if !assigned.contains(&UnitId(i as u32)) {
                errors.push(diagnostic(
                    "unowned_unit",
                    format!("$.scenario.units[{i}"),
                    "every scenario unit must belong to exactly one player slot",
                ));
            }
        }
        for (i, script) in self.authored_scripts.iter().enumerate() {
            if script.source.len() > MAX_SCRIPT_SOURCE_BYTES {
                errors.push(diagnostic(
                    "size_limit",
                    format!("$.authored_scripts[{i}].source"),
                    "script source exceeds the 256 KiB limit",
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
                    format!("$.assets[{i}"),
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
                (None, Some(name)) if !name.trim().is_empty() => {}
                _ => errors.push(diagnostic(
                    "invalid_asset",
                    format!("$.assets[{i}"),
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
