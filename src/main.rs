use bevy::{
    asset::LoadState,
    input::mouse::{MouseMotion, MouseScrollUnit, MouseWheel},
    prelude::*,
    render::camera::ScalingMode,
    time::Stopwatch,
    window::{PresentMode, WindowResolution},
};
use bevy_mod_scripting::prelude::*;
use bevy_rapier2d::prelude::*;
use prototypes::{ComponentPrototype, Movement, MovementType, Prototypes, PrototypesLoader};

use std::{
    f32::consts::PI,
    sync::{Arc, Mutex},
};

mod data_value;
mod program;
mod prototypes;

use program::UnitHandle;

const CLEAR_COLOR: Color = Color::rgb(0.1, 0.1, 0.1);
const ASPECT_RATIO: f32 = 16.0 / 9.0;
const WINDOW_HEIGHT: f32 = 900.0;

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

#[derive(Debug, Clone, PartialEq, Eq, Hash, States, Default)]
enum AppState {
    #[default]
    Loading,
    Playing,
}

#[derive(Component)]
pub struct Unit;

#[derive(Component)]
pub struct UnitClock(Stopwatch);

#[derive(Resource, Default)]
pub struct GameClock(Stopwatch);

#[derive(Resource)]
pub struct UnitSprite(Handle<Image>);
#[derive(Resource)]
pub struct WallSprite(Handle<Image>);
#[derive(Resource)]
pub struct PrototypesHandle(Handle<Prototypes>);

#[derive(Debug, Clone)]
pub struct UnitLuaAPIProvider;

impl APIProvider for UnitLuaAPIProvider {
    type APITarget = Mutex<Lua>;
    type ScriptContext = Mutex<Lua>;
    type DocTarget = LuaDocFragment;

    fn attach_api(&mut self, api: &mut Self::APITarget) -> Result<(), ScriptError> {
        let lua = api.get_mut().map_err(ScriptError::new_other)?;
        let globals = lua.globals();
        globals
            .set(
                "print",
                lua.create_function(|_, s: String| Ok(println!("{}", s)))
                    .map_err(ScriptError::new_other)?,
            )
            .map_err(ScriptError::new_other)?;
        Ok(())
    }

    fn setup_script_runtime(
        &mut self,
        world_ptr: bevy_mod_scripting::core::world::WorldPointer,
        script_data: &ScriptData,
        ctx: &mut Self::ScriptContext,
    ) -> Result<(), ScriptError> {
        let lua = ctx.get_mut().map_err(ScriptError::new_other)?;
        let globals = lua.globals();
        let entity = script_data.entity;
        let world_ptr_arc = Arc::new(world_ptr);
        let world_ptr = world_ptr_arc.clone();
        let move_fn = lua
            .create_function(move |_, args: (f32, f32)| {
                let mut world = world_ptr.write();
                let mut entity_mut = world.entity_mut(entity);
                if let Some(mut movement) = entity_mut.get_mut::<Movement>() {
                    movement.input_move = Vec2::from(args);
                }
                Ok(())
            })
            .map_err(ScriptError::new_other)?;
        let world_ptr = world_ptr_arc.clone();
        let rotate_fn = lua
            .create_function(move |_, rot: f32| {
                let mut world = world_ptr.write();
                let mut entity_mut = world.entity_mut(entity);
                if let Some(mut movement) = entity_mut.get_mut::<Movement>() {
                    movement.input_rotation = rot;
                }
                Ok(())
            })
            .map_err(ScriptError::new_other)?;
        globals
            .set("unit_move", move_fn)
            .map_err(ScriptError::new_other)?;
        globals
            .set("unit_rotate", rotate_fn)
            .map_err(ScriptError::new_other)?;
        Ok(())
    }
}

fn spawn_camera(mut commands: Commands) {
    let mut camera = Camera2dBundle::default();

    /*
    camera.projection.top = 1.0;
    camera.projection.bottom = -1.0;
    camera.projection.right = 1.0 * ASPECT_RATIO;
    camera.projection.left = -1.0 * ASPECT_RATIO;
    */

    camera.projection.scaling_mode = ScalingMode::Fixed {
        height: 2.0,
        width: 2.0 * ASPECT_RATIO,
    };

    commands.spawn(camera);
}

