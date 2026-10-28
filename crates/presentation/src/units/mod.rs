//! Units: interpolation between ticks, the pose feed and driver,
//! articulation, joint palettes and where each drawn model's parts are.
pub mod articulation;
pub mod drawn;
pub mod driver;
pub mod feed;
pub mod interpolate;
pub mod pose;
pub mod skin;

use sim::publication::reader::{Host, OwnUnit};

use crate::world::Vec3;

/// Aboard a carrier past its entry timer: inside, or still inside while
/// leaving it. A rider is off the field: nothing of it is drawn, picked,
/// heard or wades.
pub fn riding(u: &OwnUnit) -> bool {
    u.hold
        .as_ref()
        .is_some_and(|h| matches!(h.host, Host::Carrier { .. }) && &*h.phase != "entering")
}

/// The signed turn from `current` to `target`, in (−π, π] (`math`'s `deltaAngle`).
pub fn delta_angle(current: f64, target: f64) -> f64 {
    let tau = std::f64::consts::TAU;
    let diff = target - current;
    let delta = diff - tau * (diff / tau).floor();
    if delta > std::f64::consts::PI {
        delta - tau
    } else {
        delta
    }
}

fn lerp3(a: Vec3, b: Vec3, t: f64) -> Vec3 {
    [
        a[0] + t * (b[0] - a[0]),
        a[1] + t * (b[1] - a[1]),
        a[2] + t * (b[2] - a[2]),
    ]
}

/// The `n`th entry of `list`, to be rewritten in place: a frame's records
/// reuse last frame's, and their buffers.
fn slot<T: Default>(list: &mut Vec<T>, n: usize) -> &mut T {
    if n == list.len() {
        list.push(T::default());
    }
    &mut list[n]
}
