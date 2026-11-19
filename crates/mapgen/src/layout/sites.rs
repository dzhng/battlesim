//! Where settlements stand and the open ground kept beside the main one. A
//! site is the ground a settlement may build on; `towns` cuts it into
//! districts once the roads through it are laid.
use super::geometry::{
    add, area, area_above, bearing, direction, scale, sub, Outline, Point, PI, TAU,
};
use super::presets::SettlementClass;
use super::rivers;
use super::rng::Stream;
use super::roads::Skeleton;
use super::water::Water;
use super::Context;
use crate::{Diagnostic, Half};
use contract::river::River;

pub struct Site {
    pub class_id: String,
    /// The ground it may build on: convex, with a few straight sides. Its
    /// districts cover the class's share of it.
    pub outline: Outline,
}

/// Ground kept clear of settlements and woods: the corridor from `center`
/// along the unit vector `toward`, `half_width` to either side, out to `far`.
pub struct Corridor {
    center: Point,
    toward: Point,
    half_width: f64,
    far: f64,
}

impl Corridor {
    pub(crate) fn objective(center: Point, radius: f64) -> Self {
        Self {
            center: [center[0] - radius, center[1]],
            toward: [1.0, 0.0],
            half_width: radius,
            far: 2.0 * radius,
        }
    }
    /// Whether a circle of `radius` about `p` reaches into the corridor.
    pub fn blocks(&self, p: Point, radius: f64) -> bool {
        let offset = sub(p, self.center);
        let along = offset[0] * self.toward[0] + offset[1] * self.toward[1];
        let aside = (self.toward[0] * offset[1] - self.toward[1] * offset[0]).abs();
        along + radius >= 0.0 && along - radius <= self.far && aside - radius <= self.half_width
    }
}

pub struct Placed {
    /// The main settlement first.
    pub sites: Vec<Site>,
    pub reserved: Vec<Corridor>,
}

impl Placed {
    fn push(&mut self, class_id: &str, outline: Outline) {
        self.sites.push(Site {
            class_id: class_id.into(),
            outline,
        });
    }
}

/// Site the map's settlements and route its rivers. The main settlement
/// comes first, on the main junction, where a river has a course past it
/// and both halves keep an approach to it that no water crosses; the others
/// then stand clear of each other, of those approaches and of the water. A
/// draw the rest of the map does not fit is drawn again, within the
/// presets' attempts.
pub fn place(
    context: &Context,
    skeleton: &Skeleton,
    rivers: &mut rivers::Source,
) -> Result<(Placed, Vec<River>), Vec<Diagnostic>> {
    let presets = context.presets;
    let mut rng = context.stream("sites");
    let class_id = &context.cell.centre.class;
    let class = presets.class(class_id);
    let extent = context.extent;
    let attempts = presets.retries.centre;
    // Why the draw that got farthest was dropped: (how far, the refusal).
    let mut refusal: (u8, Vec<Diagnostic>) = (0, Vec::new());
    for _ in 0..attempts {
        let outline = draw(context, class, None, None, &mut rng).at(skeleton.hub);
        let margin = presets.sites.edge_margin_m;
        let inside = outline
            .ring
            .iter()
            .flatten()
            .all(|v| *v >= margin && *v <= extent - margin);
        let Some(rivers) = rivers.past(&outline) else {
            let rules = &presets.rivers;
            let stuck = context.fail(
                "river",
                format!(
                    "no course from the north edge to the south keeps {} m from a {class_id}, {} m from the main roads' junctions and {} m from the side edges, after {} courses past each of {attempts} sites",
                    rules.settlement_gap_m, rules.junction_gap_m, rules.side_margin_m, presets.retries.river
                ),
            );
            if refusal.0 == 0 {
                refusal = (0, stuck);
            }
            continue;
        };
        let water = Water::new(&rivers, [extent; 2]);
        let top = reserve(context, &outline, &water, Half::Top, &mut rng);
        let bottom = reserve(context, &outline, &water, Half::Bottom, &mut rng);
        let (true, Some(top), Some(bottom)) = (inside, top, bottom) else {
            let stuck = context.fail(
                "approach",
                format!(
                    "no {class_id} of the preset size leaves {} m of open ground across {} m in both halves after {attempts} attempts",
                    presets.approach.depth_m(extent), presets.approach.front_m
                ),
            );
            if refusal.0 <= 1 {
                refusal = (1, stuck);
            }
            continue;
        };
        let mut placed = Placed {
            sites: Vec::new(),
            reserved: vec![top, bottom],
        };
        placed.push(class_id, outline);
        let ground = Ground {
            skeleton,
            water: &water,
        };
        match settle(context, &ground, &mut placed, &mut rng) {
            Ok(()) => {
                drop(water);
                return Ok((placed, rivers));
            }
            Err(unsettled) => refusal = (2, unsettled),
        }
    }
    Err(refusal.1)
}