fn move_and_zoom_camera(
    mut camera: Query<(&mut OrthographicProjection, &mut Transform), With<Camera2d>>,
    input: Res<Input<MouseButton>>,
    mut mouse_scroll_evr: EventReader<MouseWheel>,
    mut mouse_move_evr: EventReader<MouseMotion>,
) {
    let (mut camera, mut camera_transform) = camera.single_mut();
    for scroll_event in mouse_scroll_evr.iter() {
        match scroll_event.unit {
            MouseScrollUnit::Line => {
                camera.scale = (camera.scale - 0.5 * scroll_event.y).clamp(1.0, 20.0)
            }
            MouseScrollUnit::Pixel => {
                camera.scale = (camera.scale - 0.1 * scroll_event.y).clamp(1.0, 20.0)
            }
        }
    }
    for move_event in mouse_move_evr.iter() {
        if input.pressed(MouseButton::Middle) {
            let mut delta = move_event.delta * 0.0025 * camera.scale;
            delta.x = -delta.x;
            camera_transform.translation += delta.extend(0.0);
        }
    }
}

fn spawn_unit(
    mut commands: Commands,
    unit_sprite: Res<UnitSprite>,
    prototypes_handle: Res<PrototypesHandle>,
    prototypes_assets: Res<Assets<Prototypes>>,
    mut lua_files: ResMut<Assets<LuaFile>>,
) {
    let component_prototypes = prototypes_assets.get(&prototypes_handle.0).unwrap();

    let unit_program_file = lua_files.add(LuaFile {
        bytes: r#"
        function on_tick(handle)
            unit_move(1, 1)
        end
        "#
        .as_bytes()
        .into(),
    });
    let unit_script = Script::<LuaFile>::new("unit_default".into(), unit_program_file);
    let movement = Movement::component_from_pt(component_prototypes, "default").unwrap();
    commands.spawn((
        Unit,
        UnitClock(Stopwatch::default()),
        movement,
        ScriptCollection {
            scripts: vec![unit_script],
        },
        Collider::cuboid(0.499, 0.499),
        RigidBody::KinematicPositionBased,
        SpriteBundle {
            texture: unit_sprite.0.clone(),
            sprite: Sprite {
                custom_size: Some(Vec2::splat(1.0)),
                ..default()
            },
            ..default()
        },
    ));
}

fn spawn_walls(mut commands: Commands, wall_sprite: Res<WallSprite>) {
    for i in 1..=5 {
        spawn_wall(&mut commands, i as f32, 5.0, &wall_sprite.0)
    }
    for j in 0..=4 {
        spawn_wall(&mut commands, 5.0, j as f32, &wall_sprite.0)
    }
    spawn_wall(&mut commands, -1.0, 5.0, &wall_sprite.0)
}

fn spawn_wall(commands: &mut Commands, x: f32, y: f32, sprite: &Handle<Image>) {
    let transform = TransformBundle::from(Transform::from_xyz(x, y, 0.0));
    commands.spawn((
        Collider::cuboid(0.5, 0.5),
        RigidBody::Fixed,
        SpriteBundle {
            texture: sprite.clone(),
            transform: transform.local,
            global_transform: transform.global,
            sprite: Sprite {
                custom_size: Some(Vec2::splat(1.0)),
                ..default()
            },
            ..default()
        },
    ));
}

