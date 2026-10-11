//! Brew-ups a side watched (`apps/battle-lab/src/cookOffs.ts`): a hull it
//! saw that is gone, and a wreck of its type near where it stood. The
//! simulation lays a dead vehicle's wreck where it rolls to a stop, the tick
//! it dies, and a side that knew the hull learns the wreck that tick; so a
//! side that saw the hull alive within a second of learning its wreck saw it
//! die. A wreck found later, scouted onto, was no one's to watch. An
//! airframe shot down falls first and never brews up.
//!
//! The effects blow the hull up ([`effect_cook_off`]); the models show it:
//! the hull as last seen, rolling on to where its wreck lies, until the
//! ammunition goes; then the wreck jolting whole, or its hull heaving and
//! its turret thrown (`cook_off.rs`); and what the blast threw clear lying
//! round it, then sinking away. Each frame's models are plain data
//! ([`PlacedModel`]) a client turns into its own instances.
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

use serde::Deserialize;
use sim::publication::reader::Observation;

use super::cook_off::CookOffFeel;
use super::publication;
use crate::seed::js_hypot;
use crate::units::pose::{Articulation, VehiclePose};
use crate::world::{EffectsSection, UnitsSection, Vec3, Wreck};

/// How far beyond its own half length a wreck may lie from where its hull
/// was last seen: a vehicle killed at road speed rolls a few metres on.
const ROLL_REACH_M: f64 = 6.0;
/// Seconds a vehicle gone from the frame is still remembered as last drawn:
/// longer than a cook-off waits to blow up.
const REMEMBERED_S: f64 = 2.0;
/// Seconds a moving wreck is held still before the static wreck takes over.
const LANDED_HOLD_S: f64 = 0.5;
/// How much further than its own top thrown debris sinks before it is gone:
/// ground falling away under a piece still hides it.
const DEBRIS_BURIED_M: f64 = 0.15;

/// How a unit type's hull dies, by unit type (an input the manifest is to
/// carry): its own wreck's appearance as the props fit it (`props.fit.wrecks`
/// and its `looks`), that wreck's footprint (half extents) and its whole
/// `default` state's resting bounds (min, max), and how hard the dead hull
/// brakes to a stop, m/s² (top speed over `movement.drive.wreck_stop_s`).
#[derive(Clone, Debug, Deserialize)]
pub struct WreckLook {
    pub appearance: String,
    pub footprint: Vec3,
    pub bounds: [Vec3; 2],
    pub braking: f64,
}

/// A model to draw this frame: an appearance placed at `position` turned by
/// `yaw` and scaled per axis, either in a static `state` moved first by
/// `motion` in its own frame (column-major), or, with no state, posed by
/// `articulation` and tinted by its side (a hull as last seen).
#[derive(Clone, Debug)]
pub struct PlacedModel {
    pub appearance: Arc<str>,
    pub state: Option<&'static str>,
    pub position: Vec3,
    pub yaw: f64,
    pub scale: Vec3,
    pub motion: Option<[f64; 16]>,
    pub articulation: Option<Articulation>,
    pub tint: Option<Vec3>,
}

/// A hull the side watched brew up: its wreck as the side knows it, and the
/// publication tick that first showed it.
#[derive(Clone, Debug)]
pub struct CookOff {
    /// The wreck's known prop id.
    pub prop: f64,
    /// The unit type it is the wreck of.
    pub wreck_of: Arc<str>,
    pub center: [f64; 2],
    pub yaw: f64,
    pub half: Vec3,
    pub base_z: f64,
    pub tick: f64,
}

struct Hull {
    kind: Arc<str>,
    tick: f64,
    x: f64,
    y: f64,
    /// How far its wreck may lie from where it was seen: its half length.
    reach: f64,
}

/// Which wrecks a side's publications show it watched appear (`CookOffWatch`).
pub struct CookOffWatch {
    /// Every hull seen within `window` ticks, where last seen, by (own, id).
    hulls: HashMap<(bool, u64), Hull>,
    known: HashSet<Option<u64>>,
    last_tick: f64,
    window: f64,
}

