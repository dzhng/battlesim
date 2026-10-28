//! The ordered unit state folded into battle replay identity.
use super::{Fallen, Unit, UnitOrder};
use crate::digest::Digest;
use contract::command::Engagement;

impl Unit {
    /// Fold the unit's carried state and its owner's `PlayerIndex` into `d`.
    pub fn digest(&self, d: &mut Digest, owner: Option<usize>) {
        d.u64(self.id.0 as u64);
        if let Some(owner) = owner {
            d.u64(owner as u64);
        }
        d.f64(self.position.x)
            .f64(self.position.y)
            .f64(self.position.z)
            .f64(self.yaw);
        d.u64(self.state as u64).u64(self.reversing as u64);
        d.u64(self.retired as u64).u64(self.withdrawing as u64);
        if self.is_vehicle() {
            d.f64(self.drive_speed_mps);
        }
        d.opt_f64(self.turn_to);
        if let Some(a) = self.air {
            d.f64(a.velocity.x)
                .f64(a.velocity.y)
                .f64(a.velocity.z)
                .f64(a.burner)
                .f64(a.turn_sign as f64);
            d.opt_v2(a.last_goal);
            match a.settle {
                Some(s) => d
                    .u64(1)
                    .f64(s.goal.x)
                    .f64(s.goal.y)
                    .f64(s.aim.x)
                    .f64(s.aim.y)
                    .u64(s.back as u64),
                None => d.u64(0),
            };
            d.f64(a.fuel_s)
                .u64(a.afterburner as u64)
                .u64(a.dogfight_target.map_or(u64::MAX, |id| u64::from(id.0)));
            match a.pass {
                Some(p) => {
                    d.u64(1);
                    p.target.digest(d);
                    d.f64(p.aim.x)
                        .f64(p.aim.y)
                        .f64(p.aim.z)
                        .u64(p.phase as u64)
                        .u64(p.weapon as u64)
                        .f64(p.agl)
                        .f64(p.heading);
                }
                None => {
                    d.u64(0);
                }
            }
            d.u64(a.pass_release as u64);
        }
        match self.flares {
            Some(f) => d.u64(1).u64(f.charges as u64).u64(f.ready_tick),
            None => d.u64(0),
        };
        match self.manoeuvre {
            Some(m) => d.u64(1).f64(m.turn).f64(m.driven_m),
            None => d.u64(0),
        };
        d.u64(self.orders.len() as u64);
        for o in &self.orders {
            match o {
                UnitOrder::Move(m) | UnitOrder::AttackMove(m) => {
                    d.u64(matches!(o, UnitOrder::Move(_)) as u64);
                    d.f64(m.destination.x)
                        .f64(m.destination.y)
                        .u64(m.policy as u64)
                        .u64(m.gesture)
                        .u64(m.direction as u64);
                    d.opt_f64(m.facing);
                    let short = m.short.as_ref();
                    d.opt_v2(short.map(|s| s.at));
                    d.opt_f64(short.map(|s| s.detour_m));
                    d.opt_v2(short.and_then(|s| s.nearest.as_ref()).map(|n| n.0));
                }
                UnitOrder::Attack { target, last_known } => {
                    d.u64(2);
                    target.digest(d);
                    d.opt_v2(*last_known);
                }
                UnitOrder::Garrison { building, approach } => {
                    d.u64(3)
                        .u64(*building as u64)
                        .f64(approach.x)
                        .f64(approach.y);
                }
                UnitOrder::Exit => {
                    d.u64(4);
                }
                UnitOrder::Board { carrier, gesture } => {
                    d.u64(5).u64(carrier.0 as u64).u64(*gesture);
                }
                UnitOrder::Collect { squads, gesture } => {
                    d.u64(6).u64(*gesture).u64(squads.len() as u64);
                    for squad in squads {
                        d.u64(squad.0 as u64);
                    }
                }
                UnitOrder::Unload => {
                    d.u64(7);
                }
            }
        }
        d.opt_v2(self.pursuit.map(|p| p.to));
        if let Some(p) = self.pursuit {
            d.u64(p.searching as u64);
        }
        d.opt_v2(self.planned_goal);
        d.u64(self.route.is_some() as u64);
        for p in self.route.iter().flatten() {
            d.f64(p.x).f64(p.y);
        }
        d.f64(self.route_from.x).f64(self.route_from.y);
        d.u64(self.blocker.is_some() as u64);
        if let Some(b) = self.blocker {
            d.u64(b.0 as u64);
        }
        d.u64(self.planned_revision)
            .f64(self.progress.0)
            .u64(self.progress.1)
            .f64(self.stalls.0)
            .u64(self.stalls.1);
        d.f64(self.hp).f64(self.suppression).u64(self.suppressed_at);
        d.u64(self.stock.map_or(u64::MAX, u64::from));
        d.u64(self.protection.is_some() as u64);
        if let Some(protection) = self.protection {
            protection.digest(d);
        }
        let p = self.progress_service;
        d.f64(p.ammo_s)
            .f64(p.protection_s)
            .f64(p.flare_s)
            .f64(p.hp_s)
            .f64(p.soldier_s)
            .u64(self.service as u64);
        d.f64(self.sight_forward);
        self.cover.digest(d);
        match self.anchor {
            Some(a) => d.u64(1).f64(a.at.x).f64(a.at.y).u64(a.halt as u64),
            None => d.u64(0),
        };
        crate::hold::digest(self, d);
        d.u64(self.members.len() as u64);
        for s in &self.members {
            d.u64(s.id as u64)
                .u64(s.slot as u64)
                .u64(s.active_mount.map_or(u64::MAX, |m| m as u64))
                .f64(s.hp)
                .f64(s.position.x)
                .f64(s.position.y)
                .f64(s.position.z);
            d.f64(s.velocity.x).f64(s.velocity.y);
            d.opt_v2(s.spot).u64(s.leg as u64);
            d.f64(s.lateral).f64(s.pace).u64(s.start);
            d.u64(s.path.len() as u64);
            for p in &s.path {
                d.f64(p.x).f64(p.y);
            }
            d.u64(s.path_revision).u64(s.planned_at);
            d.opt_v2(s.post).u64(s.cover.map_or(u64::MAX, |t| t as u64));
            d.u64(s.lean.is_some() as u64);
            if let Some(l) = &s.lean {
                l.digest(d);
            }
            d.u64(s.lean_since).u64(s.leaning_until).u64(s.tucked_until);
            d.u64(s.sat_out.is_some() as u64);
            if let Some(m) = s.sat_out {
                d.f64(m.at.x).f64(m.at.y).f64(m.threat.x).f64(m.threat.y);
                d.u64(m.revision).u64(u64::from(m.craters));
            }
            d.u64(s.corpse.is_some() as u64);
            if let Some(Fallen {
                at: p,
                yaw,
                support_building,
            }) = s.corpse
            {
                d.f64(p.x).f64(p.y).f64(p.z).f64(yaw);
                if let Some(owner) = support_building {
                    d.u64(u64::MAX).u64(owner as u64);
                }
            }
        }
        d.u64(self.engagement as u64)
            .u64(self.reach.can_engage as u64)
            .u64(self.reach.needs_closer as u64);
        // Only a passive AI's unit that returns fire by choice folds this in,
        // so other units keep their digests.
        if self.held_engagement == Some(Engagement::ReturnFireOnly) {
            d.u64(u64::MAX);
        }
        // Only a gun fixed in a hull sets it, so other units keep their digests.
        if let Some(face) = self.reach.face {
            d.f64(face);
        }
        d.u64(self.attackers.len() as u64);
        for a in &self.attackers {
            d.u64(a.0 as u64);
        }
        d.u64(self.deployment.is_some() as u64);
        if let Some(dep) = &self.deployment {
            d.u64(dep.current as u64)
                .u64(dep.duration as u64)
                .u64(dep.stationary as u64);
            if dep.deploy_step != dep.pack_step {
                d.u64(dep.deploy_step as u64).u64(dep.pack_step as u64);
            }
        }
        d.u64(self.mounts.len() as u64);
        for m in &self.mounts {
            m.digest(d);
        }
    }
}
