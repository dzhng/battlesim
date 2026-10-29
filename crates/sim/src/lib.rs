#![deny(clippy::disallowed_methods)]
#![recursion_limit = "256"]
//! The one battle authority: world geometry, time, bodies, sensing, knowledge,
//! weapons, flight, damage, orders, deployment and supply.
pub mod air;
pub mod arrangement;
pub mod bail_out;
pub mod battle;
pub mod carriage;
mod cell_page;
pub mod cover;
pub mod crash;
pub mod damage;
pub mod deployment;
pub mod digest;
pub mod encounter;
pub mod endurance;
pub mod entries;
#[cfg(not(target_arch = "wasm32"))]
pub mod fixtures;
pub mod flight;
pub mod formation;
pub mod garrison;
pub mod ground;
pub mod hearing;
pub mod hold;
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
pub mod shouts;
pub mod sight;
pub mod skirmish;
pub mod skirmish_ai;
pub mod structures;
pub mod supply;
pub mod units;
pub mod visibility;
pub mod weapons;
pub mod world;

pub mod withdrawal;

pub mod protection;