impl CookOffWatch {
    pub fn new(window: f64) -> Self {
        Self {
            hulls: HashMap::new(),
            known: HashSet::new(),
            last_tick: -1.0,
            window,
        }
    }

    /// The hulls `o` shows the side watched brew up since the last
    /// publication. An earlier tick is a new battle and starts over.
    pub fn note(
        &mut self,
        o: &Observation,
        units: &UnitsSection,
        effects: &EffectsSection,
    ) -> Vec<CookOff> {
        if o.tick < self.last_tick {
            self.hulls.clear();
            self.known.clear();
            self.last_tick = -1.0;
        }
        let first = self.last_tick < 0.0;
        let hull = |own: bool, id: f64, kind: &Arc<str>, at: Vec3| {
            // An airframe's death is its fall: it never brews up.
            let airborne = units.types.get(&**kind).is_some_and(|t| t.airborne);
            let half = effects
                .types
                .get(&**kind)
                .and_then(|t| t.half)
                .filter(|_| !airborne)?;
            let h = Hull {
                kind: kind.clone(),
                tick: o.tick,
                x: at[0],
                y: at[1],
                reach: half[0],
            };
            Some(((own, id.to_bits()), h))
        };
        let seen: Vec<_> = (o
            .own
            .iter()
            .filter_map(|u| hull(true, u.id, &u.kind, u.position)))
        .chain(
            o.identified
                .iter()
                .filter_map(|u| hull(false, u.id, &u.kind, u.position)),
        )
        .collect();
        // Lost from sight within the window: those it could have watched die.
        let window = self.window;
        self.hulls
            .retain(|key, h| o.tick - h.tick <= window && !seen.iter().any(|(k, _)| k == key));
        let mut out = Vec::new();
        for p in o.known_props.iter() {
            if !self.known.insert(p.id.map(f64::to_bits)) {
                continue;
            }
            let Some(wreck_of) = p.wreck_of.as_ref().filter(|_| !first && !p.destroyed) else {
                continue;
            };
            let watched = self.hulls.values().any(|h| {
                h.kind == *wreck_of
                    && js_hypot(&[p.center[0] - h.x, p.center[1] - h.y]) <= h.reach + ROLL_REACH_M
            });
            if watched {
                out.push(CookOff {
                    prop: p.id.unwrap_or(f64::NAN),
                    wreck_of: wreck_of.clone(),
                    center: p.center,
                    yaw: p.yaw,
                    half: p.half,
                    base_z: p.base_z,
                    tick: o.tick,
                });
            }
        }
        self.hulls.extend(seen);
        self.last_tick = o.tick;
        out
    }
}

/// A watched wreck while it moves, whole or cut into hull and thrown turret.
#[derive(Clone, Debug)]
pub struct Transition {
    pub cook_off: CookOff,
    /// The wreck as the props place it, in its `default` state.
    wreck: PlacedModel,
    /// Its whole state's resting bounds (min, max), which keep a whole
    /// wreck above its ground as it jolts.
    bounds: [Vec3; 2],
    /// Its turret piece's centre in its own frame, or None: it jolts whole.
    lies: Option<Vec3>,
    /// How high the turret piece's underside lies on the deck.
    underside: f64,
    /// How tall its thrown debris lies, or None: it throws none.
    debris_top: Option<f64>,
    /// The presentation second of the killing hit (its tick's start).
    pub hit_at: f64,
}

/// `c`'s wreck fitted for its death (`transitionOf`): its own unit type's
/// wreck, and what that throws (`wrecks`, by appearance); None where its
/// unit type has no wreck drawn.
pub fn transition_of(
    c: &CookOff,
    looks: &BTreeMap<String, WreckLook>,
    wrecks: &BTreeMap<String, Wreck>,
    tick_hz: f64,
) -> Option<Transition> {
    let look = looks.get(&*c.wreck_of)?;
    let pieces = wrecks.get(&look.appearance);
    Some(Transition {
        cook_off: c.clone(),
        wreck: PlacedModel {
            appearance: look.appearance.as_str().into(),
            state: Some("default"),
            position: [c.center[0], c.center[1], c.base_z],
            yaw: c.yaw,
            scale: std::array::from_fn(|i| c.half[i] / look.footprint[i]),
            motion: None,
            articulation: None,
            tint: None,
        },
        bounds: look.bounds,
        lies: pieces.and_then(|w| w.lies),
        underside: pieces.map_or(0.0, |w| w.underside),
        debris_top: pieces.and_then(|w| w.debris_top),
        hit_at: (c.tick - 1.0) / tick_hz,
    })
}

