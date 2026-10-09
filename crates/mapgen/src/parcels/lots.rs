//! Parcels along a district's streets, each cut to the template that stands
//! on it: the template's own footprint plus the district's setbacks, placed
//! by translation and rotation only.
use super::space::{clip, Rect, Run};
use super::streets::Network;
use super::Pass;
use crate::layout::geometry::{add, distance, round_cm, scale, sub, Grid, Point, TAU};
use crate::layout::rng::Stream;
use crate::layout::{CourtParking, LotRule};
use crate::{BuildingPlacement, Diagnostic, DiagnosticCode, DistrictPlan, LotPlan, MapPlan};
use contract::ground::GroundShape;
use contract::map::{BuildingPartReference, SurfaceArea, SurfaceKind};
use contract::templates::{BuildingCategory, BuildingTemplateDescriptor, PlacementFrame};

/// A template as a parcel sees it: turned so its entrances face `−Y`, the
/// street side, with the box its parts then fill.
pub struct Fit<'a> {
    pub template: &'a BuildingTemplateDescriptor,
    /// The turn that brings the entrance side to face `−Y`.
    turn: f64,
    pub min: Point,
    pub max: Point,
}

impl<'a> Fit<'a> {
    /// `None` for a template the parcel pass cannot select: one whose floors,
    /// entrances or bays are unresolved.
    pub fn new(template: &'a BuildingTemplateDescriptor) -> Option<Self> {
        template.require_complete().ok()?;
        let identity = PlacementFrame {
            translation: [0.0; 3],
            yaw: 0.0,
        };
        let placed = template.materialize(identity).ok()?;
        let street = placed.entrances.as_ref()?.first()?.normal;
        let turn = -TAU / 4.0 - libm::atan2(street[1], street[0]);
        let (sin, cos) = libm::sincos(turn);
        let mut min = [f64::INFINITY; 2];
        let mut max = [f64::NEG_INFINITY; 2];
        for part in &placed.parts {
            let center = [
                cos * part.center[0] - sin * part.center[1],
                sin * part.center[0] + cos * part.center[1],
            ];
            let (s, c) = libm::sincos(part.yaw + turn);
            let reach = [
                c.abs() * part.half_extents[0] + s.abs() * part.half_extents[1],
                s.abs() * part.half_extents[0] + c.abs() * part.half_extents[1],
            ];
            for i in 0..2 {
                min[i] = min[i].min(center[i] - reach[i]);
                max[i] = max[i].max(center[i] + reach[i]);
            }
        }
        Some(Self {
            template,
            turn,
            min,
            max,
        })
    }

    pub fn floors(&self) -> usize {
        self.template.floor_heights_m.as_ref().map_or(0, Vec::len)
    }

    /// The parcel cut for it under `rule`: its width along the street and
    /// its depth from the street.
    pub fn lot(&self, rule: &LotRule) -> [f64; 2] {
        [
            self.max[0] - self.min[0] + 2.0 * rule.side_m,
            rule.front_m + self.max[1] - self.min[1] + rule.rear_m,
        ]
    }
}

/// What the parcels of a map stand among, and what they add to it.
pub struct Ground<'a> {
    pub network: &'a Network<'a>,
    forests: Vec<&'a [Point]>,
    objectives: Vec<Rect>,
    lots: Vec<Rect>,
    lot_grid: Grid,
    pub plan_lots: Vec<LotPlan>,
    pub buildings: Vec<BuildingPlacement>,
    pub aprons: Vec<SurfaceArea>,
    next_prop: u32,
    /// Where the district being filled starts in `plan_lots`.
    first_lot: usize,
}

