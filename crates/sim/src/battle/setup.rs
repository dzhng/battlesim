//! Admitted inputs becoming the one live battle state.

use super::*;

impl Battle {
    pub fn new(setup: &ScenarioDefinition, seed: u64) -> Self {
        Self::from_prepared(
            setup,
            seed,
            crate::encounter::PreparedMap::new(&setup.map, &setup.rules),
        )
    }

    /// Consume the physical world and navigation already used to place this encounter.
    /// `prepared` must have been built from this scenario's map and rules.
    pub fn from_prepared(
        setup: &ScenarioDefinition,
        seed: u64,
        prepared: crate::encounter::PreparedMap,
    ) -> Self {
        setup
            .validate_player_contract()
            .expect("admitted scenario has a valid player contract");
        let rules = setup.rules.clone();
        debug_assert!(rules.check().is_ok(), "rule sections were admitted");
        debug_assert!(
            supply::validate(&rules).is_ok(),
            "supply rules were admitted"
        );
        let crate::encounter::PreparedMap {
            world,
            roads,
            base: grid,
            ..
        } = prepared;
        let arsenal = Arsenal::new(&rules);
        // An uploaded scenario was refused before this with these same errors.
        let unknown = setup.unknown_catalog_references();
        debug_assert!(
            unknown.is_empty(),
            "scenario names kinds its rules lack: {unknown:?}"
        );
        let mut soldier_ids = 0u32;
        let mut units = setup
            .units
            .iter()
            .enumerate()
            .map(|(i, u)| spawn_unit(i, u, &world, &rules, &arsenal, &mut soldier_ids, seed))
            .collect::<Vec<Unit>>();
        for (unit, setup) in units.iter_mut().zip(&setup.units) {
            if let Some(c) = &setup.condition {
                spawn::wear(unit, c, &arsenal, &rules);
            }
            unit.settle();
        }
        let mut events: Vec<ScenarioEvent> = setup.events.clone();
        events.sort_by_key(|e| e.tick);
        let mut scripts: Vec<ScriptedOrder> = setup.scripts.clone();
        scripts.sort_by_key(|o| o.tick);
        let authored_scripts = setup
            .authored_scripts
            .iter()
            .map(|script| {
                (
                    CompiledScript::compile(&script.source)
                        .expect("scenario authored script was admitted"),
                    script.side,
                    script.player.clone(),
                )
            })
            .collect();
        let victory_script = setup
            .victory
            .as_deref()
            .map(|source| CompiledScript::compile(source).expect("scenario victory was admitted"));
        let occlusion = OcclusionGrid::new(&world, setup.map.fog_cell_m);
        let structures = Structures::default();
        let ground = GroundLayer::new(world.width(), world.depth(), &rules.ground);
        let knowledge = std::array::from_fn(|i| {
            SideKnowledge::new(seed ^ OBSERVATION_STREAM ^ i as u64, &ground)
        });
        let mut battle = Battle {
            skirmish: setup
                .skirmish
                .clone()
                .map(|s| crate::skirmish::Skirmish::new(s, &setup.players)),
            authored_props: world.props().count() as PropId,
            shoved: Vec::new(),
            world,
            rules,
            seed,
            tick: 0,
            units,
            sides: std::array::from_fn(|_| SideGeometry::new(std::sync::Arc::clone(&grid))),
            roads,
            planner: RoutePlanner::default(),
            events: events.into(),
            scripts: scripts.into(),
            authored_scripts,
            victory_script,
            script_victory: None,
            script_error: None,
            script_terminal_tick: None,
            script_events: Vec::new(),
            script_trace: Vec::new(),
            replay_script_trace: BTreeMap::new(),
            controllers: BTreeMap::new(),
            player_ids: setup.players.iter().map(|p| p.id.clone()).collect(),
            controller_transitions: Vec::new(),
            replay_controller_transitions: BTreeMap::new(),
            knowledge,
            sightings: Default::default(),
            fired: BTreeSet::new(),
            audible: Default::default(),
            projectiles: Projectiles::new(arsenal.config.clone()),
            arsenal,
            structures,
            windows: garrison::Windows::default(),
            ground,
            rounds: BTreeMap::new(),
            suppressed: BTreeMap::new(),
            combat_rng: Rng::new(seed ^ COMBAT_STREAM),
            damage_rng: Rng::new(seed ^ DAMAGE_STREAM),
            ricochet_rng: Rng::new(seed ^ RICOCHET_STREAM),
            last_soldier: soldier_ids,

            flight_events: Vec::new(),
            segments: Vec::new(),
            blasts: Vec::new(),
            fog: std::array::from_fn(|_| occlusion.field()),
            occlusion,
            next_seq: [1; 3],
            order_bucket: [(
                u64::from(setup.rules.commands.orders_per_s) * u64::from(setup.rules.tick_hz),
                0,
            ); 3],
            player_next_seq: vec![1; setup.players.len()],
            player_order_bucket: vec![
                (
                    u64::from(setup.rules.commands.orders_per_s) * u64::from(setup.rules.tick_hz),
                    0
                );
                setup.players.len()
            ],
            opponent: setup.opponent.clone().map(|o| (o, Defender::default())),
            skirmish_ai: if setup.skirmish.is_some() {
                setup
                    .players
                    .iter()
                    .enumerate()
                    .map(|(index, player)| (player.id.clone(), policy_for_player(seed, index)))
                    .collect()
            } else {
                BTreeMap::new()
            },
            team_memory: std::array::from_fn(|_| crate::skirmish_ai::TeamMemory::default()),
            team_tuning: crate::skirmish_ai::team::TeamTuning::shipped(),
            referee: setup.encounter.clone().map(|e| (e, Referee::default())),
            director: setup.director.clone().map(DirectorRun::new),
            encounter: None,
            pending: Vec::new(),
            accepted: Vec::new(),
            replaying: None,
            seat_events: Vec::new(),
            pending_seat_events: VecDeque::new(),
            observations: Default::default(),
            home_ways: Default::default(),
            refusals: Default::default(),
            shouts: Default::default(),
            shout_before: Vec::new(),
            shout_evidence: ShoutEvidence::default(),
            shout_fired: BTreeSet::new(),
            shout_kills: BTreeSet::new(),
            near_rounds: Default::default(),
            shouted_unseen: Default::default(),
            scenario_digest: scenario_digest(setup),
            config_digest: config_digest(setup),
            fallen: BTreeMap::new(),
            crashes: Vec::new(),
            air_transits: Default::default(),
            ai_receipts: Vec::new(),
            ai_estimates: 0,
            ai_no_way: BTreeMap::new(),
        };

        // A force an authored scenario places in a skirmish counts as its
        // owner's from the start, at its price today: it weighs on the
        // owner's income and unit cap and pays a bounty like a bought unit,
        // though nobody paid for it.
        if let Some(skirmish) = battle.skirmish.as_mut() {
            // A scenario's narrowed shops, by unit type (its resolver checked
            // every name is a card its player's faction buys).
            for (player, offered) in &skirmish.setup.offered.clone() {
                let index = skirmish.player_index(player).expect("an admitted player");
                skirmish.offers[index] = Some(
                    offered
                        .iter()
                        .map(|o| {
                            contract::skirmish::OfferedPurchase::of(&battle.rules.catalog, o)
                                .expect("an admitted card")
                        })
                        .collect(),
                );
            }
            for unit in &battle.units {
                let Some(player) = unit
                    .owner
                    .as_deref()
                    .and_then(|id| skirmish.player_index(id))
                else {
                    continue;
                };
                skirmish.entered.insert(
                    unit.id,
                    crate::skirmish::EnteredUnit {
                        price: battle.rules.catalog.get(unit.kind).cost,
                        entered_tick: 0,
                        player,
                    },
                );
            }
        }

        // Controllers are installed at tick 0 for every player, including a player
        // that has not bought a unit yet. This is the authoritative controller set.
        if setup.skirmish.is_some() {
            for player in &setup.players {
                let transition = ControllerTransition {
                    tick: 0,
                    player: player.id.clone(),
                    controller: player.controller.clone(),
                };
                battle.apply_controller(&transition);
                battle.controller_transitions.push(transition);
            }
        }
        sight::snapshot(&mut battle.units, &battle.arsenal);
        shouts::remember(&mut battle.shout_before, &battle.units, &battle.rules);
        for side in Side::ALL {
            battle.sense(side, true);
            battle.sweep_fog(side);
        }
        battle.observe_all();
        battle
    }
}