/// `c` as the effects take it: its hull, and where its turret lands if its
/// wreck throws one.
pub fn effect_cook_off(c: &CookOff, transition: Option<&Transition>) -> publication::CookOff {
    let landing = transition.and_then(|f| {
        let lies = f.lies?;
        // The piece's underside under its centre, placed as the wreck is.
        let (m, [sx, sy, sz]) = (&f.wreck, f.wreck.scale);
        let (x, y) = (lies[0] * sx, lies[1] * sy);
        let (s, cos) = m.yaw.sin_cos();
        let [mx, my, mz] = m.position;
        Some([
            mx + x * cos - y * s,
            my + x * s + y * cos,
            mz + f.underside * sz,
        ])
    });
    publication::CookOff {
        center: [c.center[0], c.center[1], c.base_z],
        height: 2.0 * c.half[2],
        landing,
    }
}

/// The wreck at `clock`: jolting whole, or as its hull and thrown turret.
fn wreck_models(f: &Transition, feel: &CookOffFeel, clock: f64, out: &mut Vec<PlacedModel>) {
    let age = clock - f.hit_at;
    let seed = f.cook_off.prop;
    let piece = |state: &'static str, motion: [f64; 16]| PlacedModel {
        state: Some(state),
        motion: Some(motion),
        ..f.wreck.clone()
    };
    let Some(lies) = f.lies else {
        let mut m = feel.hull_motion(age, seed);
        let [min, max] = f.bounds;
        // A whole wreck has wheels and loose panels under its hull: raise its
        // lowest rotated bound to the resting plane.
        let low = (m[2] * min[0]).min(m[2] * max[0])
            + (m[6] * min[1]).min(m[6] * max[1])
            + (m[10] * min[2]).min(m[10] * max[2])
            + m[14];
        m[14] += (min[2] - low).max(0.0);
        out.push(piece("default", m));
        return;
    };
    out.push(piece("hull", feel.hull_motion(age, seed)));
    out.push(piece("turret", feel.turret_motion(age, lies, seed)));
}

/// The debris `f` threw at `clock`: where its wreck lies (not jolting with
/// it) from the blast, then sinking away; None while it is not drawn.
fn debris_model(f: &Transition, feel: &CookOffFeel, clock: f64) -> Option<PlacedModel> {
    let sink = feel.debris_sink(clock - f.hit_at, f.debris_top? + DEBRIS_BURIED_M)?;
    let mut motion = IDENTITY;
    motion[14] = -sink;
    Some(PlacedModel {
        state: Some("debris"),
        motion: Some(motion),
        ..f.wreck.clone()
    })
}

const IDENTITY: [f64; 16] = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
];

/// A hull as the side last saw it, and how hard it brakes, m/s².
#[derive(Clone, Debug)]
pub struct LastHull {
    pub model: PlacedModel,
    pub braking: f64,
}

/// Each recently drawn vehicle, kept to start a watched death from
/// (`LastSeenHulls`), in the order first drawn.
#[derive(Default)]
pub struct LastSeenHulls {
    hulls: Vec<(f64, VehiclePose, f64)>,
    /// The clock of the last frame noted: a hull it did not draw is gone.
    clock: f64,
}

