use proc_macro::{Span, TokenStream};
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput};



pub fn derive_buffer_label(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    if !(match &ast.data {
        Data::Struct(s) => s.fields.is_empty(),
        _ => false
    }) {
        return syn::Error::new(Span::call_site().into(), "Only empty structs are supported.")
            .into_compile_error()
            .into()
    }

    let ident = &ast.ident;

    TokenStream::from(quote! {
        impl ::paracosm_gpu::resource::buffer::BufferLabel for #ident { }
        impl ::paracosm_gpu::resource::SyncLabel for #ident { }
    })
}

pub fn derive_image_label(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    if !(match &ast.data {
        Data::Struct(s) => s.fields.is_empty(),
        _ => false
    }) {
        return syn::Error::new(Span::call_site().into(), "Only empty structs are supported.")
            .into_compile_error()
            .into()
    }

    let ident = &ast.ident;

    TokenStream::from(quote! {
        impl ::paracosm_gpu::resource::image::ImageLabel for #ident { }
        impl ::paracosm_gpu::resource::SyncLabel for #ident { }
    })
}

pub fn derive_accel_struct_label(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    if !(match &ast.data {
        Data::Struct(s) => s.fields.is_empty(),
        _ => false
    }) {
        return syn::Error::new(Span::call_site().into(), "Only empty structs are supported.")
            .into_compile_error()
            .into()
    }

    let ident = &ast.ident;

    TokenStream::from(quote! {
        impl ::paracosm_gpu::resource::accel_struct::AccelStructLabel for #ident { }
        impl ::paracosm_gpu::resource::SyncLabel for #ident { }
    })
}

pub fn derive_surface_label(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    if !(match &ast.data {
        Data::Struct(s) => s.fields.is_empty(),
        _ => false
    }) {
        return syn::Error::new(Span::call_site().into(), "Only empty structs are supported.")
            .into_compile_error()
            .into()
    }

    let ident = &ast.ident;

    TokenStream::from(quote! {
        impl ::paracosm_gpu::resource::surface::SurfaceLabel for #ident { }
        impl ::paracosm_gpu::resource::SyncLabel for #ident { }
    })
}

pub fn derive_pipeline_label(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    if !(match &ast.data {
        Data::Struct(s) => s.fields.is_empty(),
        _ => false
    }) {
        return syn::Error::new(Span::call_site().into(), "Only empty structs are supported.")
            .into_compile_error()
            .into()
    }

    let ident = &ast.ident;

    TokenStream::from(quote! {
        impl ::paracosm_gpu::pipeline::PipelineLabel for #ident { }
    })
}
