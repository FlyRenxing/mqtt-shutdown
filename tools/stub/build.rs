use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let payload = env::var("MQTT_SHUTDOWN_PAYLOAD").unwrap_or_default();
    let dest = out.join("payload.tar");
    if payload.is_empty() {
        fs::write(&dest, []).expect("write empty payload");
    } else {
        println!("cargo:rerun-if-changed={payload}");
        fs::copy(&payload, &dest).unwrap_or_else(|err| {
            panic!("copy payload {payload} -> {}: {err}", dest.display())
        });
    }

    let rc = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("..")
        .join("..")
        .join("assets")
        .join("app.rc");
    if rc.is_file() {
        embed_resource::compile(rc, embed_resource::NONE);
    }
}