impl LastSeenHulls {
    /// The vehicles the frame at `clock` draws.
    pub fn note(&mut self, vehicles: &[VehiclePose], clock: f64) {
        for v in vehicles {
            let key = if &*v.side == "blue" {
                v.unit
            } else {
                -1.0 - v.unit
            };
            match self.hulls.iter_mut().find(|h| h.0 == key) {
                Some(h) => {
                    h.1.clone_from(v);
                    h.2 = clock;
                }
                None => self.hulls.push((key, v.clone(), clock)),
            }
        }
        self.hulls.retain(|h| clock - h.2 <= REMEMBERED_S);
        self.clock = clock;
    }

    /// The hull, gone from the frame lately, nearest where `c`'s wreck lies
    /// (within a roll to a stop), as last drawn; None if none.
    pub fn at(
        &self,
        c: &CookOff,
        clock: f64,
        units: &UnitsSection,
        looks: &BTreeMap<String, WreckLook>,
    ) -> Option<LastHull> {
        let mut best: Option<(&VehiclePose, f64)> = None;
        for (_, pose, at) in &self.hulls {
            let gone = *at < self.clock && clock - at <= REMEMBERED_S;
            let d = js_hypot(&[
                pose.position[0] - c.center[0],
                pose.position[1] - c.center[1],
            ]);
            if gone
                && pose.kind == c.wreck_of
                && d <= c.half[0] + ROLL_REACH_M
                && best.is_none_or(|b| d < b.1)
            {
                best = Some((pose, d));
            }
        }
        let (pose, _) = best?;
        let appearance = units.resolve(&pose.kind, &pose.side, pose.unit, 0, None, None)?;
        Some(LastHull {
            model: PlacedModel {
                appearance: appearance.into(),
                state: None,
                position: pose.position,
                yaw: pose.yaw,
                scale: [1.0; 3],
                motion: None,
                articulation: Some(pose.articulation),
                tint: units.tints.get(&*pose.side).copied(),
            },
            braking: looks.get(&*pose.kind)?.braking,
        })
    }
}

/// What cook-off `f` draws at `clock` (`cookOffModels`): its hull whole, as
/// last seen, until the ammunition goes, then its moving wreck. A hull
/// killed on the move rolls on to where its wreck lies, slowing at its
/// braking, carrying the moving wreck with it.
pub fn cook_off_models(
    f: &Transition,
    last: Option<&LastHull>,
    feel: &CookOffFeel,
    clock: f64,
    out: &mut Vec<PlacedModel>,
) {
    let Some(LastHull {
        model: hull,
        braking,
    }) = last
    else {
        return wreck_models(f, feel, clock, out);
    };
    let t = clock - f.hit_at;
    // Constant deceleration over `length` metres: done in √(2·length/braking) s.
    let [wx, wy, _] = f.wreck.position;
    let (dx, dy) = (wx - hull.position[0], wy - hull.position[1]);
    let length = js_hypot(&[dx, dy]);
    let done = (2.0 * length / braking).sqrt();
    let s = if t >= done {
        1.0
    } else {
        (braking * done * t - braking * t * t / 2.0) / length
    };
    let (x, y) = (hull.position[0] + dx * s, hull.position[1] + dy * s);
    if t < feel.delay_s {
        let position = [x, y, hull.position[2]];
        out.push(PlacedModel {
            position,
            ..hull.clone()
        });
        return;
    }
    let from = out.len();
    wreck_models(f, feel, clock, out);
    for m in &mut out[from..] {
        m.position[0] += x - wx;
        m.position[1] += y - wy;
    }
}

/// A side's cook-offs from publication to drawn models, as the battle drives
/// them: each watched brew-up's moving wreck until a moment after it lies
/// still (when the static wreck takes over), and its thrown debris until it
/// has sunk away.
pub struct CookOffs {
    watch: CookOffWatch,
    tick_hz: f64,
    /// Moving wrecks, each with its hull as last seen once looked for.
    moving: Vec<(Transition, Option<Option<LastHull>>)>,
    /// Cook-offs whose thrown debris is still drawn: it outlives the moving wreck.
    thrown: Vec<Transition>,
    hulls: LastSeenHulls,
    last_tick: f64,
}

