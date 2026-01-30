mod structs;
mod brickhub;
mod baseplates;

pub const BIOME_BASE_WEIGHT: f32  = 0.7;
pub const SUBBIOME_BASE_WEIGHT: f32  = 0.25;

pub const SUBBIOME_OFFSET_SCALE: f32 = 256.0;

pub mod prelude {
    pub use crate::structs::*;
    pub use crate::brickhub::*;
    pub use crate::baseplates::*;
}