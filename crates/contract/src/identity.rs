//! Exact generation inputs and content identities, independent of appearance.
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};

/// A u64 seed crosses JSON as decimal text, never a floating-point number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Seed(u64);

impl Seed {
    pub fn value(self) -> u64 {
        self.0
    }
}
impl From<u64> for Seed {
    fn from(value: u64) -> Self {
        Self(value)
    }
}
impl Serialize for Seed {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(&self.0)
    }
}
impl<'de> Deserialize<'de> for Seed {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        let value = text
            .parse::<u64>()
            .map_err(|_| D::Error::custom("seed must be a canonical u64 decimal string"))?;
        if value.to_string() != text {
            return Err(D::Error::custom(
                "seed must be a canonical u64 decimal string",
            ));
        }
        Ok(Self(value))
    }
}

/// Hash the fixed JSON representation of an already-canonical typed record.
/// Each owner admits numbers and defines canonical named rows or meaningful order;
/// changing its serialization field order is an identity-contract change.
pub fn json_hash<T: Serialize + ?Sized>(value: &T) -> Result<String, serde_json::Error> {
    Ok(bytes_hash(&serde_json::to_vec(value)?))
}

pub fn bytes_hash(value: &[u8]) -> String {
    format!("{:x}", Sha256::digest(value))
}

/// Canonical spelling of the shared owner's SHA256 content identities.
pub fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

pub fn validate_version_identifier(value: &str) -> Result<(), &'static str> {
    if value.trim().is_empty() {
        Err("generation version identifiers must be nonempty")
    } else {
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct IdentityError {
    pub field: &'static str,
    pub message: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenerationIdentity {
    pub generator_version: String,
    pub preset_revision: String,
    pub seed: Seed,
    pub config_hash: String,
    pub template_catalog_hash: String,
    pub map_hash: String,
}

impl GenerationIdentity {
    pub fn validate(&self) -> Result<(), IdentityError> {
        for (field, value) in [
            ("generator_version", &self.generator_version),
            ("preset_revision", &self.preset_revision),
        ] {
            validate_version_identifier(value)
                .map_err(|message| IdentityError { field, message })?;
        }
        for (field, value) in [
            ("config_hash", &self.config_hash),
            ("template_catalog_hash", &self.template_catalog_hash),
            ("map_hash", &self.map_hash),
        ] {
            if !is_sha256(value) {
                return Err(IdentityError {
                    field,
                    message: "content hash must be canonical SHA256",
                });
            }
        }
        Ok(())
    }
}
