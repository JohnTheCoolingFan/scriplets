//! Enums for representing data stored in data storages. Takes inspiration from mlua's Value.

use bevy::utils::FloatOrd;
use bevy_mod_scripting::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(untagged)]
pub enum DataValue {
    Nil,
    Boolean(bool),
    Integer(i64),
    #[serde(with = "floatord")]
    Number(FloatOrd),
    String(String),
    Sequence(Vec<DataValue>),
    Table(BTreeMap<DataValue, DataValue>),
}

mod floatord {
    use bevy::utils::FloatOrd;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(val: &FloatOrd, ser: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        val.0.serialize(ser)
    }

    pub fn deserialize<'de, D>(deser: D) -> Result<FloatOrd, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(FloatOrd(f32::deserialize(deser)?))
    }
}

impl Default for DataValue {
    fn default() -> Self {
        Self::Nil
    }
}

impl<'lua> FromLua<'lua> for DataValue {
    fn from_lua(lua_value: LuaValue<'lua>, _lua: &'lua Lua) -> LuaResult<Self> {
        let type_name = lua_value.type_name();
        match lua_value {
            LuaValue::Nil => Ok(Self::Nil),
            LuaValue::Boolean(b) => Ok(Self::Boolean(b)),
            LuaValue::Integer(i) => Ok(Self::Integer(i)),
            LuaValue::Number(n) => Ok(Self::Number(FloatOrd(n as f32))),
            LuaValue::String(s) => Ok(Self::String(s.to_str()?.into())),
            LuaValue::Table(t) => {
                if let Ok(seq) = t
                    .clone()
                    .sequence_values::<DataValue>()
                    .collect::<LuaResult<Vec<DataValue>>>()
                {
                    Ok(Self::Sequence(seq))
                } else {
                    Ok(Self::Table(
                        t.pairs()
                            .collect::<LuaResult<BTreeMap<DataValue, DataValue>>>()?,
                    ))
                }
            }
            _ => Err(LuaError::FromLuaConversionError {
                from: type_name,
                to: "DataValue",
                message: Some("type not supported".into()),
            }),
        }
    }
}

impl<'lua> ToLua<'lua> for DataValue {
    fn to_lua(self, lua: &'lua Lua) -> LuaResult<LuaValue<'lua>> {
        match self {
            Self::Nil => Ok(LuaValue::Nil),
            Self::Boolean(b) => Ok(LuaValue::Boolean(b)),
            Self::Integer(i) => Ok(LuaValue::Integer(i)),
            Self::Number(n) => Ok(LuaValue::Number(n.0 as f64)),
            Self::String(s) => s.to_lua(lua),
            Self::Sequence(seq) => seq.to_lua(lua),
            Self::Table(t) => t.to_lua(lua),
        }
    }
}
