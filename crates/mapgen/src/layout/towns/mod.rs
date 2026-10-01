//! A settlement's districts. Its ground is cut along the roads that cross
//! it, each piece is cut again into blocks a road's depth deep, and the
//! settlement grows block by block from where its roads meet until it covers
//! its class's share of the ground. A block is one district; the settlement's
//! outline is the edge of its blocks; the cuts between blocks are its
//! avenues.
use crate::layout::geometry::{
    add, area, area_above, centroid, cross, distance, ring_distance, round_cm, scale,
    segment_distance, sub, Point,
};
use crate::layout::presets::{SettlementClass, Zone};
use crate::layout::roads::Road;
use crate::layout::sites::Site;
use crate::layout::Context;
use crate::{CategoryShare, Diagnostic, DistrictPlan, SettlementPlan};
use contract::ground::GroundShape;
use contract::map::{SurfaceArea, SurfaceKind};
use std::collections::BTreeSet;

mod ground;
mod mesh;
mod streets;

use ground::{pieces, Bound, Ground, Leaf, MEET_M};
use mesh::Mesh;
use streets::{avenues, touch};

/// One built block: a district.
pub struct Block {
    pub ring: Vec<Point>,
    pub anchor: Point,
    pub kind: String,
}

/// A settlement as it stands once its roads are laid.
pub struct Town {
    /// Where its main streets meet: the point of its roads nearest the
    /// middle of its ground.
    pub center: Point,
    /// The edge of its districts and of any open ground they enclose.
    pub outline: Vec<Point>,
    /// Its districts, nearest the centre first.
    pub blocks: Vec<Block>,
    /// The blocks it left open beside and among its districts.
    pub greens: Vec<Vec<Point>>,
    /// The streets along its blocks' edges where no road runs.
    avenues: Vec<[Point; 2]>,
    avenue_kind: SurfaceKind,
}

impl Town {
    /// Open ground between its districts and a circle of `radius` about `p`.
    pub fn built_gap_to(&self, p: Point, radius: f64) -> f64 {
        self.blocks
            .iter()
            .map(|block| ring_distance(&block.ring, p))
            .fold(f64::INFINITY, f64::min)
            - radius
    }

    /// Whether the shape `ring` stands `gap` clear of every district.
    pub fn clear_of(&self, ring: &[Point], gap: f64) -> bool {
        // Only what lies within the gap of the shape's bounds can be near it.
        let [x0, y0, x1, y1] = contract::ground::limits(ring, gap);
        let far = |other: &[Point]| {
            let [a0, b0, a1, b1] = contract::ground::limits(other, 0.0);
            a0 > x1 || a1 < x0 || b0 > y1 || b1 < y0
        };
        far(&self.outline)
            || self.blocks.iter().all(|block| {
                far(&block.ring)
                    || (ring.iter().all(|p| ring_distance(&block.ring, *p) >= gap)
                        && !block
                            .ring
                            .iter()
                            .any(|p| contract::ground::polygon_contains(ring, *p)))
            })
    }
}

/// A settlement's ground cut into blocks, and the order it grows in.
struct Plot<'a> {
    class: &'a SettlementClass,
    ground: Ground,
    mesh: Mesh,
    center: Point,
    /// The straight pieces of road on its ground.
    roads: Vec<(Point, Point, SurfaceKind)>,
    /// Leaves in the order they are built on, and how many are.
    order: Vec<usize>,
    built: usize,
    /// The fewest it keeps: the blocks about its centre.
    core: usize,
    /// Each leaf's distance from the centre as the settlement counts it:
    /// longer or shorter than the ground's by the patch the leaf lies in.
    spread: Vec<f64>,
}

impl Plot<'_> {
    fn ring(&self, leaf: usize) -> &[Point] {
        &self.ground.leaves[leaf].ring
    }
    fn built(&self) -> &[usize] {
        &self.order[..self.built]
    }
}

