//! A side's planning changes: each body it learns, sees moved or sees gone
//! bumps its revision and is logged as the circle round where it stood, so
//! a unit replans only for a change near its way.
use super::SideGeometry;
use crate::math::V2;
use crate::world::Prop;

/// A side remembers where its latest this many planning changes were.
const CHANGE_LOG: usize = 1024;

/// One planning change (a body learned, moved or gone), as the circle round
/// every place the side planned it at before and after.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Change {
    revision: u64,
    at: V2,
    radius: f64,
}

impl Change {
    pub(super) fn digest(&self, d: &mut crate::digest::Digest) {
        d.u64(self.revision)
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
    /// Bump the revision for a change round `(at, radius)`.
    pub(super) fn changed(&mut self, (at, radius): (V2, f64)) {
        self.revision += 1;
        self.changes.push_back(Change {
            revision: self.revision,
            at,
            radius,
        });
        if self.changes.len() > CHANGE_LOG {
            self.changes.pop_front();
        }
    }

    /// Whether any change after revision `since` came within `reach` of
    /// `at`: always, once the log no longer reaches back that far.
    pub fn changed_near(&self, since: u64, at: V2, reach: f64) -> bool {
        if since >= self.revision {
            return false;
        }
        if self.changes.front().is_none_or(|c| c.revision > since + 1) {
            return true;
        }
        self.changes
            .iter()
            .rev()
            .take_while(|c| c.revision > since)
            .any(|c| (c.at - at).within_radius(reach + c.radius))
    }

    /// Whether any change after revision `since` came within `reach` of
    /// the way from `from` along `route`: always, once the log no longer
    /// reaches back that far.
    pub fn changed_along(&self, since: u64, from: V2, route: &[V2], reach: f64) -> bool {
        if since >= self.revision {
            return false;
        }
        if self.changes.front().is_none_or(|c| c.revision > since + 1) {
            return true;
        }
        self.changes
            .iter()
            .rev()
            .take_while(|c| c.revision > since)
            .any(|c| {
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
    }
}
