//! Units as the authority holds them: bodies, squads, orders and movement state.
use std::collections::{BTreeSet, VecDeque};

use contract::catalog::{AltitudeLayer, FlightModel, Mobility as Moves, TypeIndex, UnitType};
use contract::command::{Engagement, MoveDirection, RoutePolicy};
use contract::ids::{Side, UnitId};
use contract::map::MoverClass;
use contract::observation::MoveState;
use contract::scenario::{Armor, Rules};

use crate::hold::{Hold, Phase};
use crate::math::{Obb2, V2, V3};
use crate::navigation::Mobility;
use crate::weapons::{Mount, Target};
use crate::world::PropId;

mod digest;
mod hull;
mod motion;
pub use hull::{face_toward, hull_face_at, HullFrame};
pub use motion::{ground_mobility, motion, Flight, Motion};

/// An aircraft smokes below this share of its health (D34).
const SMOKING_HP_SHARE: f64 = 0.5;

/// One soldier: a body of his own (L4–L6). He stands on the ground at his
/// own position, or at his building slot while garrisoned; nothing places
/// him relative to his squad.
#[derive(Clone, Debug)]
pub struct Soldier {
    pub id: u32,
    /// His place in his squad type's slots: which soldier kind he is and
    /// what he carries. Fixed when he joins.
    pub slot: usize,
    /// The carried weapon he is working on; independent of special-gun ownership.
    pub active_mount: Option<usize>,
    pub position: V3,
    /// Ground velocity over the last tick.
    pub velocity: V2,
    pub hp: f64,
    /// Where the soldier fell: a permanent record that blocks nothing (M06).
    pub corpse: Option<Fallen>,
    /// His place in the squad's arrangement where the current move ends (D1).
    pub spot: Option<V2>,
    /// The waypoint of the squad's corridor he is walking toward.
    pub leg: usize,
    /// His lane: metres left of the corridor he walks (Q6).
    pub lateral: f64,
    /// Where his pace swing starts for this move, a share of its period.
    pub pace: f64,
    /// The tick he sets off on this move (the stagger).
    pub start: u64,
    /// His own route on the exact bodies: the final stretch to his spot, or
    /// back to the corridor (`movement::final_leg`).
    pub path: Vec<V2>,
    /// The side's knowledge revision his own route was last judged against,
    /// and the tick he last planned it.
    pub path_revision: u64,
    pub planned_at: u64,
    /// Where he takes up position while his squad holds (at rest or
    /// halted): cover the squad re-resolved, or a step out to fire (D3, D5).
    pub post: Option<V2>,
    /// The cover tier his spot or post gives, as resolved (D2+ publishes it).
    pub cover: Option<crate::cover::Tier>,
    /// The lean point claimed with his spot or post, round the tall cover
    /// he fires past.
    pub lean: Option<crate::lean::Lean>,
    /// His last search for a place to fight from that found none.
    pub sat_out: Option<crate::cover::SatOut>,
    /// He leans out, firing, from `lean_since` until `leaning_until`;
    /// tucked in behind his cover otherwise. After a burst out he
    /// stays tucked in until `tucked_until`.
    pub lean_since: u64,
    pub leaning_until: u64,
    pub tucked_until: u64,
}

impl Soldier {
    pub fn new(id: u32, slot: usize, position: V3, hp: f64) -> Self {
        Soldier {
            id,
            slot,
            active_mount: None,
            position,
            velocity: V2::default(),
            hp,
            corpse: None,
            spot: None,
            leg: 0,
            lateral: 0.0,
            pace: 0.0,
            start: 0,
            path: Vec::new(),
            path_revision: 0,
            planned_at: 0,
            post: None,
            cover: None,
            lean: None,
            sat_out: None,
            lean_since: 0,
            leaning_until: 0,
            tucked_until: 0,
        }
    }

