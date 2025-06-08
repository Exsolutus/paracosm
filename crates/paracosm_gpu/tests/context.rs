use paracosm_gpu::context::*;

#[cfg(not(feature = "WSI"))]
#[test]
fn simplest() {
    let context_info = ContextInfo {
        application_name: "simplest".into(),
        application_version: (0, 0, 0, 0),
        ..Default::default()
    };
    let context = Context::new(context_info);

    assert!(context.is_ok())
}
