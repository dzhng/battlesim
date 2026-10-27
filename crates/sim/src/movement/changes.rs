//! A side's planning changes: each body it learns, sees moved or sees gone
//! bumps its revision and is logged, with the circle round where it stood,
//! so a unit replans only for a change near its way, and weighs again only
//! the bodies that changed.
use super::SideGeometry;
use crate::math::V2;
use crate::world::{Prop, PropId};

/// A side remembers where its latest this many planning changes were.
const CHANGE_LOG: usize = 1024;

/// One planning change: the body learned, moved or gone, and the circle
/// round every place the side planned it at before and after.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Change {
    revision: u64,
    prop: PropId,
    at: V2,
    radius: f64,
}

impl Change {
    pub(super) fn digest(&self, d: &mut crate::digest::Digest) {
        d.u64(self.revision)
            .u64(self.prop as u64)
            .f64(self.at.x)
            .f64(self.at.y)
            .f64(self.radius);
    }
}

/// The circle round `prop`'s footprint wherever `seen` (if anywhere) and
/// where it stands now.
pub(super) fn span(prop: &Prop, seen: Option<V2>) -> (V2, f64) {
    let r = prop.footprint_radius();
    match seen {
        Some(was) => (
            (was + prop.center) * 0.5,
            (prop.center - was).length() * 0.5 + r,
        ),
        None => (prop.center, r),
    }
}

impl SideGeometry {
    /// Bump the revision for a change to `prop` round `(at, radius)`.
    pub(super) fn changed(&mut self, prop: PropId, (at, radius): (V2, f64)) {
        self.revision += 1;
        self.changes.push_back(Change {
            revision: self.revision,
            prop,
            at,
            radius,
        });
        if self.changes.len() > CHANGE_LOG {
            self.changes.pop_front();
        }
    }

    /// The changes logged after revision `since`, newest first; `None` once
    /// the log no longer reaches back that far, when any body may have.
    fn after(&self, since: u64) -> Option<impl Iterator<Item = &Change>> {
        let lost =
            since < self.revision && self.changes.front().is_none_or(|c| c.revision > since + 1);
        (!lost).then(|| {
            self.changes
                .iter()
                .rev()
                .take_while(move |c| c.revision > since)
        })
    }

    /// Whether a replan after revision `since` weighs body `id` again (Q13):
    /// one that changed since, unless the hull is `pressed` against it. That
    /// one moves with the hull, so weighing it again would swing the route
    /// from side to side; the rest were weighed when the route was planned.
    pub fn reweighs(&self, since: u64, pressed: Vec<PropId>) -> impl Fn(PropId) -> bool {
        let changed: Option<Vec<PropId>> = self.after(since).map(|c| c.map(|c| c.prop).collect());
        move |id| !pressed.contains(&id) && changed.as_ref().is_none_or(|c| c.contains(&id))
    }

    /// Whether any change after revision `since` came within `reach` of
    /// `at`: always, once the log no longer reaches back that far.
    pub fn changed_near(&self, since: u64, at: V2, reach: f64) -> bool {
        self.after(since)
            .is_none_or(|mut c| c.any(|c| (c.at - at).within_radius(reach + c.radius)))
    }

    /// Whether any change after revision `since` came within `reach` of
    /// the way from `from` along `route`: always, once the log no longer
    /// reaches back that far.
    pub fn changed_along(&self, since: u64, from: V2, route: &[V2], reach: f64) -> bool {
        self.after(since).is_none_or(|mut changes| {
            changes.any(|c| {
                let mut a = from;
                route.iter().any(|&b| {
                    let ab = b - a;
                    let len2 = ab.dot(ab);
                    let t = if len2 > 0.0 {
                        ((c.at - a).dot(ab) / len2).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    let near = (a + ab * t - c.at).within_radius(reach + c.radius);
                    a = b;
                    near
                })
            })
        })
    }
}