fn handle_movement(
    mut units: Query<(Entity, &mut Movement, &mut Transform, &Collider), With<Unit>>,
    rapier_context: Res<RapierContext>,
) {
    for (entity, mut movement, mut transform, collider) in units.iter_mut() {
        match movement.movement_type {
            MovementType::Omnidirectional => {
                if !movement.hand_brake {
                    if movement.input_rotation != 0.0 {
                        let rotation = Quat::from_rotation_z(
                            -(movement.rotation_speed
                                * movement.input_rotation.clamp(-1.0, 1.0)
                                * PI)
                                / (180.0 * 60.0),
                        );
                        transform.rotation *= rotation;
                    }
                    if movement.input_move != Vec2::ZERO {
                        let unrotated_move =
                            movement.input_move.clamp_length_max(1.0) * (movement.speed / 60.0);
                        let delta = unrotated_move.rotate(transform.right().truncate());
                        let shape_pos = transform.translation.truncate();
                        let shape_rot = transform.rotation.to_euler(EulerRot::XYZ).2;
                        let max_toi = 1.0;
                        let filter = QueryFilter::default()
                            .exclude_collider(entity)
                            .exclude_sensors();
                        if rapier_context
                            .cast_shape(shape_pos, shape_rot, delta, collider, max_toi, filter)
                            .is_none()
                        {
                            transform.translation += delta.extend(0.0);
                        }
                        movement.input_move = Vec2::ZERO;
                    }
                }
            }
            MovementType::AcceleratedSteering => {
                let input_move_vec = movement
                    .input_move
                    .clamp(Vec2::NEG_X + Vec2::NEG_Y, Vec2::X + Vec2::Y);
                let max_speed = movement.max_speed;
                let max_speed_backwards = -movement.max_speed_backwards.unwrap_or(max_speed);
                let acceleration = movement.acceleration;
                let braking_acceleration = -movement.braking_acceleration.unwrap_or(acceleration);
                let passive_deceleration = movement.passive_deceleration;
                let is_moving_forward = movement.speed > 0.0;
                let is_moving_backwards = movement.speed < 0.0;
                let new_speed = {
                    let acceleration = {
                        if movement.hand_brake {
                            if movement.speed > 0.0 {
                                braking_acceleration
                            } else {
                                -braking_acceleration
                            }
                        } else if (movement.speed > 0.0 && input_move_vec.x > 0.0)
                            || (movement.speed < 0.0 && input_move_vec.x < 0.0)
                        {
                            acceleration
                        } else if (movement.speed > 0.0 && input_move_vec.x < 0.0)
                            || (movement.speed < 0.0 && input_move_vec.x > 0.0)
                        {
                            braking_acceleration
                        } else if movement.speed != 0.0 {
                            -passive_deceleration
                        } else {
                            acceleration
                        }
                    };
                    let new_speed_uncapped = (movement.speed
                        + acceleration * input_move_vec.x / 60.0)
                        .clamp(max_speed_backwards, max_speed);
                    if is_moving_forward {
                        new_speed_uncapped.clamp(0.0, f32::MAX)
                    } else if is_moving_backwards {
                        new_speed_uncapped.clamp(f32::MIN, 0.0)
                    } else {
                        new_speed_uncapped
                    }
                };
                movement.speed = new_speed;
                if movement.speed != 0.0 {
                    let linear_delta = movement.speed / 60.0;
                    let starting_translation = transform.translation.truncate()
                        + transform.up().truncate() * movement.rotation_offset;
                    let mut rot_angle =
                        (movement.rotation_speed * PI / (60.0 * 180.0)) * input_move_vec.y;
                    if movement.speed < 0.0 {
                        rot_angle = -rot_angle;
                    }
                    let result_rotation = transform.rotation * Quat::from_rotation_z(-rot_angle);
                    let turning_scale = linear_delta / rot_angle;
                    let rot_vec_normalized = Vec2::from_angle(rot_angle);
                    let turning_radius = transform.right().truncate()
                        + transform.up().truncate() * movement.rotation_offset * turning_scale;
                    let turning_origin = starting_translation - turning_radius;
                    let result_translation = turning_radius.rotate(rot_vec_normalized)
                        + turning_origin
                        - transform.up().truncate() * movement.rotation_offset;

                    let delta = result_translation - starting_translation;
                    let shape_pos = result_translation;
                    let shape_rot = result_rotation.to_euler(EulerRot::XYZ).2;
                    let max_toi = 1.0;
                    let filter = QueryFilter::default()
                        .exclude_collider(entity)
                        .exclude_sensors();
                    if rapier_context
                        .cast_shape(shape_pos, shape_rot, delta, collider, max_toi, filter)
                        .is_none()
                    {
                        transform.translation = result_translation.extend(0.0);
                        transform.rotation = result_rotation;
                    }
                    movement.input_move = Vec2::ZERO
                }
            }
            _ => {}
        }
    }
}