    /// The lean he is out on at `tick`: firing from his claimed lean point,
    /// and still at the place it belongs to.
    pub fn leaning(&self, tick: u64) -> Option<&crate::lean::Lean> {
        self.lean.as_ref().filter(|l| {
            tick < self.leaning_until
                && (l.from - self.position.xy()).length() <= crate::lean::AT_PLACE_M
        })
    }

    /// Where his body is at `tick`: out at his lean point while leaning,
    /// else where he stands. What rounds fly into and enemies aim at.
    pub fn exposed(&self, tick: u64) -> V3 {
        match self.leaning(tick) {
            Some(l) => l.at.with_z(self.position.z),
            None => self.position,
        }
    }
}

/// A fallen soldier's record: where, and the squad's heading at the time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fallen {
    pub at: V3,
    pub yaw: f64,
    /// Fighting-floor owner retained even after the whole squad dies.
    pub(crate) support_building: Option<PropId>,
}

impl Soldier {
    pub fn alive(&self) -> bool {
        self.hp > 0.0
    }

    /// The one way a soldier dies: no health left (a lethal hit's overkill is
    /// kept) and a persistent body record retaining its fall's facing.
    /// Elevated fighting-floor support can later disappear.
    pub fn fall(&mut self, at: V3, yaw: f64, support_building: Option<PropId>) {
        if self.hp > 0.0 {
            self.hp = 0.0;
        }
        self.corpse = Some(Fallen {
            at,
            yaw,
            support_building,
        });
        self.active_mount = None;
    }
}

#[derive(Clone, Debug)]
pub struct MoveOrder {
    pub destination: V2,
    pub policy: RoutePolicy,
    pub gesture: u64,
    /// A reverse move backs along the route, facing held (Q31).
    pub direction: MoveDirection,
    /// A right-drag's facing once there (Q9); `None`: the way it travels.
    pub facing: Option<f64>,
    /// Sent where it had no room to stand, its destination is the nearest
    /// room it can reach: where it stops instead if the way there proves
    /// the longer detour.
    pub short: Option<StopShort>,
}

/// Room on a unit's way to ground it could not stand on, short of the
/// nearest room it was sent to instead.
#[derive(Clone, Debug, PartialEq)]
pub struct StopShort {
    pub at: V2,
    /// How much farther than the way to `at` the way to the nearest room
    /// may run: what the ground it gains is worth
    /// (`navigation.stop_short_detour_ratio`).
    pub detour_m: f64,
    /// While the way to `at` is planned: the nearest room, and the route
    /// found to it.
    pub nearest: Option<(V2, Vec<V2>)>,
}

/// Where an attack order is heading this tick: the target's live position
/// while it is out of reach, or, once the side has lost it, its last report
/// (`searching`), approached like an attack-move.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pursuit {
    pub to: V2,
    pub searching: bool,
}

#[derive(Clone, Debug)]
pub enum UnitOrder {
    Move(MoveOrder),
    /// Move, halting while any weapon engages (W16).
    AttackMove(MoveOrder),
    /// Focus fire; pursue an identified target to regain a firing position,
    /// or its last reported place once identification lapses (W17).
    Attack {
        target: Target,
        last_known: Option<V2>,
    },
    /// Walk to `approach` beside the building, then enter it (L08).
    Garrison {
        building: PropId,
        approach: V2,
    },
    /// Leave the current building.
    Exit,
    /// Board a moving carrier; carriage advances the target each tick.
    Board {
        carrier: UnitId,
        gesture: u64,
    },
    /// Drive to squads and collect them.
    Collect {
        squads: Vec<UnitId>,
        gesture: u64,
    },
    /// Unload passengers from the carrier at its current position.
    Unload,
}

impl UnitOrder {
    pub fn movement(&self) -> Option<&MoveOrder> {
        match self {
            UnitOrder::Move(o) | UnitOrder::AttackMove(o) => Some(o),
            _ => None,
        }
    }

