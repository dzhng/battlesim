#![deny(clippy::disallowed_methods)]
//! The per-frame presentation both clients share: one side's publications
//! in, a [`Frame`] of flat records out. It interpolates between ticks,
//! poses every soldier and vehicle (joint palettes included),
//! makes the combat effects and their cast lights, and chooses by the camera
//! what is drawn and at which tier. It reads only the side's publication, so
//! it never sees past fog of war, and it never touches the simulation, so
//! tuning it never moves a digest or a replay.
pub mod assets;
mod clip;
pub mod effects;
pub mod fielded;
pub mod fog;
pub mod light;
pub mod presentation;
pub mod records;
pub mod scars;
pub mod seed;
pub mod shadow;
pub mod sound;
pub mod static_world;
pub mod units;
pub mod view;
pub mod wave;
pub mod wind;
pub mod world;

pub(crate) mod appearance;
