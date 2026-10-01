//! Where settlements stand, the open ground kept beside the main one, and the
//! mosaic of districts each is built from.
use super::geometry::{
    add, area, area_above, bearing, direction, distance, ring_distance, round_cm, scale,
    segment_crossing, sub, turn, Outline, Point, PI, TAU,
};
use super::presets::SettlementClass;
use super::rivers;
use super::rng::Stream;
use super::roads::Arm;
use super::water::Water;
use super::Context;
use crate::{CategoryShare, Diagnostic, DistrictPlan, Half, SettlementPlan};
use contract::ground::GroundShape;
use contract::map::{SurfaceArea, SurfaceKind};
use contract::river::River;
use std::collections::{BTreeMap, BTreeSet};

/// One piece of a settlement's ground: a ring of its outline cut into
/// sectors, each built on or left open.
pub struct Sector {
    band: usize,
    /// The outline vertices it spans, counted on past the last to wrap.
    span: core::ops::Range<usize>,
    pub ring: Vec<Point>,
    pub built: bool,
    /// A point inside it, half-way out along its middle.
    anchor: Point,
}

pub struct Site {
    pub class_id: String,
    /// The settlement's envelope; its built ground is the built `sectors`.
    pub outline: Outline,
    pub sectors: Vec<Sector>,
}

impl Site {
    fn built(&self) -> impl Iterator<Item = &Sector> {
        self.sectors.iter().filter(|sector| sector.built)
    }

    /// Open ground between its built districts and a circle of `radius` about `p`.
    pub fn built_gap_to(&self, p: Point, radius: f64) -> f64 {
        self.built()
            .map(|sector| ring_distance(&sector.ring, p))
            .fold(f64::INFINITY, f64::min)
            - radius
    }
}

/// Ground kept clear of settlements and woods: the sector about `center`
/// within `half_angle` of `bearing`, out to `far`.
pub struct Wedge {
    center: Point,
    bearing: f64,
    half_angle: f64,
    far: f64,
}

impl Wedge {
    /// Whether a circle of `radius` about `p` reaches into the wedge.
    pub fn blocks(&self, p: Point, radius: f64) -> bool {
        let away = distance(self.center, p);
        if away - radius > self.far {
            return false;
        }
        if radius >= away {
            return true;
        }
        turn(bearing(self.center, p), self.bearing) <= self.half_angle + libm::asin(radius / away)
    }
}

pub struct Placed {
    /// The main settlement first.
    pub sites: Vec<Site>,
    pub reserved: Vec<Wedge>,
}

impl Placed {
    fn push(&mut self, context: &Context, class_id: &str, outline: Outline) {
        let sectors = ground(context, self.sites.len(), class_id, &outline);
        self.sites.push(Site {
            class_id: class_id.into(),
            outline,
            sectors,
        });
    }
}