    pub fn movement_mut(&mut self) -> Option<&mut MoveOrder> {
        match self {
            UnitOrder::Move(o) | UnitOrder::AttackMove(o) => Some(o),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Unit {
    pub id: UnitId,
    pub side: Side,
    /// The id of the player who commands this unit, when a player owns it.
    pub owner: Option<String>,
    /// Its unit type, the catalog's index.
    pub kind: TypeIndex,
    /// Returned through the physical base entry; distinct from combat death.
    pub retired: bool,
    pub withdrawing: bool,
    pub position: V3,
    pub yaw: f64,
    pub motion: Motion,
    /// Vehicle hull half extents (length, width, height); infantry use members.
    pub hull: Option<V3>,
    pub members: Vec<Soldier>,
    pub orders: VecDeque<UnitOrder>,
    /// Where an attack order is currently pursuing to, set each tick.
    pub pursuit: Option<Pursuit>,
    /// The goal the current route was planned to.
    pub planned_goal: Option<V2>,
    /// Remaining waypoints of the current order, once planned.
    pub route: Option<Vec<V2>>,
    /// Where a squad's corridor or an aircraft's leg runs from to its first
    /// remaining waypoint: the planning start, then the last waypoint passed
    /// (by every soldier, for a squad).
    pub route_from: V2,
    pub state: MoveState,
    pub blocker: Option<UnitId>,
    /// Side knowledge revision the current route (or block) was judged against.
    pub planned_revision: u64,
    /// Failure-to-advance watch: best distance to the next waypoint and when.
    pub progress: (f64, u64),
    /// Give-up watch: the shortest way left seen at a stall, and the tick it
    /// was seen; a way no shorter for long enough gives the move up
    /// ([`MoveState::RouteBlocked`]).
    pub stalls: (f64, u64),
    pub engagement: Engagement,
    /// While a passive AI holds the unit at return fire, the engagement it
    /// goes back to when that controller leaves: its authored or ordered one.
    pub held_engagement: Option<Engagement>,
    pub mounts: Vec<Mount>,
    /// Enemies that attacked this unit, whom Return fire only may answer while
    /// the side still knows them (W13).
    pub attackers: BTreeSet<UnitId>,
    /// Last tick's weapon conclusions: attack-move halting and attack pursuit.
    pub reach: crate::weapons::Reach,
    /// Vehicle health; infantry health lives on each soldier.
    pub hp: f64,
    /// Infantry suppression's hidden level in [0, 1] and the tick it last
    /// grew (P14); every effect reads its tier (`SuppressionRules::tier`).
    pub suppression: f64,
    pub suppressed_at: u64,
    /// Setup progress for units that deploy in place (L01); `None` otherwise.
    pub deployment: Option<crate::deployment::Deployment>,
    /// The host this unit is entering, inside or leaving (L08).
    pub hold: Option<Hold>,
    /// A supply vehicle's remaining stock (L05); `None` for every other unit.
    pub stock: Option<u32>,
    pub protection: Option<crate::protection::State>,
    /// Its flares, for a unit whose type carries them.
    pub flares: Option<Flares>,
    /// Service received: progress toward the next item, and why or why not.
    pub progress_service: crate::supply::Progress,
    pub service: contract::observation::ServiceStatus,
    /// The bearing this unit looks along this tick, taken before fire
    /// (`sight::snapshot`): what spotting, the sweep and the publication read.
    pub sight_forward: f64,
    /// A squad's cover: what it was last resolved against, and when (Q11).
    pub cover: crate::cover::Watch,
    /// A squad's anchor: the centre of the area its soldiers fight
    /// in. Only an order moves it (a move's destination, an attack-move's
    /// halt, a script or a placement), never where the soldiers stand.
    pub anchor: Option<crate::cover::Anchor>,
    /// A wheeled vehicle's three-point turn in progress: the leg it drives
    /// against its order's direction (Q29).
    pub manoeuvre: Option<crate::movement::Manoeuvre>,
    /// The vehicle drove backwards this tick: an ordered reverse move or a
    /// three-point turn's reversing leg (the reverse beeper's cue).
    pub reversing: bool,
    /// Actual accepted hull speed; negative while reversing.
    pub drive_speed_mps: f64,
    /// A tracked vehicle's or an aircraft's ordered facing (Q9), still to
    /// turn to at rest.
    pub turn_to: Option<f64>,
    /// An aircraft's flight; `None` on the ground.
    pub air: Option<AirState>,
}

/// What an aircraft carries from tick to tick.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AirState {
    /// World velocity, metres a second.
    pub velocity: V3,
    /// Afterburner ramp state, from zero to one.
    pub burner: f64,
    /// Last non-zero turn direction, retained by the fixed-wing flight law.
    pub turn_sign: i8,
    /// Where an idle jet circles: the move goal it last reached, or where it
    /// was when its last order or attack run ended (choice 03).
    pub last_goal: Option<V2>,
    /// Its arrival at the goal it flies to last, once it has turned onto
    /// the leg that ends there.
    pub settle: Option<Settle>,
    /// Remaining endurance in seconds.
    pub fuel_s: f64,
    /// Explicit persistent afterburner toggle, consumed by the air mover.
    pub afterburner: bool,
    /// Close air engagement target while this aircraft is in a dogfight.
    pub dogfight_target: Option<UnitId>,
    /// Its attack on a ground target, in runs (`air::pass`).
    pub pass: Option<crate::air::pass::PassPlan>,
    /// The one tick in which a pass's unpowered round may leave the jet.
    pub pass_release: bool,
}

/// A unit's flares: the charges left and the tick the next is ready.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Flares {
    pub charges: u32,
    pub ready_tick: u64,
}

/// How an aircraft arrives: it aims a little past its goal, then, stopped
/// there, eases back onto it (`back`), facing as it came.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settle {
    pub goal: V2,
    pub aim: V2,
    pub back: bool,
}

