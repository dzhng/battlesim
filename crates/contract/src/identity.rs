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
    Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(value)?)))
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
