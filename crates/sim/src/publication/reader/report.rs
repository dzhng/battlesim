//! A match report's rows, decoded by the writer's field names.
use super::{at, fixed, limbs, present, Killer, Reader, ReportEvent, ReportTotals, Result, Row};
use crate::publication::{REPORT_EVENT_FIELDS, REPORT_TOTAL_FIELDS};

impl Reader {
    pub(super) fn event(&self, r: &Row) -> Result<ReportEvent> {
        const FIELDS: &[&str] = &REPORT_EVENT_FIELDS;
        let f = r.fields;
        let killer = if v!(f, "otherSide") < 0.0 {
            None
        } else {
            Some(Killer {
                side: self.name("sides", v!(f, "otherSide"))?,
                unit_id: limbs(v!(f, "otherIdLo"), v!(f, "otherIdHi")),
                unit_kind: self.name("unitKinds", v!(f, "otherKind"))?,
                owner: present(v!(f, "otherOwner")),
                cost: v!(f, "otherAmount"),
            })
        };
        let kill = v!(f, "kind") != 0.0 && v!(f, "kind") != 1.0;
        Ok(ReportEvent {
            kind: fixed(&["spawn", "refund"], v!(f, "kind")).unwrap_or("kill"),
            tick: limbs(v!(f, "tickLo"), v!(f, "tickHi")),
            side: self.name("sides", v!(f, "side"))?,
            unit_id: limbs(v!(f, "unitIdLo"), v!(f, "unitIdHi")),
            unit_kind: self.name("unitKinds", v!(f, "unitKind"))?,
            owner: present(v!(f, "owner")),
            amount: (!kill).then(|| v!(f, "amount")),
            victim_cost: kill.then(|| v!(f, "amount")),
            killer: kill.then_some(killer),
        })
    }
}

pub(super) fn totals(r: &Row) -> ReportTotals {
    const FIELDS: &[&str] = &REPORT_TOTAL_FIELDS;
    let f = r.fields;
    ReportTotals {
        spawned: v!(f, "spawned"),
        spent: v!(f, "spent"),
        killed: v!(f, "killed"),
        refunded: v!(f, "refunded"),
    }
}
