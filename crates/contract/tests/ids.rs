use contract::encounter::EncounterRecipes;
use contract::ids::Side;

const ENCOUNTERS: &str = include_str!("../../../fixtures/encounters.json");

#[test]
fn side_contract_includes_the_fixed_neutral_slot() {
    assert_eq!(Side::ALL, [Side::Blue, Side::Red, Side::Neutral]);

    for (index, side) in Side::ALL.into_iter().enumerate() {
        assert_eq!(side.index(), index);
        let encoded = serde_json::to_string(&side).unwrap();
        let decoded: Side = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, side);
    }
}

#[test]
fn binary_encounter_recipes_refuse_a_neutral_attacker() {
    let mut recipes = EncounterRecipes::from_json(ENCOUNTERS).unwrap();
    let recipe = recipes.recipes.get_mut("assault").unwrap();
    recipe.attacker = Side::Neutral;

    assert_eq!(
        recipe.validate().unwrap_err(),
        "attacker neutral is unsupported by binary encounters"
    );
}
