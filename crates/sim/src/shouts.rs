//! Deterministic, side-scoped bark arbitration. Shouts are presentation
//! evidence and deliberately never participate in the battle digest.
use std::collections::{BTreeMap, BTreeSet};

use contract::catalog::AltitudeLayer;
use contract::ids::{Side, Tick, UnitId};
use contract::observation::{
    ActionReason, Intensity, ObservedTargetId, Shout, ShoutSpeaker, ShoutTrigger, SuppressionTier,
};
use contract::scenario::Rules;

use crate::battle::Round;
use crate::flight::{FlightEvent, Projectile, ProjectileId};
use crate::knowledge::SideKnowledge;
use crate::math::Obb2;
use crate::units::Unit;
use crate::weapons::{Arsenal, Target};
use crate::world::WorldGeometry;

/// When one unit has several barks on one tick, the earliest here speaks:
/// the more specific moment first.
pub const PRIORITY: [ShoutTrigger; 14] = [
    ShoutTrigger::B16Destroyed,
    ShoutTrigger::B15IncomingMissile,
    ShoutTrigger::B10BadlyDamaged,
    ShoutTrigger::B9ManDown,
    ShoutTrigger::B8Hit,
    ShoutTrigger::B7Pinned,
    ShoutTrigger::B6Suppressed,
    ShoutTrigger::B17FriendlyDestroyedNearby,
    ShoutTrigger::B5UnderFire,
    ShoutTrigger::B14MissileAway,
    ShoutTrigger::B13OutOfAmmo,
    ShoutTrigger::B12Reloading,
    ShoutTrigger::B11Kill,
    ShoutTrigger::B4OpensFire,
];

/// What changed this tick, read by comparing the units before and after it.
#[derive(Clone, Debug, Default)]
pub struct ShoutEvidence {
    /// Every enemy each side identifies this tick, with its altitude layer.
    pub identified: Vec<(UnitId, AltitudeLayer)>,
    /// What each unit that launched this tick launched at.
    pub fired: BTreeMap<UnitId, Vec<Target>>,
    /// Units whose round destroyed an enemy this tick.
    pub kills: BTreeSet<UnitId>,
    /// Each unit's own barks read from its state.
    pub triggers: BTreeMap<UnitId, BTreeSet<ShoutTrigger>>,
}

/// The physical moments flight leaves behind for one tick.
#[derive(Clone, Debug, Default)]
pub struct Physical {
    /// Units an enemy round passed or landed near ([`under_fire`]).
    pub under_fire: BTreeSet<UnitId>,
    /// Units that see an enemy missile steering onto them ([`incoming`]).
    pub incoming: BTreeSet<UnitId>,
}

/// One unit as barks last saw it: only what a bark compares across a tick,
/// so a tick costs a few numbers per unit, not a copy of every unit.
#[derive(Clone, Debug)]
pub struct Before {
    alive: bool,
    hp: f64,
    member_hp: Vec<f64>,
    tier: SuppressionTier,
    mounts: Vec<MountBefore>,
}

#[derive(Clone, Copy, Debug)]
struct MountBefore {
    reason: ActionReason,
    shots: u32,
    missile: Option<ProjectileId>,
}

impl Before {
    /// Health as barks read it: a vehicle's share of its hull's hp; a
    /// squad's living members over its starting members.
    fn health(&self, unit: &Unit, rules: &Rules) -> f64 {
        if self.member_hp.is_empty() {
            self.hp / unit.max_hp(rules)
        } else {
            self.member_hp.iter().filter(|&&hp| hp > 0.0).count() as f64
                / self.member_hp.len() as f64
        }
    }
}

/// Note every unit as it stands, reusing the storage of the last tick's note.
pub fn remember(before: &mut Vec<Before>, units: &[Unit], rules: &Rules) {
    before.truncate(units.len());
    for (i, unit) in units.iter().enumerate() {
        if i == before.len() {
            before.push(Before {
                alive: false,
                hp: 0.0,
                member_hp: Vec::new(),
                tier: SuppressionTier::None,
                mounts: Vec::new(),
            });
        }
        let b = &mut before[i];
        b.alive = unit.alive();
        b.hp = unit.hp;
        b.member_hp.clear();
        b.member_hp.extend(unit.members.iter().map(|m| m.hp));
        b.tier = rules.suppression.tier(unit.suppression);
        b.mounts.clear();
        b.mounts.extend(unit.mounts.iter().map(|m| MountBefore {
            reason: m.reason,
            shots: m.shots,
            missile: m.support.map(|s| s.projectile),
        }));
    }
}

/// Health as barks read it: a vehicle's share of its hull's hp; a squad's
/// living members over its starting members.
fn health(unit: &Unit, rules: &Rules) -> f64 {
    if unit.members.is_empty() {
        unit.hp / unit.max_hp(rules)
    } else {
        unit.members.iter().filter(|m| m.alive()).count() as f64 / unit.members.len() as f64
    }
}

