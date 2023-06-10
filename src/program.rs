use super::{GameClock, Movement, UnitClock};
use bevy::prelude::*;
use bevy_mod_scripting::prelude::*;
use std::f32::consts::PI;

pub struct UnitHandle<'a> {
    pub movement: Option<&'a mut Movement>,
    pub transform: &'a Transform,
    pub clock: &'a UnitClock,
    pub game_clock: &'a GameClock,
}

pub struct LuaUnitHandle<'a> {
    handle: UnitHandle<'a>,
}

// TODO: after making a planet map, methods for getting nearest transition tile or a tile adjacent
//  to transition tile
impl LuaUserData for LuaUnitHandle<'_> {
    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method_mut("move", |_lua, lua_handle, args: (f32, f32)| {
            if let Some(movement) = &mut lua_handle.handle.movement {
                movement.input_move = Vec2::from(args);
            };
            Ok(())
        });
        methods.add_method_mut("rotate", |_lua, lua_handle, rot: f32| {
            if let Some(movement) = &mut lua_handle.handle.movement {
                movement.input_rotation = rot;
            }
            Ok(())
        });
        methods.add_method_mut("toggle_hand_brake", |_lua, lua_handle, ()| {
            if let Some(movement) = &mut lua_handle.handle.movement {
                movement.hand_brake = !movement.hand_brake;
            }
            Ok(())
        })
    }

    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("time_since_start", |_lua, lua_handle| {
            Ok(lua_handle.handle.clock.0.elapsed_secs())
        });
        fields.add_field_method_get("global_time", |_lua, lua_handle| {
            Ok(lua_handle.handle.game_clock.0.elapsed_secs())
        });
        fields.add_field_method_get("gps", |lua, lua_handle| {
            let position: [f32; 2] = lua_handle.handle.transform.translation.truncate().into();
            let rotation_radians = lua_handle
                .handle
                .transform
                .rotation
                .to_euler(EulerRot::XYZ)
                .2;
            let rotation_degrees = -(rotation_radians * 180.0) / PI;
            let table = lua.create_table()?;
            table.set("position", position)?;
            table.set("rotation", rotation_degrees)?;
            Ok(table)
        });
        fields.add_field_method_get("movement", |lua, lua_handle| {
            if let Some(movement) = &lua_handle.handle.movement {
                let movement_type = movement.movement_type.as_ref();
                let speed = movement.speed;
                let max_speed = movement.max_speed;
                let max_speed_backwards = movement.max_speed_backwards;
                let acceleration = movement.acceleration;
                let braking_acceleration = movement.acceleration;
                let passive_deceleration = movement.passive_deceleration;
                let rotation_speed = movement.rotation_speed;
                let hand_brake = movement.hand_brake;
                let table = lua.create_table()?;
                table.set("movement_type", movement_type)?;
                table.set("speed", speed)?;
                table.set("max_speed", max_speed)?;
                table.set("max_speed_backwards", max_speed_backwards)?;
                table.set("acceleration", acceleration)?;
                table.set("braking_acceleration", braking_acceleration)?;
                table.set("passive_deceleration", passive_deceleration)?;
                table.set("rotation_speed", rotation_speed)?;
                table.set("is_hand_brake_pulled", hand_brake)?;
                Ok(LuaValue::Table(table))
            } else {
                Ok(LuaValue::Nil)
            }
        })
    }
}
