use paracosm_gpu::context::*;

#[test]
fn simplest() {
    let context_info = ContextInfo {
        application_name: "simplest".into(),
        application_version: (0, 0, 0, 0),
        ..Default::default()
    };
    let context = Context::new(context_info, None);

    assert!(context.is_ok())
}
