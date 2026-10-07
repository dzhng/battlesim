//! The one legality check every body of street furniture, court and garden
//! goes through ([`Field::legal`]), and the bodies placed so far.
use crate::layout::geometry::{direction, round_cm, scale, Grid, Point};
use crate::layout::water::Water;
use crate::layout::{Corridor, PropBox, StreetProps};
use crate::parcels::space::Rect;
use contract::ground::{polygon_contains, GroundShape};
use contract::map::{AuthoredPropDefinition, Forest, PropDefinition};

/// One straight piece of a carriageway.
pub(super) struct Piece {
    pub(super) a: Point,
    pub(super) b: Point,
    pub(super) half_width: f64,
    pub(super) way: u32,
    /// Where it starts and ends along its way.
    pub(super) along: [f64; 2],
}

/// A body on the map so far.
pub(super) struct Body {
    pub(super) rect: Rect,
    pub(super) clear: f64,
    /// Bodies of one group (a run of cars, a construction site) stand as
    /// close to each other as they like; 0 is no group.
    pub(super) group: u32,
}

/// A body asking for ground.
#[derive(Clone, Copy)]
pub(super) struct Candidate {
    /// The box as the map will hold it: its centre in whole centimetres
    /// and its heading in microradians.
    pub(super) rect: Rect,
    pub(super) yaw: f64,
    pub(super) clear: f64,
    pub(super) group: u32,
    /// The carriageway it stands beside and how far along: the one it
    /// keeps only the lane's distance from, whatever `corner` asks of the
    /// others.
    pub(super) beside: Option<(u32, f64)>,
    /// Kept between it and any other carriageway's edge.
    pub(super) corner: f64,
    /// It stands on the carriageway it is beside, in the half its centre is
    /// on (a car abandoned in the road); the way it leaves open beside it is
    /// kept clear by its placement (`strand`).
    pub(super) in_road: bool,
}

/// Everything a body must keep clear of, and the bodies placed so far.
pub(super) struct Field<'a> {
    pub(super) size: [f64; 2],
    pub(super) rule: &'a StreetProps,
    /// No body stands nearer a carriageway's middle than this: the lane a
    /// vehicle drives beside the middle, and the room its route is checked
    /// with.
    pub(super) lane: f64,
    pub(super) pieces: Vec<Piece>,
    pub(super) piece_grid: Grid,
    pub(super) widest: f64,
    pub(super) objectives: Vec<Rect>,
    pub(super) walls: Vec<Rect>,
    pub(super) wall_grid: Grid,
    /// Each door's way to the street.
    pub(super) doors: Vec<Rect>,
    pub(super) door_grid: Grid,
    pub(super) bodies: Vec<Body>,
    pub(super) body_grid: Grid,
    pub(super) most_clear: f64,
    pub(super) water: Water<'a>,
    pub(super) bank: f64,
    /// Bridge decks with the straight run onto each.
    pub(super) decks: Vec<Rect>,
    pub(super) corridors: Vec<Corridor>,
    /// Forests no body may come within `wood_clear` of: a tree is not
    /// stood within its clearance of a body.
    pub(super) woods: Vec<&'a GroundShape>,
    pub(super) wood_grid: Grid,
    pub(super) wood_clear: f64,
    pub(super) groups: u32,
    /// Every fence panel placed, with its group: no other fence runs
    /// beside one (`beside_run`).
    pub(super) runs: Vec<(Rect, u32)>,
    pub(super) run_grid: Grid,
    pub(super) placed: Vec<AuthoredPropDefinition>,
}

/// Whether all of `rect` stays on the side of `piece`'s middle line its
/// centre is on.
fn in_its_half(rect: &Rect, piece: &Piece) -> bool {
    let run = [piece.b[0] - piece.a[0], piece.b[1] - piece.a[1]];
    let off = |p: Point| run[0] * (p[1] - piece.a[1]) - run[1] * (p[0] - piece.a[0]);
    let side = off(rect.center).signum();
    rect.corners().into_iter().all(|p| off(p) * side >= 0.0)
}

/// Whether `a` and `b` stand at least `gap` apart.
pub(super) fn apart(a: &Rect, b: &Rect, gap: f64) -> bool {
    let grown = Rect {
        half: [a.half[0] + gap, a.half[1] + gap],
        ..*a
    };
    !grown.overlaps(b, 0.0)
}