fn tier_rank(tier: SuppressionTier) -> u8 {
    match tier {
        SuppressionTier::None => 0,
        SuppressionTier::Suppressed => 1,
        SuppressionTier::Pinned => 2,
    }
}

/// Every unit that heard an enemy round this tick: one passed it, struck
/// it, glanced off it or landed within the round's suppression radius of
/// it, scored as suppression scores a near miss. Hulls hear rounds too;
/// a unit's own side's rounds never count. Read before flight forgets the
/// rounds that ended this tick.
pub fn under_fire(
    events: &[FlightEvent],
    rounds: &BTreeMap<ProjectileId, Round>,
    units: &[Unit],
    arsenal: &Arsenal,
    rules: &Rules,
) -> BTreeSet<UnitId> {
    let mut out = BTreeSet::new();
    for event in events {
        let (projectile, point) = match event {
            FlightEvent::NearMiss(n) => (n.projectile, None),
            FlightEvent::Impact(i) => (i.projectile, Some(i.point)),
            FlightEvent::Ricochet(r) => (r.projectile, Some(r.point)),
            FlightEvent::Expired(_) | FlightEvent::Pass(_) => continue,
        };
        let Some(round) = rounds.get(&projectile) else {
            continue;
        };
        let def = &arsenal.weapons[round.weapon].def;
        let enemy = |unit: &&Unit| unit.side != round.side && unit.on_field();
        let Some(point) = point else {
            // A near miss: flight measured the pass to the unit's nearest body.
            let FlightEvent::NearMiss(n) = event else {
                continue;
            };
            let unit = &units[n.unit.0 as usize];
            if enemy(&unit) && crate::damage::near_miss(def, n.distance) > 0.0 {
                out.insert(n.unit);
            }
            continue;
        };
        for unit in units.iter().filter(enemy) {
            let distance = match unit.hull {
                Some(_) => unit.hull_distance(point),
                None => unit
                    .members
                    .iter()
                    .filter(|m| m.alive())
                    .map(|m| {
                        (m.position + crate::math::v3(0.0, 0.0, rules.physics.infantry_center_m)
                            - point)
                            .length()
                    })
                    .fold(f64::INFINITY, f64::min),
            };
            if crate::damage::near_miss(def, distance) > 0.0 {
                out.insert(unit.id);
            }
        }
    }
    out
}

/// Whether `point` lies on the unit: within its hull's extents seen from
/// above, or within its squad's area.
fn on_footprint(unit: &Unit, point: crate::math::V3, rules: &Rules) -> bool {
    match unit.hull {
        Some(half) => Obb2 {
            center: unit.position.xy(),
            yaw: unit.yaw,
            half: half.xy(),
        }
        .contains(point.xy(), 0.0),
        None => {
            (point.xy() - unit.position.xy()).length()
                <= unit.footprint_radius(rules.physics.soldier_radius_m)
        }
    }
}

/// Every unit that sees an enemy guided round in flight whose commanded
/// point lies on it, supported or released.
pub fn incoming(
    world: &WorldGeometry,
    projectiles: &[Projectile],
    rounds: &BTreeMap<ProjectileId, Round>,
    units: &[Unit],
    rules: &Rules,
) -> BTreeSet<UnitId> {
    let mut out = BTreeSet::new();
    for p in projectiles {
        let (Some(guidance), Some(round)) = (p.guidance, rounds.get(&p.id)) else {
            continue;
        };
        for unit in units {
            if unit.side == round.side || !unit.on_field() || out.contains(&unit.id) {
                continue;
            }
            if on_footprint(unit, guidance.point, rules)
                && crate::sensing::sees_point(
                    world,
                    crate::sensing::eye(unit, rules),
                    p.position,
                    &crate::sight::of(unit, rules),
                    1.0,
                    &rules.sensors,
                )
            {
                out.insert(unit.id);
            }
        }
    }
    out
}