/// What a site is chosen among, beside the settlements already placed: the
/// main roads' lines and the water.
struct Ground<'a> {
    skeleton: &'a Skeleton,
    water: &'a Water<'a>,
}

/// Site every settlement after the main one.
fn settle(
    context: &Context,
    ground: &Ground,
    placed: &mut Placed,
    rng: &mut Stream,
) -> Result<(), Vec<Diagnostic>> {
    let presets = context.presets;
    let mut wanted: Vec<&String> = Vec::new();
    for (class_id, count) in &context.cell.settlements {
        for _ in 0..rng.count(*count) {
            wanted.push(class_id);
        }
    }
    // Largest first: a town needs room a hamlet can always find later.
    wanted.sort_by_key(|class_id| core::cmp::Reverse(presets.class(class_id).rank));
    for class_id in wanted {
        let (top, bottom) = halves(context, &placed.sites);
        let lighter = if top <= bottom {
            Half::Top
        } else {
            Half::Bottom
        };
        let class = presets.class(class_id);
        // When the lighter half is behind by an amount this class can make
        // up, the settlement is the size that builds it; otherwise the seed
        // draws it.
        let behind = (top - bottom).abs() / built_share(class);
        let size = class
            .area_ha
            .filter(|[low, _]| behind >= low * 1e4)
            .map(|[_, high]| behind.min(high * 1e4));
        // Mostly in the lighter half; anywhere if that half has no room.
        let side = Some((lighter, 0.3));
        let outline = site(context, ground, placed, class, size, side, rng)
            .or_else(|| site(context, ground, placed, class, None, None, rng))
            .ok_or_else(|| {
                context.fail(
                    &format!("settlement-{}", placed.sites.len()),
                    format!(
                        "no site for a {class_id} keeps {} m from its neighbours after {} attempts",
                        context.preset.gap_m, presets.retries.site
                    ),
                )
            })?;
        placed.push(class_id, outline);
    }
    balance(context, ground, placed, rng);
    Ok(())
}