impl Unit {
    /// A move order is in progress, rather than individual bodies shuffling.
    pub fn moving(&self) -> bool {
        self.state == MoveState::Moving
    }

    /// Admit an order with the same replacement and deployment semantics everywhere.
    pub(crate) fn enqueue(&mut self, order: UnitOrder, queued: bool) {
        if let Some(d) = self.deployment.as_mut() {
            d.stationary = contract::observation::Posture::Deployed;
        }
        if !queued {
            self.orders.clear();
            self.route = None;
            self.planned_goal = None;
            self.state = MoveState::Idle;
            self.turn_to = None;
        }
        self.orders.push_back(order);
    }
    pub fn is_vehicle(&self) -> bool {
        self.hull.is_some()
    }

    /// Flies: no ground rule (traffic, shoving, cover, treads, forest lanes,
    /// concealment) reaches it.
    pub fn airborne(&self) -> bool {
        matches!(self.motion, Motion::Air(_))
    }

    /// Where rounds aim on it, above its position: the one aim rule, keyed
    /// on the target. Anything in the air is aimed at its hull's centre,
    /// whatever fires; a ground vehicle at `vehicle_aim_height_fraction` of
    /// its hull's height; a soldier at `infantry_aim_m`.
    pub fn aim_height(&self, rules: &Rules) -> f64 {
        match self.hull {
            Some(half) if self.layer() != AltitudeLayer::Ground => half.z,
            Some(half) => 2.0 * half.z * rules.physics.vehicle_aim_height_fraction,
            None => rules.physics.infantry_aim_m,
        }
    }

    /// The velocity a round carries away from this unit at launch. Ground
    /// units contribute zero; aircraft carry their full world velocity.
    pub fn flight_velocity(&self) -> V3 {
        match (self.airborne(), self.air) {
            (true, Some(air)) => air.velocity,
            _ => V3::default(),
        }
    }

