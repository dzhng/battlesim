//! Where a move may end: standing room near the point a unit was sent to.
//! A squad stands as it fits; a hull parks where its whole box stands clear,
//! on ground it can drive to.
use contract::scenario::PushClass;

use super::{cell_center, cell_of, pockets, Mobility, NavGrid, Parking, Pockets, NAV_CELL_M};
use crate::math::{v2, Obb2, V2};

/// The facings a hull with none ordered is tried at for standing room.
const PARKING_FACINGS: usize = 8;

impl NavGrid {
    /// Resolve cell-edge conservatism locally before the formation searches farther.
    pub fn placement_point(&self, p: V2, m: &Mobility) -> Option<V2> {
        self.nearest_standing(p, 1, |q| self.placement_fits(q, m))
    }

    /// Where a unit may end a move near `p`. A hull parks where its box,
    /// facing the way it parks ([`Parking`]), stands clear of every known
    /// body, not even one it could shove in passing, and of every bank and
    /// map edge: a shove is made on the way, and a car pinned against a
    /// wall is not standing room. With no facing ordered it parks facing
    /// the way it drives in: the straight way from `from`, and where its box
    /// stands clear nowhere near facing that way (round a street corner the
    /// way in is nothing like the straight one), any way it stands clear.
    /// It also stands clear of the `taken` boxes: where its side's other
    /// vehicles are parked or have been sent. It needs no room to turn
    /// round there; it backs out the way it came. It must also be ground
    /// the vehicle can drive to from `from`, not a pocket its side knows to
    /// be closed off ([`Pockets`]), across water or up a cliff. Where the hull does not
    /// fit, it parks on the nearest ground it does within a hull's length;
    /// where that ground cannot be reached, on the nearest that can within
    /// [`pockets::POCKET_REACH_M`], and with none that near, it is sent
    /// where it was, and placement leaves it unplaced ([`Self::reaches`]).
    /// A squad (no hull) stands as it fits. Where the room found is a long
    /// way round, the unit may stop short of it on its way
    /// ([`Self::room_on_the_way`]).
    pub fn destination_point(
        &self,
        p: V2,
        m: &Mobility,
        hull: Option<Parking>,
        from: V2,
        pockets: &Pockets,
        taken: &[Obb2],
    ) -> Option<V2> {
        let Some(hull) = hull else {
            return self.placement_point(p, m);
        };
        let stands = |q: V2| self.hull_stands(q, m, hull, from, taken, false);
        let pocketed = std::cell::Cell::new(false);
        let reached = |q: V2| {
            self.reaches(from, q, m, pockets) || {
                pocketed.set(true);
                false
            }
        };
        let parks = |q: V2| stands(q) && reached(q);
        let parks_any_way = |q: V2| self.hull_stands(q, m, hull, from, taken, true) && reached(q);
        let reach = (2.0 * hull.half.length() / NAV_CELL_M).ceil() as isize;
        self.nearest_standing(p, reach, parks)
            .or_else(|| self.nearest_standing(p, reach, parks_any_way))
            .or_else(|| {
                if !pocketed.get() {
                    return None;
                }
                // Nothing it can reach near enough: the ground it was sent
                // to, which placement then leaves unplaced.
                let far = (pockets::POCKET_REACH_M / NAV_CELL_M).ceil() as isize;
                self.nearest_standing(p, far, parks)
                    .or_else(|| self.nearest_standing(p, reach, stands))
            })
    }

    /// Whether `hull` parks at `q`, come from `from`: its box clear of every
    /// known body, bank and map edge, facing as ordered or, with no facing
    /// ordered, the straight way from `from`; `any_way`, any facing it
    /// stands clear at. The box is covered by a row of discs along its
    /// heading, each as wide as the hull and a little over, so the disc
    /// tests the grid answers decide it, a little on the safe side.
    fn hull_stands(
        &self,
        q: V2,
        m: &Mobility,
        hull: Parking,
        from: V2,
        taken: &[Obb2],
        any_way: bool,
    ) -> bool {
        if !self.placement_fits(q, m) {
            return false;
        }
        let (length, width) = (hull.half.x, hull.half.y.max(1e-3));
        // Discs about `width` apart, each spanning the box's full width.
        let discs = ((2.0 * length / width).ceil() as usize).max(1);
        let step = 2.0 * length / discs as f64;
        let radius = libm::hypot(width, step / 2.0);
        let clear = |yaw: f64| {
            let along = v2(libm::cos(yaw), libm::sin(yaw));
            (0..discs).all(|k| {
                let c = q + along * (-length + step * (k as f64 + 0.5));
                self.ground_clear(c, radius, false)
                    && self.bodies_clear(c, radius, PushClass::None)
                    && !taken.iter().any(|held| held.contains(c, radius))
            })
        };
        let d = q - from;
        let facing = hull.facing.unwrap_or_else(|| libm::atan2(d.y, d.x));
        if hull.facing.is_some() || !any_way {
            return clear(facing);
        }
        // A box stands the same turned half round.
        let turn = std::f64::consts::PI / PARKING_FACINGS as f64;
        (1..PARKING_FACINGS).any(|k| clear(facing + turn * k as f64))
    }

    /// Room on the way from `from` to `p` for `hull`: stepping back from
    /// `p` toward `from` a cell at a time, within
    /// [`pockets::POCKET_REACH_M`], the first point where it parks and that
    /// it can drive to. Where a unit stops short of ground it could not
    /// stand on, if the room nearest that ground is a long way round.
    pub fn room_on_the_way(
        &self,
        p: V2,
        m: &Mobility,
        hull: Parking,
        from: V2,
        pockets: &Pockets,
        taken: &[Obb2],
    ) -> Option<V2> {
        let back = from - p;
        let reach = back.length().min(pockets::POCKET_REACH_M);
        let steps = (reach / NAV_CELL_M) as usize;
        (1..=steps)
            .map(|k| p + back * (k as f64 * NAV_CELL_M / back.length()))
            .find(|&q| {
                self.hull_stands(q, m, hull, from, taken, false)
                    && self.reaches(from, q, m, pockets)
            })
    }

    /// `p` if it `fits`, else the nearest cell centre within `reach` cells
    /// either way that does.
    fn nearest_standing(
        &self,
        p: V2,
        reach: isize,
        fits: impl Fn(V2) -> bool + Copy,
    ) -> Option<V2> {
        if fits(p) {
            return Some(p);
        }
        let (i, j) = cell_of(p);
        let mut nearby: Vec<_> = (-reach..=reach)
            .flat_map(|dx| {
                (-reach..=reach).filter_map(move |dy| {
                    self.index(i + dx, j + dy)
                        .map(|k| cell_center(k % self.nx, k / self.nx))
                })
            })
            .collect();
        nearby.sort_by(|a, b| (*a - p).length().total_cmp(&(*b - p).length()));
        nearby.into_iter().find(|&q| fits(q))
    }
}