impl<'a> Ground<'a> {
    pub fn new(plan: &'a MapPlan, network: &'a Network<'a>) -> Self {
        Self {
            network,
            objectives: crate::skirmish::objective_clearances(plan.skirmish.as_ref())
                .map(|(center, clearance)| Rect {
                    center,
                    axis: [1.0, 0.0],
                    half: [clearance; 2],
                })
                .collect(),
            forests: plan
                .forests
                .iter()
                .filter_map(|forest| match &forest.shape {
                    GroundShape::Polygon { ring } => Some(ring.as_slice()),
                    GroundShape::Stroke { .. } => None,
                })
                .collect(),
            lots: Vec::new(),
            lot_grid: Grid::new(plan.size, 48.0),
            plan_lots: Vec::new(),
            buildings: Vec::new(),
            aprons: Vec::new(),
            next_prop: 0,
            first_lot: 0,
        }
    }

    /// A parcel may stand here: inside its district, off every carriageway
    /// and forest, and on no other parcel.
    fn clear(&self, rect: &Rect, district: &DistrictPlan, forests: &[&[Point]]) -> bool {
        !self
            .objectives
            .iter()
            .any(|reserved| rect.overlaps(reserved, 0.0))
            && rect.inside(&district.ring)
            && !self.network.covers(rect)
            && !forests.iter().any(|ring| rect.touches(ring))
            && !self.lot_grid.any(rect.bounds(), |item| {
                // Neighbours share a boundary; a centimetre is rounding.
                rect.overlaps(&self.lots[item as usize], 0.01)
            })
    }

