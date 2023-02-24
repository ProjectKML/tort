mod compute_pipeline;
mod graphics_pipeline;
mod pipeline_cache;
mod pipeline_layout;
mod shader;
mod shader_module;

use std::hash::Hash;

pub use compute_pipeline::*;
pub use graphics_pipeline::*;
pub use pipeline_cache::*;
pub use pipeline_layout::*;
pub use shader::*;
pub use shader_module::*;
use tort_utils::Uuid;

pub trait Pipeline {
    type Desc: Clone + PartialEq + Eq + Hash + for<'a> From<&'a Self::Desc>;
    type Id: Copy + Clone + PartialEq + Eq + Hash + From<Uuid>;
}