impl CookOffs {
    /// For a battle at `tick_hz`: a side learns a wreck within a second of
    /// losing its hull.
    pub fn new(tick_hz: f64) -> Self {
        Self {
            watch: CookOffWatch::new(tick_hz),
            tick_hz,
            moving: Vec::new(),
            thrown: Vec::new(),
            hulls: LastSeenHulls::default(),
            last_tick: -1.0,
        }
    }

    /// The cook-offs `o` shows, as the effects take them (the publication's
    /// `cook_offs`); each whose wreck is drawn starts moving.
    pub fn note(
        &mut self,
        o: &Observation,
        units: &UnitsSection,
        effects: &EffectsSection,
        looks: &BTreeMap<String, WreckLook>,
    ) -> Vec<publication::CookOff> {
        if o.tick < self.last_tick {
            self.moving.clear();
            self.thrown.clear();
        }
        self.last_tick = o.tick;
        let mut out = Vec::new();
        for c in self.watch.note(o, units, effects) {
            let f = transition_of(&c, looks, &effects.wrecks, self.tick_hz);
            out.push(effect_cook_off(&c, f.as_ref()));
            if let Some(f) = f {
                if f.debris_top.is_some() {
                    self.thrown.push(f.clone());
                }
                self.moving.push((f, None));
            }
        }
        out
    }

    /// The models the cook-offs draw at `clock`, appended to `out`; `vehicles`
    /// are the vehicles this frame draws.
    pub fn models(
        &mut self,
        vehicles: &[VehiclePose],
        clock: f64,
        units: &UnitsSection,
        looks: &BTreeMap<String, WreckLook>,
        feel: &CookOffFeel,
        out: &mut Vec<PlacedModel>,
    ) {
        self.hulls.note(vehicles, clock);
        let settled = feel.landed_after() + LANDED_HOLD_S;
        self.moving.retain(|(f, _)| clock - f.hit_at <= settled);
        for (f, last) in &mut self.moving {
            let last = last.get_or_insert_with(|| self.hulls.at(&f.cook_off, clock, units, looks));
            cook_off_models(f, last.as_ref(), feel, clock, out);
        }
        self.thrown
            .retain(|f| clock - f.hit_at < feel.debris_gone_after());
        out.extend(
            self.thrown
                .iter()
                .filter_map(|f| debris_model(f, feel, clock)),
        );
    }

    /// The known props drawn here while their wreck moves: the static
    /// wrecks to leave out until it lies still.
    pub fn moving_props(&self) -> impl Iterator<Item = f64> + '_ {
        self.moving.iter().map(|(f, _)| f.cook_off.prop)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::cook_off::tests::feel;

    fn cook_off() -> CookOff {
        CookOff {
            prop: 17.0,
            wreck_of: "tank".into(),
            center: [100.0, 50.0],
            yaw: 0.4,
            half: [3.5, 1.8, 1.2],
            base_z: 2.0,
            tick: 31.0,
        }
    }

    /// A tank's wreck: thrown turret (`lies`) or jolting whole.
    fn transition(lies: Option<Vec3>) -> Transition {
        let look = WreckLook {
            appearance: "tank_wreck".into(),
            footprint: [3.5, 1.8, 1.2],
            bounds: [[-3.5, -1.8, -0.3], [3.5, 1.8, 2.4]],
            braking: 8.0,
        };
        let wreck = Wreck {
            lies,
            underside: 1.2,
            debris_top: Some(0.5),
        };
        let looks = BTreeMap::from([("tank".to_owned(), look)]);
        let wrecks = BTreeMap::from([("tank_wreck".to_owned(), wreck)]);
        transition_of(&cook_off(), &looks, &wrecks, 30.0).expect("a drawn wreck")
    }

    /// The tank as last seen, 4 m short of where its wreck lies.
    fn last() -> LastHull {
        LastHull {
            model: PlacedModel {
                appearance: "tank".into(),
                state: None,
                position: [96.0, 50.0, 2.0],
                yaw: 0.4,
                scale: [1.0; 3],
                motion: None,
                articulation: Some(Articulation::default()),
                tint: Some([0.2, 0.3, 0.8]),
            },
            braking: 8.0,
        }
    }