fn unit_tick(mut ew: PriorityEventWriter<LuaEvent<mlua::Variadic<usize>>>) {
    ew.send(
        LuaEvent {
            hook_name: "on_tick".to_string(),
            args: mlua::Variadic::from_iter([0]),
            recipients: Recipients::All,
        },
        0,
    )
}

fn tick_units_clocks(mut units: Query<&mut UnitClock, With<Unit>>, time: Res<Time>) {
    units.iter_mut().for_each(|mut unit| {
        unit.0.tick(time.delta());
    })
}

fn game_clock_tick(mut clock: ResMut<GameClock>, time: Res<Time>) {
    clock.0.tick(time.delta());
}

fn print_units_positions(units: Query<&Transform, With<Unit>>) {
    for (i, unit) in units.iter().enumerate() {
        println!(
            "Unit #{}: x {}, y {}",
            i, unit.translation.x, unit.translation.y
        )
    }
}

fn load_assets(mut commands: Commands, assets: Res<AssetServer>) {
    let unit_sprite = assets.load("unit.png");
    commands.insert_resource(UnitSprite(unit_sprite));
    let wall_sprite = assets.load("wall.png");
    commands.insert_resource(WallSprite(wall_sprite));
    let prototypes = assets.load("prototypes.json");
    commands.insert_resource(PrototypesHandle(prototypes))
}

fn check_load_assets(
    mut next_state: ResMut<NextState<AppState>>,
    unit: Res<UnitSprite>,
    wall: Res<WallSprite>,
    prototypes: Res<PrototypesHandle>,
    asset_server: Res<AssetServer>,
) {
    if let LoadState::Loaded =
        asset_server.get_group_load_state([unit.0.id(), wall.0.id(), prototypes.0.id()])
    {
        next_state.set(AppState::Playing);
    }
}

fn main() {
    App::new()
        .insert_resource(ClearColor(CLEAR_COLOR))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Scriplets".to_string(),
                present_mode: PresentMode::Fifo,
                resolution: WindowResolution::new(WINDOW_HEIGHT * ASPECT_RATIO, WINDOW_HEIGHT),
                resizable: false,
                ..default()
            }),
            ..default()
        }))
        .add_plugin(RapierDebugRenderPlugin {
            enabled: cfg!(debug_assertions),
            ..default()
        }) // Reminder: disable when building debug
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(32.0))
        .add_plugin(ScriptingPlugin)
        .add_script_host_to_base_set::<LuaScriptHost<mlua::Variadic<usize>>, _>(CoreSet::PostUpdate)
        .add_asset::<Prototypes>()
        .init_asset_loader::<PrototypesLoader>()
        .add_state::<AppState>()
        .init_resource::<GameClock>()
        .add_system(load_assets.in_schedule(OnEnter(AppState::Loading)))
        .add_system(check_load_assets.in_set(OnUpdate(AppState::Loading)))
        .add_system(spawn_walls.in_schedule(OnEnter(AppState::Playing)))
        .add_system(spawn_unit.in_schedule(OnEnter(AppState::Playing)))
        .add_system(spawn_camera.in_schedule(OnEnter(AppState::Playing)))
        .add_system(tick_units_clocks.before(unit_tick))
        .add_system(unit_tick.in_base_set(CoreSet::Update))
        .add_script_handler_to_base_set::<LuaScriptHost<mlua::Variadic<usize>>, _, 0, 0>(
            CoreSet::PostUpdate,
        )
        .add_api_provider::<LuaScriptHost<mlua::Variadic<usize>>>(Box::new(UnitLuaAPIProvider))
        .add_system(
            print_units_positions
                .in_set(OnUpdate(AppState::Playing))
                .run_if(|| cfg!(debug_assertions)),
        )
        .add_system(game_clock_tick.in_set(OnUpdate(AppState::Playing)))
        .add_system(handle_movement.in_set(OnUpdate(AppState::Playing)))
        .add_system(move_and_zoom_camera.in_set(OnUpdate(AppState::Playing)))
        .run()
}
