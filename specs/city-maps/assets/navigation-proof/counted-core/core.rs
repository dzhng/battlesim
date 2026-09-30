use super::topology::{fold, point_hash, Point, Prepared, QueryCursor, QueryStep};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    ops::Bound::{Excluded, Included, Unbounded},
    sync::Arc,
};
pub trait Physical {
    type Job;
    fn identity(&self) -> u64;
    fn begin(&self, a: Point, b: Point, road: bool) -> Self::Job;
    fn poll(&self, job: &mut Self::Job, budget: &mut usize) -> Option<Option<f64>>;
    fn digest(&self, job: &Self::Job) -> u64;
    fn reference_digest(&self, job: &Self::Job) -> u64 {
        self.digest(job)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct EdgeKey {
    from: u32,
    to: u32,
    source: Option<u32>,
}
fn edge_hash(e: EdgeKey) -> u64 {
    fold(
        fold(e.from as u64, e.to as u64),
        e.source.map_or(u64::MAX, |s| s as u64),
    )
}
#[derive(Clone, Copy, Debug)]
struct Label {
    generation: u32,
    g: f64,
    parent: Option<EdgeKey>,
}
fn label_hash(id: u32, l: Label) -> u64 {
    fold(
        fold(id as u64, l.g.to_bits()),
        fold(l.generation as u64, l.parent.map_or(0, edge_hash)),
    )
}
#[derive(Clone, Copy, Debug)]
struct Open {
    cost: f64,
    node: u32,
    serial: u64,
}
impl PartialEq for Open {
    fn eq(&self, b: &Self) -> bool {
        self.cmp(b) == Ordering::Equal
    }
}
impl Eq for Open {}
impl PartialOrd for Open {
    fn partial_cmp(&self, b: &Self) -> Option<Ordering> {
        Some(self.cmp(b))
    }
}
impl Ord for Open {
    fn cmp(&self, b: &Self) -> Ordering {
        self.cost
            .total_cmp(&b.cost)
            .then(self.node.cmp(&b.node))
            .then(self.serial.cmp(&b.serial))
    }
}
fn open_hash(o: Open) -> u64 {
    fold(fold(o.cost.to_bits(), o.node as u64), o.serial)
}
#[derive(Clone, Copy, Debug)]
struct Splice {
    edge: u32,
    t: f64,
    node: u32,
}
impl PartialEq for Splice {
    fn eq(&self, b: &Self) -> bool {
        self.cmp(b) == Ordering::Equal
    }
}
impl Eq for Splice {}
impl PartialOrd for Splice {
    fn partial_cmp(&self, b: &Self) -> Option<Ordering> {
        Some(self.cmp(b))
    }
}
impl Ord for Splice {
    fn cmp(&self, b: &Self) -> Ordering {
        self.edge
            .cmp(&b.edge)
            .then(self.t.total_cmp(&b.t))
            .then(self.node.cmp(&b.node))
    }
}
fn splice_hash(s: Splice, p: Point) -> u64 {
    fold(
        fold(s.edge as u64, s.t.to_bits()),
        fold(s.node as u64, point_hash(p)),
    )
}
#[derive(Clone, Copy, Debug)]
struct Candidate {
    edge: u32,
    t: f64,
    p: Point,
}
#[derive(Clone, Copy, Debug)]
enum Expansion {
    Start(Option<u32>),
    Static { at: usize, goal: bool },
    Portal { dir: u8, goal: bool },
}
#[derive(Clone, Copy, Debug)]
enum Stage {
    Query,
    Project(u32),
    BeginConnector,
    Connector,
    Insert,
    Seed,
    Pop,
    Expand,
    Relax(EdgeKey, f64),
    Trace(u32),
    Validate(usize),
    BeginRoad(usize, EdgeKey),
    Road(usize, EdgeKey),
    DrainQueue,
    DrainPath,
    Ready,
    Declined,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Pending,
    Ready,
    CorridorDeclined,
}
pub struct Request<P: Physical> {
    topology: Arc<Prepared>,
    physical: P,
    start: Point,
    goal: Point,
    radius: f64,
    road_cost: f64,
    stage: Stage,
    end: u8,
    link_counts: [usize; 2],
    query: QueryCursor,
    candidate: Option<Candidate>,
    physical_job: Option<P::Job>,
    connector_cost: f64,
    portals: BTreeMap<u32, Splice>,
    splices: BTreeMap<Splice, Point>,
    links: BTreeMap<(u32, u32), f64>,
    labels: BTreeMap<u32, Label>,
    queue: BTreeSet<Open>,
    excluded: BTreeSet<EdgeKey>,
    path: BTreeMap<usize, u32>,
    labels_hash: u64,
    queue_hash: u64,
    splices_hash: u64,
    portals_hash: u64,
    links_hash: u64,
    excluded_hash: u64,
    path_hash: u64,
    next_node: u32,
    serial: u64,
    generation: u32,
    current: u32,
    expansion: Expansion,
    pub work: usize,
    pub searches: usize,
    pub physical_checks: usize,
    pub actual_cost: f64,
    pub coarse_cost: f64,
}
impl<P: Physical> Request<P> {
    pub fn new(
        topology: Arc<Prepared>,
        physical: P,
        start: Point,
        goal: Point,
        radius: f64,
        road_cost: f64,
    ) -> Self {
        assert!(
            radius.is_finite() && radius >= 0.0 && road_cost.is_finite() && road_cost >= 0.0,
            "proof input admission"
        );
        let next_node = u32::try_from(topology.points.len())
            .unwrap()
            .checked_add(2)
            .unwrap();
        let query = QueryCursor::new(&topology.index, start, radius);
        Self {
            topology,
            physical,
            start,
            goal,
            radius,
            road_cost,
            stage: Stage::Query,
            end: 0,
            link_counts: [0; 2],
            query,
            candidate: None,
            physical_job: None,
            connector_cost: 0.0,
            portals: BTreeMap::new(),
            splices: BTreeMap::new(),
            links: BTreeMap::new(),
            labels: BTreeMap::new(),
            queue: BTreeSet::new(),
            excluded: BTreeSet::new(),
            path: BTreeMap::new(),
            labels_hash: 0,
            queue_hash: 0,
            splices_hash: 0,
            portals_hash: 0,
            links_hash: 0,
            excluded_hash: 0,
            path_hash: 0,
            next_node,
            serial: 0,
            generation: 0,
            current: 0,
            expansion: Expansion::Start(None),
            work: 0,
            searches: 0,
            physical_checks: 0,
            actual_cost: 0.0,
            coarse_cost: 0.0,
        }
    }
    fn point(&self, node: u32) -> Point {
        match node {
            0 => self.start,
            1 => self.goal,
            _ => self
                .portals
                .get(&node)
                .map_or_else(|| self.topology.point(node), |s| self.splices[s]),
        }
    }
    fn put_label(&mut self, id: u32, l: Label) {
        if let Some(old) = self.labels.insert(id, l) {
            self.labels_hash ^= label_hash(id, old);
        }
        self.labels_hash ^= label_hash(id, l);
    }
    fn push(&mut self, node: u32, cost: f64) {
        let o = Open {
            node,
            cost,
            serial: self.serial,
        };
        self.serial = self.serial.checked_add(1).unwrap();
        assert!(self.queue.insert(o));
        self.queue_hash ^= open_hash(o);
    }
    fn pop(&mut self) -> Option<Open> {
        let o = self.queue.pop_first()?;
        self.queue_hash ^= open_hash(o);
        Some(o)
    }
    fn insert_connector(&mut self) {
        let c = self.candidate.take().unwrap();
        let edge = self.topology.edges[c.edge as usize];
        let node = if c.t == 0.0 {
            edge.a + 2
        } else if c.t == 1.0 {
            edge.b + 2
        } else {
            let id = self.next_node;
            self.next_node = self.next_node.checked_add(1).unwrap();
            let s = Splice {
                edge: c.edge,
                t: c.t,
                node: id,
            };
            self.portals.insert(id, s);
            self.portals_hash ^= fold(id as u64, splice_hash(s, Point(0.0, 0.0)));
            self.splices.insert(s, c.p);
            self.splices_hash ^= splice_hash(s, c.p);
            id
        };
        let pair = if self.end == 0 { (0, node) } else { (node, 1) };
        if let Some(old) = self.links.insert(pair, self.connector_cost) {
            self.links_hash ^= fold(fold(pair.0 as u64, pair.1 as u64), old.to_bits());
        } else {
            self.link_counts[self.end as usize] += 1;
        }
        self.links_hash ^= fold(
            fold(pair.0 as u64, pair.1 as u64),
            self.connector_cost.to_bits(),
        );
    }
    fn first_portal(&self, edge: u32, reverse: bool) -> Option<u32> {
        let lo = Splice {
            edge,
            t: f64::NEG_INFINITY,
            node: 0,
        };
        let hi = Splice {
            edge,
            t: f64::INFINITY,
            node: u32::MAX,
        };
        let mut range = self.splices.range((Included(lo), Included(hi)));
        if reverse {
            range.next_back()
        } else {
            range.next()
        }
        .map(|(s, _)| s.node)
    }
    fn next_neighbor(&mut self) -> Option<(EdgeKey, f64)> {
        let from = self.current;
        match &mut self.expansion {
            Expansion::Start(last) => {
                let lo = last.map_or(Included((0, 0)), |v| Excluded((0, v)));
                let next =
                    self.links
                        .range((lo, Included((0, u32::MAX))))
                        .next()
                        .map(|(&(a, b), &c)| {
                            (
                                EdgeKey {
                                    from: a,
                                    to: b,
                                    source: None,
                                },
                                c,
                            )
                        });
                if let Some((e, _)) = next {
                    *last = Some(e.to);
                }
                next
            }
            Expansion::Static { at, goal } => {
                let vertex = (from - 2) as usize;
                let incident = &self.topology.incident[vertex];
                if *at < incident.len() {
                    let id = incident[*at];
                    *at += 1;
                    let e = self.topology.edges[id as usize];
                    let reverse = from == e.b + 2;
                    let to = self.first_portal(id, reverse).unwrap_or(if reverse {
                        e.a + 2
                    } else {
                        e.b + 2
                    });
                    Some((
                        EdgeKey {
                            from,
                            to,
                            source: Some(id),
                        },
                        self.point(from).distance(self.point(to)) * self.road_cost,
                    ))
                } else if !*goal {
                    *goal = true;
                    self.links.get(&(from, 1)).map(|&c| {
                        (
                            EdgeKey {
                                from,
                                to: 1,
                                source: None,
                            },
                            c,
                        )
                    })
                } else {
                    None
                }
            }
            Expansion::Portal { dir, goal } => {
                let s = self.portals[&from];
                let e = self.topology.edges[s.edge as usize];
                if *dir < 2 {
                    let reverse = *dir == 0;
                    *dir += 1;
                    let other = if reverse {
                        self.splices.range(..s).next_back()
                    } else {
                        self.splices.range((Excluded(s), Unbounded)).next()
                    };
                    let to = other
                        .filter(|(p, _)| p.edge == s.edge)
                        .map_or(if reverse { e.a + 2 } else { e.b + 2 }, |(p, _)| p.node);
                    Some((
                        EdgeKey {
                            from,
                            to,
                            source: Some(s.edge),
                        },
                        self.point(from).distance(self.point(to)) * self.road_cost,
                    ))
                } else if !*goal {
                    *goal = true;
                    self.links.get(&(from, 1)).map(|&c| {
                        (
                            EdgeKey {
                                from,
                                to: 1,
                                source: None,
                            },
                            c,
                        )
                    })
                } else {
                    None
                }
            }
        }
    }
    fn poll_physical(&mut self, budget: &mut usize) -> Option<Option<f64>> {
        let before = *budget;
        let result = self
            .physical
            .poll(self.physical_job.as_mut().unwrap(), budget);
        assert!(*budget <= before, "physical evaluator cannot mint credits");
        assert!(
            result.is_some() || *budget < before,
            "pending physical work must advance"
        );
        result
    }
    pub fn physical_context(&self) -> &P {
        &self.physical
    }
    pub fn prepared(&self) -> &Arc<Prepared> {
        &self.topology
    }
    pub fn status(&self) -> Status {
        match self.stage {
            Stage::Ready => Status::Ready,
            Stage::Declined => Status::CorridorDeclined,
            _ => Status::Pending,
        }
    }
    pub fn route(&self) -> impl Iterator<Item = Point> + '_ {
        self.path.values().rev().map(|&id| self.point(id))
    }
    pub fn poll(&mut self, budget: &mut usize) -> Status {
        let before = *budget;
        while *budget > 0 && self.status() == Status::Pending {
            if !matches!(self.stage, Stage::Connector | Stage::Road(..)) {
                *budget -= 1;
            }
            match self.stage {
                Stage::Query => match self.query.step(&self.topology.index) {
                    QueryStep::Pending => {}
                    QueryStep::Hit(id) => self.stage = Stage::Project(id as u32),
                    QueryStep::Done => {
                        if self.end == 0 {
                            self.end = 1;
                            self.query =
                                QueryCursor::new(&self.topology.index, self.goal, self.radius);
                        } else if self.link_counts[0] == 0 || self.link_counts[1] == 0 {
                            self.stage = Stage::Declined;
                        } else {
                            self.stage = Stage::Seed;
                        }
                    }
                },
                Stage::Project(id) => {
                    let e = self.topology.edges[id as usize].segment;
                    let p = if self.end == 0 { self.start } else { self.goal };
                    let (dx, dy) = (e.b.0 - e.a.0, e.b.1 - e.a.1);
                    let len = dx * dx + dy * dy;
                    if len == 0.0 {
                        self.stage = Stage::Query;
                        continue;
                    }
                    let t = (((p.0 - e.a.0) * dx + (p.1 - e.a.1) * dy) / len).clamp(0.0, 1.0);
                    let q = if t == 0.0 {
                        e.a
                    } else if t == 1.0 {
                        e.b
                    } else {
                        Point(e.a.0 + dx * t, e.a.1 + dy * t)
                    };
                    if p.distance(q) <= self.radius {
                        self.candidate = Some(Candidate { edge: id, t, p: q });
                        self.stage = Stage::BeginConnector;
                    } else {
                        self.stage = Stage::Query;
                    }
                }
                Stage::BeginConnector => {
                    let c = self.candidate.unwrap();
                    let (a, b) = if self.end == 0 {
                        (self.start, c.p)
                    } else {
                        (c.p, self.goal)
                    };
                    self.physical_job = Some(self.physical.begin(a, b, false));
                    self.physical_checks += 1;
                    self.stage = Stage::Connector;
                }
                Stage::Connector => {
                    if let Some(result) = self.poll_physical(budget) {
                        self.physical_job = None;
                        if let Some(c) = result.filter(|c| c.is_finite() && *c >= 0.0) {
                            self.connector_cost = c;
                            self.stage = Stage::Insert;
                        } else {
                            self.candidate = None;
                            self.stage = Stage::Query;
                        }
                    }
                }
                Stage::Insert => {
                    self.insert_connector();
                    self.stage = Stage::Query;
                }
                Stage::Seed => {
                    self.generation = self.generation.checked_add(1).unwrap();
                    self.searches += 1;
                    self.actual_cost = 0.0;
                    self.put_label(
                        0,
                        Label {
                            generation: self.generation,
                            g: 0.0,
                            parent: None,
                        },
                    );
                    self.push(0, 0.0);
                    self.stage = Stage::Pop;
                }
                Stage::Pop => {
                    if let Some(o) = self.pop() {
                        let l = self.labels[&o.node];
                        if l.generation != self.generation || o.cost > l.g {
                            continue;
                        }
                        if o.node == 1 {
                            self.coarse_cost = o.cost;
                            self.stage = Stage::Trace(1);
                        } else {
                            self.current = o.node;
                            self.expansion = if o.node == 0 {
                                Expansion::Start(None)
                            } else if self.portals.contains_key(&o.node) {
                                Expansion::Portal {
                                    dir: 0,
                                    goal: false,
                                }
                            } else {
                                Expansion::Static { at: 0, goal: false }
                            };
                            self.stage = Stage::Expand;
                        }
                    } else {
                        self.stage = Stage::Declined;
                    }
                }
                Stage::Expand => {
                    if let Some((edge, cost)) = self.next_neighbor() {
                        self.stage = Stage::Relax(edge, cost);
                    } else {
                        self.stage = Stage::Pop;
                    }
                }
                Stage::Relax(edge, cost) => {
                    let g = self.labels[&edge.from].g + cost;
                    if g.is_finite()
                        && !self.excluded.contains(&edge)
                        && self
                            .labels
                            .get(&edge.to)
                            .is_none_or(|l| l.generation != self.generation || g < l.g)
                    {
                        self.put_label(
                            edge.to,
                            Label {
                                generation: self.generation,
                                g,
                                parent: Some(edge),
                            },
                        );
                        self.push(edge.to, g);
                    }
                    self.stage = Stage::Expand;
                }
                Stage::Trace(node) => {
                    let i = self.path.len();
                    self.path.insert(i, node);
                    self.path_hash ^= fold(i as u64, node as u64);
                    if node == 0 {
                        self.stage = Stage::Validate(i);
                    } else {
                        self.stage = Stage::Trace(self.labels[&node].parent.unwrap().from);
                    }
                }
                Stage::Validate(i) => {
                    if i == 0 {
                        self.stage = Stage::Ready;
                    } else {
                        let to = self.path[&(i - 1)];
                        let edge = self.labels[&to].parent.unwrap();
                        if edge.source.is_none() {
                            self.actual_cost += self.links[&(edge.from, edge.to)];
                            self.stage = Stage::Validate(i - 1);
                        } else {
                            self.stage = Stage::BeginRoad(i, edge);
                        }
                    }
                }
                Stage::BeginRoad(i, edge) => {
                    self.physical_job = Some(self.physical.begin(
                        self.point(edge.from),
                        self.point(edge.to),
                        true,
                    ));
                    self.physical_checks += 1;
                    self.stage = Stage::Road(i, edge);
                }
                Stage::Road(i, edge) => {
                    if let Some(result) = self.poll_physical(budget) {
                        self.physical_job = None;
                        if let Some(c) = result.filter(|c| c.is_finite() && *c >= 0.0) {
                            self.actual_cost += c;
                            self.stage = Stage::Validate(i - 1);
                        } else {
                            assert!(
                                self.excluded.insert(edge),
                                "failed unchanged arc must not repeat"
                            );
                            self.excluded_hash ^= edge_hash(edge);
                            self.stage = Stage::DrainQueue;
                        }
                    }
                }
                Stage::DrainQueue => {
                    if self.pop().is_none() {
                        self.stage = Stage::DrainPath;
                    }
                }
                Stage::DrainPath => {
                    if let Some((i, node)) = self.path.pop_first() {
                        self.path_hash ^= fold(i as u64, node as u64);
                    } else {
                        self.stage = Stage::Seed;
                    }
                }
                Stage::Ready | Stage::Declined => unreachable!(),
            }
        }
        self.work += before - *budget;
        self.status()
    }
    fn scalar_digest(&self, reference: bool) -> u64 {
        let mut h = fold(self.topology.identity, self.physical.identity());
        for v in [
            point_hash(self.start),
            point_hash(self.goal),
            self.radius.to_bits(),
            self.road_cost.to_bits(),
            self.end as u64,
            self.link_counts[0] as u64,
            self.link_counts[1] as u64,
            self.next_node as u64,
            self.serial,
            self.generation as u64,
            self.current as u64,
            self.work as u64,
            self.searches as u64,
            self.physical_checks as u64,
            self.actual_cost.to_bits(),
            self.coarse_cost.to_bits(),
            self.connector_cost.to_bits(),
            self.query.digest(),
        ] {
            h = fold(h, v);
        }
        let (s, a, b) = match self.stage {
            Stage::Query => (0, 0, 0),
            Stage::Project(i) => (1, i as u64, 0),
            Stage::BeginConnector => (2, 0, 0),
            Stage::Connector => (3, 0, 0),
            Stage::Insert => (4, 0, 0),
            Stage::Seed => (5, 0, 0),
            Stage::Pop => (6, 0, 0),
            Stage::Expand => (7, 0, 0),
            Stage::Relax(e, c) => (8, edge_hash(e), c.to_bits()),
            Stage::Trace(n) => (9, n as u64, 0),
            Stage::Validate(i) => (10, i as u64, 0),
            Stage::BeginRoad(i, e) => (11, i as u64, edge_hash(e)),
            Stage::Road(i, e) => (12, i as u64, edge_hash(e)),
            Stage::DrainQueue => (13, 0, 0),
            Stage::DrainPath => (14, 0, 0),
            Stage::Ready => (15, 0, 0),
            Stage::Declined => (16, 0, 0),
        };
        h = fold(h, fold(s, fold(a, b)));
        let (a, b, c) = match self.expansion {
            Expansion::Start(last) => (0, last.map_or(u64::MAX, |v| v as u64), 0),
            Expansion::Static { at, goal } => (1, at as u64, goal as u64),
            Expansion::Portal { dir, goal } => (2, dir as u64, goal as u64),
        };
        h = fold(h, fold(a, fold(b, c)));
        if let Some(c) = self.candidate {
            h = fold(h, fold(c.edge as u64, fold(c.t.to_bits(), point_hash(c.p))));
        }
        if let Some(j) = &self.physical_job {
            h = fold(
                h,
                if reference {
                    self.physical.reference_digest(j)
                } else {
                    self.physical.digest(j)
                },
            );
        }
        h
    }
    pub fn digest(&self) -> u64 {
        let mut h = self.scalar_digest(false);
        for v in [
            self.labels_hash,
            self.queue_hash,
            self.splices_hash,
            self.portals_hash,
            self.links_hash,
            self.excluded_hash,
            self.path_hash,
        ] {
            h = fold(h, v);
        }
        h
    }
    pub fn reference_digest(&self) -> u64 {
        let mut h = self.scalar_digest(true);
        let parts = [
            self.labels
                .iter()
                .fold(0, |h, (&id, &l)| h ^ label_hash(id, l)),
            self.queue.iter().fold(0, |h, &o| h ^ open_hash(o)),
            self.splices
                .iter()
                .fold(0, |h, (&s, &p)| h ^ splice_hash(s, p)),
            self.portals.iter().fold(0, |h, (&id, &s)| {
                h ^ fold(id as u64, splice_hash(s, Point(0.0, 0.0)))
            }),
            self.links.iter().fold(0, |h, (&(a, b), &c)| {
                h ^ fold(fold(a as u64, b as u64), c.to_bits())
            }),
            self.excluded.iter().fold(0, |h, &e| h ^ edge_hash(e)),
            self.path
                .iter()
                .fold(0, |h, (&i, &n)| h ^ fold(i as u64, n as u64)),
        ];
        for v in parts {
            h = fold(h, v);
        }
        h
    }
    pub fn retained(&self) -> (usize, usize, usize, usize) {
        (
            self.labels.len(),
            self.queue.len(),
            self.portals.len(),
            self.excluded.len(),
        )
    }
}
