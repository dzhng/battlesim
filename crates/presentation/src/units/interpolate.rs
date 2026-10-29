//! Presentation between completed ticks: units drawn a
//! fraction of a tick behind the latest publication, blended from the
//! previous one. Only units present in both blend, and a soldier only with
//! himself (by member id), so a soldier's motion is his own and never a
//! formation slot's; nothing is extrapolated. A falling airframe blends with
//! itself likewise. Own units and identified enemies blend alike. A squad
//! riding a carrier is off the field and has no body: its owner reads it in
//! the publication, but nothing of it is drawn, picked or heard.
use std::sync::Arc;

use sim::publication::reader::{Crash, Identified, Observation, OwnUnit, PropDisplayPose};

use super::{delta_angle, lerp3, riding};
use crate::world::Vec3;

/// One unit's body at the drawn moment.
#[derive(Clone, Debug, Default)]
pub struct Pose {
    pub id: f64,
    pub position: Vec3,
    pub yaw: f64,
    pub members: Vec<Vec3>,
    /// Each member's soldier id, in `members` order.
    pub member_ids: Vec<Option<f64>>,
    /// Deployment progress in [0, 1], blended like the body; None if none.
    pub deployment: Option<f64>,
}

/// One drawn frame's view of the battle.
#[derive(Clone, Debug)]
pub struct FrameSample {
    /// The presentation clock, simulation seconds.
    pub time: f64,
    /// The publication the poses blend toward: the tick this frame draws.
    pub observation: Arc<Observation>,
    pub own: Vec<Pose>,
    pub identified: Vec<Pose>,
    /// Downed airframes still falling, blended as bodies are (by own and id).
    pub crashes: Vec<Crash>,
    /// Bodies being shoved, blended likewise (by prop id).
    pub shoved: Vec<PropDisplayPose>,
}

/// What own units and identified enemies both publish of their bodies.
struct Body<'a> {
    id: f64,
    position: Vec3,
    yaw: f64,
    members: &'a [Vec3],
    ids: &'a [Option<f64>],
}

impl<'a> From<&'a OwnUnit> for Body<'a> {
    fn from(u: &'a OwnUnit) -> Self {
        Body {
            id: u.id,
            position: u.position,
            yaw: u.yaw,
            members: &u.members,
            ids: &u.member_ids,
        }
    }
}

impl<'a> From<&'a Identified> for Body<'a> {
    fn from(u: &'a Identified) -> Self {
        Body {
            id: u.id,
            position: u.position,
            yaw: u.yaw,
            members: &u.members,
            ids: &u.member_ids,
        }
    }
}

impl Pose {
    /// `u` blended from `p` (the previous tick's same unit, if any) by `t`.
    fn blend(&mut self, p: Option<Body>, u: Body, t: f64, deployment: Option<f64>) {
        self.id = u.id;
        self.deployment = deployment;
        self.member_ids.clear();
        self.member_ids.extend_from_slice(u.ids);
        self.members.clear();
        let Some(p) = p else {
            self.position = u.position;
            self.yaw = u.yaw;
            self.members.extend_from_slice(u.members);
            return;
        };
        self.position = lerp3(p.position, u.position, t);
        self.yaw = p.yaw + delta_angle(p.yaw, u.yaw) * t;
        for (k, &m) in u.members.iter().enumerate() {
            let id = u.ids.get(k).copied().flatten();
            let was = p
                .ids
                .iter()
                .rposition(|&q| q == id)
                .and_then(|i| p.members.get(i));
            self.members.push(was.map_or(m, |&w| lerp3(w, m, t)));
        }
    }
}

pub struct TickInterpolator {
    tick_hz: f64,
    previous: Option<Arc<Observation>>,
    latest: Option<Arc<Observation>>,
    latest_at: f64,
    sample: Option<FrameSample>,
}

impl TickInterpolator {
    pub fn new(tick_hz: f64) -> Self {
        Self {
            tick_hz,
            previous: None,
            latest: None,
            latest_at: 0.0,
            sample: None,
        }
    }

