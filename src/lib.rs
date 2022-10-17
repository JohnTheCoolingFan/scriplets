use bevy::{
    prelude::*,
    time::Stopwatch,
};
use prototypes::{Movement, Prototypes};

pub mod data_value;
pub mod program;
pub mod prototypes;

// General TODO list
// - split into client and server
// - code editing gui

// General ideas
//  Black box: a component that can store data when unit is running and extracted from a unit
//  corpse as an item and be read by other units.
//
//  Items
//  Units with manipulators specify an area that they want to pick up from. They are given a list
//  of what can be picked up and then they choose what is picked up
//
//  Items with data
//  Similar to black box, can have data written and read. Can be encrypted. No actual encryption
//  will be done, just comparing the keys.
//
//  Possible new language: wasm

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AppState {
    Loading,
    Playing,
}

#[derive(Component)]
pub struct Unit;

#[derive(Component)]
pub struct UnitClock(pub Stopwatch);

pub struct GameClock(pub Stopwatch);

pub struct PrototypesHandle(pub Handle<Prototypes>);
