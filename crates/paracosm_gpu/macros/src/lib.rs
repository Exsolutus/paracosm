mod frame_graph;

use proc_macro::TokenStream;



#[proc_macro_derive(BufferLabel)]
pub fn derive_buffer_label(input: TokenStream) -> TokenStream {
    frame_graph::derive_buffer_label(input)
}

#[proc_macro_derive(ImageLabel)]
pub fn derive_image_label(input: TokenStream) -> TokenStream {
    frame_graph::derive_image_label(input)
}

#[proc_macro_derive(AccelStructLabel)]
pub fn derive_accel_struct_label(input: TokenStream) -> TokenStream {
    frame_graph::derive_accel_struct_label(input)
}

#[proc_macro_derive(PipelineLabel)]
pub fn derive_pipeline_label(input: TokenStream) -> TokenStream {
    frame_graph::derive_pipeline_label(input)
}
