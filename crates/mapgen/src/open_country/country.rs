//! What the open country keeps clear of, read off the plan it furnishes.
use super::*;

impl<'a> Country<'a> {
    pub(super) fn new(
        plan: &'a MapPlan,
        request: &'a GenerationRequest,
        presets: &'a PresetDefinitions,
    ) -> Self {
        let rules = &presets.open_country;
        let size = plan.size;
        let towns: Vec<Ground> = plan
            .settlements
            .iter()
            .map(|settlement| Ground::new(&settlement.outline))
            .collect();
        // Ground the country furnishes none of: woods, and swamps' water.
        let woods: Vec<Ground> = plan
            .forests
            .iter()
            .map(|forest| &forest.shape)
            .chain(plan.swamps.iter().map(|swamp| &swamp.shape))
            .filter_map(|shape| match shape {
                GroundShape::Polygon { ring } => Some(Ground::new(ring)),
                GroundShape::Stroke { .. } => None,
            })
            .collect();
        let water = Water::new(&plan.rivers, size);
        let bridge = &presets.rivers.bridge;
        let decks = plan
            .bridges
            .iter()
            .map(|deck| Rect {
                center: deck.center,
                axis: direction(deck.yaw),
                half: [
                    deck.half_extents[0] + bridge.approach_m,
                    deck.half_extents[1],
                ],
            })
            .collect();
        // A settlement's widest approach in each half.
        let unwrapped = |approach: &crate::ApproachPlan| {
            if approach.to_rad < approach.from_rad {
                approach.to_rad + TAU
            } else {
                approach.to_rad
            }
        };
        let middle_of =
            |approach: &crate::ApproachPlan| (approach.from_rad + unwrapped(approach)) / 2.0;
        let mut widest: BTreeMap<(usize, u8), usize> = BTreeMap::new();
        for (index, approach) in plan.approaches.iter().enumerate() {
            let width = |approach: &crate::ApproachPlan| unwrapped(approach) - approach.from_rad;
            let key = u8::from(approach.half == crate::Half::Top);
            let best = widest.entry((approach.settlement, key)).or_insert(index);
            if width(approach) > width(&plan.approaches[*best]) {
                *best = index;
            }
        }
        let mut keep: Vec<usize> = widest.into_values().collect();
        keep.sort_unstable();
        keep.dedup();
        // The corridor of each: the bearings measured either side of its
        // middle, and the middle itself.
        let mut corridors = Vec::new();
        let mut kept = Vec::new();
        for approach in keep.into_iter().map(|index| &plan.approaches[index]) {
            let settlement = &plan.settlements[approach.settlement];
            let step = crate::layout::bearing_step(settlement, approach.depth_m);
            let (first, last) = (
                libm::round(approach.from_rad / step),
                libm::round(unwrapped(approach) / step),
            );
            let middle = (first + last) / 2.0;
            kept.push((approach.settlement, middle_of(approach)));
            for bearing in [
                libm::floor(middle) * step,
                libm::ceil(middle) * step,
                middle_of(approach),
            ] {
                let toward = direction(bearing);
                let edge = corridor_start(
                    &settlement.outline,
                    settlement.center,
                    toward,
                    approach.front_m,
                );
                let corridor = Rect {
                    center: add(
                        settlement.center,
                        scale(toward, edge + approach.depth_m / 2.0),
                    ),
                    axis: toward,
                    half: [approach.depth_m / 2.0, approach.front_m / 2.0],
                };
                corridors.push((corridor, approach.settlement == 0));
            }
        }
        // Country roads and tracks, cut where they run through a settlement.
        let mut roads = Vec::new();
        for area in &plan.surfaces {
            let (
                SurfaceKind::CountryRoad | SurfaceKind::DirtTrack,
                GroundShape::Stroke {
                    centerline,
                    width_m,
                },
            ) = (area.kind, &area.shape)
            else {
                continue;
            };
            let whole = Run::new(centerline.samples().to_vec());
            let steps = libm::ceil(whole.length() / presets.open_country.walk_m).max(1.0) as usize;
            let mut stretch: Vec<Point> = Vec::new();
            for step in 0..=steps {
                let p = whole.at(whole.length() * step as f64 / steps as f64);
                let country = towns
                    .iter()
                    .all(|town| town.clear(p, rules.clear.settlement_m));
                if country && stretch.last() != Some(&p) {
                    stretch.push(p);
                }
                if (!country || step == steps) && !stretch.is_empty() {
                    if stretch.len() > 1 {
                        roads.push(Road {
                            run: Run::new(core::mem::take(&mut stretch)),
                            half_width: width_m / 2.0,
                        });
                    }
                    stretch.clear();
                }
            }
        }
        let road_m = roads.iter().map(|road| road.run.length()).sum();
        let clearance = presets.rivers.bank_m() + presets.roads.dirt_track_width_m / 2.0;
        Self {
            request,
            presets,
            rules,
            size,
            objectives: crate::skirmish::objective_clearances(plan.skirmish.as_ref()).collect(),
            network: Network::new(plan, clearance, &presets.parcels.geometry),
            towns,
            woods,
            water,
            decks,
            corridors,
            kept,
            main_only: false,
            roads,
            road_m,
            proof_fill: false,
            work: None,
            yards: Vec::new(),
            groves: Vec::new(),
            bodies: Vec::new(),
            steads: Vec::new(),
            tally: Tally::default(),
            next_prop: plan
                .buildings
                .iter()
                .map(|building| building.parts.len() as u32)
                .sum(),
            lanes: Vec::new(),
            lots: Vec::new(),
            buildings: Vec::new(),
            forests: Vec::new(),
            props: Vec::new(),
        }
    }
}
