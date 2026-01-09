//Models and shared values essentially anything that mutiple parts of the project can use
mod math;
mod structs;

//Constants
pub const RAYMARCH_PATH: &str = "shaders/raymarch.wgsl";
pub const RASTERIZE_PATH: &str = "shaders/rasterize.wgsl";
pub const LIGHTING_PATH: &str = "shaders/lighting.wgsl";
pub const PREFILL_PATH: &str = "shaders/prefill_depth.wgsl";

pub trait IterableRef {
    type Item;
    fn iter(self) -> std::vec::IntoIter<(&'static str, Self::Item)>;
    fn iter_val(self) -> std::vec::IntoIter<Self::Item>;
}

pub trait IterableMut {
    type Item;
    fn iter_mut(self) -> std::vec::IntoIter<(&'static str, Self::Item)>;
    fn iter_mut_val(self) -> std::vec::IntoIter<Self::Item>;
}

pub mod prelude {
    pub use crate::IterableRef;
    pub use crate::IterableMut;
    pub use crate::math::*;
    pub use crate::structs::*;
}
