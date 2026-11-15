//! Stand-ins keep observed bodies visible and pickable while their art is absent.
use contract::ids::Side;
use presentation::fielded;
use presentation::presentation::{Presentation, Ui};
use presentation::view::Camera;
use presentation::world::{BattleInput, SoldierBody, StandIn, UnitsSection, WorldInput, Xray};
use serde_json::json;
use sim::battle::Battle;
use sim::publication::{self, Publisher};
use std::path::PathBuf;

const STAND_IN: &str = "__battle_stand_in__";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn setup() -> (Battle, presentation::fielded::Fielded) {
    let battle = Battle::new(
        &serde_json::from_value(json!({
            "map":{"size":[600,400],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35},
            "rules":sim::fixtures::test_rules(),
            "units":[
                {"side":"blue","kind":"test_tank","position":[0,0]},
                {"side":"blue","kind":"test_rifle","position":[12,0]},
                {"side":"red","kind":"test_tank","position":[500,300]}
            ],"events":[],"scripts":[]
        }))
        .unwrap(),
        1,
    );
    let fielded = fielded::fielded(
        &root(),
        battle.rules(),
        &["test_tank".into(), "test_rifle".into()],
    )
    .unwrap();
    (battle, fielded)
}

fn with_art_withheld(mut units: UnitsSection) -> UnitsSection {
    units
        .types
        .get_mut("test_tank")
        .unwrap()
        .draws
        .values_mut()
        .for_each(|d| d.hull.clear());
    units
        .types
        .get_mut("test_rifle")
        .unwrap()
        .draws
        .values_mut()
        .for_each(|d| {
            d.slots.iter_mut().for_each(Vec::clear);
            d.hull.clear();
        });
    units
}

fn presented(
    units: UnitsSection,
    effects: presentation::world::EffectsSection,
    battle: &Battle,
    stand_in: Option<StandIn>,
) -> Presentation {
    let mut p = Presentation::new(Some(&publication::layout_json(battle)), 30.0).unwrap();
    p.load_world(
        WorldInput {
            battle: Some(BattleInput {
                units,
                effects,
                side: "blue".into(),
                tick_hz: 30.0,
                soldier: SoldierBody {
                    radius_m: 0.2,
                    height_m: 1.8,
                },
                xray: Xray {
                    own: [0.0; 4],
                    selected: [0.0; 4],
                    ally: [0.0; 4],
                },
                blocked: [0.0; 4],
                stand_in,
            }),
            sun_azimuth: 0.0,
            sun_elevation: 0.7,
            buildings: None,
        },
        Default::default(),
        None,
    )
    .unwrap();
    let words: Vec<u32> = Publisher::new()
        .publish(battle, Side::Blue)
        .unwrap()
        .iter()
        .map(|f| f.to_bits())
        .collect();
    assert!(p.push_publication(&words, 0.0).unwrap());
    p
}

fn stand_in(shadow: bool) -> StandIn {
    serde_json::from_value(json!({
        "appearance": STAND_IN,
        "half": [0.6, 0.6, 0.6],
        "tint": [0.3, 0.4, 0.5],
        "shadow": shadow,
    }))
    .unwrap()
}

fn camera() -> Camera {
    Camera {
        eye: [20.0, -70.0, 55.0],
        target: [8.0, 0.0, 0.0],
        up: [0.0, 0.0, 1.0],
        fov_y: 0.9,
        near: 0.1,
        size: [1280, 720],
    }
}

fn stand_in_instances(p: &mut Presentation) -> Vec<presentation::records::Instance> {
    let f = p.frame(0.0, &camera(), &Ui::default(), 0).unwrap();
    let batches = f.batches.clone();
    let instances = f.instances.clone();
    let names = p
        .world()
        .appearances
        .iter()
        .map(|r| p.world().name(r).to_owned())
        .collect::<Vec<_>>();
    batches
        .iter()
        .filter(|b| names[b.appearance as usize] == STAND_IN)
        .flat_map(|b| {
            instances[b.first as usize..(b.first + b.count) as usize]
                .iter()
                .copied()
        })
        .collect()
}

#[test]
fn seen_unarted_units_draw_one_scaled_stand_in_per_body() {
    let (battle, fielded) = setup();
    let half_tank = fielded.units.body_half["test_tank"].unwrap();
    let half_soldier = [0.2, 0.2, 0.9];
    let mut p = presented(
        with_art_withheld(fielded.units),
        fielded.effects,
        &battle,
        Some(stand_in(true)),
    );
    let picks = p
        .frame(0.0, &camera(), &Ui::default(), 0)
        .unwrap()
        .picks
        .clone();
    let instances = stand_in_instances(&mut p);
    let squad_members = picks.iter().filter(|pick| pick.member != u32::MAX).count();
    assert_eq!(instances.len(), 1 + squad_members);
    assert!(instances
        .iter()
        .any(|i| i.placement[0] == 0.0 && i.scale[..3] == half_tank.map(|v| (v / 0.6) as f32)));
    assert_eq!(
        instances.iter().filter(|i| i.placement[0] != 0.0).count(),
        squad_members
    );
    assert!(instances
        .iter()
        .filter(|i| i.placement[0] != 0.0)
        .all(|i| i.scale[..3] == half_soldier.map(|v| (v / 0.6) as f32)));
}

