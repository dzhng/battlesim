//! The skirmish's rows: capture objectives, pending purchases and players,
//! decoded by the writer's field names.
use super::{at, optional_fixed, Objective, PendingPurchase, Player, Reader, Result, Row, LIMB};
use crate::publication::{OBJECTIVE_FIELDS, PENDING_FIELDS, PLAYER_FIELDS};

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
            unit_count: v!(f, "unitCount"),
            cap: v!(f, "cap"),
            pending: Vec::new(),
        })
    }
}
