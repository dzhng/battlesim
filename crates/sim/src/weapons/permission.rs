//! Whether a unit's standing orders let it fire at a target (W12–W13).
use super::{FireContext, Target};
use crate::units::Unit;
use contract::command::Engagement;

/// Whether the unit's engagement policy allows firing at this target (W12–W13).
pub(super) fn permitted(ctx: &FireContext, unit: &Unit, target: Target) -> bool {
    match unit.engagement {
        // Which weapons fire at a radar contact is theirs to say
        // (`anti_radiation`, [`preferred_kind`]).
        Engagement::FireAtWill => true,
        Engagement::ReturnFireOnly => match target {
            Target::Unit(u) => unit.attackers.contains(&u),
            Target::Contact(c) => ctx.knowledge[unit.side.index()]
                .contact(c)
                .is_some_and(|contact| unit.attackers.contains(&contact.emitter)),
            Target::Ground(_) => false,
            Target::Round(_) => true,
        },
    }
}