/// Site the map's settlements and route its rivers. The main settlement
/// comes first, where a river has a course past it and both halves keep an
/// approach to it that no water crosses; the others then stand clear of each
/// other, of those approaches and of the water. A draw the rest of the map
/// does not fit is drawn again, within the presets' attempts.
pub fn place(
    context: &Context,
    skeleton: &[Arm],
    rivers: &mut rivers::Source,
) -> Result<(Placed, Vec<River>), Vec<Diagnostic>> {
    let presets = context.presets;
    let mut rng = context.stream("sites");
    let class_id = &context.preset.centre.class;
    let class = presets.class(class_id);
    let extent = context.extent;
    let attempts = presets.retries.centre;
    // Why the last draw was dropped.
    let mut refusal = Vec::new();
    for _ in 0..attempts {
        let outline = draw(context, class, None, &mut rng);
        let offset = context
            .preset
            .centre
            .offset
            .map(|share| (2.0 * rng.unit() - 1.0) * share * extent);
        let outline = outline.at([extent / 2.0 + offset[0], extent / 2.0 + offset[1]]);
        let margin = presets.sites.edge_margin_m;
        let inside = outline
            .ring
            .iter()
            .flatten()
            .all(|v| *v >= margin && *v <= extent - margin);
        let Some(rivers) = rivers.past(&outline) else {
            let rules = &presets.rivers;
            refusal = context.fail(
                "river",
                format!(
                    "no course from the north edge to the south keeps {} m from a {class_id}, {} m from the main roads' junctions and {} m from the side edges, after {} courses past each of {attempts} sites",
                    rules.settlement_gap_m, rules.junction_gap_m, rules.side_margin_m, presets.retries.river
                ),
            );
            continue;
        };
        let water = Water::new(&rivers, [extent; 2]);
        let top = reserve(context, &outline, &water, Half::Top, &mut rng);
        let bottom = reserve(context, &outline, &water, Half::Bottom, &mut rng);
        let (true, Some(top), Some(bottom)) = (inside, top, bottom) else {
            refusal = context.fail(
                "approach",
                format!(
                    "no {class_id} of the preset size leaves {} m of open ground across {} m in both halves after {attempts} attempts",
                    presets.approach.depth_m, presets.approach.front_m
                ),
            );
            continue;
        };
        let mut placed = Placed {
            sites: Vec::new(),
            reserved: vec![top, bottom],
        };
        placed.push(context, class_id, outline);
        let ground = Ground {
            skeleton,
            water: &water,
        };
        match settle(context, &ground, &mut placed, &mut rng) {
            Ok(()) => {
                drop(water);
                return Ok((placed, rivers));
            }
            Err(unsettled) => refusal = unsettled,
        }
    }
    Err(refusal)
}

/// What a site is chosen among, beside the settlements already placed: the
/// main roads' lines and the water.
struct Ground<'a> {
    skeleton: &'a [Arm],
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
        let (top, bottom) = halves(&placed.sites, context.extent);
        let lighter = if top <= bottom {
            Half::Top
        } else {
            Half::Bottom
        };
        let class = presets.class(class_id);
        // When the lighter half is behind by an amount this class can make
        // up, the settlement is that size; otherwise the seed draws it.
        let behind = (top - bottom).abs();
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
        placed.push(context, class_id, outline);
    }
    balance(context, ground, placed, rng);
    Ok(())
}

/// Pick one bearing in `half` along which a wedge of the reserved front and
/// depth fits between the outline and the playable edge, with no water in it.
fn reserve(
    context: &Context,
    outline: &Outline,
    water: &Water,
    half: Half,
    rng: &mut Stream,
) -> Option<Wedge> {
    let rule = context.presets.approach;
    let extent = context.extent;
    let base = if half == Half::Top { 0.0 } else { PI };
    let phase = rng.unit();
    let pick = rng.unit();
    let fits: Vec<Wedge> = (0..rule.bearing_candidates)
        .filter_map(|candidate| {
            let bearing =
                base + PI * (f64::from(candidate) + phase) / f64::from(rule.bearing_candidates);
            let half_angle =
                rule.reserve_front_m / (outline.edge(bearing) + rule.depth_m / 2.0) / 2.0;
            let mut far: f64 = 0.0;
            for step in 0..=8 {
                let angle = bearing + half_angle * (f64::from(step) / 4.0 - 1.0);
                let edge = outline.edge(angle);
                let reach = edge + rule.depth_m + rule.reserve_margin_m;
                let end = add(outline.center, scale(direction(angle), reach));
                let middle = add(
                    outline.center,
                    scale(direction(angle), edge + rule.depth_m / 2.0),
                );
                let in_half = (middle[1] >= extent / 2.0) == (half == Half::Top);
                if !in_half || end.iter().any(|v| *v < 0.0 || *v > extent) {
                    return None;
                }
                far = far.max(reach);
            }
            let wedge = Wedge {
                center: outline.center,
                bearing,
                half_angle,
                far,
            };
            let wet = water
                .rivers()
                .iter()
                .flat_map(|river| river.points())
                .any(|point| wedge.blocks(point.xy, point.width_m / 2.0));
            (!wet).then_some(wedge)
        })
        .collect();
    let count = fits.len();
    fits.into_iter().nth((pick * count as f64) as usize)
}

fn draw(
    context: &Context,
    class: &SettlementClass,
    area_m2: Option<f64>,
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
        rotation,
        rng,
    )
}

