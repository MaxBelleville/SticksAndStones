//Models and shared values essentially anything that mutiple parts of the project can use
mod math;
mod structs;
//Constants
pub const RAYMARCH_PATH: &str = "shaders/raymarch.wgsl";
pub const RASTERIZE_PATH: &str = "shaders/rasterize.wgsl";
pub const LIGHTING_PATH: &str = "shaders/lighting.wgsl";
pub const PREFILL_PATH: &str = "shaders/prefill_depth.wgsl";

pub mod prelude {
    pub use crate::math::*;
    pub use crate::structs::*;
}