    /// A publication, received at wall-clock `received_at_s`.
    pub fn push(&mut self, observation: Arc<Observation>, received_at_s: f64) {
        self.previous = self.latest.replace(observation);
        self.latest_at = received_at_s;
    }

    /// The presentation clock at wall-clock `now_s`, in simulation seconds:
    /// the previous tick blended toward the latest by how far into a tick
    /// `now_s` is since the latest arrived. It stands still while no tick
    /// arrives, so a paused battle's poses hold. None before the first
    /// publication.
    pub fn time(&self, now_s: f64) -> Option<f64> {
        let latest = self.latest.as_ref()?;
        let from = self.previous.as_ref().map_or(latest.tick, |p| p.tick);
        let fraction = ((now_s - self.latest_at) * self.tick_hz).clamp(0.0, 1.0);
        Some((from + (latest.tick - from) * fraction) / self.tick_hz)
    }

    /// Everything one drawn frame reads of the battle at presentation time
    /// `time_s` (from [`time`](Self::time)): the poses and the publication
    /// they blend toward, so all a frame draws comes from one tick. Rewritten
    /// by every call; None before the first publication.
    pub fn frame(&mut self, time_s: f64) -> Option<&FrameSample> {
        let latest = self.latest.clone()?;
        let previous = self.previous.as_deref();
        let from = previous.map_or(latest.tick, |p| p.tick);
        let span = latest.tick - from;
        let t = if span > 0.0 {
            ((time_s * self.tick_hz - from) / span).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let out = self.sample.get_or_insert_with(|| FrameSample {
            time: time_s,
            observation: latest.clone(),
            own: Vec::new(),
            identified: Vec::new(),
            crashes: Vec::new(),
            shoved: Vec::new(),
        });
        let on_field = || latest.own.iter().filter(|u| !riding(u));
        out.own.resize_with(on_field().count(), Pose::default);
        for (pose, u) in out.own.iter_mut().zip(on_field()) {
            let p = previous.and_then(|o| o.own.iter().find(|p| p.id == u.id));
            // Own units also blend their deployment progress.
            let deployment = u.deployment.as_ref().map(|d| {
                p.and_then(|p| p.deployment.as_ref())
                    .map_or(d.progress, |was| {
                        was.progress + (d.progress - was.progress) * t
                    })
            });
            pose.blend(p.map(Body::from), u.into(), t, deployment);
        }
        out.identified
            .resize_with(latest.identified.len(), Pose::default);
        for (pose, u) in out.identified.iter_mut().zip(&latest.identified) {
            let p = previous.and_then(|o| o.identified.iter().find(|p| p.id == u.id));
            pose.blend(p.map(Body::from), u.into(), t, None);
        }
        out.crashes.clear();
        for c in &latest.crashes {
            let mut c = c.clone();
            if let Some(p) =
                previous.and_then(|o| o.crashes.iter().find(|q| q.id == c.id && q.own == c.own))
            {
                c.position = lerp3(p.position, c.position, t);
                c.yaw = p.yaw + delta_angle(p.yaw, c.yaw) * t;
                c.pitch = p.pitch + (c.pitch - p.pitch) * t;
                c.roll = p.roll + (c.roll - p.roll) * t;
            }
            out.crashes.push(c);
        }
        out.shoved.clear();
        for s in &latest.prop_display_poses {
            let mut s = s.clone();
            if let Some(p) =
                previous.and_then(|o| o.prop_display_poses.iter().find(|q| q.prop_id == s.prop_id))
            {
                s.center = [0, 1].map(|k| p.center[k] + (s.center[k] - p.center[k]) * t);
                s.yaw = p.yaw + delta_angle(p.yaw, s.yaw) * t;
                s.base_z = p.base_z + (s.base_z - p.base_z) * t;
            }
            out.shoved.push(s);
        }
        out.time = time_s;
        out.observation = latest;
        Some(out)
    }
}
