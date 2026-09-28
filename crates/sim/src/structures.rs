//! Prop integrity (Q17, 34c): the one store of how much damage every
//! destroyable body has taken, and what destroyed bodies left behind. A
//! type's body row says whether it can be destroyed (`hp`),
//! how much of a direct round it takes (`armor`), and what it becomes
//! (`destroyed`); buildings are one row of it (L10). A prop no round has
//! worn down is whole, so only damaged props are stored.
use std::collections::{BTreeMap, BTreeSet};

use crate::digest::Digest;
use crate::world::{Prop, PropId, WorldGeometry};

#[derive(Clone, Debug, Default)]
pub struct Structures {
    /// Structural damage taken by each standing destroyable prop worn down.
    damage: BTreeMap<PropId, f64>,
    /// Each prop a destroyed body left in its place → the body it replaced
    /// (a ruin's building, sandbags' rubble, a lighter wreck).
    replaced: BTreeMap<PropId, PropId>,
    /// Bodies destroyed with nothing in their place (a crate blown away);
    /// felled trees are their cleared ground instead.
    removed: BTreeMap<PropId, Prop>,
    /// Every body destroyed so far, of any state.
    destroyed: BTreeSet<PropId>,
}

impl Structures {
    /// What is left of `id`'s integrity: None if it is gone or its kind
    /// cannot be destroyed.
    pub fn hp(&self, world: &WorldGeometry, id: PropId) -> Option<f64> {
        let hp = world.prop(id)?.body.hp?;
        Some(hp - self.damage.get(&id).copied().unwrap_or(0.0))
    }

    /// Wear `id` down by `amount`; true, once, when it must be destroyed.
    pub fn damage(&mut self, world: &WorldGeometry, id: PropId, amount: f64) -> bool {
        let Some(hp) = world.prop(id).and_then(|p| p.body.hp) else {
            return false;
        };
        if amount <= 0.0 || self.destroyed.contains(&id) {
            return false;
        }
        let taken = self.damage.entry(id).or_insert(0.0);
        *taken += amount;
        if *taken >= hp {
            self.damage.remove(&id);
            self.destroyed.insert(id);
            return true;
        }
        false
    }

    /// `prop` was destroyed with nothing in its place.
    pub fn note_removed(&mut self, prop: Prop) {
        self.removed.insert(prop.id, prop);
    }

    /// `by` stands in place of the destroyed `prop`.
    pub fn note_replaced(&mut self, by: PropId, prop: PropId) {
        self.replaced.insert(by, prop);
    }

    /// The body a destroyed body's remains stand in place of.
    pub fn replaced_by(&self, remains: PropId) -> Option<PropId> {
        self.replaced.get(&remains).copied()
    }

    /// Bodies destroyed with nothing in their place, as they stood.
    pub fn removed(&self) -> impl Iterator<Item = &Prop> {
        self.removed.values()
    }

    pub fn digest(&self, d: &mut Digest) {
        d.u64(self.damage.len() as u64);
        for (id, taken) in &self.damage {
            d.u64(*id as u64).f64(*taken);
        }
        d.u64(self.replaced.len() as u64);
        for (by, prop) in &self.replaced {
            d.u64(*by as u64).u64(*prop as u64);
        }
        d.u64(self.destroyed.len() as u64);
        for id in &self.destroyed {
            d.u64(*id as u64);
        }
    }
}