/// A settlement as it grows. A block is built only where a carriageway
/// leads to it: a road, or the edge of a block already built, which becomes
/// a street.
struct Growth<'a> {
    ground: &'a Ground,
    mesh: &'a Mesh,
    buildable: &'a [bool],
    /// The carriageways so far.
    live: Vec<[Point; 2]>,
    /// For each leaf, whether a carriageway leads to it, and how many of
    /// `live` it has been asked of: one that leads to it always will.
    led: Vec<(bool, usize)>,
    order: Vec<usize>,
    taken: BTreeSet<usize>,
    /// Buildable neighbours of the blocks built.
    frontier: BTreeSet<usize>,
}

impl Growth<'_> {
    /// The edges of a leaf that may carry a street: all but the
    /// settlement's limit.
    fn edges(&self, leaf: usize) -> Vec<[Point; 2]> {
        let leaf = &self.ground.leaves[leaf];
        (0..leaf.ring.len())
            .filter(|edge| leaf.bounds[*edge] != Bound::Limit)
            .map(|edge| {
                let (a, b) = leaf.edge(edge);
                [a, b]
            })
            .collect()
    }

    /// Whether a carriageway leads to the leaf and building on it would
    /// leave the built ground's edge clear of itself.
    fn open_to(&mut self, leaf: usize) -> bool {
        self.led_to(leaf) && !self.mesh.pinches(leaf, &self.taken)
    }

    fn led_to(&mut self, leaf: usize) -> bool {
        let (led, asked) = self.led[leaf];
        if !led {
            let edges = self.edges(leaf);
            let found = self.live[asked..]
                .iter()
                .any(|other| edges.iter().any(|edge| touch(*edge, *other)));
            self.led[leaf] = (found, self.live.len());
        }
        self.led[leaf].0
    }

    fn build(&mut self, leaf: usize) {
        // Its edges become streets, each once a carriageway reaches it.
        let mut edges = self.edges(leaf);
        while let Some(at) = edges
            .iter()
            .position(|edge| self.live.iter().any(|other| touch(*edge, *other)))
        {
            self.live.push(edges.swap_remove(at));
        }
        self.taken.insert(leaf);
        self.order.push(leaf);
        self.frontier.remove(&leaf);
        let (mesh, buildable, taken) = (self.mesh, self.buildable, &self.taken);
        self.frontier.extend(
            mesh.neighbours(leaf)
                .iter()
                .copied()
                .filter(|other| buildable[*other] && !taken.contains(other)),
        );
    }

    fn covered(&self) -> f64 {
        self.order
            .iter()
            .map(|leaf| area(&self.ground.leaves[*leaf].ring))
            .sum()
    }
}