    /// The forests near enough `district` to matter.
    fn forests_near(&self, district: &DistrictPlan) -> Vec<&'a [Point]> {
        let [x0, y0, x1, y1] = contract::ground::limits(&district.ring, 0.0);
        self.forests
            .iter()
            .copied()
            .filter(|ring| {
                let b = contract::ground::limits(ring, 0.0);
                !(b[0] > x1 || b[2] < x0 || b[1] > y1 || b[3] < y0)
            })
            .collect()
    }

    /// Cut parcels along every carriageway that runs through the district
    /// and stand a template on each. Returns how many buildings it placed.
    pub fn fill(&mut self, pass: &Pass, district: &DistrictPlan) -> Result<usize, Vec<Diagnostic>> {
        let preset = pass.district(district)?;
        let mut rng = pass.stream(&format!("lots/{}", district.id));
        let site = Site {
            district,
            rule: preset.lots,
            yards_paved: preset.props.courts.paved,
            choices: pass.choices(district)?,
            forests: self.forests_near(district),
        };
        let before = self.buildings.len();
        self.first_lot = self.plan_lots.len();
        for (run, half_width) in self.runs(district) {
            for side in [1.0, -1.0] {
                let frontage = Frontage {
                    run: &run,
                    side,
                    offset: half_width + pass.presets.parcels.verge_m,
                };
                self.march(pass, &site, frontage, &mut rng);
            }
        }
        Ok(self.buildings.len() - before)
    }

    /// Car parks along every carriageway that runs through the district, on
    /// ground its parcels left: each a rectangle `rule` sizes, fronting the
    /// street as a parcel does, at most one to `rule.every_m` of a street's
    /// side, each as long as the most bays that fit. They are kept as
    /// parcels are, so no later parcel or car park stands on one.
    pub fn parking(
        &mut self,
        pass: &Pass,
        district: &DistrictPlan,
        rule: &CourtParking,
    ) -> Vec<[Point; 4]> {
        let mut rng = pass.stream(&format!("parking/{}", district.id));
        let forests = self.forests_near(district);
        let width = 2.0 * rule.bay_m[1] + rule.aisle_m;
        let step = pass.presets.parcels.lot_step_m;
        // A verge kept off every other carriageway, as a parcel's street
        // edge is off its own.
        let verge = pass.presets.parcels.verge_m - 0.01;
        let mut parks = Vec::new();
        for (run, half_width) in self.runs(district) {
            for side in [1.0, -1.0] {
                let frontage = Frontage {
                    run: &run,
                    side,
                    offset: half_width + pass.presets.parcels.verge_m,
                };
                let mut s = rng.unit() * rule.every_m;
                while s < run.length() {
                    let found = (rule.bays[0]..=rule.bays[1]).rev().find_map(|bays| {
                        let depth = bays as f64 * rule.bay_m[0];
                        frontage.place(s, width, depth).filter(|park| {
                            let kept = Rect {
                                center: add(park.rect.center, scale(park.inward, verge / 2.0)),
                                half: [park.rect.half[0] + verge, park.rect.half[1] + verge / 2.0],
                                ..park.rect
                            };
                            self.clear(&park.rect, district, &forests)
                                && !self.network.covers(&kept)
                        })
                    });
                    let Some(park) = found else {
                        s += step;
                        continue;
                    };
                    self.lot_grid
                        .insert(park.rect.bounds(), self.lots.len() as u32);
                    self.lots.push(park.rect);
                    parks.push(park.rect.corners().map(round_cm));
                    s += width + rule.every_m;
                }
            }
        }
        parks
    }

    /// The stretches of each carriageway inside `district`, with its half
    /// width.
    fn runs(&self, district: &DistrictPlan) -> Vec<(Run, f64)> {
        let [x0, y0, x1, y1] = contract::ground::limits(&district.ring, 0.0);
        let apart = |b: &[f64; 4]| b[0] > x1 || b[2] < x0 || b[1] > y1 || b[3] < y0;
        let mut found = Vec::new();
        for way in self.network.ways.iter().filter(|way| !apart(&way.bounds)) {
            // The stretches of this carriageway inside the district.
            let mut runs: Vec<Vec<Point>> = Vec::new();
            for pair in way.samples.windows(2) {
                let (a, b) = (pair[0], pair[1]);
                if apart(&[
                    a[0].min(b[0]),
                    a[1].min(b[1]),
                    a[0].max(b[0]),
                    a[1].max(b[1]),
                ]) {
                    continue;
                }
                // A whole segment keeps its own end, so the run continues.
                let at = |share: f64| {
                    if share == 1.0 {
                        b
                    } else {
                        add(a, scale(sub(b, a), share))
                    }
                };
                for [from, to] in clip(a, b, &district.ring, way.half_width) {
                    let (p, q) = (at(from), at(to));
                    match runs.last_mut() {
                        Some(run) if run[run.len() - 1] == p => run.push(q),
                        _ => runs.push(vec![p, q]),
                    }
                }
            }
            found.extend(
                runs.into_iter()
                    .map(|points| (Run::new(points), way.half_width)),
            );
        }
        found
    }

    /// Walk one side of a street, cutting a parcel wherever a template of
    /// the district fits and stepping on where none does.
    fn march(&mut self, pass: &Pass, site: &Site, frontage: Frontage, rng: &mut Stream) {
        let parcels = &pass.presets.parcels;
        let rule = &site.rule;
        let mut s = 0.0;
        while s < frontage.run.length() {
            let mut advance = parcels.lot_step_m;
            // One category for this place, then a few of its templates: a
            // large one that does not fit gives way to its own kind, not to
            // the district's other category.
            let eligible = site.choices.category(rng);
            for _ in 0..pass.presets.retries.fit {
                let fit = eligible[rng.below(eligible.len() as u64) as usize];
                let Some(lot) = frontage.lot(s, fit, rule) else {
                    continue;
                };
                if !self.clear(&lot.rect, site.district, &site.forests) {
                    continue;
                }
                advance = 2.0 * lot.rect.half[0];
                let number = self.plan_lots.len() - self.first_lot;
                let id = format!("{}/lot-{number}", site.district.id);
                self.lot_grid
                    .insert(lot.rect.bounds(), self.lots.len() as u32);
                self.lots.push(lot.rect);
                self.plan_lots.push(LotPlan {
                    id: id.clone(),
                    ring: lot.rect.corners().map(round_cm).to_vec(),
                });
                // A district is built ground: its first parcel always is, and
                // the rest by the district's coverage.
                let first = self.plan_lots.len() - self.first_lot == 1;
                if rng.chance(rule.coverage) || first {
                    self.build(id, &lot, fit, site, parcels.verge_m, &parcels.prop_kind);
                }
                break;
            }
            s += advance;
        }
    }

    fn build(&mut self, id: String, lot: &Lot, fit: &Fit, site: &Site, verge: f64, kind: &str) {
        let rule = &site.rule;
        self.buildings
            .push(placement(id, lot, fit, rule, kind, &mut self.next_prop));
        // The apron is as wide as the building, so each stands on its own
        // yard with the side setbacks left open between them. It is paving:
        // hard ground, no way through. A parcel paved whole as its yard
        // (`courts`) has its apron only across the verge in front of it.
        if rule.apron_m.min(rule.front_m) > 0.0 {
            let depth = if site.yards_paved {
                0.0
            } else {
                rule.apron_m.min(rule.front_m)
            };
            let half = (fit.max[0] - fit.min[0]) / 2.0;
            let ring = [
                [-half, -verge],
                [half, -verge],
                [half, depth],
                [-half, depth],
            ]
            .map(|[x, y]| round_cm(lot.point(x, y)));
            if let Ok(shape) = GroundShape::polygon(ring.to_vec()) {
                self.aprons.push(SurfaceArea {
                    kind: SurfaceKind::Paving,
                    shape,
                });
            }
        }
    }
}

