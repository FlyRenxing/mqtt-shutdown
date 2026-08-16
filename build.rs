fn main() {
    println!("cargo:rerun-if-changed=assets/app.ico");
    println!("cargo:rerun-if-changed=assets/app.rc");
    println!("cargo:rerun-if-env-changed=MQTT_SHUTDOWN_SELF_CONTAINED");

    let _ = embed_resource::compile("assets/app.rc", embed_resource::NONE);

    // Always copy microsoft.windowsappruntime.bootstrap.dll next to the exe.
    // The reactor crate statically imports it, even for self-contained apps.
    windows_reactor_setup::as_framework_dependent();
    if std::env::var("MQTT_SHUTDOWN_SELF_CONTAINED").is_ok() {
        windows_reactor_setup::as_self_contained();
    }
}
