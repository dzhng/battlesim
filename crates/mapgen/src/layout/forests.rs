//! Woods: the seed sets how much of the map they cover, each half gets the
//! same share, and none stands on built ground, a reserved approach or a
//! river's bank. They may come right up to a town and fill the ground it
//! leaves open.
use super::geometry::{add, area, area_above, distance, scale, Outline, Point, PI};
use super::presets::OutlineShape;
use super::rng::Stream;
use super::sites::Placed;
use super::water::Water;
use super::Context;
use crate::Half;
use contract::ground::GroundShape;
use contract::map::Forest;

struct Woods<'a> {
    context: &'a Context<'a>,
    placed: &'a Placed,
    water: &'a Water<'a>,
    rng: Stream,
    /// Each wood's bounding circle, to keep later woods off it.
    circles: Vec<(Point, f64)>,
    forests: Vec<Forest>,
    /// Wooded area north and south of the midline.
    top: f64,
    bottom: f64,
}

pub fn grow(context: &Context, placed: &Placed, water: &Water, woodland: f64) -> Vec<Forest> {
    let rules = &context.presets.forests;
    let extent = context.extent;
    let [low, high] = context.preset.forest_share;
    // Each half is filled to half the seed's coverage, so the halves agree
    // without mirroring a single wood.
    let half_target = (low + (high - low) * woodland) * extent * extent / 2.0;
    let large_chance =
        rules.large_chance[0] + (rules.large_chance[1] - rules.large_chance[0]) * woodland;
    let least = PI * rules.min_radius_m * rules.min_radius_m;
    let mut woods = Woods {
        context,
        placed,
        water,
        rng: context.stream("forests"),
        circles: Vec::new(),
        forests: Vec::new(),
        top: 0.0,
        bottom: 0.0,
    };
    // Woods first take some of the ground settlements left open between
    // their districts, where both halves still have room for them.
    for sector in placed.sites.iter().flat_map(|site| &site.sectors) {
        if sector.built || !woods.rng.chance(rules.infill_chance) {
            continue;
        }
        let above = area_above(&sector.ring, extent / 2.0);
        let below = area(&sector.ring) - above;
        if woods.top + above <= half_target && woods.bottom + below <= half_target {
            woods.claim(&sector.ring);
        }
    }
    for _ in 0..rules.max_woods {
        let half = if woods.top <= woods.bottom {
            Half::Top
        } else {
            Half::Bottom
        };
        let wanted = half_target - woods.top.min(woods.bottom);
        if wanted < least {
            break;
        }
        let radius = if woods.rng.chance(large_chance) {
            woods.rng.range(rules.large_radius_m)
        } else {
            woods.rng.range(rules.small_radius_m)
        };
        let aspect = woods.rng.range(rules.aspect);
        let size = (PI * radius * radius * aspect).min(wanted);
        // It may reach over the midline only as far as the other half has room.
        let spill = (half_target - woods.top.max(woods.bottom)).max(0.0);
        woods.scatter(size, aspect, half, spill);
    }
    // Woods that meet the playable edge are cut short, which leaves the
    // halves a little apart: close that with woods of exactly the difference.
    for _ in 0..context.presets.retries.repair_woods {
        let difference = (woods.top - woods.bottom).abs();
        let half = if woods.top < woods.bottom {
            Half::Top
        } else {
            Half::Bottom
        };
        let aspect = woods.rng.range(rules.aspect);
        if difference < least || !woods.scatter(difference, aspect, half, 0.0) {
            break;
        }
    }
    woods.forests
}

impl Woods<'_> {
    /// Stand a wood on `ring` if the shared ground contract admits the ring.
    fn claim(&mut self, ring: &[Point]) -> bool {
        let Ok(shape) = GroundShape::polygon(ring.to_vec()) else {
            return false;
        };
        let above = area_above(ring, self.context.extent / 2.0);
        self.top += above;
        self.bottom += area(ring) - above;
        let sum = ring.iter().fold([0.0, 0.0], |sum, p| add(sum, *p));
        let center = scale(sum, 1.0 / ring.len() as f64);
        let reach = ring
            .iter()
            .map(|p| distance(center, *p))
            .fold(0.0, f64::max);
        self.circles.push((center, reach));
        self.forests.push(Forest { shape });
        true
    }

    /// Stand one wood of `size` centred in `half`, with at most `spill` of it
    /// over the midline, clear of built ground, reserved approaches, water
    /// and other woods. False when no attempt finds room.
    fn scatter(&mut self, size: f64, aspect: f64, half: Half, spill: f64) -> bool {
        let context = self.context;
        let rules = &context.presets.forests;
        let extent = context.extent;
        let shape = OutlineShape {
            exponent: 2.0,
            noise: rules.outline_noise,
            points: rules.outline_points,
        };
        for _ in 0..context.presets.retries.forest {
            let rotation = self.rng.range([0.0, PI]);
            let outline = Outline::draw(shape, size, aspect, rotation, &mut self.rng);
            let y = match half {
                Half::Top => [extent / 2.0, extent],
                Half::Bottom => [0.0, extent / 2.0],
            };
            let p = [self.rng.range([0.0, extent]), self.rng.range(y)];
            let reach = outline.reach;
            let clear = self
                .placed
                .sites
                .iter()
                .all(|site| site.built_gap_to(p, reach) >= rules.settlement_gap_m)
                && !self
                    .placed
                    .reserved
                    .iter()
                    .any(|wedge| wedge.blocks(p, reach))
                && self.circles.iter().all(|(center, other)| {
                    distance(*center, p) >= other + reach + rules.forest_gap_m
                });
            if !clear {
                continue;
            }
            let wood = outline.at(p).clipped(extent);
            let bank = context.presets.rivers.forest_gap_m;
            if self.water.ring_gap(&wood.ring, bank) < bank {
                continue;
            }
            let above = area_above(&wood.ring, extent / 2.0);
            let over = match half {
                Half::Top => area(&wood.ring) - above,
                Half::Bottom => above,
            };
            if over <= spill && self.claim(&wood.ring) {
                return true;
            }
        }
        false
    }
}
