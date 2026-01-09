mod structs;
mod brickhub;
mod baseplates;

pub const REGION_BASE_WEIGHT: f32 = 1.0;
pub const BIOME_BASE_WEIGHT: f32  = 0.7;
pub const SUBBIOME_BASE_WEIGHT: f32  = 0.25;


pub mod prelude {
    pub use crate::structs::*;
    pub use crate::brickhub::*;
    pub use crate::baseplates::*;
}