mod labels;

use proc_macro::TokenStream;



#[proc_macro_derive(BufferLabel)]
pub fn derive_buffer_label(input: TokenStream) -> TokenStream {
    labels::derive_buffer_label(input)
}

#[proc_macro_derive(ImageLabel)]
pub fn derive_image_label(input: TokenStream) -> TokenStream {
    labels::derive_image_label(input)
}

#[proc_macro_derive(AccelStructLabel)]
pub fn derive_accel_struct_label(input: TokenStream) -> TokenStream {
    labels::derive_accel_struct_label(input)
}

#[proc_macro_derive(SurfaceLabel)]
pub fn derive_surface_label(input: TokenStream) -> TokenStream {
    labels::derive_surface_label(input)
}

#[proc_macro_derive(PipelineLabel)]
pub fn derive_pipeline_label(input: TokenStream) -> TokenStream {
    labels::derive_pipeline_label(input)
}