fn plot<'a>(context: &'a Context, index: usize, site: &Site, roads: &[Road]) -> Plot<'a> {
    let presets = context.presets;
    let class = presets.class(&site.class_id);
    let rules = &presets.towns;
    // Its own stream: reshaping one town never moves another, or any site.
    let mut rng = context.stream(&format!("town/{index}"));
    let limit = &site.outline.ring;
    let roads = pieces(limit, roads);
    let mut ground = Ground {
        cuts: Vec::new(),
        leaves: vec![Leaf {
            ring: limit.clone(),
            bounds: vec![Bound::Limit; limit.len()],
        }],
    };
    for (a, b, kind) in &roads {
        ground.cut(*a, *b, *kind);
    }
    ground.subdivide(&class.block, rules, &mut rng);
    let mesh = Mesh::new(&ground.leaves);

    // Where its roads come nearest the middle of its ground.
    let middle = site.outline.center;
    let center = roads
        .iter()
        .map(|(a, b, _)| {
            let step = sub(*b, *a);
            let offset = sub(middle, *a);
            let share = ((offset[0] * step[0] + offset[1] * step[1])
                / (step[0] * step[0] + step[1] * step[1]))
                .clamp(0.0, 1.0);
            add(*a, scale(step, share))
        })
        .min_by(|a, b| distance(*a, middle).total_cmp(&distance(*b, middle)))
        .map_or(middle, round_cm);

    // A block is built on when it holds the ground one of the class's
    // district kinds needs and none of its corners is a spike.
    let back = presets.setback_m();
    let buildable: Vec<bool> = ground
        .leaves
        .iter()
        .map(|leaf| {
            leaf.sharpest_corner() >= rules.corner_min_deg.to_radians()
                && kinds(class)
                    .any(|kind| ground.holds(leaf, presets.districts[kind].ground_m, back, None))
        })
        .collect();
    // It grows outward from its centre, along its roads first: a block's
    // place in the queue is its distance from the centre and from the
    // nearest road. That distance is drawn longer or shorter by the patch of
    // ground the block lies in, a few blocks across, so the settlement
    // reaches out in some directions and leaves others to the fields.
    let across = rules.growth_patch_blocks * class.block.length_m[1];
    let [x0, y0, x1, y1] = contract::ground::limits(limit, 0.0);
    let patches = libm::ceil((x1 - x0) * (y1 - y0) / (across * across)) as u32;
    let patches = patches.max(rules.growth_patches_min);
    let patches: Vec<(Point, f64)> = (0..patches)
        .map(|_| {
            let at = [rng.range([x0, x1]), rng.range([y0, y1])];
            (at, 2.0 * rng.unit() - 1.0)
        })
        .collect();
    let spread: Vec<f64> = ground
        .leaves
        .iter()
        .map(|leaf| {
            let middle = centroid(&leaf.ring);
            let patch = patches
                .iter()
                .min_by(|a, b| distance(a.0, middle).total_cmp(&distance(b.0, middle)))
                .map_or(0.0, |(_, drawn)| *drawn);
            distance(middle, center) * (1.0 + rules.growth_noise * patch)
        })
        .collect();
    let keys: Vec<f64> = ground
        .leaves
        .iter()
        .zip(&spread)
        .map(|(leaf, spread)| {
            let middle = centroid(&leaf.ring);
            let road = roads
                .iter()
                .map(|(a, b, _)| segment_distance(*a, *b, middle))
                .fold(f64::INFINITY, f64::min);
            spread + class.ribbon * road.min(site.outline.reach)
        })
        .collect();
    let target = rng.range(class.built_share) * area(limit);
    let by_key = |a: &usize, b: &usize| keys[*a].total_cmp(&keys[*b]).then(a.cmp(b));
    let mut growth = Growth {
        ground: &ground,
        mesh: &mesh,
        buildable: &buildable,
        live: roads.iter().map(|(a, b, _)| [*a, *b]).collect(),
        led: vec![(false, 0); ground.leaves.len()],
        order: Vec::new(),
        taken: BTreeSet::new(),
        frontier: BTreeSet::new(),
    };
    // It starts with the blocks about its centre, or failing those the
    // nearest block a road leads to.
    let mut first: Vec<usize> = (0..ground.leaves.len())
        .filter(|leaf| buildable[*leaf])
        .collect();
    first.retain(|leaf| growth.led_to(*leaf));
    first.sort_by(by_key);
    let about: Vec<usize> = first
        .iter()
        .copied()
        .filter(|leaf| ring_distance(&ground.leaves[*leaf].ring, center) <= 1.0)
        .collect();
    let start = if about.is_empty() {
        first.into_iter().take(1).collect()
    } else {
        about
    };
    for leaf in start {
        if !growth.mesh.pinches(leaf, &growth.taken) {
            growth.build(leaf);
        }
    }
    let core = growth.order.len();
    let (mut covered, mut built) = (growth.covered(), core);
    // A settlement on the midline grows on whichever side of it has less
    // of its built ground, so each half of the map gets its share of it.
    let midline = context.extent / 2.0;
    let lean = |leaf: usize| {
        let ring = &ground.leaves[leaf].ring;
        2.0 * area_above(ring, midline) - area(ring)
    };
    // Every block it can reach is queued, past the share it builds, so that
    // `balance` can even out the map's halves by a block more or fewer.
    loop {
        let mut reachable: Vec<usize> = growth.frontier.iter().copied().collect();
        reachable.retain(|leaf| growth.open_to(*leaf));
        let north: f64 = growth.order.iter().map(|leaf| lean(*leaf)).sum();
        let lighter = reachable
            .iter()
            .copied()
            .filter(|leaf| lean(*leaf) * north < 0.0)
            .min_by(by_key);
        let Some(leaf) = lighter.or(reachable.into_iter().min_by(by_key)) else {
            break;
        };
        growth.build(leaf);
        if covered < target {
            covered = growth.covered();
            built = growth.order.len();
        }
    }
    let order = growth.order;
    Plot {
        class,
        ground,
        mesh,
        center,
        roads,
        order,
        built,
        core,
        spread,
    }
}

