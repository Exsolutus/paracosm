pub mod context;
pub mod device;
pub mod queue;
pub mod node;
pub mod pipeline;
pub mod resource;
#[cfg(debug_assertions)] mod validation;


pub mod prelude {
    pub use crate::context::{
        Context,
        ContextInfo,
    };
    pub use crate::queue::{
        Queue,
        commands::{
            CommonCommands as _,
            compute::ComputeCommands as _,
            graphics::GraphicsCommands as _,
            transfer::TransferCommands as _
        }
    };
    pub use crate::node::{
        interface::*,
        resource::{Read, Write}
    };
    pub use crate::pipeline::{
        PipelineLabel,
        PipelineInfo,
        ShaderSource,
        graphics::*
    };
    pub use crate::resource::{
        TransferMode,
        image::{
            Format
        },
    };
    pub use crate::resource::surface::{
        SurfaceLabel,
        PrimarySurface,
        HasSurfaceHandles,
        SurfaceConfig
    };
    pub use paracosm_gpu_macros::*;
}