/// The first site for one settlement that keeps its distance: on a main
/// road's line, beside the main settlement, on a river's bank, or anywhere,
/// as often as the map type says. `side` holds it to a half: its centre
/// stays at least that share of its reach beyond the midline.
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
        let outline = draw(context, class, area_m2, rng);
        let inset = outline.reach + presets.sites.edge_margin_m;
        let mut y = [inset, extent - inset];
        match side {
            Some((Half::Top, share)) => y[0] = y[0].max(extent / 2.0 + share * outline.reach),
            Some((Half::Bottom, share)) => y[1] = y[1].min(extent / 2.0 - share * outline.reach),
            None => (),
        }
        let mode = rng.unit();
        let (a, b) = (rng.unit(), rng.unit());
        let p = if mode < siting.on_road {
            // Strung on a main road: the road will run through its centre.
            let arm = &skeleton[(a * skeleton.len() as f64) as usize];
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
                .any(|wedge| wedge.blocks(p, outline.reach));
        if inside && clear {
            let sited = outline.at(p);
            if water.ring_gap(&sited.ring, bank) >= bank {
                return Some(sited);
            }
        }
    }
    None
}

/// Built area north and south of the midline.
fn halves(sites: &[Site], extent: f64) -> (f64, f64) {
    sites
        .iter()
        .flat_map(Site::built)
        .fold((0.0, 0.0), |(top, bottom), sector| {
            let above = area_above(&sector.ring, extent / 2.0);
            (top + above, bottom + area(&sector.ring) - above)
        })
}

/// Close what is left of the top/bottom difference with a few small
/// settlements in the lighter half, each sized to the difference.
fn balance(context: &Context, ground: &Ground, placed: &mut Placed, rng: &mut Stream) {
    let presets = context.presets;
    let playable = context.extent * context.extent;
    for _ in 0..presets.retries.repair_settlements {
        let (top, bottom) = halves(&placed.sites, context.extent);
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
                (low * 1e4 <= difference).then_some((class_id, class, difference.min(high * 1e4)))
            })
            .collect();
        fitting.sort_by_key(|(_, class, _)| core::cmp::Reverse(class.rank));
        let side = Some((lighter, 1.0));
        let added = fitting.into_iter().find_map(|(class_id, class, size)| {
            let outline = site(context, ground, placed, class, Some(size), side, rng)?;
            Some((class_id, outline))
        });
        match added {
            Some((class_id, outline)) => placed.push(context, class_id, outline),
            None => return,
        }
    }
}

/// Cut a settlement's outline into its bands of sectors and decide which are
/// built. Larger settlements leave some open and stop others short of the
/// edge, so the town is a loose group of districts with fields and woods
/// reaching in, not a filled disc.
fn ground(context: &Context, index: usize, class_id: &str, outline: &Outline) -> Vec<Sector> {
    let class = context.presets.class(class_id);
    // Its own stream: reshaping one town never moves another, or any site.
    let mut rng = context.stream(&format!("ground/{index}"));
    let ring = &outline.ring;
    let center = outline.center;
    let count = ring.len();
    let at = |vertex: usize, share: f64| {
        let vertex = ring[vertex % count];
        if share == 1.0 {
            vertex
        } else {
            round_cm(add(center, scale(sub(vertex, center), share)))
        }
    };
    let mut sectors = Vec::new();
    let mut from = 0.0;
    for (band_index, band) in class.bands.iter().enumerate() {
        let pieces = band.sectors as usize;
        let start = rng.below(count as u64) as usize;
        // Open sectors are taken turn about from the settlement's north and
        // south sides, so one that sits on the midline gives each half the
        // same built ground.
        let middle = |piece: usize| start + (2 * piece + 1) * count / (2 * pieces);
        let mut sides: [Vec<usize>; 2] = [Vec::new(), Vec::new()];
        for piece in 0..pieces {
            sides[usize::from(at(middle(piece), 1.0)[1] < center[1])].push(piece);
        }
        let mut open = Vec::new();
        let mut side = rng.below(2) as usize;
        for _ in 0..(band.open * pieces as f64 + rng.unit()) as usize {
            if sides[side].is_empty() {
                side = 1 - side;
            }
            let pick = rng.below(sides[side].len() as u64) as usize;
            open.push(sides[side].swap_remove(pick));
            side = 1 - side;
        }
        for piece in 0..pieces {
            let built = !open.contains(&piece);
            let depth = rng.range([band.ragged, 1.0]);
            let to = if built {
                from + (band.to - from) * depth
            } else {
                band.to
            };
            let span = if pieces == 1 {
                0..count
            } else {
                start + piece * count / pieces..start + (piece + 1) * count / pieces
            };
            let whole = pieces == 1 && from == 0.0;
            let mut ring: Vec<Point> = if pieces == 1 {
                span.clone().map(|vertex| at(vertex, to)).collect()
            } else {
                (span.start..=span.end)
                    .map(|vertex| at(vertex, to))
                    .collect()
            };
            if pieces > 1 {
                if from == 0.0 {
                    ring.push(center);
                } else {
                    ring.extend((span.start..=span.end).rev().map(|vertex| at(vertex, from)));
                }
            }
            let anchor = if whole {
                center
            } else {
                at((span.start + span.end) / 2, (from + to) / 2.0)
            };
            sectors.push(Sector {
                band: band_index,
                span,
                ring,
                built,
                anchor,
            });
        }
        from = band.to;
    }
    sectors
}

