use bevy::{
    asset::LoadState,
    input::mouse::{MouseMotion, MouseScrollUnit, MouseWheel},
    prelude::*,
    render::camera::ScalingMode,
    time::Stopwatch,
    window::PresentMode,
};
use bevy_rapier2d::prelude::*;

const CLEAR_COLOR: Color = Color::rgb(0.1, 0.1, 0.1);
const RESOLUTION: f32 = 16.0 / 9.0;

pub struct UnitSprite(pub Handle<Image>);
pub struct WallSprite(pub Handle<Image>);

fn main() {
    println!("Hello, world! This is a Scriplets client");
}
