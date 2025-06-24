pub mod context;
pub mod device;
pub mod queue;
pub mod node;
pub mod pipeline;
pub mod resource;
#[cfg(feature = "WSI")]pub mod surface;
#[cfg(debug_assertions)] mod validation;


pub mod prelude {
    pub use crate::context::{
        Context,
        ContextInfo,
    };
    pub use crate::queue::{
        Queue,
        SubmitInfo,
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
    pub use crate::pipeline::PipelineLabel;
    pub use crate::resource::{
        BufferLabel,
        ImageLabel,
        AccelStructLabel
    };
    #[cfg(feature = "WSI")]
    pub use crate::surface::{
        HasSurfaceHandles,
        SurfaceConfig
    };
    pub use paracosm_gpu_macros::*;
}