/// The plan record of one settlement: each built sector becomes a district
/// of one kind. A sector a main road crosses the settlement's edge in picks
/// from the band's roadside kinds, so industry lines the road.
pub fn settlement(
    context: &Context,
    index: usize,
    site: &Site,
    surfaces: &[SurfaceArea],
) -> SettlementPlan {
    let presets = context.presets;
    let class = presets.class(&site.class_id);
    let ring = &site.outline.ring;
    let count = ring.len();
    let center = site.outline.center;
    let crossed: BTreeSet<usize> = surfaces
        .iter()
        .filter(|area| area.kind <= SurfaceKind::CountryRoad)
        .flat_map(|area| {
            match &area.shape {
                GroundShape::Stroke { centerline, .. } => centerline.control_points(),
                GroundShape::Polygon { .. } => &[],
            }
            .windows(2)
        })
        .filter(|run| {
            run.iter()
                .any(|p| distance(*p, center) <= site.outline.reach + distance(run[0], run[1]))
        })
        .flat_map(|run| {
            (0..count).filter(|edge| {
                segment_crossing(run[0], run[1], ring[*edge], ring[(*edge + 1) % count]).is_some()
            })
        })
        .collect();
    let mut rng = context.stream(&format!("districts/{index}"));
    let districts = site
        .built()
        .enumerate()
        .map(|(number, sector)| {
            let band = &class.bands[sector.band];
            let roadside = sector
                .span
                .clone()
                .any(|vertex| crossed.contains(&(vertex % count)));
            let weights: &BTreeMap<String, f64> = match &band.roadside {
                Some(roadside_weights) if roadside => roadside_weights,
                _ => &band.districts,
            };
            let mut pick = rng.unit() * weights.values().sum::<f64>();
            let kind = weights
                .iter()
                .find(|(_, weight)| {
                    pick -= **weight;
                    pick < 0.0
                })
                .or(weights.iter().next_back())
                .map(|(kind, _)| kind.clone())
                .unwrap_or_default();
            DistrictPlan {
                id: format!("settlement-{index}/district-{number}"),
                categories: presets.districts[&kind]
                    .mix
                    .iter()
                    .map(|(category, weight)| CategoryShare {
                        category: *category,
                        weight: *weight,
                    })
                    .collect(),
                kind,
                ring: sector.ring.clone(),
                area_m2: libm::round(area(&sector.ring)),
                anchor: sector.anchor,
                max_floors: context.preset.max_floors,
            }
        })
        .collect();
    SettlementPlan {
        id: format!("settlement-{index}"),
        class: site.class_id.clone(),
        center,
        outline: ring.clone(),
        districts,
    }
}
