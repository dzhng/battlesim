//! Steadings outside the settlements: a group of pieces, each in its own
//! yard, at a station on a country road or track.
use super::*;

impl<'a> Country<'a> {
    /// One group of homes at a place on a road in `half`.
    pub(super) fn stead(&mut self, half: usize, fits: &[Choices], rng: &mut Stream) -> bool {
        let (rules, presets) = (self.rules, self.presets);
        let h = &rules.homesteads;
        let Some((road, s)) = self.station(rng) else {
            return false;
        };
        let half_width = self.roads[road].half_width;
        let (at, ahead) = {
            let run = &self.roads[road].run;
            (run.at(s), run.at((s + 1.0).min(run.length())))
        };
        if at == ahead
            || self.half(at) != half
            || self
                .steads
                .iter()
                .any(|other| distance(*other, at) < h.apart_m)
        {
            return false;
        }
        let along = scale(sub(ahead, at), 1.0 / distance(at, ahead));
        let (group, choices) = rng
            .pick(h.groups.iter().zip(fits).map(|pair| (pair, pair.0.weight)))
            .expect("the open country has homestead groups");
        if choices.is_empty() {
            return false;
        }
        let homes = rng.count(group.homes);
        let side = if rng.chance(0.5) { 1.0 } else { -1.0 };
        let lane = rng.chance(h.lane_chance);
        let lane_m = rng.range(h.lane_m);
        let verge = presets.parcels.verge_m;
        // The line the yards front, and how far from its middle they start.
        let (front, offset, lane) = if lane {
            let out = scale([-along[1], along[0]], side);
            let (from, to) = (
                round_cm(sub(at, scale(out, JOIN_OVERSHOOT_M))),
                round_cm(add(at, scale(out, lane_m))),
            );
            let width = presets.roads.dirt_track_width_m;
            // The lane's own ground, from the road's edge to its end.
            let ground = Ask {
                radius: width / 2.0,
                blocks_sight: false,
                road_gap: 0.0,
                own_yard: None,
            };
            let first = half_width + width;
            let steps = libm::ceil((lane_m - first) / width).max(1.0) as usize;
            let free = (0..=steps).all(|step| {
                let reach = first + (lane_m - first) * step as f64 / steps as f64;
                let p = add(at, scale(out, reach));
                self.open(p, ground) && self.apart(p, ground.radius)
            });
            let Ok(shape) = GroundShape::stroke(vec![from, to], width) else {
                return false;
            };
            if !free {
                return false;
            }
            // Yards stand along the lane's far end.
            let near = add(at, scale(out, (lane_m * 0.35).max(first + verge)));
            (
                Run::new(vec![near, to]),
                width / 2.0 + verge,
                Some((shape, [from, to], width / 2.0)),
            )
        } else {
            let run = &self.roads[road].run;
            let reach = f64::from(homes) * 60.0 + h.spread_m;
            let mut line: Vec<Point> = Vec::new();
            for step in 0..=8 {
                let p = run.at((s + reach * f64::from(step) / 8.0).min(run.length()));
                if line.last() != Some(&p) {
                    line.push(p);
                }
            }
            if line.len() < 2 {
                return false;
            }
            (Run::new(line), half_width + verge, None)
        };
        let rule = LotRule {
            front_m: h.yard.front_m,
            side_m: h.yard.side_m,
            rear_m: h.yard.rear_m,
            coverage: 1.0,
            apron_m: 0.0,
        };
        let stead = self.steads.len();
        let lots_before = self.lots.len();
        let mut cursor = [0.0_f64; 2];
        for _ in 0..homes {
            // A category by its weight, then one of its templates.
            let eligible = choices.category(rng);
            let fit = eligible[rng.below(eligible.len() as u64) as usize];
            let across = if lane.is_some() || homes > 2 {
                usize::from(rng.chance(0.5))
            } else {
                usize::from(side < 0.0)
            };
            let frontage = Frontage {
                run: &front,
                side: if across == 0 { 1.0 } else { -1.0 },
                offset,
            };
            let gap = rng.range([0.0, h.spread_m]);
            let Some(lot) = frontage.lot(cursor[across], fit, &rule) else {
                continue;
            };
            // A lane is a carriageway its yards keep off, like any other.
            let on_lane = lane.as_ref().is_some_and(|(_, [from, to], half)| {
                lot.rect.segment_gap(*from, *to) < half - 0.01
            });
            if on_lane || !self.yard_fits(&lot.rect) {
                cursor[across] += presets.parcels.lot_step_m * 4.0;
                continue;
            }
            cursor[across] += 2.0 * lot.rect.half[0] + gap;
            let id = format!(
                "country/stead-{stead}/lot-{}",
                self.lots.len() - lots_before
            );
            self.yards.push(lot.rect);
            self.lots.push(LotPlan {
                id: id.clone(),
                ring: lot.rect.corners().map(round_cm).to_vec(),
            });
            let building = placement(
                id,
                &lot,
                fit,
                &rule,
                &presets.parcels.prop_kind,
                &mut self.next_prop,
            );
            self.furnish_yard(&lot, fit, &building.frame, rng);
            let [x, y, _] = building.frame.translation;
            self.tally.homes[self.half([x, y])] += 1.0;
            self.buildings.push(building);
        }
        if self.lots.len() == lots_before {
            // No yard found room: a lane would lead nowhere, so none is laid.
            return false;
        }
        if let Some((shape, ..)) = lane {
            self.network.add(&shape);
            self.lanes.push(SurfaceArea {
                kind: SurfaceKind::DirtTrack,
                shape,
            });
        }
        self.steads.push(at);
        true
    }