/// Even out the built ground of the two halves: while they differ by more
/// than half the tolerance, the settlement whose next or last few blocks
/// best close the difference builds them or leaves them open.
fn balance(context: &Context, plots: &mut [Plot]) {
    let middle = context.extent / 2.0;
    let playable = context.extent * context.extent;
    // A block's ground north of the midline less its ground south of it.
    let lean = |plot: &Plot, leaf: usize| {
        let ring = plot.ring(leaf);
        2.0 * area_above(ring, middle) - area(ring)
    };
    // Each pass closes some of the difference; a block is turned at most
    // a few times over.
    let reach = context.presets.retries.repair_blocks as usize;
    let passes: usize = plots.iter().map(|plot| plot.order.len()).sum();
    for _ in 0..passes {
        let (mut top, mut bottom) = (0.0, 0.0);
        for plot in plots.iter() {
            for leaf in plot.built() {
                let above = area_above(plot.ring(*leaf), middle);
                top += above;
                bottom += area(plot.ring(*leaf)) - above;
            }
        }
        let difference = top - bottom;
        let allowance = context
            .presets
            .fairness
            .town
            .allowance(top, bottom, playable);
        if difference.abs() <= 0.5 * allowance {
            return;
        }
        // (what is left of the difference, settlement, blocks built). A
        // settlement may build or leave open a few blocks at once: its next
        // block may lie in the wrong half and the one after in the right.
        let mut best: Option<(f64, usize, usize)> = None;
        for (index, plot) in plots.iter().enumerate() {
            let mut change = 0.0;
            for built in (plot.core.max(plot.built.saturating_sub(reach))..plot.built).rev() {
                change -= lean(plot, plot.order[built]);
                let left = (difference + change).abs();
                if left < difference.abs() && best.is_none_or(|(known, ..)| left < known) {
                    best = Some((left, index, built));
                }
            }
            let mut change = 0.0;
            for built in plot.built..plot.order.len().min(plot.built + reach) {
                change += lean(plot, plot.order[built]);
                let left = (difference + change).abs();
                if left < difference.abs() && best.is_none_or(|(known, ..)| left < known) {
                    best = Some((left, index, built + 1));
                }
            }
        }
        match best {
            Some((_, index, built)) => plots[index].built = built,
            None => return,
        }
    }
}

/// Every district kind a class's zones can pick.
fn kinds(class: &SettlementClass) -> impl Iterator<Item = &String> {
    class.zones.iter().flat_map(|zone| {
        zone.districts
            .keys()
            .chain(zone.roadside.iter().flatten().map(|(kind, _)| kind))
    })
}

/// A ring on the plan's centimetre grid, without the corners that fall
/// together there.
fn on_grid(ring: &[Point]) -> Vec<Point> {
    let mut ring: Vec<Point> = ring.iter().map(|p| round_cm(*p)).collect();
    ring.dedup();
    if ring.len() > 1 && ring[0] == ring[ring.len() - 1] {
        ring.pop();
    }
    ring
}

