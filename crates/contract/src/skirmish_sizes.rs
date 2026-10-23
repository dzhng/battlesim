//! The skirmish size rule, authored in `fixtures/skirmish-sizes.json` and
//! shared with the lobby: how many players a side has decides the smallest
//! map that seats them, that map's extent, and how many roads each side
//! enters by.
use crate::generation::MapSize;
use serde::{Deserialize, Serialize};

/// One row of the rule: the side sizes it covers and what they get.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SizeTier {
    pub min_players: usize,
    /// Absent on the last tier, which covers every larger side.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_players: Option<usize>,
    pub size: MapSize,
    pub extent_m: f64,
    pub entry_count: usize,
}

/// Every tier, smallest first.
pub fn tiers() -> &'static [SizeTier] {
    static DATA: std::sync::OnceLock<Vec<SizeTier>> = std::sync::OnceLock::new();
    DATA.get_or_init(|| {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fixture {
            tiers: Vec<SizeTier>,
        }
        serde_json::from_str::<Fixture>(include_str!("../../../fixtures/skirmish-sizes.json"))
            .expect("valid skirmish-sizes fixture")
            .tiers
    })
}

/// The tier that seats a side of `players`; a side has at least one.
pub fn tier(players: usize) -> &'static SizeTier {
    let players = players.max(1);
    tiers()
        .iter()
        .find(|tier| {
            tier.min_players <= players && tier.max_players.is_none_or(|max| players <= max)
        })
        .expect("skirmish size fixture covers every player count")
}

/// The extent of a skirmish map of `size`.
pub fn extent_m(size: MapSize) -> f64 {
    tiers()
        .iter()
        .find(|tier| tier.size == size)
        .expect("skirmish size fixture contains every map size")
        .extent_m
}

/// The smallest size whose sides enter by at least `entries` roads, if any.
pub fn floor_for_entries(entries: usize) -> Option<MapSize> {
    tiers()
        .iter()
        .filter(|tier| tier.entry_count >= entries)
        .map(|tier| tier.size)
        .min()
}
