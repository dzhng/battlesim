//! An own unit's order step: its kind (indexing stepKinds), its point (NaN
//! when the side knows none), whether a move is an attack-move, the target
//! an attack names and the host a step enters.
use contract::observation::{OrderStep, StepHost};

use super::cells::{host_cells, target_cells};
use super::tags::{tag, STEP_KINDS};

pub(super) const STEP_FIELDS: [&str; 12] = [
    "kind",
    "x",
    "y",
    "attackMove",
    "targetKind",
    "targetId",
    "targetX",
    "targetY",
    "targetZ",
    "hostKind",
    "hostLo",
    "hostHi",
];

pub(super) fn step_row(step: &OrderStep) -> [f32; 12] {
    let (kind, point, target, host) = match *step {
        OrderStep::Move { point, .. } => ("move", Some(point), None, None),
        OrderStep::Attack { target, point } => ("attack", point, target, None),
        OrderStep::Garrison { point, building } => {
            ("garrison", Some(point), None, Some(("building", building)))
        }
        OrderStep::Board { point, carrier } => {
            ("board", Some(point), None, Some(("carrier", carrier.0)))
        }
        OrderStep::Collect { point } => ("collect", point, None, None),
        OrderStep::Unload { point, into } => (
            "unload",
            Some(point),
            None,
            into.map(|h| match h {
                StepHost::Building { id } => ("building", id),
                StepHost::Carrier { id } => ("carrier", id.0),
            }),
        ),
        OrderStep::Exit { point } => ("exit", Some(point), None, None),
        OrderStep::ReturnToBase { point } => ("return_to_base", Some(point), None, None),
    };
    let [x, y] = point.map_or([f32::NAN; 2], |p| [p[0] as f32, p[1] as f32]);
    let attack_move = matches!(
        step,
        OrderStep::Move {
            attack_move: true,
            ..
        }
    );
    let [tk, ti, tx, ty, tz] = target_cells(target);
    let [hk, hl, hh] = host_cells(host);
    [
        tag(&STEP_KINDS, &kind),
        x,
        y,
        attack_move as u8 as f32,
        tk,
        ti,
        tx,
        ty,
        tz,
        hk,
        hl,
        hh,
    ]
}