/// The district kind a block picks from `zone` with the draw `drawn` (a
/// share of the weights), among those `fits` admits: the kinds its ground
/// can hold along its streets. A block too small for any of its zone's
/// kinds takes the first of the class's that fits. `None` when none does.
fn pick(
    class: &SettlementClass,
    zone: &Zone,
    roadside: bool,
    fits: impl Fn(&String) -> bool,
    drawn: f64,
) -> Option<String> {
    let weights = match &zone.roadside {
        Some(weights) if roadside => weights,
        _ => &zone.districts,
    };
    let fitting: Vec<(&String, f64)> = weights
        .iter()
        .filter(|(kind, _)| fits(kind))
        .map(|(kind, weight)| (kind, *weight))
        .collect();
    let mut pick = drawn * fitting.iter().map(|(_, weight)| weight).sum::<f64>();
    fitting
        .iter()
        .find(|(_, weight)| {
            pick -= weight;
            pick < 0.0
        })
        .or(fitting.last())
        .map(|(kind, _)| *kind)
        .or_else(|| kinds(class).find(|kind| fits(kind)))
        .cloned()
}

fn finish(
    context: &Context,
    index: usize,
    site: &Site,
    plot: Plot,
) -> Result<Town, Vec<Diagnostic>> {
    let Plot {
        class,
        ground,
        mesh,
        center,
        ..
    } = &plot;
    // The stretches of a block's edges that carry a road or one of
    // `streets`: where its parcels can front. (edge, from, to), in metres.
    let fronts = |leaf: &Leaf, streets: &[[Point; 2]]| -> Vec<(usize, f64, f64)> {
        let mut fronts = Vec::new();
        for edge in 0..leaf.ring.len() {
            let (a, b) = leaf.edge(edge);
            let span = distance(a, b);
            let toward = scale(sub(b, a), 1.0 / span);
            let carried = plot
                .roads
                .iter()
                .map(|(c, d, _)| [*c, *d])
                .chain(streets.iter().copied())
                .filter(|ends| {
                    ends.iter()
                        .all(|p| cross(toward, sub(*p, a)).abs() <= MEET_M)
                })
                .map(|ends| {
                    let at = ends.map(|p| {
                        let offset = sub(p, a);
                        offset[0] * toward[0] + offset[1] * toward[1]
                    });
                    (at[0].min(at[1]).max(0.0), at[0].max(at[1]).min(span))
                })
                .filter(|(from, to)| to > from);
            fronts.extend(carried.map(|(from, to)| (edge, from, to)));
        }
        fronts
    };
    let back = context.presets.setback_m();
    let room = |leaf: usize, kind: &String, streets: &[[Point; 2]]| {
        let shape = &ground.leaves[leaf];
        let fronts = fronts(shape, streets);
        ground.holds(
            shape,
            context.presets.districts[kind].ground_m,
            back,
            Some(&fronts),
        )
    };
    let fronted =
        |leaf: usize, streets: &[[Point; 2]]| kinds(class).any(|kind| room(leaf, kind, streets));
    // A block that no street gives room for a parcel is left open after
    // all, and with it any block it alone joined to the rest.
    let mut built: BTreeSet<usize> = plot.built().iter().copied().collect();
    let streets = loop {
        let streets = avenues(site, &plot, &built, &fronted);
        let cramped: Vec<usize> = built
            .iter()
            .copied()
            .filter(|leaf| !fronted(*leaf, &streets))
            .collect();
        if cramped.is_empty() {
            break streets;
        }
        for leaf in cramped {
            built.remove(&leaf);
        }
        let mut joined = BTreeSet::new();
        let mut walk: Vec<usize> = plot
            .built()
            .iter()
            .copied()
            .find(|leaf| built.contains(leaf))
            .into_iter()
            .collect();
        while let Some(leaf) = walk.pop() {
            if joined.insert(leaf) {
                walk.extend(
                    mesh.neighbours(leaf)
                        .iter()
                        .copied()
                        .filter(|other| built.contains(other)),
                );
            }
        }
        built = joined;
    };
    if built.is_empty() {
        return Err(context.fail(
            &format!("settlement-{index}"),
            format!(
                "its {:.1} ha of ground holds no block that one of its district kinds fits on beside a road",
                area(&site.outline.ring) / 1e4
            ),
        ));
    }
    // Its ground is its districts and whatever they enclose: an open leaf
    // that no chain of open leaves joins to the settlement's limit.
    let mut open: BTreeSet<usize> = (0..ground.leaves.len())
        .filter(|leaf| !built.contains(leaf) && ground.leaves[*leaf].bounds.contains(&Bound::Limit))
        .collect();
    let mut walk: Vec<usize> = open.iter().copied().collect();
    while let Some(leaf) = walk.pop() {
        for other in mesh.neighbours(leaf) {
            if !built.contains(other) && open.insert(*other) {
                walk.push(*other);
            }
        }
    }
    let mut held: BTreeSet<usize> = (0..ground.leaves.len())
        .filter(|leaf| !open.contains(leaf))
        .collect();
    // Growth leaves no two districts meeting only at a corner. Should the
    // enclosed ground make such a corner, the open leaf beside it is the
    // settlement's too: its edge never touches itself.
    let outline = loop {
        let (ring, pinched) = mesh.outline(&held);
        let Some(corner) = pinched.first() else {
            break ring;
        };
        let filler = (0..ground.leaves.len())
            .filter(|leaf| !held.contains(leaf))
            .filter(|leaf| {
                mesh.pieces[*leaf]
                    .iter()
                    .any(|(from, to, _)| from == corner || to == corner)
            })
            .min_by(|a, b| area(plot.ring(*a)).total_cmp(&area(plot.ring(*b))));
        match filler {
            Some(leaf) => held.insert(leaf),
            None => break ring,
        };
    };
    if outline.len() < 3 {
        return Err(context.fail(
            &format!("settlement-{index}"),
            "its districts have no edge to draw an outline along".into(),
        ));
    }

    // District kinds by zone, counted out from the centre by built ground,
    // in the order the settlement counts distance: its zones reach out
    // where it grew out, and are no more rings than its outline is.
    let mut outward: Vec<usize> = built.iter().copied().collect();
    outward.sort_by(|a, b| plot.spread[*a].total_cmp(&plot.spread[*b]).then(a.cmp(b)));
    let total: f64 = outward.iter().map(|leaf| area(plot.ring(*leaf))).sum();
    let mut rng = context.stream(&format!("districts/{index}"));
    // At the settlement's edge, on a country road: where industry stands.
    let roadside = |leaf: usize| {
        let at_edge = mesh.pieces[leaf].iter().any(|(from, to, _)| {
            !mesh
                .owner
                .get(&(*to, *from))
                .is_some_and(|other| built.contains(other))
        });
        let on_road = ground.leaves[leaf].bounds.iter().any(|bound| match bound {
            Bound::Cut(id) => ground.cuts[*id]
                .road
                .is_some_and(|kind| kind == SurfaceKind::CountryRoad),
            Bound::Limit => false,
        });
        at_edge && on_road
    };
    // Blocks take their kind a neighbourhood at a time: a few blocks side by
    // side, all of the kind the first of them draws, so a town is suburbs,
    // estates and a centre and not a chequer of single blocks. The first is
    // the block nearest the centre that has no kind yet, and its zone is by
    // the built ground nearer the centre than it.
    let mut kind_of: Vec<Option<String>> = vec![None; ground.leaves.len()];
    let mut covered = 0.0;
    for (place, first) in outward.iter().enumerate() {
        let nearer = covered;
        covered += area(plot.ring(*first));
        if kind_of[*first].is_some() {
            continue;
        }
        let zone = class
            .zones
            .iter()
            .find(|zone| nearer < zone.to * total)
            .unwrap_or(&class.zones[class.zones.len() - 1]);
        let (size, drawn) = (rng.count(class.neighbourhood) as usize, rng.unit());
        let by_road = roadside(*first);
        let mut members = vec![*first];
        while members.len() < size {
            // The nearest to the first of the blocks beside the
            // neighbourhood that are no farther in and have no kind yet.
            let next = outward[place..]
                .iter()
                .copied()
                .filter(|leaf| kind_of[*leaf].is_none() && !members.contains(leaf))
                .filter(|leaf| {
                    mesh.neighbours(*leaf)
                        .iter()
                        .any(|other| members.contains(other))
                })
                .min_by(|a, b| {
                    let from_first = |leaf: &usize| {
                        distance(centroid(plot.ring(*leaf)), centroid(plot.ring(*first)))
                    };
                    from_first(a).total_cmp(&from_first(b)).then(a.cmp(b))
                });
            match next {
                Some(leaf) => members.push(leaf),
                None => break,
            }
        }
        for leaf in members {
            let fits = |kind: &String| room(leaf, kind, &streets);
            kind_of[leaf] = pick(class, zone, by_road, fits, drawn);
        }
    }
    // Every block still built has room for one of its class's kinds: the
    // streets were laid to give it a front. Districts are listed nearest
    // the centre first.
    let from_centre = |leaf: &usize| distance(centroid(plot.ring(*leaf)), *center);
    outward.sort_by(|a, b| from_centre(a).total_cmp(&from_centre(b)).then(a.cmp(b)));
    let blocks = outward
        .iter()
        .map(|leaf| {
            let ring = on_grid(plot.ring(*leaf));
            Some(Block {
                anchor: round_cm(centroid(&ring)),
                ring,
                kind: kind_of[*leaf].clone()?,
            })
        })
        .collect::<Option<Vec<Block>>>()
        .ok_or_else(|| {
            context.fail(
                &format!("settlement-{index}"),
                "a block has no district kind that fits on it".into(),
            )
        })?;

    // Open blocks beside its districts: where a wood may stand.
    let greens = (0..ground.leaves.len())
        .filter(|leaf| !built.contains(leaf))
        .filter(|leaf| {
            mesh.neighbours(*leaf)
                .iter()
                .any(|other| built.contains(other))
        })
        .map(|leaf| plot.ring(leaf).to_vec())
        .collect();

    Ok(Town {
        center: *center,
        outline: on_grid(&outline),
        blocks,
        greens,
        avenues: streets,
        avenue_kind: context.presets.avenue(class.road).0,
    })
}

