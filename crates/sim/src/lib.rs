//! The one battle authority: world geometry, time, bodies, sensing, knowledge,
//! weapons, flight, damage, orders, deployment and supply.
pub mod arrangement;
pub mod battle;
mod cell_page;
pub mod cover;
pub mod damage;
pub mod deployment;
pub mod digest;
pub mod encounter;
pub mod endurance;
#[cfg(not(target_arch = "wasm32"))]
pub mod fixtures;
pub mod flight;
pub mod formation;
pub mod garrison;
pub mod ground;
pub mod hearing;
pub mod knowledge;
pub mod lean;
pub mod map_analysis;
#[cfg(not(target_arch = "wasm32"))]
pub mod maps;
pub mod math;
pub mod movement;
pub mod navigation;
pub mod objectives;
pub mod publication;
pub mod route_planner;
pub mod sensing;
pub mod settlement;
pub mod sight;
pub mod skirmish;
pub mod skirmish_ai;
pub mod structures;
pub mod supply;
pub mod units;
pub mod village;
pub mod visibility;
pub mod weapons;
pub mod world;

pub mod withdrawal;

pub mod protection;
