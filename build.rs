fn main() {
    println!("cargo:rerun-if-changed=assets/app.ico");
    println!("cargo:rerun-if-changed=assets/app.rc");
    println!("cargo:rerun-if-env-changed=MQTT_SHUTDOWN_SELF_CONTAINED");
    println!("cargo:rerun-if-env-changed=CI");

    let _ = embed_resource::compile("assets/app.rc", embed_resource::NONE);

    if self_contained() {
        windows_reactor_setup::as_self_contained();
    } else {
        windows_reactor_setup::as_framework_dependent();
    }
}

fn self_contained() -> bool {
    std::env::var("MQTT_SHUTDOWN_SELF_CONTAINED").is_ok()
        || std::env::var("CI").is_ok_and(|v| v == "true")
}