/// Grow every settlement on its site, along the roads that cross it.
pub fn grow(
    context: &Context,
    sites: &[Site],
    roads: &[Road],
) -> Result<Vec<Town>, Vec<Diagnostic>> {
    let mut plots: Vec<Plot> = sites
        .iter()
        .enumerate()
        .map(|(index, site)| plot(context, index, site, roads))
        .collect();
    balance(context, &mut plots);
    plots
        .into_iter()
        .zip(sites)
        .enumerate()
        .map(|(index, (plot, site))| finish(context, index, site, plot))
        .collect()
}

/// The plan record of one settlement: each block is a district of one kind.
pub fn settlement(context: &Context, index: usize, site: &Site, town: &Town) -> SettlementPlan {
    let presets = context.presets;
    let districts = town
        .blocks
        .iter()
        .enumerate()
        .map(|(number, block)| DistrictPlan {
            id: format!("settlement-{index}/district-{number}"),
            categories: presets.districts[&block.kind]
                .mix
                .iter()
                .map(|(category, weight)| CategoryShare {
                    category: *category,
                    weight: *weight,
                })
                .collect(),
            kind: block.kind.clone(),
            ring: block.ring.clone(),
            area_m2: libm::round(area(&block.ring)),
            anchor: block.anchor,
            max_floors: context.preset.max_floors,
        })
        .collect();
    SettlementPlan {
        id: format!("settlement-{index}"),
        class: site.class_id.clone(),
        center: town.center,
        outline: town.outline.clone(),
        districts,
    }
}

/// Every settlement's avenues as surfaces: streets, or lanes where the
/// settlement's own road is a dirt track.
pub fn surfaces(context: &Context, towns: &[Town]) -> Result<Vec<SurfaceArea>, Vec<Diagnostic>> {
    let presets = context.presets;
    let mut laid = Vec::new();
    for (index, town) in towns.iter().enumerate() {
        let (_, width) = presets.avenue(town.avenue_kind);
        for [a, b] in &town.avenues {
            let shape = GroundShape::stroke(vec![*a, *b], width)
                .map_err(|message| context.fail(&format!("settlement-{index}"), message))?;
            laid.push(SurfaceArea {
                kind: town.avenue_kind,
                shape,
            });
        }
    }
    Ok(laid)
}