/// The template of `fit` stood on `lot`: its box centred across the parcel
/// and set back from the front, its parts numbered on from `next_prop`.
pub fn placement(
    id: String,
    lot: &Lot,
    fit: &Fit,
    rule: &LotRule,
    kind: &str,
    next_prop: &mut u32,
) -> BuildingPlacement {
    let parts: Vec<BuildingPartReference> = fit
        .template
        .parts
        .iter()
        .map(|part| {
            *next_prop += 1;
            BuildingPartReference {
                part: part.id.clone(),
                prop: *next_prop - 1,
            }
        })
        .collect();
    // The template's origin, from the parcel's front centre.
    let origin = lot.point(-(fit.min[0] + fit.max[0]) / 2.0, rule.front_m - fit.min[1]);
    let [x, y] = round_cm(origin);
    let yaw = fit.turn + libm::atan2(lot.rect.axis[1], lot.rect.axis[0]);
    BuildingPlacement {
        id,
        template_id: fit.template.id.clone(),
        kind: kind.into(),
        owner: parts[0].prop,
        parts,
        frame: PlacementFrame {
            translation: [x, y, 0.0],
            // Microradians: a tenth of a millimetre across a parcel.
            yaw: libm::round(yaw * 1e6) / 1e6,
        },
    }
}

/// The district being filled: its rule, what it may select and the woods
/// near enough to matter.
struct Site<'a> {
    district: &'a DistrictPlan,
    rule: LotRule,
    /// Whether its built parcels are paved whole as their yards.
    yards_paved: bool,
    choices: Choices<'a>,
    forests: Vec<&'a [Point]>,
}

/// One side of a stretch of carriageway that parcels may front.
pub struct Frontage<'a> {
    pub run: &'a Run,
    /// `1` for the left of the run's direction, `−1` for the right.
    pub side: f64,
    /// From the centreline to a parcel's front.
    pub offset: f64,
}

pub struct Lot {
    pub rect: Rect,
    /// The middle of its street edge.
    pub front: Point,
    /// Unit vector from the street into the parcel.
    pub inward: Point,
}

impl Lot {
    /// A point `x` metres along the parcel's front and `y` into it.
    pub fn point(&self, x: f64, y: f64) -> Point {
        add(
            self.front,
            add(scale(self.rect.axis, x), scale(self.inward, y)),
        )
    }
}