#[test]
fn stand_in_and_committed_art_have_the_same_picks_and_hull_size() {
    let (battle, fielded) = setup();
    let expected = fielded.units.body_half["test_tank"]
        .unwrap()
        .map(|v| v as f32);
    let mut stand = presented(
        with_art_withheld(fielded.units.clone()),
        fielded.effects.clone(),
        &battle,
        Some(stand_in(true)),
    );
    let stand_picks = stand
        .frame(0.0, &camera(), &Ui::default(), 0)
        .unwrap()
        .picks
        .clone();
    let mut art = presented(
        fielded.units,
        fielded.effects,
        &battle,
        Some(stand_in(true)),
    );
    let art_picks = art
        .frame(0.0, &camera(), &Ui::default(), 0)
        .unwrap()
        .picks
        .clone();
    assert_eq!(stand_picks, art_picks);
    assert_eq!(stand_picks[0].half[..3], expected);
}

#[test]
fn committing_art_removes_the_stand_in_for_that_type() {
    let (battle, fielded) = setup();
    let detail = fielded.units.detail;
    let art_units = fielded.units.clone();
    let mut p = presented(
        with_art_withheld(fielded.units),
        fielded.effects.clone(),
        &battle,
        Some(stand_in(true)),
    );
    let before = p
        .frame(0.0, &camera(), &Ui::default(), 0)
        .unwrap()
        .picks
        .clone();
    assert!(!stand_in_instances(&mut p).is_empty());
    assert!(p
        .stage_battle(
            WorldInput {
                battle: Some(BattleInput {
                    units: art_units,
                    effects: fielded.effects,
                    side: "blue".into(),
                    tick_hz: 30.0,
                    soldier: SoldierBody {
                        radius_m: 0.2,
                        height_m: 1.8
                    },
                    xray: Xray {
                        own: [0.0; 4],
                        selected: [0.0; 4],
                        ally: [0.0; 4]
                    },
                    blocked: [0.0; 4],
                    stand_in: Some(stand_in(true)),
                }),
                sun_azimuth: 0.0,
                sun_elevation: 0.7,
                buildings: None,
            },
            Default::default(),
        )
        .unwrap());
    let draft = p
        .stage_catalog(
            Default::default(),
            Default::default(),
            detail,
            Default::default(),
        )
        .unwrap();
    assert!(p.commit_catalog(draft, Vec::new()).unwrap());
    assert!(stand_in_instances(&mut p).is_empty());
    assert_eq!(p.last_frame().picks, before);
}

#[test]
fn an_unseen_unarted_unit_draws_nothing() {
    let (battle, fielded) = setup();
    let mut p = presented(
        with_art_withheld(fielded.units),
        fielded.effects,
        &battle,
        Some(stand_in(true)),
    );
    let frame = p.frame(0.0, &camera(), &Ui::default(), 0).unwrap();
    let unseen = battle.observe(Side::Red).own[0].id;
    assert!(frame.picks.iter().all(|pick| pick.unit != unseen.0));
    assert!(stand_in_instances(&mut p)
        .iter()
        .all(|i| i.placement[0] < 100.0));
}

#[test]
fn a_stand_in_takes_its_given_colour_and_its_own_shadow_policy() {
    let (battle, fielded) = setup();
    let expected_art = fielded.units.tints["blue"].map(|v| v as f32);
    let look = stand_in(false);
    let expected_stand = look
        .tint
        .map(|v| presentation::static_world::srgb_to_linear(v) as f32);
    let mut stand = presented(
        with_art_withheld(fielded.units.clone()),
        fielded.effects.clone(),
        &battle,
        Some(look),
    );
    let frame = stand.frame(0.0, &camera(), &Ui::default(), 0).unwrap();
    assert!(frame.batches.iter().all(|b| b.caster == u32::MAX));
    assert!(frame
        .instances
        .iter()
        .all(|i| i.tint[..3] == expected_stand));
    let mut art = presented(
        fielded.units,
        fielded.effects,
        &battle,
        Some(stand_in(false)),
    );
    let frame = art.frame(0.0, &camera(), &Ui::default(), 0).unwrap();
    assert!(frame.batches.iter().any(|b| b.caster != u32::MAX));
    assert!(frame.instances.iter().all(|i| i.tint[..3] == expected_art));
}
