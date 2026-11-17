//! Decodes the identified-enemy row and its variable body sections.
use super::{
    at, limbs, member_column, points, present, weapon_poses, Deployment, Identified, Reader,
    Result, Row, IDENTIFIED_FIELDS,
};

impl Reader {
    pub(super) fn identified(&self, r: &Row) -> Result<Identified> {
        const FIELDS: &[&str] = &IDENTIFIED_FIELDS;
        let f = r.fields;
        let [members, member_ids, leans, poses] = r.sections[..] else {
            unreachable!("identified rows carry four sections")
        };
        Ok(Identified {
            id: v!(f, "id"),
            kind: self.name("unitKinds", v!(f, "kind"))?,
            cost: v!(f, "cost"),
            position: [v!(f, "x"), v!(f, "y"), v!(f, "z")],
            yaw: v!(f, "yaw"),
            velocity: [v!(f, "vx"), v!(f, "vy")],
            members: points(members),
            member_ids: member_column(member_ids, |p| limbs(p[0] as f64, p[1] as f64)),
            member_slots: member_column(member_ids, |p| p[2] as f64),
            member_active_mounts: member_column(member_ids, |p| present(p[3] as f64)),
            member_leans: self.leans(leans)?,
            weapon_poses: weapon_poses(poses),
            reversing: v!(f, "reversing") == 1.0,
            smoking: v!(f, "smoking") == 1.0,
            deployment: self
                .optional("postures", v!(f, "deployTarget"))?
                .map(|target| Deployment {
                    progress: v!(f, "deployProgress"),
                    target,
                }),
            grounded: v!(f, "grounded") == 1.0,
            building: limbs(v!(f, "buildingLo"), v!(f, "buildingHi")),
        })
    }
}
