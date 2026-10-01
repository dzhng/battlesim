//! The simulation's integration tests, one module per file, linked as one
//! binary so every test runs on one pool (Cargo runs test binaries one at a
//! time). One file: `cargo test -p sim --test sim village::`.
mod common;

mod animation_feed;
mod battle_authority;
mod bodies;
mod buildings;
mod catalog;
mod contacts;
mod cover;
mod damage;
mod deployment;
mod destruction;
mod drive;
mod encounter;
mod endurance;
mod fire_through;
mod flight_ballistics;
mod flight_collision;
mod flight_load;
mod forest;
mod formation;
mod garrison;
mod ground;
mod ground_delivery;
mod guidance;
mod lean;
mod maps;
mod movement;
mod movement_scenarios;
mod navigation;
mod order_markers;
mod publication;
mod ricochet;
mod rivers;
mod road_journeys;
mod route_planning;
mod sensing;
mod sight;
mod soldier_bodies;
mod supply;
mod suppression;
mod village;
mod weapons;
mod world_geometry;