    fn models(f: &Transition, last: Option<&LastHull>, t: f64) -> Vec<PlacedModel> {
        let mut out = Vec::new();
        cook_off_models(f, last, &feel(), f.hit_at + t, &mut out);
        out
    }

    #[test]
    fn a_hull_is_drawn_as_last_seen_until_its_ammunition_goes_then_as_the_wrecks_pieces() {
        let f = transition(Some([0.5, 0.2, 2.0]));
        let before = models(&f, Some(&last()), 0.1);
        assert_eq!(before.len(), 1);
        assert_eq!((before[0].state, &*before[0].appearance), (None, "tank"));
        let after = models(&f, Some(&last()), 0.5);
        let states: Vec<_> = after.iter().map(|m| m.state).collect();
        assert_eq!(states, [Some("hull"), Some("turret")]);
        // Rolled to a stop, the pieces lie where the wreck does.
        for m in models(&f, Some(&last()), 5.0) {
            assert_eq!(m.position, [100.0, 50.0, 2.0]);
        }
    }

    #[test]
    fn a_hull_killed_on_the_move_glides_on_to_its_wreck_slowing_as_it_brakes() {
        let f = transition(Some([0.5, 0.2, 2.0]));
        let x = |t: f64| models(&f, Some(&last()), t)[0].position[0];
        let xs: Vec<f64> = (0..=10).map(|k| x(k as f64 * 0.1)).collect();
        let steps: Vec<f64> = xs.windows(2).map(|w| w[1] - w[0]).collect();
        assert!(steps.iter().all(|&s| s > 0.0), "{xs:?}");
        assert!(steps.windows(2).all(|w| w[1] < w[0] + 1e-9), "{steps:?}");
        // Four metres at 8 m/s²: stopped after a second.
        assert!((x(1.0) - 100.0).abs() < 1e-9);
    }

    #[test]
    fn only_a_wreck_with_a_turret_piece_throws_a_turret() {
        let whole = transition(None);
        let pieces = models(&whole, None, 0.5);
        assert_eq!(
            pieces.iter().map(|m| m.state).collect::<Vec<_>>(),
            [Some("default")]
        );
        assert_eq!(effect_cook_off(&whole.cook_off, Some(&whole)).landing, None);
        let tank = transition(Some([0.5, 0.2, 2.0]));
        let landing = effect_cook_off(&tank.cook_off, Some(&tank))
            .landing
            .expect("lands");
        // On the deck at its underside, beside the ring as the wreck is turned.
        let (s, c) = 0.4f64.sin_cos();
        let want = [
            100.0 + 0.5 * c - 0.2 * s,
            50.0 + 0.5 * s + 0.2 * c,
            2.0 + 1.2,
        ];
        assert!(landing.iter().zip(want).all(|(a, b)| (a - b).abs() < 1e-12));
    }

    #[test]
    fn a_wreck_jolting_whole_never_sinks_below_where_it_rests() {
        let f = transition(None);
        let [min, max] = f.bounds;
        for k in 0..60 {
            let m = models(&f, None, k as f64 * 0.05)[0]
                .motion
                .expect("a motion");
            let low = (0..3)
                .map(|i| (m[2 + 4 * i] * min[i]).min(m[2 + 4 * i] * max[i]))
                .sum::<f64>()
                + m[14];
            assert!(low >= min[2] - 1e-9, "at {k}: {low}");
        }
    }

    #[test]
    fn thrown_debris_lies_round_the_wreck_then_sinks_away_and_is_gone() {
        let (f, feel) = (transition(Some([0.5, 0.2, 2.0])), feel());
        let sink = |t: f64| debris_model(&f, &feel, f.hit_at + t).map(|m| -m.motion.unwrap()[14]);
        assert_eq!(sink(0.1), None);
        assert_eq!(sink(feel.delay_s + 1.0), Some(0.0));
        let late = feel.debris_gone_after() - 0.01;
        assert!(sink(late).unwrap() > 0.5);
        assert_eq!(sink(feel.debris_gone_after()), None);
    }
}