    /// It trails smoke (D19, D34): an aircraft below half its health. What a
    /// viewer sees of its damage, own or enemy; never its health itself.
    pub fn smoking(&self, rules: &Rules) -> bool {
        self.airborne()
            && self.alive()
            && (self.unit_type(rules).hull()).is_some_and(|h| self.hp < SMOKING_HP_SHARE * h.hp)
    }

    /// The height band it occupies, which decides what can engage it.
    pub fn layer(&self) -> AltitudeLayer {
        if self.airborne() {
            match self.motion {
                Motion::Air(flight) => flight.layer,
                Motion::Ground(_) => unreachable!("airborne unit has ground motion"),
            }
        } else {
            AltitudeLayer::Ground
        }
    }

    /// It flies on wings: never stopping, attacking in passes, arriving and
    /// leaving by its back edge. False for a rotor aircraft and on the ground.
    pub fn fixed_wing(&self) -> bool {
        matches!(
            self.motion,
            Motion::Air(Flight {
                model: FlightModel::FixedWing,
                ..
            })
        )
    }

    /// How it moves over the ground. Only ground movement, routing and
    /// placement ask, and an aircraft never reaches them.
    pub fn ground(&self) -> &Mobility {
        match &self.motion {
            Motion::Ground(m) => m,
            Motion::Air(_) => panic!("an aircraft never reaches ground movement"),
        }
    }

    /// A squad stands where its living soldiers stand: their centroid. Called
    /// whenever soldiers move, arrive, fall or join; a vehicle is unchanged.
    pub fn settle(&mut self) {
        if self.is_vehicle() {
            return;
        }
        let (mut sum, mut n) = (V3::default(), 0.0);
        for s in self.members.iter().filter(|s| s.alive()) {
            sum = sum + s.position;
            n += 1.0;
        }
        if n > 0.0 {
            self.position = sum * (1.0 / n);
        }
    }

    /// At its building's perimeter slots (inside or leaving, not entering).
    pub fn garrisoned(&self) -> bool {
        self.hold.as_ref().is_some_and(|hold| {
            matches!(hold.host, crate::hold::Host::Building(_))
                && matches!(hold.phase, Phase::Inside | Phase::Exiting(_))
        })
    }