/// The four faces of `wall`, each as the way it looks, how far out from
/// the wall's middle it stands and half its length.
pub(super) fn faces(wall: &Rect) -> [(Point, f64, f64); 4] {
    let turn = [-wall.axis[1], wall.axis[0]];
    [
        (wall.axis, wall.half[0], wall.half[1]),
        (scale(wall.axis, -1.0), wall.half[0], wall.half[1]),
        (turn, wall.half[1], wall.half[0]),
        (scale(turn, -1.0), wall.half[1], wall.half[0]),
    ]
}

pub(super) fn grow(bounds: [f64; 4], by: f64) -> [f64; 4] {
    [
        bounds[0] - by,
        bounds[1] - by,
        bounds[2] + by,
        bounds[3] + by,
    ]
}

impl<'a> Field<'a> {
    /// The one legality check: whether `c` may stand where it asks.
    pub(super) fn legal(&self, c: &Candidate) -> bool {
        self.open(c)
            && !self.door_grid.any(c.rect.bounds(), |item| {
                c.rect.overlaps(&self.doors[item as usize], 0.0)
            })
    }

    /// Whether `c` may be kept as open ground: everywhere a body may stand,
    /// and on ground already kept clear (a door's way, the ground round a
    /// court group), which it leaves as open as it was.
    pub(super) fn open(&self, c: &Candidate) -> bool {
        let rect = &c.rect;
        let bounds = rect.bounds();
        if bounds[0] < self.rule.edge_m
            || bounds[1] < self.rule.edge_m
            || bounds[2] > self.size[0] - self.rule.edge_m
            || bounds[3] > self.size[1] - self.rule.edge_m
        {
            return false;
        }
        // Off every carriageway and the lane driven beside its middle; and,
        // where the row asks, back from the corners other carriageways make.
        let reach = self
            .lane
            .max(self.widest + self.rule.kerb_gap_m.max(c.corner));
        // The stretch of its own carriageway a body stands beside: as far
        // along as a piece of it, running straight on, could still lie
        // within the corner's distance. Past that the way has come round
        // again, and is another road to the body.
        let window = reach + rect.half[0] + rect.half[1] + self.widest;
        let on_road = self.piece_grid.any(grow(bounds, reach), |item| {
            let piece = &self.pieces[item as usize];
            let lane = self.lane.max(piece.half_width + self.rule.kerb_gap_m);
            let beside = c.beside.is_some_and(|(way, s)| {
                way == piece.way && piece.along[1] >= s - window && piece.along[0] <= s + window
            });
            if beside && c.in_road {
                return !in_its_half(rect, piece);
            }
            let need = if beside {
                lane
            } else {
                lane.max(piece.half_width + c.corner)
            };
            rect.segment_gap(piece.a, piece.b) < need
        });
        if on_road {
            return false;
        }
        let wall = self.rule.wall_gap_m;
        if self.objectives.iter().any(|r| rect.overlaps(r, 0.0)) {
            return false;
        }
        if self.wall_grid.any(grow(bounds, wall), |item| {
            !apart(rect, &self.walls[item as usize], wall)
        }) {
            return false;
        }
        let crowded = self
            .body_grid
            .any(grow(bounds, self.most_clear.max(c.clear)), |item| {
                let body = &self.bodies[item as usize];
                if c.group != 0 && body.group == c.group {
                    // Neighbours of one group may touch; a centimetre is rounding.
                    rect.overlaps(&body.rect, 0.02)
                } else {
                    !apart(rect, &body.rect, body.clear.max(c.clear))
                }
            });
        if crowded {
            return false;
        }
        let points = rect.corners().into_iter().chain([rect.center]);
        for p in points {
            if !self.water.is_empty() && self.water.gap(p, self.bank) < self.bank {
                return false;
            }
            if self.corridors.iter().any(|corridor| corridor.contains(p)) {
                return false;
            }
        }
        if self.decks.iter().any(|deck| rect.overlaps(deck, 0.0)) {
            return false;
        }
        let clear = self.wood_clear;
        !self.wood_grid.any(grow(bounds, clear), |item| {
            match self.woods[item as usize] {
                GroundShape::Polygon { ring } => {
                    polygon_contains(ring, rect.center)
                        || contract::ground::edges(ring)
                            .any(|(a, b)| rect.segment_gap(*a, *b) < clear)
                }
                GroundShape::Stroke {
                    centerline,
                    width_m,
                } => centerline
                    .samples()
                    .windows(2)
                    .any(|pair| rect.segment_gap(pair[0], pair[1]) < width_m / 2.0 + clear),
            }
        })
    }

