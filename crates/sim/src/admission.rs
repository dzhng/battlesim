//! The admission boundary for every external battle rules input.
use contract::scenario::Rules;
pub use contract::scenario::RulesError;

/// Refuse rules before constructing a battle, navigation or world handle.
pub fn admit(rules: &Rules) -> Result<(), RulesError> {
    rules.check()?;
    rules.forests.check_placement(&rules.catalog)?;
    crate::weapons::check_rules(rules)?;
    crate::supply::validate(rules)?;
    Ok(())
}