pub fn evidence(
    before: &[Before],
    after: &[Unit],
    world: &WorldGeometry,
    physical: &Physical,
    fired: BTreeSet<UnitId>,
    identified: Vec<(UnitId, AltitudeLayer)>,
    rules: &Rules,
) -> ShoutEvidence {
    let mut e = ShoutEvidence {
        identified,
        ..Default::default()
    };
    let mut destroyed = Vec::new();
    for (old, now) in before.iter().zip(after) {
        // Already gone, or withdrawn rather than destroyed.
        if !old.alive || now.retired {
            continue;
        }
        let mut t = BTreeSet::new();
        let lives = now.alive();
        if !lives {
            t.insert(ShoutTrigger::B16Destroyed);
            destroyed.push(now);
        }
        let (was, is) = (old.health(now, rules), health(now, rules));
        let wounded = now
            .members
            .iter()
            .zip(&old.member_hp)
            .any(|(n, &o)| n.hp < o);
        if lives && (now.hp < old.hp || wounded) {
            t.insert(ShoutTrigger::B8Hit);
        }
        let dead_now = now.members.iter().filter(|m| !m.alive()).count();
        let dead_then = old.member_hp.iter().filter(|&&hp| hp <= 0.0).count();
        if lives && dead_now > dead_then {
            t.insert(ShoutTrigger::B9ManDown);
        }
        if lives && was > 1.0 / 3.0 && is <= 1.0 / 3.0 {
            t.insert(ShoutTrigger::B10BadlyDamaged);
        }
        let tier = rules.suppression.tier(now.suppression);
        if tier_rank(tier) > tier_rank(old.tier) {
            t.insert(match tier {
                SuppressionTier::Pinned => ShoutTrigger::B7Pinned,
                _ => ShoutTrigger::B6Suppressed,
            });
        }
        // A suppressed or pinned squad's fire is B6's and B7's moment.
        if lives && tier == SuppressionTier::None && physical.under_fire.contains(&now.id) {
            t.insert(ShoutTrigger::B5UnderFire);
        }
        if lives && physical.incoming.contains(&now.id) {
            t.insert(ShoutTrigger::B15IncomingMissile);
        }
        for (n, o) in now.mounts.iter().zip(&old.mounts) {
            let became = |r: ActionReason| n.reason == r && o.reason != r;
            // A rifleman calls his reload; a crew's loader works unheard.
            if became(ActionReason::Reloading) && !now.members.is_empty() {
                t.insert(ShoutTrigger::B12Reloading);
            }
            if became(ActionReason::OutOfAmmo) {
                t.insert(ShoutTrigger::B13OutOfAmmo);
            }
            if n.support.is_some_and(|s| o.missile != Some(s.projectile)) {
                t.insert(ShoutTrigger::B14MissileAway);
            }
        }
        if fired.contains(&now.id) {
            let targets = now
                .mounts
                .iter()
                .zip(&old.mounts)
                .filter(|(n, o)| n.shots != o.shots)
                .filter_map(|(n, _)| n.lock.as_ref().map(|l| l.target))
                .collect();
            e.fired.insert(now.id, targets);
        }
        if !t.is_empty() {
            e.triggers.insert(now.id, t);
        }
    }
    // A loss is called by one witness, the nearest friendly that sees where
    // it happened (where its crew or soldiers stood, not the bare ground
    // under it), never by a chorus.
    for dead in destroyed {
        let at = crate::sensing::eye(dead, rules);
        let witness = after
            .iter()
            .filter(|u| u.side == dead.side && u.on_field())
            .map(|u| (u, (u.position.xy() - at.xy()).length()))
            .filter(|(u, _)| {
                crate::sensing::sees_point(
                    world,
                    crate::sensing::eye(u, rules),
                    at,
                    &crate::sight::of(u, rules),
                    1.0,
                    &rules.sensors,
                )
            })
            .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.id.cmp(&b.0.id)));
        if let Some((witness, _)) = witness {
            e.triggers
                .entry(witness.id)
                .or_default()
                .insert(ShoutTrigger::B17FriendlyDestroyedNearby);
        }
    }
    e
}

#[derive(Clone, Debug, Default)]
pub struct ShoutLedger {
    pub last: BTreeMap<(UnitId, ShoutTrigger), Tick>,
    pub talking_until: BTreeMap<UnitId, Tick>,
    /// The enemies this side identified on the last tick.
    pub identified: BTreeSet<UnitId>,
    /// What each own unit last launched at.
    pub targets: BTreeMap<UnitId, Vec<Target>>,
}

impl ShoutLedger {
    fn can_start(&self, unit: UnitId, trigger: ShoutTrigger, now: Tick, rules: &Rules) -> bool {
        let cooldown = (rules.shouts.cooldown_s * rules.tick_hz as f64).round() as Tick;
        self.talking_until.get(&unit).copied().unwrap_or(0) <= now
            && self
                .last
                .get(&(unit, trigger))
                .is_none_or(|last| now >= last.saturating_add(cooldown))
    }

    fn push(
        &mut self,
        unit: &Unit,
        trigger: ShoutTrigger,
        now: Tick,
        rules: &Rules,
        out: &mut Vec<Shout>,
    ) {
        if !self.can_start(unit.id, trigger, now, rules) {
            return;
        }
        let talk = (rules.shouts.talk_s * rules.tick_hz as f64).round() as Tick;
        self.last.insert((unit.id, trigger), now);
        self.talking_until.insert(unit.id, now.saturating_add(talk));
        out.push(Shout {
            speaker: ShoutSpeaker::Own(unit.id),
            kind: unit.kind,
            position: [unit.position.x, unit.position.y, unit.position.z],
            trigger,
            intensity: intensity(unit, rules),
        });
    }
}

