//! The skirmish's rows: capture objectives, pending purchases and players,
//! decoded by the writer's field names.
use super::{
    at, optional_fixed, Objective, Offer, PendingPurchase, Player, Reader, Result, Row, LIMB,
};
use crate::publication::{OBJECTIVE_FIELDS, OFFER_FIELDS, PENDING_FIELDS, PLAYER_FIELDS};

impl Reader {
    pub(super) fn objective(&self, r: &Row) -> Result<Objective> {
        const FIELDS: &[&str] = &OBJECTIVE_FIELDS;
        let f = r.fields;
        Ok(Objective {
            id: self.name("objectiveIds", v!(f, "id"))?,
            center: [v!(f, "x"), v!(f, "y")],
            radius_m: v!(f, "radius"),
            owner: optional_fixed(&["blue", "red"], v!(f, "owner")),
            capturing: optional_fixed(&["blue", "red"], v!(f, "capturing")),
            capture_progress: v!(f, "captureProgress"),
            contested: v!(f, "contested") != 0.0,
        })
    }

    pub(super) fn pending(&self, r: &Row) -> Result<PendingPurchase> {
        const FIELDS: &[&str] = &PENDING_FIELDS;
        let f = r.fields;
        Ok(PendingPurchase {
            owner: v!(f, "owner"),
            id: v!(f, "idLo") + v!(f, "idHi") * LIMB,
            kind: self.name("unitKinds", v!(f, "kind"))?,
            destination: [v!(f, "x"), v!(f, "y")],
            confirmed_tick: v!(f, "confirmedLo") + v!(f, "confirmedHi") * LIMB,
            blocked: v!(f, "blocked") != 0.0,
            arrival_tick: v!(f, "arrivalLo") + v!(f, "arrivalHi") * LIMB,
            carrier: self.optional("unitKinds", v!(f, "carrier"))?,
        })
    }

    pub(super) fn player(&self, r: &Row) -> Result<Player> {
        const FIELDS: &[&str] = &PLAYER_FIELDS;
        let f = r.fields;
        let id_index = v!(f, "id");
        let id = self
            .tables
            .get("playerIds")
            .and_then(|ids| ids.get(id_index as usize))
            .cloned()
            .ok_or_else(|| format!("player id index {id_index} is outside its table"))?;
        Ok(Player {
            index: v!(f, "index"),
            id,
            controller: self.name("controllers", v!(f, "controller"))?,
            ready: v!(f, "ready") != 0.0,
            credits: v!(f, "credits"),
            income_per_minute: v!(f, "incomePerMinute"),
            unit_count: v!(f, "unitCount"),
            cap: v!(f, "cap"),
            pending: Vec::new(),
            offered: (v!(f, "shopNarrowed") != 0.0).then(Vec::new),
        })
    }

    /// An offers row: its owner's seat index and the purchase.
    pub(super) fn offer(&self, r: &Row) -> Result<(f64, Offer)> {
        const FIELDS: &[&str] = &OFFER_FIELDS;
        let f = r.fields;
        Ok((
            v!(f, "owner"),
            Offer {
                kind: self.name("unitKinds", v!(f, "kind"))?,
                carrier: self.optional("unitKinds", v!(f, "carrier"))?,
            },
        ))
    }
}
