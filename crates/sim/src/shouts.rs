//! Deterministic, side-scoped bark arbitration. Shouts are presentation
//! evidence and deliberately never participate in the battle digest.
use std::collections::{BTreeMap, BTreeSet};

use contract::catalog::AltitudeLayer;
use contract::ids::{Side, Tick, UnitId};
use contract::observation::{
    ActionReason, Intensity, Shout, ShoutSpeaker, ShoutTrigger, SuppressionTier,
};
use contract::scenario::Rules;

use crate::knowledge::SideKnowledge;
use crate::units::Unit;
use crate::weapons::Target;

/// When one unit has several barks on one tick, the earliest here speaks:
/// the more specific moment first.
pub const PRIORITY: [ShoutTrigger; 11] = [
    ShoutTrigger::B16Destroyed,
    ShoutTrigger::B10BadlyDamaged,
    ShoutTrigger::B9ManDown,
    ShoutTrigger::B8Hit,
    ShoutTrigger::B7Pinned,
    ShoutTrigger::B6Suppressed,
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

pub fn evidence(
    before: &[Unit],
    after: &[Unit],
    fired: BTreeSet<UnitId>,
    identified: Vec<(UnitId, AltitudeLayer)>,
    rules: &Rules,
) -> ShoutEvidence {
    let mut e = ShoutEvidence {
        identified,
        ..Default::default()
    };
    for (old, now) in before.iter().zip(after) {
        // Already gone, or withdrawn rather than destroyed.
        if !old.alive() || now.retired {
            continue;
        }
        let mut t = BTreeSet::new();
        let lives = now.alive();
        if !lives {
            t.insert(ShoutTrigger::B16Destroyed);
        }
        let (was, is) = (health(old, rules), health(now, rules));
        let wounded = now
            .members
            .iter()
            .zip(&old.members)
            .any(|(n, o)| n.hp < o.hp);
        if lives && (now.hp < old.hp || wounded) {
            t.insert(ShoutTrigger::B8Hit);
        }
        let dead = |u: &Unit| u.members.iter().filter(|m| !m.alive()).count();
        if lives && dead(now) > dead(old) {
            t.insert(ShoutTrigger::B9ManDown);
        }
        if lives && was > 1.0 / 3.0 && is <= 1.0 / 3.0 {
            t.insert(ShoutTrigger::B10BadlyDamaged);
        }
        let tier = |u: &Unit| rules.suppression.tier(u.suppression);
        if tier_rank(tier(now)) > tier_rank(tier(old)) {
            t.insert(match tier(now) {
                SuppressionTier::Pinned => ShoutTrigger::B7Pinned,
                _ => ShoutTrigger::B6Suppressed,
            });
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
            if n.support
                .is_some_and(|s| o.support.is_none_or(|p| p.projectile != s.projectile))
            {
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
        let talk = (rules.shouts.talk_s.get(&trigger).copied().unwrap_or(1.5)
            * rules.tick_hz as f64)
            .round() as Tick;
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
        AltitudeLayer::LowAir => ShoutTrigger::B3ContactAir,
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
        .filter(|u| u.side == side && u.alive())
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
            let ShoutSpeaker::Own(unit) = s.speaker else {
                return None;
            };
            let track = knowledge
                .track(unit)
                .filter(|_| knowledge.identifies(unit, tick))?;
            Some(Shout {
                speaker: ShoutSpeaker::Seen(track.id),
                ..s.clone()
            })
        })
        .collect()
}