fn intensity(unit: &Unit, rules: &Rules) -> Intensity {
    if health(unit, rules) <= 0.5
        || rules.suppression.tier(unit.suppression) != SuppressionTier::None
    {
        Intensity::Stressed
    } else {
        Intensity::Steady
    }
}

fn contact_trigger(target: &Unit, layer: AltitudeLayer) -> ShoutTrigger {
    match layer {
        AltitudeLayer::LowAir | AltitudeLayer::HighAir => ShoutTrigger::B3ContactAir,
        AltitudeLayer::Ground if target.hull.is_some() => ShoutTrigger::B2ContactVehicle,
        AltitudeLayer::Ground => ShoutTrigger::B1ContactInfantry,
    }
}

/// This side's own barks this tick, every one spoken by a named own unit.
pub fn build(
    side: Side,
    evidence: &ShoutEvidence,
    units: &[Unit],
    knowledge: &SideKnowledge,
    rules: &Rules,
    tick: Tick,
    ledger: &mut ShoutLedger,
) -> Vec<Shout> {
    let mut out = Vec::new();
    let own: Vec<&Unit> = units
        .iter()
        .filter(|u| u.side == side && u.on_field())
        .collect();

    // Contact: an enemy this side did not identify last tick. The nearest
    // own unit that sees it calls it.
    let identified: BTreeSet<UnitId> = evidence
        .identified
        .iter()
        .filter(|(id, _)| units[id.0 as usize].side != side)
        .map(|(id, _)| *id)
        .collect();
    for &(enemy, layer) in &evidence.identified {
        let target = &units[enemy.0 as usize];
        if target.side == side || ledger.identified.contains(&enemy) {
            continue;
        }
        let distance = |u: &&&Unit| (u.position.xy() - target.position.xy()).length();
        let handle = knowledge.track(enemy).map(|t| t.id);
        let seers: Vec<&&Unit> = own
            .iter()
            .filter(|u| handle.is_some_and(|h| knowledge.own_sensor(u.id).contains(&h)))
            .collect();
        let candidates = if seers.is_empty() {
            own.iter().collect()
        } else {
            seers
        };
        let Some(listener) = candidates
            .into_iter()
            .min_by(|a, b| distance(a).total_cmp(&distance(b)))
        else {
            continue;
        };
        ledger.push(
            listener,
            contact_trigger(target, layer),
            tick,
            rules,
            &mut out,
        );
    }
    ledger.identified = identified;

    for unit in units.iter().filter(|u| u.side == side && !u.retired) {
        let mut triggers = evidence.triggers.get(&unit.id).cloned().unwrap_or_default();
        if evidence.kills.contains(&unit.id) {
            triggers.insert(ShoutTrigger::B11Kill);
        }
        if let Some(targets) = evidence.fired.get(&unit.id) {
            let known = ledger.targets.get(&unit.id);
            if targets.iter().any(|t| known.is_none_or(|k| !k.contains(t))) {
                triggers.insert(ShoutTrigger::B4OpensFire);
            }
            ledger.targets.insert(unit.id, targets.clone());
        }
        if let Some(&trigger) = PRIORITY.iter().find(|t| triggers.contains(t)) {
            ledger.push(unit, trigger, tick, rules, &mut out);
        }
    }
    out
}

/// What a side hears of the enemy's own barks: only those whose speaker it
/// identifies this tick, under its side-scoped handle.
pub fn heard(enemy: &[Shout], knowledge: &SideKnowledge, tick: Tick) -> Vec<Shout> {
    enemy
        .iter()
        .filter_map(|s| {
            Some(Shout {
                speaker: ShoutSpeaker::Seen(seen(s, knowledge, tick)?),
                ..s.clone()
            })
        })
        .collect()
}

/// The enemy speakers a side does not identify as they shout: hearing
/// turns each into an anonymous cue.
pub fn unheard<'a>(
    enemy: &'a [Shout],
    knowledge: &'a SideKnowledge,
    tick: Tick,
) -> impl Iterator<Item = UnitId> + 'a {
    enemy.iter().filter_map(move |s| match s.speaker {
        ShoutSpeaker::Own(unit) if seen(s, knowledge, tick).is_none() => Some(unit),
        _ => None,
    })
}

/// The handle a side hears an enemy's shout under, while it identifies the
/// speaker.
fn seen(s: &Shout, knowledge: &SideKnowledge, tick: Tick) -> Option<ObservedTargetId> {
    let ShoutSpeaker::Own(unit) = s.speaker else {
        return None;
    };
    knowledge
        .track(unit)
        .filter(|_| knowledge.identifies(unit, tick))
        .map(|t| t.id)
}