impl Frontage<'_> {
    /// The parcel for `fit` whose front starts `s` metres along the run:
    /// `None` when the run ends first or bends too far to front it.
    pub fn lot(&self, s: f64, fit: &Fit, rule: &LotRule) -> Option<Lot> {
        let [width, depth] = fit.lot(rule);
        self.place(s, width, depth)
    }

    /// A rectangle `width` along the front from `s` metres along the run
    /// and `depth` deep: `None` when the run ends first or bends too far to
    /// front it.
    pub fn place(&self, s: f64, width: f64, depth: f64) -> Option<Lot> {
        if s + width > self.run.length() {
            return None;
        }
        let (a, b) = (self.run.at(s), self.run.at(s + width));
        let chord = distance(a, b);
        // A front is straight; a bend that shortens it by more than this
        // would push the parcel's corners into the carriageway.
        if chord < 0.98 * width {
            return None;
        }
        let along = scale(sub(b, a), 1.0 / chord);
        let inward = scale([-along[1], along[0]], self.side);
        let front = add(scale(add(a, b), 0.5), scale(inward, self.offset));
        Some(Lot {
            rect: Rect {
                center: add(front, scale(inward, depth / 2.0)),
                // Right-handed with `inward`, so the template's `+X` runs
                // along the front whichever side of the street it is on.
                axis: [inward[1], -inward[0]],
                half: [width / 2.0, depth / 2.0],
            },
            front,
            inward,
        })
    }
}

/// The templates a place may select: for each building category of its mix,
/// by the category's weight, the templates of the map's one regional family
/// within its floors.
pub struct Choices<'a> {
    /// (weight, the category's eligible templates), in the mix's order.
    categories: Vec<(f64, Vec<&'a Fit<'a>>)>,
}

impl<'a> Choices<'a> {
    /// Every category of `mix`, each with its eligible templates (perhaps
    /// none), in `mix`'s order.
    fn eligible(
        fits: &'a [Fit<'a>],
        mix: impl IntoIterator<Item = (BuildingCategory, f64)>,
        family: &str,
        max_floors: Option<u32>,
    ) -> Vec<(BuildingCategory, f64, Vec<&'a Fit<'a>>)> {
        mix.into_iter()
            .map(|(category, weight)| {
                let of_it = fits
                    .iter()
                    .filter(|fit| {
                        fit.template.category == category
                            && fit.template.regional_family == family
                            && max_floors.is_none_or(|most| fit.floors() <= most as usize)
                    })
                    .collect();
                (category, weight, of_it)
            })
            .collect()
    }

    /// A district's choices. A category of its mix that the catalogue cannot
    /// build is refused: the district would not be what its kind says.
    pub fn district(
        fits: &'a [Fit<'a>],
        district: &DistrictPlan,
        family: &str,
    ) -> Result<Self, Diagnostic> {
        let mix = district.categories.iter().map(|s| (s.category, s.weight));
        let mut categories = Vec::new();
        for (category, weight, eligible) in Self::eligible(fits, mix, family, district.max_floors) {
            if eligible.is_empty() {
                return Err(Diagnostic {
                    code: DiagnosticCode::MissingTemplate,
                    feature: Some(district.id.clone()),
                    location: "$.catalogue".into(),
                    message: format!(
                        "the {family} family has no complete {category:?} template within {:?} floors",
                        district.max_floors
                    ),
                });
            }
            categories.push((weight, eligible));
        }
        Ok(Self { categories })
    }

    /// The choices of `mix` the catalogue can build; a category it cannot is
    /// left out, and none at all leaves nothing to choose.
    pub fn available(
        fits: &'a [Fit<'a>],
        mix: impl IntoIterator<Item = (BuildingCategory, f64)>,
        family: &str,
        max_floors: Option<u32>,
    ) -> Self {
        let categories = Self::eligible(fits, mix, family, max_floors)
            .into_iter()
            .filter(|(_, _, eligible)| !eligible.is_empty())
            .map(|(_, weight, eligible)| (weight, eligible))
            .collect();
        Self { categories }
    }

    pub fn is_empty(&self) -> bool {
        self.categories.is_empty()
    }

    /// One category's eligible templates, drawn by the categories' weights.
    /// There must be a category to draw.
    pub fn category(&self, rng: &mut Stream) -> &[&'a Fit<'a>] {
        rng.pick(
            self.categories
                .iter()
                .map(|(weight, eligible)| (eligible.as_slice(), *weight)),
        )
        .expect("a choice has a category")
    }
}