    /// A yard may stand here: on open ground at every corner, off every
    /// carriageway, yard and kept corridor.
    fn yard_fits(&self, rect: &Rect) -> bool {
        let c = &self.rules.clear;
        let corner = Ask {
            radius: 0.0,
            blocks_sight: false,
            road_gap: 0.0,
            own_yard: None,
        };
        let grown = |by: f64| Rect {
            half: [rect.half[0] + by, rect.half[1] + by],
            ..*rect
        };
        rect.corners()
            .into_iter()
            .chain([rect.center])
            .all(|p| self.open(p, corner) && self.apart(p, 0.0))
            && !self.network.covers(rect)
            && !self
                .yards
                .iter()
                .any(|yard| grown(c.yard_m).overlaps(yard, 0.0))
            && !self
                .corridors
                .iter()
                .any(|(corridor, _)| grown(c.approach_m).overlaps(corridor, 0.0))
            && !self.towns.iter().any(|town| rect.touches(town.ring))
            && !self.woods.iter().any(|wood| rect.touches(wood.ring))
    }

    /// The clump of trees behind a yard and the body beside its house.
    fn furnish_yard(&mut self, lot: &Lot, fit: &Fit, frame: &PlacementFrame, rng: &mut Stream) {
        let rules = self.rules;
        let h = &rules.homesteads;
        let own = self.yards.len() - 1;
        let depth = 2.0 * lot.rect.half[1];
        if rng.chance(h.tree_chance) {
            let sides = [rng.range(h.tree_plot_m), rng.range(h.tree_plot_m)];
            let reach = (lot.rect.half[0] - sides[0] / 2.0).max(0.0);
            let plot = Rect {
                center: lot.point(rng.range([-reach, reach]), depth + sides[1] / 2.0 + 0.5),
                axis: lot.rect.axis,
                half: [sides[0] / 2.0, sides[1] / 2.0],
            };
            let radius = libm::hypot(plot.half[0], plot.half[1]);
            let ask = Ask {
                own_yard: Some(own),
                ..self.trees(radius)
            };
            if self.open(plot.center, ask) && self.apart(plot.center, radius) {
                self.plant(plot.corners().to_vec(), plot.center, radius);
                self.tally.trees[self.half(plot.center)] += 1.0;
            }
        }
        if rng.chance(h.body_chance) {
            let row = rng
                .pick(h.bodies.iter().map(|row| (row, row.weight)))
                .expect("a yard has bodies");
            // Beside the house, in the side setback, nose to the road.
            let side = if rng.chance(0.5) { 1.0 } else { -1.0 };
            let box_half = (fit.max[0] - fit.min[0]) / 2.0;
            let at = lot.point(
                side * (box_half + h.yard.side_m / 2.0),
                h.yard.front_m + row.half_extents_m[0] + rng.range([0.0, 3.0]),
            );
            let radius = libm::hypot(row.half_extents_m[0], row.half_extents_m[1]);
            let doors = fit
                .template
                .materialize(*frame)
                .map(|placed| placed.entrances)
                .unwrap_or_default();
            let clear = doors.iter().all(|door| {
                let from = [door.position[0], door.position[1]];
                let to = add(from, scale(door.normal, h.yard.front_m));
                crate::layout::geometry::segment_distance(from, to, at)
                    >= radius + h.yard.door_clear_m
            });
            let ask = Ask {
                radius,
                blocks_sight: false,
                road_gap: rules.clear.road_m,
                own_yard: Some(own),
            };
            let yaw = libm::atan2(lot.inward[1], lot.inward[0]);
            if clear && self.open(at, ask) {
                self.body(row, at, yaw, radius);
            }
        }
    }
}