/// Pick one bearing in `half` along which a corridor of the reserved front
/// runs from the settlement, past the limit of its ground and the reserved
/// depth beyond, inside the playable area with no water in it. The measured
/// approach then starts wherever the settlement's districts end along it.
fn reserve(
    context: &Context,
    outline: &Outline,
    water: &Water,
    half: Half,
    rng: &mut Stream,
) -> Option<Corridor> {
    let rule = &context.presets.approach;
    let depth_m = rule.depth_m(context.extent);
    let extent = context.extent;
    let base = if half == Half::Top { 0.0 } else { PI };
    let phase = rng.unit();
    let pick = rng.unit();
    let fits: Vec<Corridor> = (0..rule.bearing_candidates)
        .filter_map(|candidate| {
            let bearing =
                base + PI * (f64::from(candidate) + phase) / f64::from(rule.bearing_candidates);
            let toward = direction(bearing);
            let aside = [-toward[1], toward[0]];
            // The farthest its ground reaches along the bearing.
            let limit = outline
                .ring
                .iter()
                .map(|p| {
                    let offset = sub(*p, outline.center);
                    offset[0] * toward[0] + offset[1] * toward[1]
                })
                .fold(0.0, f64::max);
            let far = limit + depth_m + rule.reserve_margin_m;
            let half_width = rule.reserve_front_m / 2.0;
            let at = |along: f64, across: f64| {
                add(
                    outline.center,
                    add(scale(toward, along), scale(aside, across)),
                )
            };
            let inside = [-half_width, half_width]
                .into_iter()
                .flat_map(|across| [at(0.0, across), at(far, across)])
                .all(|p| p.iter().all(|v| *v >= 0.0 && *v <= extent));
            // Wherever its districts end, the corridor's middle is in this half.
            let in_half = [0.0, limit].into_iter().all(|edge| {
                (at(edge + depth_m / 2.0, 0.0)[1] >= extent / 2.0) == (half == Half::Top)
            });
            let corridor = Corridor {
                center: outline.center,
                toward,
                half_width,
                far,
            };
            let wet = water
                .rivers()
                .iter()
                .flat_map(|river| river.points())
                .any(|point| corridor.blocks(point.xy, point.width_m / 2.0));
            (inside && in_half && !wet).then_some(corridor)
        })
        .collect();
    let count = fits.len();
    fits.into_iter().nth((pick * count as f64) as usize)
}

/// The ground of one settlement of `class`: of `area_m2` when given, and
/// with its long axis along `axis` when given; otherwise as the seed draws.
fn draw(
    context: &Context,
    class: &SettlementClass,
    area_m2: Option<f64>,
    axis: Option<f64>,
    rng: &mut Stream,
) -> Outline {
    let playable = context.extent * context.extent;
    let drawn = match (class.area_ha, class.area_share) {
        (Some(hectares), _) => rng.range(hectares) * 1e4,
        (None, Some(share)) => rng.range(share) * playable,
        (None, None) => unreachable!("presets admit a class only with a size"),
    };
    let aspect = rng.range(class.aspect);
    let rotation = rng.range(class.rotation);
    Outline::draw(
        class.outline,
        area_m2.unwrap_or(drawn),
        aspect,
        axis.unwrap_or(rotation),
        rng,
    )
    .hull()
}

/// The share of a class's ground its districts are expected to cover.
fn built_share(class: &SettlementClass) -> f64 {
    (class.built_share[0] + class.built_share[1]) / 2.0
}

