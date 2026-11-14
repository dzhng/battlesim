//! Steadings outside the settlements: a group of pieces, each in its own
//! yard, at a station on a country road or track. Homes stand as many of a
//! group as find room; a group of bodies (a depot) stands whole or not at all.
use super::*;

/// What a group places: homes of its templates, or bodies in order.
#[derive(Clone, Copy)]
enum Group<'g, 'f> {
    Homes(&'g Homes, &'g Choices<'f>),
    Bodies(&'g [Piece]),
}

/// One piece of a group as it will stand: a home of its template, furnished
/// from its homes' rows, or a row of bodies.
enum Stand<'g, 'f> {
    Home(&'f Fit<'f>, &'g Homes),
    Body(Row<'g>),
}

/// `count` bodies of `kind` side by side, the net over them if any, and the
/// box the whole takes.
struct Row<'g> {
    kind: &'g str,
    body: [f64; 3],
    count: u32,
    net: Option<(&'g str, [f64; 3])>,
    half: [f64; 3],
}

impl<'a> Country<'a> {
    /// One group of `kind`'s pieces at a place on a road in `half`.
    pub(super) fn stead<P: Pieced>(
        &mut self,
        half: usize,
        kind: &Steading<P>,
        rng: &mut Stream,
    ) -> bool {
        let presets = self.presets;
        let pieces = kind.pieces.pieces();
        let name = pieces.name();
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
                .any(|(of, other)| *of == name && distance(*other, at) < kind.apart_m)
        {
            return false;
        }
        let along = scale(sub(ahead, at), 1.0 / distance(at, ahead));
        let (group, count) = match pieces {
            Pieces::Homes(homes, fits) => {
                let (group, choices) = rng
                    .pick(
                        homes
                            .groups
                            .iter()
                            .zip(fits)
                            .map(|pair| (pair, pair.0.weight)),
                    )
                    .expect("the open country has homestead groups");
                if choices.is_empty() {
                    return false;
                }
                (Group::Homes(homes, choices), rng.count(group.homes))
            }
            Pieces::Bodies(groups) => {
                let group = rng
                    .pick(groups.iter().map(|group| (group, group.weight)))
                    .expect("depots have groups");
                (Group::Bodies(&group.pieces), group.pieces.len() as u32)
            }
        };
        let side = if rng.chance(0.5) { 1.0 } else { -1.0 };
        let lane = rng.chance(kind.lane_chance);
        let lane_m = rng.range(kind.lane_m);
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
                own_from: usize::MAX,
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
            let reach = f64::from(count) * 60.0 + kind.spread_m;
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
            front_m: kind.yard.front_m,
            side_m: kind.yard.side_m,
            rear_m: kind.yard.rear_m,
            coverage: 1.0,
            apron_m: 0.0,
        };
        let stead = self.steads.iter().filter(|(of, _)| *of == name).count();
        let lots_before = self.lots.len();
        let before = (
            self.yards.len(),
            self.props.len(),
            self.bodies.len(),
            self.tally,
        );
        let mut cursor = [0.0_f64; 2];
        // A home is tried once. A depot's piece that finds no room is tried
        // again further along, a few times over.
        let retry = matches!(group, Group::Bodies(_));
        let tries = if retry { 4 * count } else { count };
        let mut index = 0;
        for _ in 0..tries {
            if index == count as usize {
                break;
            }
            let piece = match group {
                // A category by its weight, then one of its templates.
                Group::Homes(homes, choices) => {
                    let eligible = choices.category(rng);
                    Stand::Home(eligible[rng.below(eligible.len() as u64) as usize], homes)
                }
                Group::Bodies(pieces) => Stand::Body(self.row(&pieces[index], rng)),
            };
            if !retry {
                index += 1;
            }
            // A depot is one compound on one side of its road; larger
            // groups of homes stand either side.
            let across = match group {
                Group::Homes(..) if lane.is_some() || count > 2 => usize::from(rng.chance(0.5)),
                _ => usize::from(side < 0.0),
            };
            let frontage = Frontage {
                run: &front,
                side: if across == 0 { 1.0 } else { -1.0 },
                offset,
            };
            let gap = rng.range([0.0, kind.spread_m]);
            let lot = match &piece {
                Stand::Home(fit, _) => frontage.lot(cursor[across], fit, &rule),
                Stand::Body(row) => frontage.place(
                    cursor[across],
                    2.0 * (row.half[0] + rule.side_m),
                    rule.front_m + 2.0 * row.half[1] + rule.rear_m,
                ),
            };
            let Some(lot) = lot else {
                if retry {
                    cursor[across] += presets.parcels.lot_step_m * 4.0;
                }
                continue;
            };
            // A lane is a carriageway its yards keep off, like any other.
            let on_lane = lane.as_ref().is_some_and(|(_, [from, to], half)| {
                lot.rect.segment_gap(*from, *to) < half - 0.01
            });
            // A depot is one compound: its pieces' yards may touch.
            let kin = match group {
                Group::Homes(..) => self.yards.len(),
                Group::Bodies(_) => before.0,
            };
            if on_lane || !self.yard_fits(&lot.rect, kin) {
                cursor[across] += presets.parcels.lot_step_m * 4.0;
                continue;
            }
            cursor[across] += 2.0 * lot.rect.half[0] + gap;
            if retry {
                index += 1;
            }
            let id = pieces.lot_id(stead, self.lots.len() - lots_before);
            self.yards.push(lot.rect);
            self.lots.push(LotPlan {
                id: id.clone(),
                ring: lot.rect.corners().map(round_cm).to_vec(),
            });
            match piece {
                Stand::Home(fit, homes) => {
                    let building = placement(
                        id,
                        &lot,
                        fit,
                        &rule,
                        &presets.parcels.prop_kind,
                        &mut self.next_prop,
                    );
                    self.furnish_yard(&lot, fit, &building.frame, &kind.yard, homes, rng);
                    let [x, y, _] = building.frame.translation;
                    self.tally.homes[self.half([x, y])] += 1.0;
                    self.buildings.push(building);
                }
                Stand::Body(row) => self.stand(&lot, &row, &kind.yard),
            }
        }
        let stood = self.lots.len() - lots_before;
        if stood == 0 {
            // No yard found room: a lane would lead nowhere, so none is laid.
            return false;
        }
        if let Group::Bodies(pieces) = group {
            if stood < pieces.len() {
                // A depot without all its pieces is left out.
                let (yards, props, bodies, tally) = before;
                self.yards.truncate(yards);
                self.lots.truncate(lots_before);
                self.props.truncate(props);
                self.bodies.truncate(bodies);
                self.tally = tally;
                return false;
            }
            let middle = ring_middle(&self.lots[lots_before].ring);
            self.tally.depots[self.half(middle)] += 1.0;
            // A depot's yards are its hard standing.
            for lot in &self.lots[lots_before..] {
                if let Ok(shape) = GroundShape::polygon(lot.ring.clone()) {
                    self.surfaces.push(SurfaceArea {
                        kind: SurfaceKind::Paving,
                        shape,
                    });
                }
            }
        }
        if let Some((shape, ..)) = lane {
            self.network.add(&shape);
            self.surfaces.push(SurfaceArea {
                kind: SurfaceKind::DirtTrack,
                shape,
            });
        }
        self.steads.push((name, at));
        true
    }

    /// `piece` as it will stand: its count drawn, its boxes from the
    /// presets' bodies (admission proved each kind is there).
    fn row<'g>(&self, piece: &'g Piece, rng: &mut Stream) -> Row<'g> {
        let boxes = &self.presets.street_props.bodies;
        let body = boxes[&piece.kind].half_extents_m;
        let count = rng.count(piece.count);
        let net = piece
            .net
            .as_deref()
            .map(|net| (net, boxes[net].half_extents_m));
        Row {
            kind: &piece.kind,
            body,
            count,
            net,
            half: net.map_or(Piece::row(body, count), |(_, half)| half),
        }
    }

    /// `row` stood in `lot`: its bodies side by side along the front, set
    /// back by the yard's front, and its net over them.
    fn stand(&mut self, lot: &Lot, row: &Row, yard: &Yard) {
        let middle = lot.point(0.0, yard.front_m + row.half[1]);
        let yaw = libm::atan2(lot.rect.axis[1], lot.rect.axis[0]);
        let reach = |half: [f64; 3]| libm::hypot(half[0], half[1]);
        let first = Piece::row(row.body, row.count)[0] - row.body[0];
        for index in 0..row.count {
            let x = f64::from(index) * (2.0 * row.body[0] + ROW_GAP_M) - first;
            let at = add(middle, scale(lot.rect.axis, x));
            self.body(row.kind, row.body, at, yaw, reach(row.body));
        }
        if let Some((net, half)) = row.net {
            self.body(net, half, middle, yaw, reach(half));
        }
    }

    /// A yard may stand here: on open ground at every corner, off every
    /// carriageway, yard and kept corridor; the yards from `kin` on are its
    /// own group's, which it need only not overlap.
    fn yard_fits(&self, rect: &Rect, kin: usize) -> bool {
        let c = &self.rules.clear;
        let corner = Ask {
            radius: 0.0,
            blocks_sight: false,
            road_gap: 0.0,
            own_from: kin,
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
                .enumerate()
                .any(|(i, yard)| grown(if i < kin { c.yard_m } else { 0.0 }).overlaps(yard, 0.0))
            && !self
                .corridors
                .iter()
                .any(|(corridor, _)| grown(c.approach_m).overlaps(corridor, 0.0))
            && !self.towns.iter().any(|town| rect.touches(town.ring))
            && !self.woods.iter().any(|wood| rect.touches(wood.ring))
    }

    /// The clump of trees behind a home's yard and the body beside its house.
    fn furnish_yard(
        &mut self,
        lot: &Lot,
        fit: &Fit,
        frame: &PlacementFrame,
        yard: &Yard,
        homes: &Homes,
        rng: &mut Stream,
    ) {
        let rules = self.rules;
        let own = self.yards.len() - 1;
        let depth = 2.0 * lot.rect.half[1];
        if rng.chance(homes.tree_chance) {
            let sides = [rng.range(homes.tree_plot_m), rng.range(homes.tree_plot_m)];
            let reach = (lot.rect.half[0] - sides[0] / 2.0).max(0.0);
            let plot = Rect {
                center: lot.point(rng.range([-reach, reach]), depth + sides[1] / 2.0 + 0.5),
                axis: lot.rect.axis,
                half: [sides[0] / 2.0, sides[1] / 2.0],
            };
            let radius = libm::hypot(plot.half[0], plot.half[1]);
            let ask = Ask {
                own_from: own,
                ..self.trees(radius)
            };
            if self.open(plot.center, ask) && self.apart(plot.center, radius) {
                self.plant(plot.corners().to_vec(), plot.center, radius);
                self.tally.trees[self.half(plot.center)] += 1.0;
            }
        }
        if rng.chance(homes.body_chance) {
            let row = rng
                .pick(homes.bodies.iter().map(|row| (row, row.weight)))
                .expect("a yard has bodies");
            // Beside the house, in the side setback, nose to the road.
            let side = if rng.chance(0.5) { 1.0 } else { -1.0 };
            let box_half = (fit.max[0] - fit.min[0]) / 2.0;
            let at = lot.point(
                side * (box_half + yard.side_m / 2.0),
                yard.front_m + row.half_extents_m[0] + rng.range([0.0, 3.0]),
            );
            let radius = libm::hypot(row.half_extents_m[0], row.half_extents_m[1]);
            let doors = fit
                .template
                .materialize(*frame)
                .map(|placed| placed.entrances)
                .unwrap_or_default();
            let clear = doors.iter().all(|door| {
                let from = [door.position[0], door.position[1]];
                let to = add(from, scale(door.normal, yard.front_m));
                crate::layout::geometry::segment_distance(from, to, at)
                    >= radius + homes.door_clear_m
            });
            let ask = Ask {
                radius,
                blocks_sight: false,
                road_gap: rules.clear.road_m,
                own_from: own,
            };
            let yaw = libm::atan2(lot.inward[1], lot.inward[0]);
            if clear && self.open(at, ask) {
                self.body(&row.kind, row.half_extents_m, at, yaw, radius);
            }
        }
    }
}

/// Homes with the templates each of their groups may build on this map.
pub(super) struct Eligible<'h, 'f> {
    homes: &'h Homes,
    choices: Vec<Choices<'f>>,
}

impl Pieced for Eligible<'_, '_> {
    fn pieces(&self) -> Pieces<'_> {
        Pieces::Homes(self.homes, &self.choices)
    }
}

impl Homes {
    /// These homes with each group's templates: its categories, of the
    /// map's one family, within the type's floors and categories.
    pub(super) fn eligible<'f>(
        &self,
        fits: &'f [Fit<'f>],
        preset: &crate::layout::TypePreset,
        family: &str,
    ) -> Eligible<'_, 'f> {
        let choices = self
            .groups
            .iter()
            .map(|group| {
                let mix = group
                    .mix
                    .iter()
                    .copied()
                    .filter(|(category, _)| preset.categories.contains(category));
                Choices::available(fits, mix, family, preset.max_floors)
            })
            .collect();
        Eligible {
            homes: self,
            choices,
        }
    }
}
