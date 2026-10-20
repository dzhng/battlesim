//! Building a unit from its setup: its soldiers, mounts and starting state.

use super::*;

/// Physical construction is shared by authored placements and reinforcements.
pub(super) fn spawn_unit(
    i: usize,
    u: &contract::scenario::UnitSetup,
    world: &WorldGeometry,
    rules: &Rules,
    arsenal: &Arsenal,
    soldier_ids: &mut u32,
    seed: u64,
) -> Unit {
    let kind = rules
        .catalog
        .index(&u.kind)
        .unwrap_or_else(|| panic!("unit {i} names unknown unit type {:?}", u.kind));
    let t = rules.catalog.get(kind);
    let xy = v2(u.position[0], u.position[1]);
    // A squad starts spread out like any squad that has just
    // arrived: a seeded arrangement around its position.
    let slots = t.slots().unwrap_or_default();
    let count = slots.len();
    let solid = |p: &crate::world::Prop| p.blocks(MoverClass::Infantry);
    let mut draws = arrangement::rng(seed, i as u32, 0);
    let members: Vec<Soldier> = arrangement::squad_spots(
        world,
        xy,
        count,
        &rules.infantry_movement,
        rules.physics.soldier_radius_m,
        &solid,
        &mut draws,
    )
    .into_iter()
    .enumerate()
    .map(|(slot, p)| {
        *soldier_ids += 1;
        let z = world.surface_at(p.x, p.y).map_or(0.0, |s| s.z);
        let hp = rules.catalog.soldier(&slots[slot]).hp;
        Soldier::new(*soldier_ids, slot, p.with_z(z), hp)
    })
    .collect();
    let mounts = arsenal.mounts_for(kind, u.yaw, &members);
    // An aircraft enters at cruise height over the ground beneath it.
    let motion = units::motion(t, rules);
    let airborne = matches!(motion, units::Motion::Air(_));
    let lift = if airborne {
        rules.air.cruise_agl_m(&t.mobility)
    } else {
        0.0
    };
    Unit {
        id: UnitId(i as u32),
        side: u.side,
        owner: u.owner.clone(),
        kind,
        position: xy.with_z(world.surface_at(xy.x, xy.y).map_or(0.0, |s| s.z) + lift),
        yaw: u.yaw,
        motion,
        hull: t.hull().map(|h| {
            let [x, y, z] = h.half_extents_m;
            crate::math::v3(x, y, z)
        }),
        members,
        orders: VecDeque::new(),
        route: None,
        route_from: xy,
        state: MoveState::Idle,
        blocker: None,
        planned_revision: 0,
        progress: (f64::INFINITY, 0),
        stalls: (f64::INFINITY, 0),
        pursuit: None,
        planned_goal: None,
        engagement: controllers::own_engagement(u.side, u.engagement),
        held_engagement: None,
        air: airborne.then(|| {
            let fuel_s = t
                .capabilities
                .air
                .as_ref()
                .map_or(f64::INFINITY, |air| air.fuel_capacity_s);
            units::AirState {
                fuel_s,
                ..Default::default()
            }
        }),
        mounts,
        attackers: BTreeSet::new(),
        reach: Default::default(),
        hp: t.hull().map_or(0.0, |h| h.hp),
        retired: false,
        withdrawing: false,
        suppression: 0.0,
        suppressed_at: 0,
        deployment: deployment::initial(t, rules),
        hold: None,
        stock: t.capabilities.supply.map(|s| u.stock.unwrap_or(s.stock)),
        protection: t
            .capabilities
            .active_protection
            .as_ref()
            .map(crate::protection::State::new),
        flares: t.capabilities.flares.map(|f| units::Flares {
            charges: f.capacity,
            ready_tick: 0,
        }),
        progress_service: Default::default(),
        service: ServiceStatus::OutOfRange,
        sight_forward: u.yaw,
        cover: Default::default(),
        // A squad placed by the scenario holds round where it was put.
        anchor: t.hull().is_none().then_some(crate::cover::Anchor {
            at: xy,
            halt: false,
        }),
        manoeuvre: None,
        reversing: false,
        drive_speed_mps: 0.0,
        turn_to: None,
    }
}