/// The first site for one settlement that keeps its distance: on a main
/// road's line, beside the main settlement, on a river's bank, or anywhere,
/// as often as the map type says. One on a main road lies along it. `side`
/// holds it to a half: its centre stays at least that share of its reach
/// beyond the midline.
fn site(
    context: &Context,
    ground: &Ground,
    placed: &Placed,
    class: &SettlementClass,
    area_m2: Option<f64>,
    side: Option<(Half, f64)>,
    rng: &mut Stream,
) -> Option<Outline> {
    let presets = context.presets;
    let extent = context.extent;
    let siting = context.preset.siting;
    let (skeleton, water) = (ground.skeleton, ground.water);
    let bank = presets.rivers.settlement_gap_m;
    let main = &placed.sites[0].outline;
    for _ in 0..presets.retries.site {
        let mode = rng.unit();
        let (a, b) = (rng.unit(), rng.unit());
        let arms = &skeleton.arms;
        let arm = &arms[(a * arms.len() as f64) as usize];
        let on_road = mode < siting.on_road;
        let axis = on_road.then(|| bearing(arm.exit, arm.target));
        let outline = draw(context, class, area_m2, axis, rng);
        let inset = outline.reach + presets.sites.edge_margin_m;
        let mut y = [inset, extent - inset];
        match side {
            Some((Half::Top, share)) => y[0] = y[0].max(extent / 2.0 + share * outline.reach),
            Some((Half::Bottom, share)) => y[1] = y[1].min(extent / 2.0 - share * outline.reach),
            None => (),
        }
        let p = if on_road {
            // Strung on a main road: the road will run through its centre.
            add(arm.exit, scale(sub(arm.target, arm.exit), 0.12 + 0.76 * b))
        } else if mode < siting.on_road + siting.near_main {
            // One of the cluster around the main settlement.
            let toward = TAU * a;
            let away = main.edge(toward)
                + context.preset.gap_m
                + outline.reach
                + b * presets.sites.cluster_reach_m;
            add(main.center, scale(direction(toward), away))
        } else if mode >= 1.0 - siting.beside_river && !water.is_empty() {
            // On a river's bank: as near the water as its own edge allows.
            let (edge, away) = water.shore(a, b);
            let toward = bearing(away, [0.0, 0.0]);
            add(edge, scale(away, bank + outline.edge(toward)))
        } else {
            [inset + a * (extent - 2.0 * inset), y[0] + b * (y[1] - y[0])]
        };
        let inside = p[0] >= inset && p[0] <= extent - inset && p[1] >= y[0] && p[1] <= y[1];
        let clear = placed
            .sites
            .iter()
            .all(|other| other.outline.gap_to(p, outline.reach) >= context.preset.gap_m)
            && !placed
                .reserved
                .iter()
                .any(|corridor| corridor.blocks(p, outline.reach));
        if inside && clear {
            let sited = outline.at(p);
            if water.ring_gap(&sited.ring, bank) >= bank {
                return Some(sited);
            }
        }
    }
    None
}

/// The built ground expected north and south of the midline: each site's
/// ground there, by the share of it its class builds on. `towns` closes
/// what the districts then differ by.
fn halves(context: &Context, sites: &[Site]) -> (f64, f64) {
    let middle = context.extent / 2.0;
    sites.iter().fold((0.0, 0.0), |(top, bottom), site| {
        let share = built_share(context.presets.class(&site.class_id));
        let above = area_above(&site.outline.ring, middle);
        (
            top + share * above,
            bottom + share * (area(&site.outline.ring) - above),
        )
    })
}

/// Close what is left of the top/bottom difference with a few small
/// settlements in the lighter half, each sized to the difference.
fn balance(context: &Context, ground: &Ground, placed: &mut Placed, rng: &mut Stream) {
    let presets = context.presets;
    let playable = context.extent * context.extent;
    for _ in 0..presets.retries.repair_settlements {
        let (top, bottom) = halves(context, &placed.sites);
        let difference = (top - bottom).abs();
        // Aim well inside the tolerance: a split at its limit reads as uneven.
        if difference <= 0.5 * presets.fairness.town.allowance(top, bottom, playable) {
            return;
        }
        let lighter = if top < bottom {
            Half::Top
        } else {
            Half::Bottom
        };
        // The largest class of this map that the difference can hold and the
        // lighter half has room for.
        let mut fitting: Vec<(&String, &SettlementClass, f64)> = context
            .cell
            .settlements
            .keys()
            .filter_map(|class_id| {
                let class = presets.class(class_id);
                let [low, high] = class.area_ha?;
                let ground = difference / built_share(class);
                (low * 1e4 <= ground).then_some((class_id, class, ground.min(high * 1e4)))
            })
            .collect();
        fitting.sort_by_key(|(_, class, _)| core::cmp::Reverse(class.rank));
        let side = Some((lighter, 1.0));
        let added = fitting.into_iter().find_map(|(class_id, class, size)| {
            let outline = site(context, ground, placed, class, Some(size), side, rng)?;
            Some((class_id, outline))
        });
        match added {
            Some((class_id, outline)) => placed.push(class_id, outline),
            None => return,
        }
    }
}