    /// Every body `props` already stands on the map, kept clear of as a
    /// placed body is: by the room its row keeps, or none for a kind street
    /// furniture does not place.
    pub(super) fn stand_plan_bodies(&mut self, props: &[AuthoredPropDefinition]) {
        for prop in props {
            let p = &prop.geometry;
            let rect = Rect {
                center: p.center,
                axis: direction(p.yaw),
                half: [p.half_extents[0], p.half_extents[1]],
            };
            self.body_grid
                .insert(rect.bounds(), self.bodies.len() as u32);
            self.bodies.push(Body {
                rect,
                clear: self.rule.bodies.get(&p.kind).map_or(0.0, |b| b.clear_m),
                group: 0,
            });
        }
    }

    /// Keep every body `clear` off each of `forests`.
    pub(super) fn keep_off_woods(&mut self, forests: &'a [Forest], clear: f64) {
        self.wood_clear = clear + super::SLACK_M;
        for forest in forests {
            self.wood_grid
                .insert(forest.shape.limits(), self.woods.len() as u32);
            self.woods.push(&forest.shape);
        }
    }

    /// Whether a fence panel `c` would run beside another group's fence,
    /// the way it runs and nearer than `gap`: two runs side by side, with
    /// ground between them that is no one's. One that meets it end on (a
    /// side against a neighbour's rear) does not.
    pub(super) fn beside_run(&self, c: &Candidate, gap: f64) -> bool {
        self.run_grid.any(grow(c.rect.bounds(), gap), |item| {
            let (run, group) = &self.runs[item as usize];
            let along = c.rect.axis[0] * run.axis[0] + c.rect.axis[1] * run.axis[1];
            *group != c.group && libm::fabs(along) > 0.95 && !apart(&c.rect, run, gap)
        })
    }

    /// Stand fence panel `c` as a body of `kind`, as [`Field::place`] does,
    /// and remember it as a run.
    pub(super) fn place_run(&mut self, kind: &str, body: &PropBox, c: &Candidate) {
        self.run_grid
            .insert(c.rect.bounds(), self.runs.len() as u32);
        self.runs.push((c.rect, c.group));
        self.place(kind, body, c);
    }

    pub(super) fn group(&mut self) -> u32 {
        self.groups += 1;
        self.groups
    }

    /// Whether no body placed so far stands on `rect`.
    pub(super) fn clear_of_bodies(&self, rect: &Rect) -> bool {
        !self.body_grid.any(rect.bounds(), |item| {
            rect.overlaps(&self.bodies[item as usize].rect, 0.0)
        })
    }

    /// Keep `way` open, as a door's way to the street is: no body may
    /// stand on it.
    pub(super) fn keep_clear(&mut self, way: Rect) {
        self.door_grid.insert(way.bounds(), self.doors.len() as u32);
        self.doors.push(way);
    }

    /// Stand `c` on the map as a body of `kind`, `body`'s height and its
    /// own length and width (a cut-to-fit panel's may be cut shorter).
    pub(super) fn place(&mut self, kind: &str, body: &PropBox, c: &Candidate) {
        self.body_grid
            .insert(c.rect.bounds(), self.bodies.len() as u32);
        self.bodies.push(Body {
            rect: c.rect,
            clear: c.clear,
            group: c.group,
        });
        self.placed.push(AuthoredPropDefinition {
            id: None,
            geometry: PropDefinition {
                kind: kind.into(),
                center: c.rect.center,
                yaw: c.yaw,
                half_extents: [c.rect.half[0], c.rect.half[1], body.half_extents_m[2]],
                base_z: None,
                wreck_of: None,
            },
        });
    }
}

impl Candidate {
    /// A body of `body`'s box centred on `center`, lying along `along`, in
    /// no group and beside no carriageway.
    pub(super) fn new(body: &PropBox, center: Point, along: Point) -> Self {
        let yaw = libm::round(libm::atan2(along[1], along[0]) * 1e6) / 1e6;
        Self {
            rect: Rect {
                center: round_cm(center),
                axis: direction(yaw),
                half: [body.half_extents_m[0], body.half_extents_m[1]],
            },
            yaw,
            clear: body.clear_m,
            group: 0,
            beside: None,
            corner: 0.0,
            in_road: false,
        }
    }
}