    /// World positions of living members.
    pub fn member_positions(&self) -> impl Iterator<Item = V3> + '_ {
        self.members
            .iter()
            .filter(|s| s.alive())
            .map(|s| s.position)
    }

    /// A squad lives while any member does; a vehicle while its hull has health.
    pub fn alive(&self) -> bool {
        if self.retired {
            return false;
        }
        match self.hull {
            Some(_) => self.hp > 0.0,
            None => self.members.iter().any(|s| s.alive()),
        }
    }

    /// Alive and physically present in the world. A squad inside a carrier is
    /// still counted for accounting, but cannot collide, sense, fight, hear or
    /// affect objectives until it exits.
    pub fn on_field(&self) -> bool {
        self.alive() && self.hold.as_ref().and_then(|hold| hold.riding()).is_none()
    }

    /// Its type's record.
    pub fn unit_type<'a>(&self, rules: &'a Rules) -> &'a UnitType {
        rules.catalog.get(self.kind)
    }

    /// The size of the area a contact this unit causes covers: the fixture's
    /// factor over its type's footprint radius, from the catalog alone (a
    /// hull's half-diagonal, or half a full squad's spread plus a soldier's
    /// body), so it says "vehicle-sized" or "squad-sized", never how many
    /// soldiers are left or how they stand.
    pub fn contact_radius(&self, rules: &Rules) -> f64 {
        let t = self.unit_type(rules);
        let footprint = match t.hull() {
            Some(h) => libm::hypot(h.half_extents_m[0], h.half_extents_m[1]),
            None => {
                crate::arrangement::spread(&rules.infantry_movement, t.squad_size()) / 2.0
                    + rules.physics.soldier_radius_m
            }
        };
        rules.sensors.contact_radius_factor * footprint
    }

    /// A vehicle's full health (0 for a squad, whose health is per soldier).
    pub fn max_hp(&self, rules: &Rules) -> f64 {
        self.unit_type(rules).hull().map_or(0.0, |h| h.hp)
    }

    /// The hull's armour, for vehicles.
    pub fn armor<'a>(&self, rules: &'a Rules) -> Option<&'a Armor> {
        self.unit_type(rules).hull().map(|h| &h.armor)
    }

    /// The hull's footprint on the ground: what traffic waits for, soldiers
    /// step out of and lean on. `None` for a squad and for an aircraft, which
    /// stands on nothing.
    pub fn ground_footprint(&self) -> Option<Obb2> {
        self.hull.filter(|_| !self.airborne()).map(|h| Obb2 {
            center: self.position.xy(),
            yaw: self.yaw,
            half: h.xy(),
        })
    }

    /// Radius of the unit's ground footprint, for traffic spacing and sensing
    /// reach (a garrison spans its building's perimeter): a squad's reaches
    /// its outermost soldier's body, of `soldier_radius_m`.
    pub fn footprint_radius(&self, soldier_radius_m: f64) -> f64 {
        match self.hull {
            Some(h) => libm::hypot(h.x, h.y),
            None => {
                self.member_positions()
                    .map(|p| (p.xy() - self.position.xy()).length())
                    .fold(0.0, f64::max)
                    + soldier_radius_m
            }
        }
    }

    /// Where movement should head now and by which policy: an ordered
    /// destination, an attack's pursuit point or a building's approach. A unit
    /// with a building (entering, inside or leaving) goes nowhere.
    /// The order at `index` lets the passengers out where it ends: an
    /// Unload follows it.
    pub fn unloads_after(&self, index: usize) -> bool {
        matches!(self.orders.get(index + 1), Some(UnitOrder::Unload))
    }

    /// The destinations of the movement orders queued behind the current
    /// one, each with whether it lets the passengers out there.
    pub fn queued_steps(&self) -> impl Iterator<Item = (V2, bool)> + '_ {
        self.orders
            .iter()
            .enumerate()
            .skip(1)
            .filter_map(|(k, o)| o.movement().map(|m| (m.destination, self.unloads_after(k))))
    }

    pub fn movement_goal(&self) -> Option<(V2, RoutePolicy)> {
        if self.hold.is_some() {
            return None;
        }
        match self.orders.front()? {
            UnitOrder::Move(o) | UnitOrder::AttackMove(o) => Some((o.destination, o.policy)),
            UnitOrder::Attack { .. } => self.pursuit.map(|p| (p.to, RoutePolicy::Shortest)),
            UnitOrder::Garrison { approach, .. } => Some((*approach, RoutePolicy::Shortest)),
            UnitOrder::Exit
            | UnitOrder::Board { .. }
            | UnitOrder::Collect { .. }
            | UnitOrder::Unload => None,
        }
    }

    /// Which way the current order drives: a reverse move backs (Q31).
    pub fn direction(&self) -> MoveDirection {
        match self.orders.front() {
            Some(UnitOrder::Move(o)) if self.hold.is_none() => o.direction,
            _ => MoveDirection::Forward,
        }
    }

    /// The movement gate: a deploying unit translates only when fully packed.
    pub fn may_translate(&self) -> bool {
        self.deployment
            .as_ref()
            .is_none_or(crate::deployment::Deployment::packed)
    }

    /// Attack-move pauses its advance while it can engage something, and so
    /// does an attack searching for a target it has lost.
    pub fn halted(&self) -> bool {
        self.reach.can_engage
            && match self.orders.front() {
                Some(UnitOrder::AttackMove(_)) => true,
                Some(UnitOrder::Attack { .. }) => self.pursuit.is_some_and(|p| p.searching),
                _ => false,
            }
    }

    pub fn attack_target(&self) -> Option<Target> {
        match self.orders.front()? {
            UnitOrder::Attack { target, .. } => Some(*target),
            _ => None,
        }
    }
}
